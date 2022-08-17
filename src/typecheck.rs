// Querybinder -- Generate boilerplate from SQL for statically typed languages
// Copyright 2022 Ruud van Asseldonk

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// A copy of the License has been included in the root of the repository.

//! The "typecheck" phase.
//!
//! There is not much to really typecheck, but we do have to resolve some
//! references and perform a few consistency checks. For example, every
//! parameter should be listed in the function signature, or it needs to have a
//! type annotation. For lack of a better word, we call this the "typecheck"
//! phase.

use std::collections::hash_map::{Entry, HashMap};
use std::collections::hash_set::HashSet;

use crate::ast::{Annotation, Document, Fragment, PrimitiveType, Query, Section, Type, TypedIdent};
use crate::error::{TResult, TypeError};
use crate::Span;

fn resolve_type(input: &str, type_: Type<Span>) -> TResult<Type<Span>> {
    match type_ {
        Type::Unit => Ok(type_),
        Type::Primitive(..) => unreachable!("We don't have primitive types yet at this stage."),
        Type::Iterator(s, inner) => Ok(Type::Iterator(s, Box::new(resolve_type(input, *inner)?))),
        Type::Option(s, inner) => Ok(Type::Option(s, Box::new(resolve_type(input, *inner)?))),
        Type::Tuple(s, ts) => {
            let resolved: Vec<_> = ts
                .into_iter()
                .map(|t| resolve_type(input, t))
                .collect::<TResult<Vec<_>>>()?;
            Ok(Type::Tuple(s, resolved))
        }
        Type::Struct(..) => unreachable!("We don't have struct types yet at this stage."),
        Type::Simple(span) => {
            match span.resolve(input) {
                "str" => Ok(Type::Primitive(span, PrimitiveType::Str)),
                "i32" => Ok(Type::Primitive(span, PrimitiveType::I32)),
                "i64" => Ok(Type::Primitive(span, PrimitiveType::I64)),
                "bytes" => Ok(Type::Primitive(span, PrimitiveType::Bytes)),
                other
                    if other
                        .bytes()
                        .next()
                        .map(|ch| ch.is_ascii_uppercase())
                        .unwrap_or(false) =>
                {
                    // If it starts with an uppercase letter, then we assume
                    // it's a struct.
                    Ok(Type::Struct(span, Vec::new()))
                }
                _other => {
                    // If it doesn't start with an uppercase letter though and
                    // we also didn't resolve it to a primitive type already,
                    // then we don't know what to do and report an error.
                    let err = TypeError::with_hint(
                        span,
                        "Unknown type.",
                        "User-defined types should start with an uppercase letter.",
                    );
                    Err(err)
                }
            }
        }
    }
}

fn resolve_annotation(input: &str, ann: Annotation<Span>) -> TResult<Annotation<Span>> {
    let mut parameters = Vec::with_capacity(ann.parameters.len());

    for param in ann.parameters {
        parameters.push(TypedIdent {
            type_: resolve_type(input, param.type_)?,
            ..param
        });
    }

    let result_type = resolve_type(input, ann.result_type)?;

    let result = Annotation {
        parameters: parameters,
        result_type: result_type,
        ..ann
    };

    Ok(result)
}

fn resolve_fragments(input: &str, fragments: Vec<Fragment<Span>>) -> TResult<Vec<Fragment<Span>>> {
    let mut result = Vec::with_capacity(fragments.len());

    for fragment in fragments {
        let new_fragment = match fragment {
            Fragment::Verbatim(_) => fragment,
            Fragment::TypedIdent(span, ti) => Fragment::TypedIdent(
                span,
                TypedIdent {
                    type_: resolve_type(input, ti.type_)?,
                    ident: ti.ident,
                },
            ),
            Fragment::Param(_) => fragment,
            Fragment::TypedParam(span, ti) => Fragment::TypedParam(
                span,
                TypedIdent {
                    type_: resolve_type(input, ti.type_)?,
                    ident: ti.ident,
                },
            ),
        };
        result.push(new_fragment);
    }

    Ok(result)
}

/// Holds the state across various stages of checking a query.
struct QueryChecker<'a> {
    /// Input file that the spans reference.
    input: &'a str,

    /// All the parameters specified in the annotation.
    query_args: HashMap<&'a str, TypedIdent<Span>>,

    /// Parameters that are referenced in the query body.
    query_args_used: HashSet<&'a str>,

    /// Typed parameters in the query body.
    ///
    /// The key does not include the leading `:`, but the typed ident value does.
    input_fields: HashMap<&'a str, TypedIdent<Span>>,

    /// Typed parameters in the query body in the order in which they occur.
    ///
    /// Does not contain duplicates, only the first reference.
    input_fields_vec: Vec<TypedIdent<Span>>,

    /// Typed identifiers (outputs) in the query body.
    output_fields: HashMap<&'a str, TypedIdent<Span>>,

    /// Typed identifiers in the query body in the order in which they occur.
    output_fields_vec: Vec<TypedIdent<Span>>,
}

impl<'a> QueryChecker<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            query_args: HashMap::new(),
            query_args_used: HashSet::new(),
            input_fields: HashMap::new(),
            input_fields_vec: Vec::new(),
            output_fields: HashMap::new(),
            output_fields_vec: Vec::new(),
        }
    }

    /// Check the query for consistency and resolve its types.
    ///
    /// Resolving means converting `Type::Simple` into either `Type::Primitive` or
    /// `Type::Struct`. Furthermore, we ensure that every query parameter that
    /// occurs in the query is known (either because the query argument is a struct,
    /// or because the parameter was listed explicitly).
    pub fn check_and_resolve<'b: 'a>(input: &'b str, query: Query<Span>) -> TResult<Query<Span>> {
        let mut annotation = resolve_annotation(input, query.annotation)?;
        let fragments = resolve_fragments(input, query.fragments)?;

        let mut checker = Self::new(input);
        checker.populate_query_args(&annotation)?;
        checker.populate_inputs_outputs(&fragments)?;

        checker.fill_input_struct(input, &mut annotation)?;

        let query = Query {
            annotation: annotation,
            fragments: fragments,
            ..query
        };

        Ok(query)
    }

    fn populate_query_args(&mut self, annotation: &Annotation<Span>) -> TResult<()> {
        // Populate the query args map with the args those provided in the
        // annotation, and at the same time ensure there are no duplicates.
        for arg in &annotation.parameters {
            let name = arg.ident.resolve(self.input);
            match self.query_args.entry(name) {
                Entry::Vacant(vacancy) => vacancy.insert(arg.clone()),
                Entry::Occupied(previous) => {
                    let error = TypeError::with_note(
                        arg.ident,
                        "Redefinition of query parameter.",
                        previous.get().ident,
                        "First defined here.",
                    );
                    return Err(error);
                }
            };
        }
        Ok(())
    }

    /// Handle fragments of the query body, populate inputs and outputs.
    fn populate_inputs_outputs(&mut self, fragments: &[Fragment<Span>]) -> TResult<()> {
        for fragment in fragments {
            self.populate_input_output(fragment)?;
        }
        Ok(())
    }

    /// Handle a single fragment of the query body, populate inputs and outputs.
    fn populate_input_output(&mut self, fragment: &Fragment<Span>) -> TResult<()> {
        match fragment {
            Fragment::Verbatim(..) => return Ok(()),
            Fragment::TypedIdent(_span, ti) => {
                // A typed identifier is an output that the query selects.
                let name = ti.ident.resolve(self.input);
                match self.output_fields.entry(name) {
                    Entry::Vacant(vacancy) => {
                        vacancy.insert(ti.clone());
                        self.output_fields_vec.push(ti.clone());
                    }
                    Entry::Occupied(previous) => {
                        let error = TypeError::with_note(
                            ti.ident,
                            "Redefinition of query output.",
                            previous.get().ident,
                            "First defined here.",
                        );
                        return Err(error);
                    }
                }
            }
            Fragment::Param(span) => {
                // If there is a bare parameter without type annotation, then it
                // must be defined already.

                // Trim off the `:` that query parameters start with.
                let name = span.trim_start(1).resolve(self.input);

                // Record that the argument was used, so that we can
                // warn about unused arguments later.
                self.query_args_used.insert(name);

                if self.query_args.get(name).is_none() {
                    let error = TypeError::with_hint(
                        *span,
                        "Undefined query parameter.",
                        "Define the parameter in the query signature, \
                        or add a type annotation here.",
                    );
                    return Err(error);
                }
            }
            Fragment::TypedParam(_span, ti) => {
                // A typed parameter is an input to the query that should not
                // occur in the arguments already.
                let name = ti.ident.trim_start(1).resolve(self.input);
                self.query_args_used.insert(name);

                match self.input_fields.entry(name) {
                    Entry::Vacant(vacancy) => {
                        vacancy.insert(ti.clone());
                        self.input_fields_vec.push(ti.clone());
                    }
                    Entry::Occupied(previous) => {
                        let prev_type = previous.get().type_.resolve(self.input);
                        let self_type = ti.type_.resolve(self.input);
                        if !prev_type.is_equal_to(&self_type) {
                            let error = TypeError::with_note(
                                ti.type_.span(),
                                "Parameter type differs from an earlier definition.",
                                previous.get().type_.span(),
                                "First defined here.",
                            );
                            return Err(error);
                        }
                        // If the parameter was already defined, but the types
                        // are compatible, there is nothing to do here.
                    }
                }

                if let Some(previous) = self.query_args.get(name) {
                    // If the parameter is typed but it was also defined in the
                    // arguments, then check they agree.
                    let prev_type = previous.type_.resolve(self.input);
                    let self_type = ti.type_.resolve(self.input);
                    if !prev_type.is_equal_to(&self_type) {
                        let error = TypeError::with_note(
                            ti.type_.span(),
                            "Parameter type differs from an earlier definition.",
                            previous.type_.span(),
                            "First defined here.",
                        );
                        return Err(error);
                    }
                }
            }
        }

        Ok(())
    }

    /// If there is a struct type among the inputs, fill its fields.
    ///
    /// This moves the fields out of `self.input_fields_vec`, which becomes
    /// empty.
    fn fill_input_struct(
        &mut self,
        input: &'a str,
        annotation: &mut Annotation<Span>,
    ) -> TResult<()> {
        let mut first_struct = None;

        // Before we can put the fields in, make sure that parameter to put them
        // in is unique -- there can only be one struct per query.
        for param in annotation.parameters.iter() {
            match (param.type_.inner(), first_struct) {
                (Type::Struct(name_span, _), None) => {
                    first_struct = Some(name_span);
                    // A type struct that we fill is not unused.
                    self.query_args_used.insert(name_span.resolve(input));
                }
                (Type::Struct(name_span, _), Some(prev)) => {
                    let mut error = TypeError::with_note(
                        *name_span,
                        "Encountered a second struct parameter.",
                        *prev,
                        "First struct parameter defined here.",
                    );
                    error.hint =
                        Some("There can be at most one struct parameter per query.".into());
                    return Err(error);
                }
                _ => {}
            }
        }

        // Before we put the fields in, check if we have any. If not, but there
        // is a struct param, that's an error, because we would make an empty
        // struct.
        if self.input_fields_vec.len() == 0 {
            match first_struct {
                None => return Ok(()),
                Some(name_span) => {
                    let error = TypeError::with_hint(
                        *name_span,
                        "Annotation contains a struct parameter, \
                        but the query body contains no typed outputs.",
                        "Add type annotations to your query outputs \
                        to turn them into fields of the struct.",
                    );
                    return Err(error);
                }
            }
        }

        // Now that we know the struct is unique, and it won't be empty, we can
        // do a second pass and put the params in.
        for param in annotation.parameters.iter_mut() {
            if let Type::Struct(_, ref mut fields) = param.type_.inner_mut() {
                fields.extend(self.input_fields_vec.drain(..));
            }
        }

        Ok(())
    }
}

/// Apply `check_and_resolve` to every query in the document.
pub fn check_document(input: &str, doc: Document<Span>) -> TResult<Document<Span>> {
    let mut sections = Vec::with_capacity(doc.sections.len());

    for section in doc.sections {
        match section {
            Section::Verbatim(s) => sections.push(Section::Verbatim(s)),
            Section::Query(q) => {
                sections.push(Section::Query(QueryChecker::check_and_resolve(input, q)?))
            }
        }
    }

    let result = Document { sections };

    Ok(result)
}

#[cfg(test)]
mod test {
    use crate::Span;
    use crate::ast::{Query, Section, Type};
    use crate::error::Result;
    use super::QueryChecker;

    fn check_and_resolve_query(input: &str) -> Result<Query<Span>> {
        use crate::lexer::document::Lexer;
        use crate::parser::document::Parser;

        let lexer = Lexer::new(&input);
        let tokens = lexer.run()?;
        let mut parser = Parser::new(&input, &tokens);
        let mut doc = parser.parse_document()?;

        assert_eq!(doc.sections.len(), 1, "Input should consist of a single section.");
        let query = match doc.sections.pop().unwrap() {
            Section::Verbatim(..) => panic!("Expected input to be a single query."),
            Section::Query(q) => q,
        };

        Ok(QueryChecker::check_and_resolve(&input, query)?)
    }

    #[test]
    fn fill_input_struct_populates_top_level() {
        let input =
          "-- @query f(user: User)
          select id /* :i64 */, name /* :str */ from users;";

        let mut query = check_and_resolve_query(input).unwrap();

        assert_eq!(query.annotation.parameters.len(), 1);
        let param = query.annotation.parameters.pop().unwrap();

        match param.type_.resolve(&input) {
            Type::Struct("User", fields) => {
                // TODO: Check fields.
            }
            _ => panic!("Incorrect type for this parameter.")
        }
    }
}

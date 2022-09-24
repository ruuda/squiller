// Squiller -- Generate boilerplate from SQL for statically typed languages
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

use crate::ast::{
    Annotation, ArgType, ComplexType, Document, Fragment, Query, Section, Statement, TypedIdent,
};
use crate::error::{TResult, TypeError};
use crate::Span;

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
    /// We ensure that every query parameter that occurs in the query is known
    /// (either because the query argument is a struct, or because the parameter
    /// was listed explicitly). We also fill the fields of structs.
    pub fn check_and_resolve<'b: 'a>(input: &'b str, query: Query<Span>) -> TResult<Query<Span>> {
        let mut annotation = query.annotation;

        let mut checker = Self::new(input);
        checker.populate_query_args(&annotation)?;
        checker.populate_inputs_outputs(&query.statements)?;

        checker.fill_input_struct(&mut annotation)?;
        checker.fill_output_struct(&mut annotation)?;

        let query = Query {
            annotation: annotation,
            ..query
        };

        Ok(query)
    }

    /// Walk the arguments in the query's annotation and record them.
    fn populate_query_args(&mut self, annotation: &Annotation<Span>) -> TResult<()> {
        // Populate the query args map with the args those provided in the
        // annotation, and at the same time ensure there are no duplicates.
        let args = match &annotation.arguments {
            ArgType::Struct { .. } => return Ok(()),
            ArgType::Args(args) => args,
        };

        for arg in args {
            let name = arg.ident.resolve(self.input);
            match self.query_args.entry(name) {
                Entry::Vacant(vacancy) => vacancy.insert(arg.clone()),
                Entry::Occupied(previous) => {
                    let error = TypeError::with_note(
                        arg.ident,
                        "Redefinition of argument.",
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
    fn populate_inputs_outputs(&mut self, statements: &[Statement<Span>]) -> TResult<()> {
        for statement in statements {
            for fragment in &statement.fragments {
                self.populate_input_output(fragment)?;
            }
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

    /// If the input is a struct type, fill its fields.
    ///
    /// This moves the fields out of `self.input_fields_vec`, which becomes
    /// empty.
    fn fill_input_struct(&mut self, annotation: &mut Annotation<Span>) -> TResult<()> {
        // Before we put the fields in, check if we have any. If not, but there
        // is a struct argument, that's an error, because we would make an empty
        // struct.
        if self.input_fields_vec.len() == 0 {
            match &annotation.arguments {
                ArgType::Struct { type_name, .. } => {
                    let error = TypeError::with_hint(
                        *type_name,
                        "Annotation contains a struct argument, \
                        but the query body contains no typed query parameters.",
                        "Add query parameters with type annotations to the query, \
                        to turn them into fields of the struct.",
                    );
                    return Err(error);
                }
                ArgType::Args(..) => return Ok(()),
            }
        }

        // Conversely, if there are parameters, but no struct, then we have
        // nowhere to put them.
        let fields = match &mut annotation.arguments {
            ArgType::Args(..) => {
                // Does not go out of bounds, if it was empty we returned already.
                let ti = &self.input_fields_vec[0];
                let error = TypeError::with_hint(
                    ti.ident,
                    "Cannot create a field, query has no struct parameter.",
                    "Annotated query parameters in the query body \
                become fields of a struct, but this query has no struct \
                parameter in its signature.",
                );
                return Err(error);
            }
            ArgType::Struct { fields, .. } => fields,
        };

        // Originally, all the typed idents for the parameter include the colon,
        // but we don't want those in the field names, so remove them.
        for mut ti in self.input_fields_vec.drain(..) {
            ti.ident = ti.ident.trim_start(1);
            fields.push(ti);
        }

        Ok(())
    }

    /// If the result type is a struct, fill its fields.
    ///
    /// This moves the fields out of `self.output_fields_vec`, which becomes
    /// empty.
    fn fill_output_struct(&mut self, annotation: &mut Annotation<Span>) -> TResult<()> {
        // Before we put the fields in, check if we have any. If not, but there
        // is a struct result type, that's an error, because we would make an
        // empty struct.
        if self.output_fields_vec.len() == 0 {
            match annotation.result_type.get() {
                Some(ComplexType::Struct(name_span, _fields)) => {
                    let error = TypeError::with_hint(
                        *name_span,
                        "The annotation specifies a struct as result type, \
                        but the query body contains no annotated outputs.",
                        "Add a SELECT or RETURNING clause with type annotations \
                        to the query, to turn them into fields of the struct.",
                    );
                    return Err(error);
                }
                _ => return Ok(()),
            }
        }

        // Conversely, if there are outputs, but no struct, then we have nowhere
        // to put them.
        let fields = match annotation.result_type.get_mut() {
            Some(ComplexType::Struct(_name_span, fields)) => fields,
            _not_struct => {
                // Does not go out of bounds, if it was empty we returned already.
                let ti = &self.output_fields_vec[0];
                let error = TypeError::with_hint(
                    ti.ident,
                    "Cannot create a field, query does not return a struct.",
                    "Annotated outputs in the query body become fields of a \
                    struct, so this query would need to return a struct.",
                );
                return Err(error);
            }
        };

        for ti in self.output_fields_vec.drain(..) {
            fields.push(ti);
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
    use super::QueryChecker;
    use crate::ast::{
        ArgType, ComplexType, PrimitiveType, Query, ResultType, Section, SimpleType, TypedIdent,
    };
    use crate::error::Result;
    use crate::Span;

    fn check_and_resolve_query(input: &str) -> Result<Query<Span>> {
        use crate::lexer::document::Lexer;
        use crate::parser::document::Parser;

        let lexer = Lexer::new(&input);
        let tokens = lexer.run()?;
        let mut parser = Parser::new(&input, &tokens);
        let mut doc = parser.parse_document()?;

        assert_eq!(
            doc.sections.len(),
            1,
            "Input should consist of a single section."
        );
        let query = match doc.sections.pop().unwrap() {
            Section::Verbatim(..) => panic!("Expected input to be a single query."),
            Section::Query(q) => q,
        };

        Ok(QueryChecker::check_and_resolve(&input, query)?)
    }

    #[test]
    fn fill_input_struct_populates_top_level() {
        let input = "\
          -- @query f(user: User) ->1 i64
          select
            max(karma)
          from
            users
          where
            id = :id /* :i64 */
            and name = :name /* :str */
          ;";

        let expected = ArgType::Struct {
            var_name: "user",
            type_name: "User",
            fields: vec![
                TypedIdent {
                    ident: "id",
                    type_: SimpleType::Primitive {
                        inner: "i64",
                        type_: PrimitiveType::I64,
                    },
                },
                TypedIdent {
                    ident: "name",
                    type_: SimpleType::Primitive {
                        inner: "str",
                        type_: PrimitiveType::Str,
                    },
                },
            ],
        };

        let query = check_and_resolve_query(input).unwrap();
        assert_eq!(query.annotation.arguments.resolve(&input), expected);
    }

    #[test]
    fn fill_output_struct_populates_top_level() {
        let input = "\
          -- @query get_admin() ->1 User
          select
            id   /* :i64 */,
            name /* :str */
          from
            users
          where
            id = 13
          ;";

        let query = check_and_resolve_query(input).unwrap();
        match query.annotation.result_type.resolve(&input) {
            ResultType::Single(ComplexType::Struct("User", fields)) => {
                let expected = [
                    TypedIdent {
                        ident: "id",
                        type_: SimpleType::Primitive {
                            inner: "i64",
                            type_: PrimitiveType::I64,
                        },
                    },
                    TypedIdent {
                        ident: "name",
                        type_: SimpleType::Primitive {
                            inner: "str",
                            type_: PrimitiveType::Str,
                        },
                    },
                ];
                assert_eq!(&fields, &expected);
            }
            _ => panic!("Incorrect result type."),
        }
    }

    #[test]
    fn fill_output_struct_populates_inner_types() {
        let input = "\
          -- @query iterate_parents() ->* Node
          select
            id        /* :i64 */,
            parent_id /* :i64? */
          from
            nodes
          ;";

        let query = check_and_resolve_query(input).unwrap();
        match query.annotation.result_type.resolve(&input) {
            ResultType::Iterator(inner) => match inner {
                ComplexType::Struct("Node", fields) => {
                    let expected = [
                        TypedIdent {
                            ident: "id",
                            type_: SimpleType::Primitive {
                                inner: "i64",
                                type_: PrimitiveType::I64,
                            },
                        },
                        TypedIdent {
                            ident: "parent_id",
                            type_: SimpleType::Option {
                                outer: "i64?",
                                inner: "i64",
                                type_: PrimitiveType::I64,
                            },
                        },
                    ];
                    assert_eq!(&fields, &expected);
                }
                _ => panic!("Incorrect result type."),
            },
            _ => panic!("Incorrect result type."),
        }
    }
}

# Squiller

Squiller generates boilerplate code from annotated SQL queries.

Working with SQL is often tedious, especially in statically typed settings. You
need to explicitly bind values to parameters, and extract the resulting values
for every column with the right type. Squiller can generate this boilerplate
code from a SQL file, based on annotations in comments.

## Status

Squiller is a work in progress, beta quality at best. Basic code generation for
the `rust-sqlite` target works, other targets are experimental and incomplete.

It was the author’s hypothesis that boilerplate code generation may be a nicer
way to work with SQL than e.g. heavy reliance on proc macros in Rust. Squiller
is used in [Musium](https://github.com/ruuda/musium) and so far it works well,
but it is too early to draw conclusions.

## Example

Given the following input:

```sql
-- Look up a user by username.
-- @query get_user_by_name(name: str) ->? User
select id /* :i64 */, name /* :str */, email /* :str */
from users
where name = :name;
```

When targeting Rust and the `sqlite` crate, Squiller would generate roughly*:

```rust
struct User {
    id: i64,
    name: String,
    email: String,
}

/// Look up a user by username.
pub fn get_user_by_name(tx: &mut Transaction, name: &str) -> Result<Option<User>> {
    let mut statement = tx.prepare(
        r#"
        select id, name, email
        from users
        where name = :name;
        "#
    )?;
    statement.bind(1, name)?;
    match statement.next()? {
        State::Done => Ok(None),
        State::Row => {
            let result = User {
                id: statement.read(0)?,
                name: statement.read(1)?,
                email: statement.read(2)?,
            };
            Ok(Some(result))
        }
    }
}
```

\* The example generated code is simplified to fit here, the actually generated
code consists of even more boilerplate.

## Limitations

 * Squiller is not fully hygienic. This means that it can generate invalid code,
   when user-specified names overlap with names that Squiller uses internally in
   the generated code. Because the generated code is intended to be readable, it
   should be easy to work around this by choosing different names in the input
   SQL.

 * The generated code may not satisfy your style requirements. Squiller does try
   to generate readable code with sensible indentation, but it does not break
   long lines, and it may generate output that your code formatter disapproves
   of. If this is an issue, simply run the output through your formatter of
   choice.

## Testing

To fuzz the parser:

    cargo +nightly-2022-06-25 install cargo-fuzz --version 0.11.0
    cargo +nightly-2022-06-25 fuzz run parse

To run the golden tests:

    golden/run.py

To update golden tests, if a change is intentional:

    golden/run.py --rewrite-output

To run a particular golden test:

    golden/run.py golden/error/annotation_token_after_paren.test

## License

Squiller is licensed under the [Apache 2.0][apache2] license. Output of the
program (code generated by Squiller) is not covered by this license. Please
do not open an issue if you disagree with the choice of license.

[apache2]: https://www.apache.org/licenses/LICENSE-2.0

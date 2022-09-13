# Squiller

Squiller generates boilerplate code from annotated <abbr>SQL</abbr> queries.

Working with <abbr>SQL</abbr> is often tedious, especially in statically typed
settings. You need to explicitly bind values to parameters, and extract the
resulting values for every column with the right type. Squiller can generate
this boilerplate code from a <abbr>SQL</abbr> file, based on annotations in
comments.

_Vaporware warning: This is a work in progress. Basic code generation for
rust-sqlite works, but the project is pre-alpha quality._

## Example

Given the following input:

```sql
-- Look up a user by username.
-- @query get_user_by_name(name: str) ->? User
select
  id    /* :i64 */,
  name  /* :str */,
  email /* :str */
from
  users
where
  name = :name;
```

When targeting Rust and the `sqlite` crate, Squiller would generate
roughly*:

```rust
struct User {
    id: i64,
    name: String,
    email: String,
}

/// Look up a user by username.
pub fn get_user_by_name(
    tx: &mut Transaction,
    name: &str
) -> Result<Option<User>> {
    let mut statement = tx.prepare(
        r#"
        select
          id,
          name,
          email
        from
          users
        where
          name = :name;
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

\* In reality the generated code is a bit more verbose for several reasons.
There are more intermediate variables to make the code easier to generate. There
is an additional call to `next` to avoid leaving statements in progress. And
finally, we cache the prepared statement in a hash table, instead of preparing
it every call.

# Annotating queries

The inputs to Querybinder are plain <abbr>SQL</abbr> files, with annotations in
comments. This means that you can run the exact same files through e.g. `sqlite`
or `psql`, and confirm that the queries are valid. For every annotated query,
Querybinder generates a corresponding function in the target language that
executes that query. If needed, it also generates types for the inputs and
outputs of the function.

## Annotations

Querybinder ignores all content, until it encounters the marker `@query` in a
comment. This marks the following query as an _annotated_ query, that it will
generate code for. Following the `@query` is the query _signature_, which
specifies its name, arguments, argument types, and result type, similar to
function signatures in other languages. Let’s look at an example:

```sql
-- Return how many users with the given name exist.
-- @query count_users_with_name(name: str) ->? int
select
  count(*)
from
  users
where
  name = :name;
```

In this example, the signature is

```
@query count_users_with_name(name: str) ->? int
```

The name of the query is `count_users_with_name`, and this name will be used for
the generated function. The query takes one argument, `name`, of type `str`.
This will become an argument of the generated function, and that function will
bind the provided value to the `:name` query parameter.

After the name and arguments, is an arrow, and then the result type. The arrow
includes a _cardinality_:

 * `->?` for a query that returns zero or one rows.
 * `->1` for a query that returns exactly one row.
 * `->*` for a query that returns zero or more rows.

The exact types that these arrows map to depends on the target, but generally
they translate as follows:

 * `->? T` maps to `Option<T>`.
 * `->1 T` maps to just `T`.
 * `->* T` maps to `Iterator<T>`.

## Query parameters

Querybinder supports named query parameters with `:name` syntax. This is
[one of the syntaxes supported by SQLite][sqlite], and it allows for named
parameters which is less error-prone than position-based parameters. For
databases that use a different syntax, such as [PostgreSQL][postgres],
Querybinder substitutes the correct syntax in the <abbr>SQL</abbr> string
literal in the generated code.

[sqlite]:   https://www.sqlite.org/c3ref/bind_blob.html
[postgres]: https://www.postgresql.org/docs/current/sql-prepare.html

## Documentation comments

Querybinder preserves any comments immediately preceding the `@query` marker,
up to the first blank line before that marker, as _documentation comments_.
These are included in the output. For example, in Rust they are included as
`///`-style documentation comments, in Python as docstrings.

## Tuple result types

The result type can be a tuple. In this case, the number of columns that the
query returns should match the arity of the tuple. Querybinder does not verify
this. For example, for the following query it would generate code that fails at
runtime, because it tries to access a non-existent third column:

```sql
-- @query incorrect_result_type() ->1 (str, str, i32)
select name, email from users;
```

## Struct result types

Because a type such as `(str, str, i32)` is a bit meaningless, Querybinder also
supports struct types. Struct types must start with an uppercase
<abbr>ascii</abbr> letter. The fields of the struct, and their types, are
extracted from the query body. This means that type annotations are needed in
the body:

```sql
-- @query get_all_users() ->* User
select
  name  /* :str */,
  email /* :str */,
  karma /* :i32 */
from
  users;
```

In this example the fields are `name: str`, `email: str`, and `karma: i32`.

When using struct types, every column that the query selects, should have a type
annotation, because Querybinder generates code that reads the columns by index.
Querybinder does not verify that every column is annotated, because it does not
do the advanced parsing of the query that would be necessary for this.

Every comment between the `@query` marker and the terminating `;` that starts
with a `:` is considered a type annotation, and turns into a struct field. The
identifier that immediately precedes the annotation becomes the name of the
field, so it can be used with `as` to control the name:

```sql
-- @query get_all_users() ->* User
select
  users.name  /* :str */,
  users.email /* :str */,
  sum(karma_earned) as karma /* :i32 */
from
  users, karma_history
where
  users.id = karma_history.user_id
group by
  users.id, users.name, users.email;
```

As before, this example has fields `name: str`, `email: str`, and `karma: i32`.

## Struct arguments

Like in result types, structs can be used in arguments. (Unlike tuples, which
can only be used in result types.) Struct types can only be used for
queries that take a single argument. The name of that argument is preserved in
the generated function. As with result types, the fields are extracted from the
query body, so all query parameters need a type annotation:

```sql
-- @query insert_user(user: User) ->1 i64
insert into
  users (name, email, karma)
values
  (:name /* :str */, :email /* :str */, :initial_karma /* :i32 */)
returning
  id;
```

## Nullable types

All primitive types can be made _optional_ or _nullable_ by appending a `?`.
Primitive types are all types except for structs and tuples, so structs and
tuples cannot be made nullable. This is because structs and tuples map to an
entire row in <abbr>SQL</abbr>, not to individual columns. To specify
optionality at the row level, use a `->?` result type arrow instead of `->1`.

Note, this means that the following two queries would have the same signature
in the generated code, even though they have different signatures in
<abbr>SQL</abbr>:

```sql
-- @query select_longest_email_length_1() ->1 i64?
select
  max(length(email))
from
  users;

-- @query select_longest_email_length_2() ->? i64
select
  length(email)
from
  users
order by
  length(email) desc
limit
  1;
```

Note also that annotating the first query with `->? i64` would result in a
runtime error when the `users` table is empty (because null cannot be decoded
into `i64`), and annotating the second query with `->1 i64?` would result in a
runtime error when the `users` table is empty as well (because it expects at
least one row).

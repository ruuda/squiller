# Terminology

Querybinder accepts <abbr>SQL</abbr> as input, with annotations in comments.
Let’s break that down with an example.

    -- Get a user by user id.
    -- @query select_user_by_id(id: i64) -> User
    SELECT
      name  /* :str */,
      email /* :str */,
      karma /* :i64 */
    FROM
      users
    WHERE
      id = :id;

This example contains a single **query**. A query consists of one statement,
terminated by a semicolon. Not all queries are processed by Querybinder, only
those marked with an annotation are. An **annotation** is a comment that starts
with `@query`, and it defines the **signature** of the query, similar to a
function signature in a programming language. The comments directly preceding
the annotation are **documentation comments**, and they are included as
documentation in the generated code. For the most part Querybinder does not
process the contents of a query, aside from parameters and outputs.
**Parameters** are inputs to the query that need to be provided at runtime. In
the query body, they are prefixed with a colon. In the above example, `:id` is
the only parameter. **Outputs** are values that the query returns. In the above
example `name`, `email`, and `karma` are outputs.

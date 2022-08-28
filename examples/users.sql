.mode box

-- @query setup_schema()
create table if not exists users
  ( id    integer primary key
  , name  string not null
  , email string not null
  );

insert into
  users (name, email)
values 
  ("henk", "henk@example.com"),
  ("piet", "piet@example.com");

-- Insert a new user and return its id.
-- @query insert_user(name: str, email: str) ->1 i64
insert into
  users (name, email)
values
  (:name, :email)
returning
  id;

-- TODO: Add global type detection, use a single "User" type everywhere.
-- Insert a new user and return it.
-- @query insert_user_alt_return(name: str, email: str) ->1 User1
insert into
  users (name, email)
values
  (:name, :email)
returning
  id    /* :i64 */,
  name  /* :str */,
  email /* :str */;

-- Insert a new user and return its id.
-- @query insert_user_alt_arg(user: InsertUser) ->1 i64
insert into
  users (name, email)
values
  (:name /* :str */, :email /* :str */)
returning
  id;

-- Select a particular user by id.
--
-- We make a choice here to always expect one row, with "->1". If a user with
-- the given id does not exist, the function will panic. Alternatively, we could
-- write "->?", and then the return type would be wrapped in option in the
-- generated code, allowing us to handle the error.
-- @query select_user_by_id(id: i64) ->1 User2
select
  id    /* :i64 */,
  name  /* :str */,
  email /* :str */
from
  users
where
  id = :id;

-- Iterate over all users ordered by id.
-- @query select_all_users() ->* User3
select
  id    /* :i64 */,
  name  /* :str */,
  email /* :str */
from
  users
order by
  id asc;

-- Select the length of the longest email address.
-- Note, `max` returns null when the table is empty, hence the `?` on the `i64`.
-- @query select_longest_email_length() ->1 i64?
select
  max(length(email))
from
  users;

-- Select the length of the longest email address.
-- This query returns the same result as [`select_longest_email_length`], and
-- will have the same type in the generated code, but it works differently under
-- the hood: it returns zero or one rows with a non-null column, as opposed to
-- returning exactly one row with a nullable column.
-- @query select_longest_email_length_alt() ->? i64
select
  length(email)
from
  users
order by
  length(email) desc
limit
  1;

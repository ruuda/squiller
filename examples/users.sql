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
-- @query insert_user(name: str, email: str) -> i64
insert into
  users (name, email)
values
  (:name, :email)
returning
  id;

-- Insert a new user and return it.
-- @query insert_user_alt_return(name: str, email: str) -> User
insert into
  users (name, email)
values
  (:name, :email)
returning
  id    /* :str */,
  name  /* :str */,
  email /* :str */;

-- Insert a new user and return its id.
-- @query insert_user_alt_arg(user: InsertUser) -> i64
insert into
  users (name, email)
values
  (:name /* :str */, :email /* :str */)
returning
  id;

-- Select a particular user by id.
-- @query select_user_by_id(id: i64) -> User
select
  id    /* :i64 */,
  name  /* :str */,
  email /* :str */
from
  users
where
  id = :id;

-- Iterate over all users ordered by id.
-- @query select_all_users() -> Iterator<User>
select
  id    /* :i64 */,
  name  /* :str */,
  email /* :str */
from
  users
order by
  id asc;

-- Select the length of the longest email address.
-- @query select_longest_email_length() -> Option<i64>
select
  max(length(email))
from
  users;

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
-- @query add_user(name: &str, email: &str) -> i64
insert into
  users (name, email)
values
  (:name, :email)
returning
  id;

-- Insert a new user and return it.
-- @query add_user_alt(name: &str, email: &str) -> NewUser
insert into
  users (name, email)
values
  (:name, :email)
returning
  id as "id: i64",
  name as "name: String",
  email as "email: String";

-- Select a particular user by id.
-- @query get_user_by_id(id: i64) -> User
select
  id as "id: i64",
  name as "name: String",
  email as "email: String"
from
  users
where
  id = :id;

-- Iterate over all users ordered by id.
-- @query list_all_users() -> Iterator<User>
select
  id as "id: i64",
  name as "name: String",
  email as "email: String"
from
  users
order by
  id asc;

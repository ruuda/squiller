.mode box

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

-- @method get_user
-- Select a particular user by id.
-- @returns User
-- @arg id: u32
select
  name as "name: String",
  email as "email: String"
from
  users
where
  id = :id;

-- @method list_all_users
-- Iterate over all users ordered by id.
-- @returns Iterator<User>
select
  id as "id: u32",
  name as "name: String",
  email as "email: String"
from
  users
order by
  id asc;

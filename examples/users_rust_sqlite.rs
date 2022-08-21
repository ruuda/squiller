// This file was generated by Querybinder <TODO: version>.
// Input files:
// - examples/users.sql

use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::hash_map::HashMap;

use sqlite;
use sqlite::Statement;

pub type Result<T> = sqlite::Result<T>;

pub struct Connection<'a> {
    connection: &'a sqlite::Connection,
    statements: HashMap<u64, Statement<'a>>,
}

pub struct Transaction<'tx, 'a> {
    connection: &'a sqlite::Connection,
    statements: &'tx mut HashMap<u64, Statement<'a>>,
}

impl<'a> Connection<'a> {
    pub fn new(connection: &'a sqlite::Connection) -> Self {
        Self {
            connection,
            // TODO: We could do with_capacity here, because we know the number
            // of queries.
            statements: HashMap::new(),
        }
    }

    /// Begin a new transaction by executing the `BEGIN` statement.
    pub fn begin<'tx>(&'tx mut self) -> Result<Transaction<'tx, 'a>> {
        self.connection.execute("BEGIN;")?;
        let result = Transaction {
            connection: &self.connection,
            statements: &mut self.statements,
        };
        Ok(result)
    }
}

impl<'tx, 'a> Transaction<'tx, 'a> {
    /// Execute `COMMIT` statement.
    pub fn commit(self) -> Result<()> {
        self.connection.execute("COMMIT;")
    }

    /// Execute `ROLLBACK` statement.
    pub fn rollback(self) -> Result<()> {
        self.connection.execute("ROLLBACK;")
    }
}

pub fn setup_schema(tx: &mut Transaction) -> Result<()> {
    let sql = r#"
create table if not exists users
  ( id    integer primary key
  , name  string not null
  , email string not null
  );
    "#;

    let sql_hash = 0;
    let statement = match tx.statements.entry(sql_hash) {
        Occupied(entry) => entry.get_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    Ok(())
}

/// Insert a new user and return its id.
pub fn insert_user(tx: &mut Transaction, name: &str, email: &str) -> Result<i64> {
    let sql = r#"
insert into
  users (name, email)
values
  (:name, :email)
returning
  id;
    "#;

    let sql_hash = 0;
    let statement = match tx.statements.entry(sql_hash) {
        Occupied(entry) => entry.get_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    Ok(())
}

pub struct User1 {
    pub id: i64,
    pub name: String,
    pub email: String,
}

/// TODO: Add global type detection, use a single "User" type everywhere.
/// Insert a new user and return it.
pub fn insert_user_alt_return(tx: &mut Transaction, name: &str, email: &str) -> Result<User1> {
    let sql = r#"
insert into
  users (name, email)
values
  (:name, :email)
returning
  id,
  name,
  email;
    "#;

    let sql_hash = 0;
    let statement = match tx.statements.entry(sql_hash) {
        Occupied(entry) => entry.get_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    Ok(())
}

pub struct InsertUser {
    pub name: &'a str,
    pub email: &'a str,
}

/// Insert a new user and return its id.
pub fn insert_user_alt_arg(tx: &mut Transaction, user: InsertUser) -> Result<i64> {
    let sql = r#"
insert into
  users (name, email)
values
  (:name, :email)
returning
  id;
    "#;

    let sql_hash = 0;
    let statement = match tx.statements.entry(sql_hash) {
        Occupied(entry) => entry.get_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    Ok(())
}

pub struct User2 {
    pub id: i64,
    pub name: String,
    pub email: String,
}

/// Select a particular user by id.
pub fn select_user_by_id(tx: &mut Transaction, id: i64) -> Result<User2> {
    let sql = r#"
select
  id,
  name,
  email
from
  users
where
  id = :id;
    "#;

    let sql_hash = 0;
    let statement = match tx.statements.entry(sql_hash) {
        Occupied(entry) => entry.get_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    Ok(())
}

pub struct User3 {
    pub id: i64,
    pub name: String,
    pub email: String,
}

/// Iterate over all users ordered by id.
pub fn select_all_users(tx: &mut Transaction) -> Result<impl Iterator<Item = User3>> {
    let sql = r#"
select
  id,
  name,
  email
from
  users
order by
  id asc;
    "#;

    let sql_hash = 0;
    let statement = match tx.statements.entry(sql_hash) {
        Occupied(entry) => entry.get_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    Ok(())
}

/// Select the length of the longest email address.
pub fn select_longest_email_length(tx: &mut Transaction) -> Result<Option<i64>> {
    let sql = r#"
select
  max(length(email))
from
  users;
    "#;

    let sql_hash = 0;
    let statement = match tx.statements.entry(sql_hash) {
        Occupied(entry) => entry.get_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    Ok(())
}

// A useless main function, included only to make the example compile with
// Cargo’s default settings for examples.
fn main() {
    let raw_connection = sqlite::open(":memory:").unwrap();
    let mut connection = Connection::new(&raw_connection);

    let tx = connection.begin().unwrap();
    tx.rollback().unwrap();

    let tx = connection.begin().unwrap();
    tx.commit().unwrap();
}

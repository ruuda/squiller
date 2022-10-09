// This file was generated by Squiller 0.3.0-dev (unspecified checkout).
// Input files:
// - examples/users.sql

use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::hash_map::HashMap;

use sqlite;
use sqlite::{
    State::{Done, Row},
    Statement,
};

pub type Result<T> = sqlite::Result<T>;

pub struct Connection<'a> {
    connection: &'a sqlite::Connection,
    statements: HashMap<*const u8, Statement<'a>>,
}

pub struct Transaction<'tx, 'a> {
    connection: &'a sqlite::Connection,
    statements: &'tx mut HashMap<*const u8, Statement<'a>>,
}

pub struct Iter<'i, 'a, T> {
    statement: &'i mut Statement<'a>,
    decode_row: fn(&Statement<'a>) -> Result<T>,
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

impl<'i, 'a, T> Iterator for Iter<'i, 'a, T> {
    type Item = Result<T>;

    fn next(&mut self) -> Option<Result<T>> {
        match self.statement.next() {
            Ok(Row) => Some((self.decode_row)(self.statement)),
            Ok(Done) => None,
            Err(err) => Some(Err(err)),
        }
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
    let statement = match tx.statements.entry(sql.as_ptr()) {
        Occupied(entry) => entry.into_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    statement.reset()?;
    let result = match statement.next()? {
        Row => panic!("Query 'setup_schema' unexpectedly returned a row."),
        Done => (),
    };
    Ok(result)
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
    let statement = match tx.statements.entry(sql.as_ptr()) {
        Occupied(entry) => entry.into_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    statement.reset()?;
    statement.bind(1, name)?;
    statement.bind(2, email)?;
    let decode_row = |statement: &Statement| Ok(statement.read(0)?);
    let result = match statement.next()? {
        Row => decode_row(statement)?,
        Done => panic!("Query 'insert_user' should return exactly one row."),
    };
    if statement.next()? != Done {
        panic!("Query 'insert_user' should return exactly one row.");
    }
    Ok(result)
}

#[derive(Debug)]
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
    let statement = match tx.statements.entry(sql.as_ptr()) {
        Occupied(entry) => entry.into_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    statement.reset()?;
    statement.bind(1, name)?;
    statement.bind(2, email)?;
    let decode_row = |statement: &Statement| {
        Ok(User1 {
            id: statement.read(0)?,
            name: statement.read(1)?,
            email: statement.read(2)?,
        })
    };
    let result = match statement.next()? {
        Row => decode_row(statement)?,
        Done => panic!("Query 'insert_user_alt_return' should return exactly one row."),
    };
    if statement.next()? != Done {
        panic!("Query 'insert_user_alt_return' should return exactly one row.");
    }
    Ok(result)
}

#[derive(Debug)]
pub struct InsertUser<'a> {
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
    let statement = match tx.statements.entry(sql.as_ptr()) {
        Occupied(entry) => entry.into_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    statement.reset()?;
    statement.bind(1, user.name)?;
    statement.bind(2, user.email)?;
    let decode_row = |statement: &Statement| Ok(statement.read(0)?);
    let result = match statement.next()? {
        Row => decode_row(statement)?,
        Done => panic!("Query 'insert_user_alt_arg' should return exactly one row."),
    };
    if statement.next()? != Done {
        panic!("Query 'insert_user_alt_arg' should return exactly one row.");
    }
    Ok(result)
}

#[derive(Debug)]
pub struct User2 {
    pub id: i64,
    pub name: String,
    pub email: String,
}

/// Select a particular user by id.
///
/// We make a choice here to always expect one row, with "->1". If a user with
/// the given id does not exist, the function will panic. Alternatively, we could
/// write "->?", and then the return type would be wrapped in option in the
/// generated code, allowing us to handle the error.
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
    let statement = match tx.statements.entry(sql.as_ptr()) {
        Occupied(entry) => entry.into_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    statement.reset()?;
    statement.bind(1, id)?;
    let decode_row = |statement: &Statement| {
        Ok(User2 {
            id: statement.read(0)?,
            name: statement.read(1)?,
            email: statement.read(2)?,
        })
    };
    let result = match statement.next()? {
        Row => decode_row(statement)?,
        Done => panic!("Query 'select_user_by_id' should return exactly one row."),
    };
    if statement.next()? != Done {
        panic!("Query 'select_user_by_id' should return exactly one row.");
    }
    Ok(result)
}

#[derive(Debug)]
pub struct User3 {
    pub id: i64,
    pub name: String,
    pub email: String,
}

/// Iterate over all users ordered by id.
pub fn select_all_users<'i, 't, 'a>(
    tx: &'i mut Transaction<'t, 'a>,
) -> Result<Iter<'i, 'a, User3>> {
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
    let statement = match tx.statements.entry(sql.as_ptr()) {
        Occupied(entry) => entry.into_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    statement.reset()?;
    let decode_row = |statement: &Statement| {
        Ok(User3 {
            id: statement.read(0)?,
            name: statement.read(1)?,
            email: statement.read(2)?,
        })
    };
    let result = Iter {
        statement,
        decode_row,
    };
    Ok(result)
}

/// Select the length of the longest email address.
/// Note, `max` returns null when the table is empty, hence the `?` on the `i64`.
pub fn select_longest_email_length(tx: &mut Transaction) -> Result<Option<i64>> {
    let sql = r#"
select
  max(length(email))
from
  users;
    "#;
    let statement = match tx.statements.entry(sql.as_ptr()) {
        Occupied(entry) => entry.into_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    statement.reset()?;
    let decode_row = |statement: &Statement| Ok(statement.read(0)?);
    let result = match statement.next()? {
        Row => decode_row(statement)?,
        Done => panic!("Query 'select_longest_email_length' should return exactly one row."),
    };
    if statement.next()? != Done {
        panic!("Query 'select_longest_email_length' should return exactly one row.");
    }
    Ok(result)
}

/// Select the length of the longest email address.
/// This query returns the same result as [`select_longest_email_length`], and
/// will have the same type in the generated code, but it works differently under
/// the hood: it returns zero or one rows with a non-null column, as opposed to
/// returning exactly one row with a nullable column.
pub fn select_longest_email_length_alt(tx: &mut Transaction) -> Result<Option<i64>> {
    let sql = r#"
select
  length(email)
from
  users
order by
  length(email) desc
limit
  1;
    "#;
    let statement = match tx.statements.entry(sql.as_ptr()) {
        Occupied(entry) => entry.into_mut(),
        Vacant(vacancy) => vacancy.insert(tx.connection.prepare(sql)?),
    };
    statement.reset()?;
    let decode_row = |statement: &Statement| Ok(statement.read(0)?);
    let result = match statement.next()? {
        Row => Some(decode_row(statement)?),
        Done => None,
    };
    if result.is_some() {
        if statement.next()? != Done {
            panic!("Query 'select_longest_email_length_alt' should return at most one row.");
        }
    }
    Ok(result)
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

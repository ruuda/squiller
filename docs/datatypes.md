# Data types

Data types in Querybinder are inspired by Rust and Python. Strictly speaking
they do not map to any <abbr>SQL</abbr> types by themselves, only to types in
the target language. It is up to the database driver to then map those to
<abbr>SQL</abbr> types. However, it is still useful to explain the types that
Querybinder supports in terms of the corresponding <abbr>SQL</abbr> data types.

## Supported types

_Vaporware warning: Not all of these are implemented._

| Querybinder | PostgreSQL    | SQLite                   |
|-------------|---------------|--------------------------|
| i32         | int           | integer                  |
| i64         | bigint        | integer                  |
| str         | text          | text                     |
| bytes       | bytea         | blob                     |
| bool        | bool          | integer                  |
| instant     | timestamptz   | text<sup>1</sup> |

<sup>1</sup> Encoded to text as an <abbr>ISO-8601</abbr> timestamp with Z
suffix.

## Language mapping

_Vaporware warning: Not all of these are implemented._

| Querybinder | Rust                   | Python                         | Haskell      |
|-------------|------------------------|--------------------------------|--------------|
| i32         | i32                    | int                            | Int32        |
| i64         | i64                    | int                            | Int64        |
| str         | &str or String         | str                            | Text         |
| bytes       | &[u8] or Vec&lt;u8&gt; | bytes                          | ByteString   |
| bool        | bool                   | bool                           | Bool         |
| instant     | DateTime&lt;Utc&gt;    | datetime<sup>1</sup>           | UtcTime      |

<sup>1</sup> Non-naive datetime, where `tzinfo` is not `None`.

## See also

 * [PostgreSQL data type documentation](https://www.postgresql.org/docs/current/datatype.html)
 * [SQLite data type documentation](https://www.sqlite.org/datatype3.html)

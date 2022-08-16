# Data types

## Supported types

**Vaporware warning**: Not all of these are implemented.

| Querybinder | PostgreSQL    | SQLite                     |
|-------------|---------------|----------------------------|
| `int32`     | `int`         | `integer`                  |
| `int64`     | `bigint`      | `integer`                  |
| `str`       | `text`        | `text`                     |
| `bytes`     | `bytea`       | `blob`                     |
| `bool`      | `bool`        | `integer`                  |
| `instant`   | `timestamptz` | `text`&thinsp;<sup>1</sup> |

<sup>1</sup> Encoded to text as an <abbr>ISO-8601</abbr> timestamp with Z
suffix.

## Language mapping

**Vaporware warning**: None of this is implemented.

| Querybinder | Rust                 | Python                         | Haskell      |
|-------------|----------------------|--------------------------------|--------------|
| `int32`     | `i32`                | `int`                          | `Int32`      |
| `int64`     | `i64`                | `int`                          | `Int64`      |
| `str`       | `&str` or `String`   | `str`                          | `Text`       |
| `bytes`     | `&[u8]` or `Vec<u8>` | `bytes`                        | `ByteString` |
| `bool`      | `bool`               | `bool`                         | `Bool`       |
| `instant`   | `DateTime<Utc>`      | `datetime`&thinsp;<sup>1</sup> | `UtcTime`    |

<sup>1</sup> Non-naive datetime, where `tzinfo` is not `None`.

## See also

 * [PostgreSQL data type documentation](https://www.postgresql.org/docs/current/datatype.html)
 * [SQLite data type documentation](https://www.sqlite.org/datatype3.html)

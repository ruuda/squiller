# Querybinder

Querybinder generates boilerplate code from annotated SQL queries.

Working with SQL is often tedious, especially in statically typed settings. You
need to explicitly bind values to parameters, and extract the resulting values
for every column with the right type. Querybinder can generate this boilerplate
code from a SQL file, based on annotations in comments.

_Vaporware warning: This is a work in progress. Basic code generation for
rust-sqlite works, but the project is pre-alpha quality._

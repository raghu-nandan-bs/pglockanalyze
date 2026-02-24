# pglockanalyze&emsp;[![Build Status]][actions] [![Latest Version]][crates.io]

[Build Status]: https://img.shields.io/github/actions/workflow/status/agis/pglockanalyze/ci.yml?branch=main
[actions]: https://github.com/agis/pglockanalyze/actions?query=branch%3Amain
[Latest Version]: https://img.shields.io/crates/v/pglockanalyze.svg
[crates.io]: https://crates.io/crates/pglockanalyze

*See what locks your Postgres migrations will acquire—before you run them in production.*

<p align="center">
  <img src="https://github.com/user-attachments/assets/3539ef87-8bce-436c-a826-fbdc4a7da526" />
</p>

To be used in CI and development environments; see
[pglockanalyze-action](https://github.com/agis/pglockanalyze-action) for
integration with GitHub Actions.

## Status

This software is in alpha stage - expect breaking changes between releases and a lot of rough edges.

## Rationale

Understanding the locks your migrations will acquire is crucial to avoiding
downtime in  production traffic. Tools like the [official Postgres
docs](https://www.postgresql.org/docs/current/explicit-locking.html) and
[strong_migrations](https://github.com/ankane/strong_migrations) are invaluable;
however, reasoning your way through complex DDL statements is not always
practical.

pglockanalyze is meant to complement, not replace such tools, by executing your
migrations against a test database (that you have to provision) and dynamically
identifying the locks acquired at runtime. It then prints a report of the locks
that were acquired.

By default, pglockanalyze rolls back the transactions it analyzes, so you can
safely run it against a test database without worrying about leaving it in and
inconsistent state. If you want to commit the transactions, you can use the
`--commit` option.

## Installation

You can install pglockanalyze using [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html):

```shell
$ cargo install pglockanalyze
```

We do not distribute binaries yet, but we may do so in the future.

## Usage

```shell
$ echo 'ALTER TABLE users ALTER COLUMN name SET NOT NULL' | pglockanalyze --db 'postgres://foo@bar'
ALTER TABLE users ALTER COLUMN name SET NOT NULL
	acquired `AccessExclusive` lock on relation `users` (oid=16386)
```

Use `--help` to see all options:

```shell
Usage: pglockanalyze [OPTIONS] --db <postgres connection string> [INPUT]

Arguments:
  [INPUT]  The DDL statements to analyze. If not provided or is -, read from standard input [default: -]

Options:
      --db <postgres connection string>
          The database to connect to
  -f, --format <FORMATTER>
          The output format of the analysis [default: plain] [possible values: plain, json]
      --distinct-transactions
          Execute each statement in its own transaction. By default all statements are executed in a single transaction. Implies --commit
      --commit
          Commit the transactions. By default they are rolled back
  -h, --help
          Print help
  -V, --version
          Print version
```

## License

pglockanalyze is licensed under the [Apache 2.0 license](LICENSE).


---

## Updates:

Added a line that indicates whethere there was table rewrites or not.

```
ALTER TABLE test_data DROP COLUMN status
	acquired `AccessShare` lock on relation `pg_class_tblspc_relfilenode_index` (oid=3455)
	acquired `AccessShare` lock on relation `pg_class_oid_index` (oid=2662)
	acquired `AccessExclusive` lock on relation `test_data` (oid=16386)
	acquired `AccessShare` lock on relation `pg_class_relname_nsp_index` (oid=2663)
	acquired `AccessShare` lock on relation `pg_class` (oid=1259)
	(no table rewrites)
╭─  raghu@atomic ~/code/aw-01/pglockanalyze
╰─❯ ./target/debug/pglockanalyze --db "postgres://postgres@127.0.0.1:5432/testdb" sample.sql
ALTER TABLE test_data ADD COLUMN seq SERIAL
	acquired `Share` lock on relation `16400` (oid=16400)
	acquired `AccessExclusive` lock on relation `16397` (oid=16397)
	acquired `AccessExclusive` lock on relation `test_data_pkey` (oid=16393)
	acquired `AccessExclusive` lock on relation `pg_toast.pg_toast_16386` (oid=16391)
	acquired `AccessShare` lock on relation `pg_class_oid_index` (oid=2662)
	acquired `AccessShare` lock on relation `pg_class` (oid=1259)
	acquired `AccessExclusive` lock on relation `16401` (oid=16401)
	acquired `AccessExclusive` lock on object `16398 (class: pg_type)` (oid=16398)
	acquired `AccessExclusive` lock on relation `16400` (oid=16400)
	acquired `Share` lock on relation `test_data` (oid=16386)
	acquired `AccessShare` lock on relation `test_data` (oid=16386)
	acquired `ShareRowExclusive` lock on relation `16395` (oid=16395)
	acquired `AccessShare` lock on object `2200 (class: pg_namespace)` (oid=2200)
	acquired `ShareUpdateExclusive` lock on relation `16401` (oid=16401)
	acquired `AccessShare` lock on relation `pg_class_tblspc_relfilenode_index` (oid=3455)
	acquired `AccessExclusive` lock on object `16399 (class: pg_type)` (oid=16399)
	acquired `AccessExclusive` lock on relation `test_data` (oid=16386)
	acquired `AccessShare` lock on relation `pg_class_relname_nsp_index` (oid=2663)
	acquired `AccessExclusive` lock on relation `pg_toast.pg_toast_16386_index` (oid=16392)
	acquired `RowExclusive` lock on relation `16395` (oid=16395)
	acquired `AccessExclusive` lock on relation `16395` (oid=16395)
	rewrote table `test_data`
	```

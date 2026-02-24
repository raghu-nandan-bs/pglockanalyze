use crate::analyzer::AnalyzeError;
use crate::lock::{Lock, Locks};
use crate::rewrite::{RelfilenodeSnapshot, TableRewrites};
use postgres as pg;
use serde::{Deserialize, Serialize};
use sqlparser::ast::Spanned;
use sqlparser::ast::Statement as AstStatement;
use std::collections::HashSet;
use std::fmt;

/// Starting and ending lines in the original input where a statement appears
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Location {
    pub start_line: u64,
    pub end_line: u64,
}

impl From<sqlparser::tokenizer::Span> for Location {
    fn from(span: sqlparser::tokenizer::Span) -> Self {
        Self {
            start_line: span.start.line,
            end_line: span.end.line,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Statement {
    pub sql: String,
    pub locks_acquired: Locks,
    #[serde(default)]
    pub table_rewrites: TableRewrites,
    pub location: Location,
}

impl Statement {
    pub(crate) fn analyze(
        db: &pg::Config,
        tx: &mut pg::Transaction,
        pid: i32,
        stmt: AstStatement,
    ) -> Result<Self, AnalyzeError> {
        let locks_before = Self::detect_locks(db, pid)?;
        // Snapshot relfilenodes on the TRANSACTION connection (not the observer),
        // because pg_class changes are visible within the same transaction but
        // not from a separate connection until commit.
        let relfilenodes_before = Self::snapshot_relfilenodes(tx)?;

        let sql = stmt.to_string();
        tx.execute(&sql, &[])?;

        let locks_after = Self::detect_locks(db, pid)?;
        let relfilenodes_after = Self::snapshot_relfilenodes(tx)?;

        let locks_acquired = Locks::compute_acquired(locks_before, locks_after);
        let table_rewrites = TableRewrites::compute(relfilenodes_before, relfilenodes_after);

        Ok(Statement {
            sql,
            locks_acquired,
            table_rewrites,
            location: stmt.span().into(),
        })
    }

    pub(crate) fn detect_locks(
        config: &pg::Config,
        pid: i32,
    ) -> Result<HashSet<Lock>, AnalyzeError> {
        const SQL: &str = "\
SELECT
    l.locktype,
    l.database,
    d.datname AS database_name,
    l.relation,
    l.objid,
    l.mode,
    CASE l.locktype
        WHEN 'relation' THEN l.relation::regclass::text
        WHEN 'object'   THEN l.objid::text || ' (class: ' || l.classid::regclass::text || ')'
    END AS target
FROM
    pg_catalog.pg_locks l
LEFT JOIN
    pg_catalog.pg_database d
ON
    l.database = d.oid
WHERE
    l.pid = $1
    AND l.locktype IN ('relation', 'object')
    AND l.granted";

        config
            .connect(postgres::NoTls)?
            .query(SQL, &[&pid])?
            .into_iter()
            .map(Lock::try_from)
            .collect()
    }

    /// Snapshot relfilenodes for all tables in the public schema.
    ///
    /// Must be called on the TRANSACTION connection (not the observer),
    /// because pg_class changes are visible within the same transaction
    /// but not from a separate connection until commit.
    fn snapshot_relfilenodes(
        tx: &mut pg::Transaction,
    ) -> Result<Vec<RelfilenodeSnapshot>, AnalyzeError> {
        const SQL: &str = "\
SELECT oid, relname, relfilenode
FROM pg_class
WHERE relnamespace = 'public'::regnamespace
    AND relkind IN ('r', 'p')
    AND relfilenode != 0";

        let rows = tx.query(SQL, &[])?;

        Ok(rows
            .iter()
            .map(|row| RelfilenodeSnapshot {
                oid: row.get::<_, u32>("oid"),
                relname: row.get::<_, String>("relname"),
                relfilenode: row.get::<_, u32>("relfilenode"),
            })
            .collect())
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n{}\n{}",
            self.sql, self.locks_acquired, self.table_rewrites
        )
    }
}

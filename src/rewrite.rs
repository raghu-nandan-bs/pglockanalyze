use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A snapshot of a table's relfilenode from pg_class.
#[derive(Debug, Clone)]
pub(crate) struct RelfilenodeSnapshot {
    pub oid: u32,
    pub relname: String,
    pub relfilenode: u32,
}

/// A detected table rewrite event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableRewrite {
    pub table: String,
}

/// Collection of table rewrites detected for a single statement.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableRewrites(Vec<TableRewrite>);

impl TableRewrites {
    /// Compare before/after relfilenode snapshots to find rewrites.
    ///
    /// Joins on `oid` (not `relname`) so that table renames are correctly
    /// identified as non-rewrites.
    ///
    /// Only reports rewrites for tables that exist in both snapshots with a
    /// changed relfilenode. New tables and dropped tables are omitted.
    pub(crate) fn compute(
        before: Vec<RelfilenodeSnapshot>,
        after: Vec<RelfilenodeSnapshot>,
    ) -> Self {
        let before_map: HashMap<u32, RelfilenodeSnapshot> =
            before.into_iter().map(|s| (s.oid, s)).collect();

        let mut rewrites = Vec::new();
        for after_snap in &after {
            if let Some(before_snap) = before_map.get(&after_snap.oid) {
                if before_snap.relfilenode != after_snap.relfilenode {
                    rewrites.push(TableRewrite {
                        table: after_snap.relname.clone(),
                    });
                }
            }
        }

        TableRewrites(rewrites)
    }
}

impl fmt::Display for TableRewrite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rewrote table `{}`", self.table)
    }
}

impl fmt::Display for TableRewrites {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rewrites = self
            .0
            .iter()
            .map(|r| format!("\t{r}"))
            .collect::<Vec<String>>();

        let s = if rewrites.is_empty() {
            "\t(no table rewrites)"
        } else {
            &rewrites.join("\n")
        };

        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(oid: u32, relname: &str, relfilenode: u32) -> RelfilenodeSnapshot {
        RelfilenodeSnapshot {
            oid,
            relname: relname.to_string(),
            relfilenode,
        }
    }

    #[test]
    fn test_rewrite_detected() {
        let before = vec![snap(1, "products", 100)];
        let after = vec![snap(1, "products", 200)];
        let result = TableRewrites::compute(before, after);
        assert_eq!(
            result,
            TableRewrites(vec![TableRewrite {
                table: "products".to_string()
            }])
        );
    }

    #[test]
    fn test_no_rewrite() {
        let before = vec![snap(1, "products", 100)];
        let after = vec![snap(1, "products", 100)];
        let result = TableRewrites::compute(before, after);
        assert_eq!(result, TableRewrites(Vec::new()));
    }

    #[test]
    fn test_new_table_not_reported() {
        let before = vec![];
        let after = vec![snap(1, "products", 100)];
        let result = TableRewrites::compute(before, after);
        assert_eq!(result, TableRewrites(Vec::new()));
    }

    #[test]
    fn test_dropped_table_not_reported() {
        let before = vec![snap(1, "products", 100)];
        let after = vec![];
        let result = TableRewrites::compute(before, after);
        assert_eq!(result, TableRewrites(Vec::new()));
    }

    #[test]
    fn test_rename_not_reported_as_rewrite() {
        // oid stays the same, relname changes, relfilenode stays the same
        let before = vec![snap(1, "orders", 100)];
        let after = vec![snap(1, "orders_archive", 100)];
        let result = TableRewrites::compute(before, after);
        assert_eq!(result, TableRewrites(Vec::new()));
    }

    #[test]
    fn test_multiple_tables_one_rewritten() {
        let before = vec![snap(1, "users", 100), snap(2, "products", 200)];
        let after = vec![snap(1, "users", 100), snap(2, "products", 300)];
        let result = TableRewrites::compute(before, after);
        assert_eq!(
            result,
            TableRewrites(vec![TableRewrite {
                table: "products".to_string()
            }])
        );
    }

    #[test]
    fn test_display_no_rewrites() {
        let rewrites = TableRewrites(Vec::new());
        assert_eq!(format!("{rewrites}"), "\t(no table rewrites)");
    }

    #[test]
    fn test_display_with_rewrite() {
        let rewrites = TableRewrites(vec![TableRewrite {
            table: "products".to_string(),
        }]);
        assert_eq!(format!("{rewrites}"), "\trewrote table `products`");
    }
}

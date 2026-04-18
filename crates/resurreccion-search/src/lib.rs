//! Full-text and structured search over workspace names, snapshot manifests,
//! and event payloads stored in `SQLite` via FTS5.

use rusqlite::Connection;

/// A full-text search index backed by an in-memory `SQLite` FTS5 table.
pub struct SearchIndex {
    conn: Connection,
}

/// A single result returned by [`SearchIndex::search`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    /// The kind of indexed item: `"workspace"`, `"snapshot"`, or `"event"`.
    pub kind: String,
    /// The opaque identifier of the indexed item.
    pub id: String,
    /// A highlighted snippet of the matching text.
    pub snippet: String,
    /// The FTS5 rank score (lower is better; negative values indicate better matches).
    pub score: f64,
}

impl SearchIndex {
    /// Create an in-memory search index.
    ///
    /// # Errors
    ///
    /// Returns an error if the `SQLite` connection or FTS5 table creation fails.
    pub fn new() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(kind, id, text);",
        )?;
        Ok(Self { conn })
    }

    /// Index a workspace for search.
    ///
    /// # Errors
    ///
    /// Returns an error if the INSERT fails.
    pub fn index_workspace(&self, id: &str, name: &str, root_path: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO search_fts (kind, id, text) VALUES ('workspace', ?1, ?2)",
            rusqlite::params![id, format!("{name} {root_path}")],
        )?;
        Ok(())
    }

    /// Index a snapshot manifest for search.
    ///
    /// # Errors
    ///
    /// Returns an error if the INSERT fails.
    pub fn index_snapshot(&self, id: &str, manifest_json: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO search_fts (kind, id, text) VALUES ('snapshot', ?1, ?2)",
            rusqlite::params![id, manifest_json],
        )?;
        Ok(())
    }

    /// Full-text search across all indexed items.
    ///
    /// Returns up to 50 results ordered by FTS5 rank.
    ///
    /// # Errors
    ///
    /// Returns an error if the query preparation or execution fails.
    pub fn search(&self, query: &str) -> anyhow::Result<Vec<SearchResult>> {
        // Wrap the raw query in double quotes so FTS5 treats it as a phrase
        // and does not interpret special characters (hyphens, colons, etc.).
        // Escape any existing double quotes in the user input first.
        let fts_query = format!("\"{}\"", query.replace('"', "\"\""));
        let mut stmt = self.conn.prepare(
            "SELECT kind, id, snippet(search_fts, 2, '[', ']', '...', 10), rank
             FROM search_fts WHERE search_fts MATCH ?1
             ORDER BY rank LIMIT 50",
        )?;
        let rows = stmt.query_map(rusqlite::params![fts_query], |row| {
            Ok(SearchResult {
                kind: row.get(0)?,
                id: row.get(1)?,
                snippet: row.get(2)?,
                score: row.get::<_, f64>(3).unwrap_or(0.0),
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new().expect("in-memory search index")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_workspace_then_search_finds_it() {
        let idx = SearchIndex::new().unwrap();
        idx.index_workspace("ws-001", "my-project", "/home/user/my-project")
            .unwrap();

        let results = idx.search("my-project").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, "workspace");
        assert_eq!(results[0].id, "ws-001");
    }

    #[test]
    fn index_snapshot_then_search_finds_it() {
        let idx = SearchIndex::new().unwrap();
        let manifest = r#"{"name":"daily-backup","tags":["important","prod"]}"#;
        idx.index_snapshot("snap-42", manifest).unwrap();

        let results = idx.search("daily-backup").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, "snapshot");
        assert_eq!(results[0].id, "snap-42");
    }

    #[test]
    fn search_nonexistent_term_returns_empty() {
        let idx = SearchIndex::new().unwrap();
        idx.index_workspace("ws-002", "alpha", "/repos/alpha")
            .unwrap();

        let results = idx.search("zzznomatchzzz").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_across_multiple_kinds() {
        let idx = SearchIndex::new().unwrap();
        idx.index_workspace("ws-10", "shared-lib", "/repos/shared-lib")
            .unwrap();
        idx.index_snapshot("snap-10", r#"{"workspace":"shared-lib"}"#)
            .unwrap();

        let results = idx.search("shared").unwrap();
        assert_eq!(results.len(), 2);
        let kinds: Vec<&str> = results.iter().map(|r| r.kind.as_str()).collect();
        assert!(kinds.contains(&"workspace"));
        assert!(kinds.contains(&"snapshot"));
    }
}

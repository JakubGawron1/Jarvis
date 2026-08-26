use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

pub struct Memory {
    conn: Connection,
}

impl Memory {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS facts (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS turns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                lang TEXT NOT NULL,
                ts TEXT NOT NULL
            );
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn remember(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO facts(key,value,updated_at) VALUES(?1,?2,?3)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
            params![key, value, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn recall(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM facts WHERE key=?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn log_turn(&self, role: &str, content: &str, lang: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO turns(role,content,lang,ts) VALUES(?1,?2,?3,?4)",
            params![role, content, lang, chrono::Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn recent_turns(&self, limit: usize) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT role, content, lang FROM turns ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        out.reverse();
        Ok(out)
    }

    pub fn all_facts(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM facts")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

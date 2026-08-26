use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub title: String,
    pub status: String,
    pub detail: String,
    pub deferred_until: Option<String>,
    pub checkpoint: Option<String>,
}

pub struct TaskQueue {
    conn: Connection,
}

impl TaskQueue {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                detail TEXT NOT NULL,
                deferred_until TEXT,
                checkpoint TEXT,
                created_at TEXT NOT NULL
            );
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn enqueue(&self, title: &str, detail: &str) -> Result<Job> {
        let job = Job {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            status: "queued".into(),
            detail: detail.to_string(),
            deferred_until: None,
            checkpoint: None,
        };
        self.conn.execute(
            "INSERT INTO jobs(id,title,status,detail,created_at) VALUES(?1,?2,?3,?4,?5)",
            params![
                job.id,
                job.title,
                job.status,
                job.detail,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
        Ok(job)
    }

    pub fn defer(&self, id: &str, until: &str, message: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE jobs SET status='deferred', deferred_until=?1, detail=?2 WHERE id=?3",
            params![until, message, id],
        )?;
        Ok(())
    }

    pub fn set_status(&self, id: &str, status: &str, detail: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE jobs SET status=?1, detail=?2 WHERE id=?3",
            params![status, detail, id],
        )?;
        Ok(())
    }

    pub fn checkpoint(&self, id: &str, data: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE jobs SET checkpoint=?1 WHERE id=?2",
            params![data, id],
        )?;
        Ok(())
    }

    pub fn list_open(&self) -> Result<Vec<Job>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,title,status,detail,deferred_until,checkpoint FROM jobs
             WHERE status NOT IN ('done','cancelled') ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Job {
                id: r.get(0)?,
                title: r.get(1)?,
                status: r.get(2)?,
                detail: r.get(3)?,
                deferred_until: r.get(4)?,
                checkpoint: r.get(5)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn wake_deferred(&self, until_tag: &str) -> Result<Vec<Job>> {
        self.conn.execute(
            "UPDATE jobs SET status='queued' WHERE status='deferred' AND deferred_until=?1",
            params![until_tag],
        )?;
        self.list_open()
    }
}

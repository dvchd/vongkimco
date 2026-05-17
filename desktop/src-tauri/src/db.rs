use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::Serialize;

pub struct LocalDb {
    pub path: PathBuf,
}

impl LocalDb {
    pub fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir).ok();
        let path = dir.join("vkc_local.db");
        let me = Self { path };
        me.with(|c| {
            c.execute_batch(SCHEMA)?;
            Ok(())
        })?;
        Ok(me)
    }

    pub fn with<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let c = Connection::open(&self.path).context("open sqlite")?;
        c.pragma_update(None, "journal_mode", &"WAL").ok();
        c.pragma_update(None, "synchronous", &"NORMAL").ok();
        c.execute_batch("PRAGMA foreign_keys = ON;").ok();
        f(&c)
    }

    pub fn create_session(&self, started_at: &str, note: Option<&str>) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.with(|c| {
            c.execute(
                "INSERT INTO sessions (id, started_at, note, synced) VALUES (?, ?, ?, 0)",
                params![id, started_at, note],
            )?;
            Ok(())
        })?;
        Ok(id)
    }

    pub fn end_session(&self, id: &str, ended_at: &str) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE sessions SET ended_at = ?, synced = 0 WHERE id = ?",
                params![ended_at, id],
            )?;
            Ok(())
        })
    }

    pub fn insert_activity(
        &self,
        session_id: &str,
        sampled_at: &str,
        state: &str,
        idle_seconds: i64,
        keyboard: i64,
        mouse: i64,
    ) -> Result<()> {
        let id = uuid::Uuid::new_v4().to_string();
        self.with(|c| {
            c.execute(
                "INSERT INTO activity_samples
                 (id, session_id, sampled_at, state, idle_seconds, keyboard_events, mouse_events, synced)
                 VALUES (?, ?, ?, ?, ?, ?, ?, 0)",
                params![id, session_id, sampled_at, state, idle_seconds, keyboard, mouse],
            )?;
            Ok(())
        })
    }

    pub fn insert_app_snapshot(
        &self,
        session_id: &str,
        sampled_at: &str,
        foreground_app: Option<&str>,
        foreground_title: Option<&str>,
        apps_json: &str,
    ) -> Result<()> {
        let id = uuid::Uuid::new_v4().to_string();
        self.with(|c| {
            c.execute(
                "INSERT INTO app_snapshots
                 (id, session_id, sampled_at, foreground_app, foreground_title, apps_json, synced)
                 VALUES (?, ?, ?, ?, ?, ?, 0)",
                params![id, session_id, sampled_at, foreground_app, foreground_title, apps_json],
            )?;
            Ok(())
        })
    }

    pub fn insert_screenshot(
        &self,
        session_id: &str,
        captured_at: &str,
        file_path: &str,
        bytes: usize,
    ) -> Result<()> {
        let id = uuid::Uuid::new_v4().to_string();
        self.with(|c| {
            c.execute(
                "INSERT INTO screenshots
                 (id, session_id, captured_at, file_path, bytes, synced)
                 VALUES (?, ?, ?, ?, ?, 0)",
                params![id, session_id, captured_at, file_path, bytes as i64],
            )?;
            Ok(())
        })
    }

    pub fn list_sessions(&self, limit: i64) -> Result<Vec<LocalSession>> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT s.id, s.started_at, s.ended_at, s.note,
                        CASE WHEN s.synced = 1 AND s.remote_id IS NOT NULL THEN 1 ELSE 0 END AS synced,
                        (SELECT COALESCE(SUM(keyboard_events), 0) FROM activity_samples a WHERE a.session_id = s.id),
                        (SELECT COALESCE(SUM(mouse_events), 0) FROM activity_samples a WHERE a.session_id = s.id)
                 FROM sessions s ORDER BY started_at DESC LIMIT ?",
            )?;
            let out: Vec<LocalSession> = stmt
                .query_map(params![limit], |r| {
                    Ok(LocalSession {
                        id: r.get::<_, String>(0)?,
                        started_at: r.get::<_, String>(1)?,
                        ended_at: r.get::<_, Option<String>>(2)?,
                        note: r.get::<_, Option<String>>(3)?,
                        synced: r.get::<_, i64>(4)? == 1,
                        keyboard_events: r.get::<_, i64>(5)?,
                        mouse_events: r.get::<_, i64>(6)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(out)
        })
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct LocalSession {
    pub id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub note: Option<String>,
    pub synced: bool,
    pub keyboard_events: i64,
    pub mouse_events: i64,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    remote_id TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    note TEXT,
    synced INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS activity_samples (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    sampled_at TEXT NOT NULL,
    state TEXT NOT NULL,
    idle_seconds INTEGER NOT NULL DEFAULT 0,
    keyboard_events INTEGER NOT NULL DEFAULT 0,
    mouse_events INTEGER NOT NULL DEFAULT 0,
    synced INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_activity_unsynced ON activity_samples(synced, session_id);

CREATE TABLE IF NOT EXISTS app_snapshots (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    sampled_at TEXT NOT NULL,
    foreground_app TEXT,
    foreground_title TEXT,
    apps_json TEXT NOT NULL,
    synced INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_appsnap_unsynced ON app_snapshots(synced, session_id);

CREATE TABLE IF NOT EXISTS screenshots (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    file_path TEXT NOT NULL,
    bytes INTEGER,
    synced INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_screenshots_unsynced ON screenshots(synced, session_id);
"#;

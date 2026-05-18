//! Periodic synchronization of local data to the backend.
//!
//! Strategy: every N seconds, attempt to push unsynced sessions, activity samples,
//! app snapshots, and screenshots. Anything that fails stays marked unsynced
//! and is retried next tick. The desktop app works fully offline; sync resumes
//! whenever the network/server returns.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::multipart::{Form, Part};
use rusqlite::params;
use serde_json::json;
use tokio::time::sleep;

use crate::state::{emit_status, AppState};

const SYNC_INTERVAL: Duration = Duration::from_secs(20);

/// One pending session row: (local_id, started_at, ended_at, note, remote_id).
type SessionRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// One activity sample queued for upload, keyed by remote session id.
/// Fields after `local_id`: (sampled_at, idle_seconds, keyboard_events, mouse_events).
type ActivityItem = (String, String, i64, i64, i64);

/// One app-snapshot row pulled from the local DB:
/// (local_id, remote_session_id, sampled_at, foreground_app, foreground_title, apps_json).
type AppSnapshotRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
);

/// One app-snapshot grouped for upload: (local_id, sampled_at, foreground_app, foreground_title, parsed apps).
type AppSnapshotItem = (
    String,
    String,
    Option<String>,
    Option<String>,
    serde_json::Value,
);

pub async fn start_sync_loop(state: AppState) {
    tokio::spawn(async move {
        loop {
            let token = state.auth_token();
            let online_before = state.session.read().online;
            if let Some(token) = token {
                let server = state.server_url();
                match sync_once(&state, &server, &token).await {
                    Ok(_) => {
                        if !online_before {
                            state.session.write().online = true;
                            emit_status(&state);
                        }
                    }
                    Err(e) => {
                        log::warn!("sync failed: {e:?}");
                        if online_before {
                            state.session.write().online = false;
                            emit_status(&state);
                        }
                    }
                }
            }
            update_pending_counts(&state);
            sleep(SYNC_INTERVAL).await;
        }
    });
}

pub fn update_pending_counts(state: &AppState) {
    let pending: i64 = state
        .db
        .with(|c| {
            let a: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM activity_samples WHERE synced = 0",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let b: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM app_snapshots WHERE synced = 0",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let s: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM screenshots WHERE synced = 0",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            Ok(a + b + s)
        })
        .unwrap_or(0);
    state.session.write().pending_sync = pending;
}

pub async fn sync_once(state: &AppState, server: &str, token: &str) -> Result<()> {
    // 1) Upsert any sessions that don't have a remote_id yet OR whose end time changed locally
    push_sessions(state, server, token).await?;
    // 2) Push activity samples
    push_activity_samples(state, server, token).await?;
    // 3) Push app snapshots
    push_app_snapshots(state, server, token).await?;
    // 4) Push screenshots
    push_screenshots(state, server, token).await?;
    Ok(())
}

async fn push_sessions(state: &AppState, server: &str, token: &str) -> Result<()> {
    let rows: Vec<SessionRow> = state.db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT id, started_at, ended_at, note, remote_id
                 FROM sessions WHERE synced = 0",
        )?;
        let out = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    })?;

    let client = reqwest::Client::new();
    for (local_id, started_at, ended_at, note, _remote_id) in rows {
        let body = json!({
            "client_session_id": local_id,
            "started_at": started_at,
            "ended_at": ended_at,
            "note": note,
        });
        let resp = client
            .post(format!("{}/api/v1/sessions", server.trim_end_matches('/')))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .context("post session")?;
        if !resp.status().is_success() {
            return Err(anyhow!("session sync HTTP {}", resp.status()));
        }
        let resp_json: serde_json::Value = resp.json().await?;
        let remote_id = resp_json
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        state.db.with(|c| {
            c.execute(
                "UPDATE sessions SET synced = 1, remote_id = ? WHERE id = ?",
                params![remote_id, local_id],
            )?;
            Ok(())
        })?;
    }
    Ok(())
}

async fn push_activity_samples(state: &AppState, server: &str, token: &str) -> Result<()> {
    loop {
        let rows: Vec<(String, String, String, String, i64, i64, i64)> = state.db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT a.id, a.session_id, s.remote_id, a.sampled_at, a.idle_seconds, a.keyboard_events, a.mouse_events
                 FROM activity_samples a JOIN sessions s ON s.id = a.session_id
                 WHERE a.synced = 0 AND s.remote_id IS NOT NULL
                 ORDER BY a.sampled_at LIMIT 200"
            )?;
            let out = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, i64>(6)?,
                ))
            })?.collect::<Result<Vec<_>, _>>()?;
            Ok(out)
        })?;
        if rows.is_empty() {
            break;
        }

        // Group by remote session id
        let mut groups: std::collections::HashMap<String, Vec<ActivityItem>> = Default::default();
        let mut local_ids: Vec<String> = Vec::with_capacity(rows.len());
        for (id, _local_sid, remote_sid, sampled_at, idle, kb, mo) in rows {
            local_ids.push(id.clone());
            groups
                .entry(remote_sid)
                .or_default()
                .push((id, sampled_at, idle, kb, mo));
        }

        let client = reqwest::Client::new();
        let mut acknowledged: Vec<String> = Vec::new();
        for (remote_sid, items) in groups {
            let samples: Vec<serde_json::Value> = items
                .iter()
                .map(|(_, sampled_at, idle, kb, mo)| {
                    json!({
                        "sampled_at": sampled_at,
                        "state": if *idle >= 120 { "idle" } else { "active" },
                        "idle_seconds": idle,
                        "keyboard_events": kb,
                        "mouse_events": mo,
                    })
                })
                .collect();
            let body = json!({ "session_id": remote_sid, "samples": samples });
            let resp = client
                .post(format!("{}/api/v1/activity", server.trim_end_matches('/')))
                .bearer_auth(token)
                .json(&body)
                .send()
                .await
                .context("post activity")?;
            if !resp.status().is_success() {
                return Err(anyhow!("activity sync HTTP {}", resp.status()));
            }
            for (id, _, _, _, _) in items {
                acknowledged.push(id);
            }
        }

        state.db.with(|c| {
            for id in &acknowledged {
                c.execute(
                    "UPDATE activity_samples SET synced = 1 WHERE id = ?",
                    params![id],
                )?;
            }
            Ok(())
        })?;
    }
    Ok(())
}

async fn push_app_snapshots(state: &AppState, server: &str, token: &str) -> Result<()> {
    loop {
        let rows: Vec<AppSnapshotRow> = state.db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT a.id, s.remote_id, a.sampled_at, a.foreground_app, a.foreground_title, a.apps_json
                 FROM app_snapshots a JOIN sessions s ON s.id = a.session_id
                 WHERE a.synced = 0 AND s.remote_id IS NOT NULL
                 ORDER BY a.sampled_at LIMIT 100"
            )?;
            let out = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, String>(5)?,
                ))
            })?.collect::<Result<Vec<_>, _>>()?;
            Ok(out)
        })?;
        if rows.is_empty() {
            break;
        }

        let mut groups: std::collections::HashMap<String, Vec<AppSnapshotItem>> =
            Default::default();
        for (id, remote_sid, sampled_at, fg_app, fg_title, apps_json) in rows {
            let apps: serde_json::Value = serde_json::from_str(&apps_json).unwrap_or(json!([]));
            groups
                .entry(remote_sid)
                .or_default()
                .push((id, sampled_at, fg_app, fg_title, apps));
        }

        let client = reqwest::Client::new();
        let mut acked: Vec<String> = Vec::new();
        for (remote_sid, items) in groups {
            let snapshots: Vec<serde_json::Value> = items
                .iter()
                .map(|(_, sampled_at, fg, ftitle, apps)| {
                    json!({
                        "sampled_at": sampled_at,
                        "foreground_app": fg,
                        "foreground_title": ftitle,
                        "apps": apps,
                    })
                })
                .collect();
            let body = json!({ "session_id": remote_sid, "snapshots": snapshots });
            let resp = client
                .post(format!(
                    "{}/api/v1/app-snapshots",
                    server.trim_end_matches('/')
                ))
                .bearer_auth(token)
                .json(&body)
                .send()
                .await
                .context("post app snapshots")?;
            if !resp.status().is_success() {
                return Err(anyhow!("app snapshot sync HTTP {}", resp.status()));
            }
            for (id, _, _, _, _) in items {
                acked.push(id);
            }
        }

        state.db.with(|c| {
            for id in &acked {
                c.execute(
                    "UPDATE app_snapshots SET synced = 1 WHERE id = ?",
                    params![id],
                )?;
            }
            Ok(())
        })?;
    }
    Ok(())
}

async fn push_screenshots(state: &AppState, server: &str, token: &str) -> Result<()> {
    let rows: Vec<(String, String, String, String)> = state.db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT sh.id, s.remote_id, sh.captured_at, sh.file_path
             FROM screenshots sh JOIN sessions s ON s.id = sh.session_id
             WHERE sh.synced = 0 AND s.remote_id IS NOT NULL
             ORDER BY sh.captured_at LIMIT 20",
        )?;
        let out = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(out)
    })?;

    let client = reqwest::Client::new();
    for (id, remote_sid, captured_at, file_path) in rows {
        let full = state.screenshot_dir.join(&file_path);
        let bytes = match tokio::fs::read(&full).await {
            Ok(b) => b,
            Err(e) => {
                log::warn!("missing screenshot file {file_path}: {e}");
                state.db.with(|c| {
                    c.execute(
                        "UPDATE screenshots SET synced = 1 WHERE id = ?",
                        params![id],
                    )?;
                    Ok(())
                })?;
                continue;
            }
        };

        let filename = format!("{}.jpg", id);
        let part = Part::bytes(bytes)
            .file_name(filename)
            .mime_str("image/jpeg")?;
        let form = Form::new()
            .text("session_id", remote_sid)
            .text("captured_at", captured_at.clone())
            .part("image", part);

        let resp = client
            .post(format!(
                "{}/api/v1/screenshots",
                server.trim_end_matches('/')
            ))
            .bearer_auth(token)
            .multipart(form)
            .send()
            .await
            .context("post screenshot")?;
        if !resp.status().is_success() {
            return Err(anyhow!("screenshot sync HTTP {}", resp.status()));
        }

        state.db.with(|c| {
            c.execute(
                "UPDATE screenshots SET synced = 1 WHERE id = ?",
                params![id],
            )?;
            Ok(())
        })?;

        // Optional: delete local file once uploaded to save space
        let _ = tokio::fs::remove_file(&full).await;
    }
    Ok(())
}

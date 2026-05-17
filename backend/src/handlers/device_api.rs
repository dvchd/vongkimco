//! Authenticated device API used by the desktop app to sync activity data.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Multipart, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::DeviceAuth;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SessionUpsert {
    pub client_session_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub note: Option<String>,
}

#[derive(Serialize)]
pub struct SessionResp {
    pub id: String,
    pub client_session_id: String,
}

pub async fn upsert_session(
    State(state): State<Arc<AppState>>,
    auth: DeviceAuth,
    Json(body): Json<SessionUpsert>,
) -> AppResult<Json<SessionResp>> {
    // Try to find existing session for this client_session_id (idempotent sync)
    if let Some(s) = sqlx::query_as::<_, crate::models::Session>(
        "SELECT * FROM sessions WHERE client_session_id = ? AND user_id = ?",
    )
    .bind(&body.client_session_id)
    .bind(&auth.user.id)
    .fetch_optional(&state.db)
    .await?
    {
        sqlx::query("UPDATE sessions SET ended_at = COALESCE(?, ended_at), note = COALESCE(?, note) WHERE id = ?")
            .bind(&body.ended_at)
            .bind(&body.note)
            .bind(&s.id)
            .execute(&state.db)
            .await?;
        return Ok(Json(SessionResp { id: s.id, client_session_id: body.client_session_id }));
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, device_id, started_at, ended_at, note, client_session_id)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&auth.user.id)
    .bind(&auth.device_id)
    .bind(&body.started_at)
    .bind(&body.ended_at)
    .bind(&body.note)
    .bind(&body.client_session_id)
    .execute(&state.db)
    .await?;

    Ok(Json(SessionResp { id, client_session_id: body.client_session_id }))
}

#[derive(Deserialize)]
pub struct ActivityBatch {
    pub session_id: String,
    pub samples: Vec<ActivityIn>,
}

#[derive(Deserialize)]
pub struct ActivityIn {
    pub sampled_at: String,
    pub state: String,
    pub idle_seconds: i64,
    pub keyboard_events: i64,
    pub mouse_events: i64,
}

pub async fn ingest_activity(
    State(state): State<Arc<AppState>>,
    auth: DeviceAuth,
    Json(batch): Json<ActivityBatch>,
) -> AppResult<Json<Value>> {
    if batch.samples.is_empty() {
        return Ok(Json(json!({ "accepted": 0 })));
    }
    if batch.samples.len() > 1000 {
        return Err(AppError::BadRequest("batch too large".into()));
    }

    // Verify session belongs to this user
    let sess = sqlx::query_as::<_, crate::models::Session>(
        "SELECT * FROM sessions WHERE id = ? AND user_id = ?",
    )
    .bind(&batch.session_id)
    .bind(&auth.user.id)
    .fetch_optional(&state.db)
    .await?;
    let Some(sess) = sess else {
        return Err(AppError::NotFound);
    };

    let mut tx = state.db.begin().await?;
    let mut accepted = 0i64;
    for s in batch.samples {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO activity_samples (id, session_id, user_id, sampled_at, state, idle_seconds, keyboard_events, mouse_events)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&sess.id)
        .bind(&auth.user.id)
        .bind(&s.sampled_at)
        .bind(&s.state)
        .bind(s.idle_seconds)
        .bind(s.keyboard_events)
        .bind(s.mouse_events)
        .execute(&mut *tx)
        .await?;
        accepted += 1;
    }
    tx.commit().await?;

    Ok(Json(json!({ "accepted": accepted })))
}

#[derive(Deserialize)]
pub struct AppSnapshotBatch {
    pub session_id: String,
    pub snapshots: Vec<AppSnapIn>,
}

#[derive(Deserialize)]
pub struct AppSnapIn {
    pub sampled_at: String,
    pub foreground_app: Option<String>,
    pub foreground_title: Option<String>,
    pub apps: Vec<String>,
}

pub async fn ingest_app_snapshots(
    State(state): State<Arc<AppState>>,
    auth: DeviceAuth,
    Json(batch): Json<AppSnapshotBatch>,
) -> AppResult<Json<Value>> {
    if batch.snapshots.is_empty() {
        return Ok(Json(json!({ "accepted": 0 })));
    }
    if batch.snapshots.len() > 500 {
        return Err(AppError::BadRequest("batch too large".into()));
    }

    let sess = sqlx::query_as::<_, crate::models::Session>(
        "SELECT * FROM sessions WHERE id = ? AND user_id = ?",
    )
    .bind(&batch.session_id)
    .bind(&auth.user.id)
    .fetch_optional(&state.db)
    .await?;
    let Some(sess) = sess else {
        return Err(AppError::NotFound);
    };

    let mut tx = state.db.begin().await?;
    let mut accepted = 0i64;
    for snap in batch.snapshots {
        let id = uuid::Uuid::new_v4().to_string();
        let apps_json = serde_json::to_string(&snap.apps).unwrap_or("[]".into());
        sqlx::query(
            "INSERT INTO app_snapshots (id, session_id, user_id, sampled_at, foreground_app, foreground_title, apps_json)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&sess.id)
        .bind(&auth.user.id)
        .bind(&snap.sampled_at)
        .bind(&snap.foreground_app)
        .bind(&snap.foreground_title)
        .bind(&apps_json)
        .execute(&mut *tx)
        .await?;
        accepted += 1;
    }
    tx.commit().await?;

    Ok(Json(json!({ "accepted": accepted })))
}

/// POST /api/v1/screenshots — multipart upload
/// fields: session_id (text), captured_at (text), image (file: jpeg)
pub async fn upload_screenshot(
    State(state): State<Arc<AppState>>,
    auth: DeviceAuth,
    mut multipart: Multipart,
) -> AppResult<Json<Value>> {
    let mut session_id: Option<String> = None;
    let mut captured_at: Option<String> = None;
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut mime = "image/jpeg".to_string();

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "session_id" => {
                session_id = Some(field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "captured_at" => {
                captured_at = Some(field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "image" => {
                if let Some(ct) = field.content_type() {
                    mime = ct.to_string();
                }
                let bytes = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                if bytes.len() > state.config.max_screenshot_bytes {
                    return Err(AppError::BadRequest("image too large".into()));
                }
                image_bytes = Some(bytes.to_vec());
            }
            _ => {}
        }
    }

    let Some(session_id) = session_id else { return Err(AppError::BadRequest("session_id required".into())) };
    let Some(captured_at) = captured_at else { return Err(AppError::BadRequest("captured_at required".into())) };
    let Some(image) = image_bytes else { return Err(AppError::BadRequest("image required".into())) };

    let sess = sqlx::query_as::<_, crate::models::Session>(
        "SELECT * FROM sessions WHERE id = ? AND user_id = ?",
    )
    .bind(&session_id)
    .bind(&auth.user.id)
    .fetch_optional(&state.db)
    .await?;
    let Some(sess) = sess else {
        return Err(AppError::NotFound);
    };

    let id = uuid::Uuid::new_v4().to_string();
    let ext = match mime.as_str() {
        "image/png" => "png",
        "image/webp" => "webp",
        _ => "jpg",
    };

    let date_dir = chrono::Utc::now().format("%Y/%m/%d").to_string();
    let dir: PathBuf = state.config.screenshot_dir.join(&auth.user.id).join(&date_dir);
    tokio::fs::create_dir_all(&dir).await.map_err(|e| AppError::Internal(e.to_string()))?;
    let filename = format!("{}.{}", id, ext);
    let full_path = dir.join(&filename);
    tokio::fs::write(&full_path, &image).await.map_err(|e| AppError::Internal(e.to_string()))?;

    let rel_path = full_path
        .strip_prefix(&state.config.screenshot_dir)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| full_path.to_string_lossy().to_string());

    sqlx::query(
        "INSERT INTO screenshots (id, session_id, user_id, captured_at, file_path, bytes, mime)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&sess.id)
    .bind(&auth.user.id)
    .bind(&captured_at)
    .bind(&rel_path)
    .bind(image.len() as i64)
    .bind(&mime)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "id": id, "bytes": image.len() })))
}

pub async fn whoami(auth: DeviceAuth) -> Json<Value> {
    Json(json!({
        "user": {
            "id": auth.user.id,
            "email": auth.user.email,
            "name": auth.user.name,
            "picture": auth.user.picture,
            "is_admin": auth.user.is_admin_bool(),
        },
        "device_id": auth.device_id,
    }))
}

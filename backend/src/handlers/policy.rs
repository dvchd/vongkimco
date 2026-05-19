//! Device policy: the single source of truth for runtime knobs the admin
//! controls (collection intervals, screenshot toggle, …). Stored as a single
//! row in `device_policy`. Desktop clients GET /api/v1/config to fetch it and
//! re-poll every `refresh_interval_secs`.
//!
//! Local desktop fields (server_url, hotkeys, autostart) intentionally stay
//! on each machine and are not part of this policy.

use std::collections::HashMap;
use std::sync::Arc;

use askama::Template;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Form, Json};
use serde::{Deserialize, Serialize};

use crate::auth::{AdminUser, DeviceAuth};
use crate::error::AppResult;
use crate::state::AppState;
use crate::time_fmt::TsCell;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DevicePolicy {
    pub capture_screenshots: i64,
    pub screenshot_interval_secs: i64,
    pub activity_sample_interval_secs: i64,
    pub app_snapshot_interval_secs: i64,
    pub idle_threshold_secs: i64,
    pub screenshot_quality: i64,
    pub screenshot_max_width: i64,
    pub refresh_interval_secs: i64,
    pub updated_at: String,
}

async fn load(state: &AppState) -> AppResult<DevicePolicy> {
    let row = sqlx::query_as::<_, DevicePolicy>(
        "SELECT capture_screenshots, screenshot_interval_secs, activity_sample_interval_secs,
                app_snapshot_interval_secs, idle_threshold_secs, screenshot_quality,
                screenshot_max_width, refresh_interval_secs, updated_at
         FROM device_policy WHERE id = 1",
    )
    .fetch_one(&state.db)
    .await?;
    Ok(row)
}

/// GET /api/v1/config — desktop pulls policy. `version` is the row's
/// `updated_at`; client can compare to skip applying unchanged config.
pub async fn get_config(
    State(state): State<Arc<AppState>>,
    _auth: DeviceAuth,
) -> AppResult<Json<serde_json::Value>> {
    let p = load(&state).await?;
    Ok(Json(serde_json::json!({
        "version": p.updated_at,
        "policy": {
            "capture_screenshots": p.capture_screenshots != 0,
            "screenshot_interval_secs": p.screenshot_interval_secs,
            "activity_sample_interval_secs": p.activity_sample_interval_secs,
            "app_snapshot_interval_secs": p.app_snapshot_interval_secs,
            "idle_threshold_secs": p.idle_threshold_secs,
            "screenshot_quality": p.screenshot_quality,
            "screenshot_max_width": p.screenshot_max_width,
            "refresh_interval_secs": p.refresh_interval_secs,
        }
    })))
}

// ---------- Admin page ----------

#[derive(Template)]
#[template(path = "admin_policy.html")]
pub struct PolicyTemplate {
    pub user_email: String,
    pub policy: DevicePolicy,
    pub updated_at_cell: TsCell,
    pub flash: Option<String>,
}

pub async fn admin_page(
    State(state): State<Arc<AppState>>,
    AdminUser(u): AdminUser,
    Query(q): Query<HashMap<String, String>>,
) -> AppResult<Response> {
    let p = load(&state).await?;
    let tz = state.config.app_timezone;
    let flash = q
        .get("saved")
        .map(|_| "Đã lưu. Các desktop sẽ áp dụng cấu hình mới trong vòng vài phút.".to_string());
    Ok(PolicyTemplate {
        user_email: u.email,
        updated_at_cell: TsCell::new(&p.updated_at, tz),
        policy: p,
        flash,
    }
    .into_response())
}

#[derive(Deserialize)]
pub struct PolicyForm {
    #[serde(default)]
    pub capture_screenshots: Option<String>,
    pub screenshot_interval_secs: i64,
    pub activity_sample_interval_secs: i64,
    pub app_snapshot_interval_secs: i64,
    pub idle_threshold_secs: i64,
    pub screenshot_quality: i64,
    pub screenshot_max_width: i64,
    pub refresh_interval_secs: i64,
}

pub async fn admin_save(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    Form(form): Form<PolicyForm>,
) -> AppResult<Response> {
    // HTML checkboxes only POST when checked → absence means unchecked.
    let capture = form.capture_screenshots.is_some() as i64;

    // Clamp every numeric field server-side so a stray value can't brick
    // desktops (e.g. interval=0 → tight loop, max_width=10 → unreadable).
    let shot_interval = form.screenshot_interval_secs.clamp(30, 3600);
    let activity_interval = form.activity_sample_interval_secs.clamp(5, 300);
    let app_interval = form.app_snapshot_interval_secs.clamp(10, 600);
    let idle_threshold = form.idle_threshold_secs.clamp(30, 1800);
    let quality = form.screenshot_quality.clamp(20, 95);
    let max_width = form.screenshot_max_width.clamp(640, 3840);
    let refresh = form.refresh_interval_secs.clamp(60, 3600);

    sqlx::query(
        "UPDATE device_policy SET
            capture_screenshots = ?,
            screenshot_interval_secs = ?,
            activity_sample_interval_secs = ?,
            app_snapshot_interval_secs = ?,
            idle_threshold_secs = ?,
            screenshot_quality = ?,
            screenshot_max_width = ?,
            refresh_interval_secs = ?,
            updated_at = datetime('now'),
            updated_by = ?
         WHERE id = 1",
    )
    .bind(capture)
    .bind(shot_interval)
    .bind(activity_interval)
    .bind(app_interval)
    .bind(idle_threshold)
    .bind(quality)
    .bind(max_width)
    .bind(refresh)
    .bind(&admin.id)
    .execute(&state.db)
    .await?;

    Ok(Redirect::to("/admin/policy?saved=1").into_response())
}

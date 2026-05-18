//! Admin web UI handlers (Askama templates).

use std::path::PathBuf;
use std::sync::Arc;

use askama::Template;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::auth::{AdminUser, CurrentUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub reason: Option<String>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub user_email: String,
    pub total_users: i64,
    pub total_sessions: i64,
    pub total_screenshots: i64,
    pub active_sessions: i64,
    pub recent_sessions: Vec<SessionRow>,
}

#[derive(Template)]
#[template(path = "users.html")]
pub struct UsersTemplate {
    pub user_email: String,
    pub users: Vec<UserRow>,
}

#[derive(Template)]
#[template(path = "user_detail.html")]
pub struct UserDetailTemplate {
    pub user_email: String,
    pub viewing: UserRow,
    pub sessions: Vec<SessionRow>,
}

#[derive(Template)]
#[template(path = "session_detail.html")]
pub struct SessionDetailTemplate {
    pub user_email: String,
    pub session: SessionRow,
    pub owner_email: String,
    pub activity_summary: ActivitySummary,
    pub recent_apps: Vec<AppRow>,
    pub screenshots: Vec<ScreenshotRow>,
}

#[derive(Template)]
#[template(path = "screenshots.html")]
pub struct ScreenshotsTemplate {
    pub user_email: String,
    pub screenshots: Vec<ScreenshotRow>,
}

#[derive(Template)]
#[template(path = "admin_members.html")]
pub struct AdminMembersTemplate {
    pub user_email: String,
    pub env_members: Vec<String>,
    pub db_members: Vec<DbMemberRow>,
    pub pending_requests: Vec<crate::handlers::membership::PendingRequestRow>,
    pub flash: Option<String>,
}

pub struct DbMemberRow {
    pub email: String,
    pub note: String,
    pub created_at: String,
    pub added_by_email: String,
    pub has_user: bool,
}

pub struct SessionRow {
    pub id: String,
    pub user_email: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration: String,
    pub note: String,
}

pub struct UserRow {
    pub id: String,
    pub email: String,
    pub name: String,
    pub is_admin: bool,
    pub session_count: i64,
    pub last_seen: String,
}

pub struct ActivitySummary {
    pub total_samples: i64,
    pub active_samples: i64,
    pub idle_samples: i64,
    pub active_pct: i64,
    pub keyboard_events: i64,
    pub mouse_events: i64,
}

pub struct AppRow {
    pub sampled_at: String,
    pub foreground_app: String,
    pub foreground_title: String,
}

pub struct ScreenshotRow {
    pub id: String,
    pub captured_at: String,
    pub bytes: i64,
    pub session_id: String,
    pub user_email: String,
}

fn fmt_duration(seconds: i64) -> String {
    if seconds < 0 {
        return "—".into();
    }
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}

fn parse_dt(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|n| chrono::DateTime::from_naive_utc_and_offset(n, chrono::Utc))
        })
}

pub async fn login_page(Query(q): Query<std::collections::HashMap<String, String>>) -> Response {
    LoginTemplate {
        reason: q.get("reason").cloned(),
    }
    .into_response()
}

pub async fn admin_root(
    State(state): State<Arc<AppState>>,
    CurrentUser(u): CurrentUser,
) -> AppResult<Response> {
    if !u.is_admin_bool() {
        return Err(AppError::Forbidden);
    }

    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    let total_sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
        .fetch_one(&state.db)
        .await?;
    let total_screenshots: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM screenshots")
        .fetch_one(&state.db)
        .await?;
    let active_sessions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE ended_at IS NULL")
            .fetch_one(&state.db)
            .await?;

    let rows = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>)>(
        "SELECT s.id, u.email, s.started_at, s.ended_at, s.note
         FROM sessions s JOIN users u ON u.id = s.user_id
         ORDER BY s.started_at DESC LIMIT 25",
    )
    .fetch_all(&state.db)
    .await?;

    let recent_sessions: Vec<SessionRow> = rows
        .into_iter()
        .map(|(id, email, started_at, ended_at, note)| {
            let dur = match (
                parse_dt(&started_at),
                ended_at.as_ref().and_then(|s| parse_dt(s)),
            ) {
                (Some(a), Some(b)) => fmt_duration((b - a).num_seconds()),
                (Some(a), None) => format!(
                    "ongoing ({})",
                    fmt_duration((chrono::Utc::now() - a).num_seconds())
                ),
                _ => "—".into(),
            };
            SessionRow {
                id,
                user_email: email,
                started_at,
                ended_at: ended_at.unwrap_or_else(|| "—".into()),
                duration: dur,
                note: note.unwrap_or_default(),
            }
        })
        .collect();

    Ok(DashboardTemplate {
        user_email: u.email,
        total_users,
        total_sessions,
        total_screenshots,
        active_sessions,
        recent_sessions,
    }
    .into_response())
}

pub async fn users_list(
    State(state): State<Arc<AppState>>,
    AdminUser(u): AdminUser,
) -> AppResult<Response> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>, i64, i64, Option<String>)>(
        "SELECT u.id, u.email, u.name, u.is_admin,
                (SELECT COUNT(*) FROM sessions s WHERE s.user_id = u.id) AS session_count,
                (SELECT MAX(s.started_at) FROM sessions s WHERE s.user_id = u.id) AS last_seen
         FROM users u
         ORDER BY u.created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let users: Vec<UserRow> = rows
        .into_iter()
        .map(|(id, email, name, is_admin, count, last)| UserRow {
            id,
            email,
            name: name.unwrap_or_default(),
            is_admin: is_admin != 0,
            session_count: count,
            last_seen: last.unwrap_or_else(|| "—".into()),
        })
        .collect();

    Ok(UsersTemplate {
        user_email: u.email,
        users,
    }
    .into_response())
}

pub async fn user_detail(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    Path(user_id): Path<String>,
) -> AppResult<Response> {
    let target = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    let session_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = ?")
        .bind(&target.id)
        .fetch_one(&state.db)
        .await?;
    let last_seen: Option<String> =
        sqlx::query_scalar("SELECT MAX(started_at) FROM sessions WHERE user_id = ?")
            .bind(&target.id)
            .fetch_one(&state.db)
            .await?;

    let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<String>)>(
        "SELECT id, started_at, ended_at, note FROM sessions WHERE user_id = ? ORDER BY started_at DESC LIMIT 100",
    )
    .bind(&target.id)
    .fetch_all(&state.db)
    .await?;

    let sessions: Vec<SessionRow> = rows
        .into_iter()
        .map(|(id, started_at, ended_at, note)| {
            let dur = match (
                parse_dt(&started_at),
                ended_at.as_ref().and_then(|s| parse_dt(s)),
            ) {
                (Some(a), Some(b)) => fmt_duration((b - a).num_seconds()),
                (Some(a), None) => format!(
                    "ongoing ({})",
                    fmt_duration((chrono::Utc::now() - a).num_seconds())
                ),
                _ => "—".into(),
            };
            SessionRow {
                id,
                user_email: target.email.clone(),
                started_at,
                ended_at: ended_at.unwrap_or_else(|| "—".into()),
                duration: dur,
                note: note.unwrap_or_default(),
            }
        })
        .collect();

    let viewing = UserRow {
        id: target.id.clone(),
        email: target.email.clone(),
        name: target.name.clone().unwrap_or_default(),
        is_admin: target.is_admin_bool(),
        session_count,
        last_seen: last_seen.unwrap_or_else(|| "—".into()),
    };

    Ok(UserDetailTemplate {
        user_email: admin.email,
        viewing,
        sessions,
    }
    .into_response())
}

pub async fn session_detail(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    Path(session_id): Path<String>,
) -> AppResult<Response> {
    let s = sqlx::query_as::<_, crate::models::Session>("SELECT * FROM sessions WHERE id = ?")
        .bind(&session_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    let owner = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = ?")
        .bind(&s.user_id)
        .fetch_one(&state.db)
        .await?;

    let dur = match (
        parse_dt(&s.started_at),
        s.ended_at.as_ref().and_then(|x| parse_dt(x)),
    ) {
        (Some(a), Some(b)) => fmt_duration((b - a).num_seconds()),
        (Some(a), None) => format!(
            "ongoing ({})",
            fmt_duration((chrono::Utc::now() - a).num_seconds())
        ),
        _ => "—".into(),
    };

    let session = SessionRow {
        id: s.id.clone(),
        user_email: owner.email.clone(),
        started_at: s.started_at.clone(),
        ended_at: s.ended_at.clone().unwrap_or_else(|| "—".into()),
        duration: dur,
        note: s.note.clone().unwrap_or_default(),
    };

    let (total, active, kb, mo): (i64, i64, Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT COUNT(*),
                COALESCE(SUM(CASE WHEN state = 'active' THEN 1 ELSE 0 END), 0),
                SUM(keyboard_events),
                SUM(mouse_events)
         FROM activity_samples WHERE session_id = ?",
    )
    .bind(&s.id)
    .fetch_one(&state.db)
    .await?;
    let idle = total - active;
    let active_pct = if total > 0 { (active * 100) / total } else { 0 };

    let app_rows = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
        "SELECT sampled_at, foreground_app, foreground_title FROM app_snapshots
         WHERE session_id = ? ORDER BY sampled_at DESC LIMIT 50",
    )
    .bind(&s.id)
    .fetch_all(&state.db)
    .await?;
    let recent_apps: Vec<AppRow> = app_rows
        .into_iter()
        .map(|(t, app, title)| AppRow {
            sampled_at: t,
            foreground_app: app.unwrap_or_default(),
            foreground_title: title.unwrap_or_default(),
        })
        .collect();

    let shot_rows = sqlx::query_as::<_, (String, String, Option<i64>)>(
        "SELECT id, captured_at, bytes FROM screenshots
         WHERE session_id = ? ORDER BY captured_at DESC LIMIT 100",
    )
    .bind(&s.id)
    .fetch_all(&state.db)
    .await?;
    let screenshots: Vec<ScreenshotRow> = shot_rows
        .into_iter()
        .map(|(id, t, bytes)| ScreenshotRow {
            id,
            captured_at: t,
            bytes: bytes.unwrap_or(0),
            session_id: s.id.clone(),
            user_email: owner.email.clone(),
        })
        .collect();

    Ok(SessionDetailTemplate {
        user_email: admin.email,
        session,
        owner_email: owner.email,
        activity_summary: ActivitySummary {
            total_samples: total,
            active_samples: active,
            idle_samples: idle,
            active_pct,
            keyboard_events: kb.unwrap_or(0),
            mouse_events: mo.unwrap_or(0),
        },
        recent_apps,
        screenshots,
    }
    .into_response())
}

pub async fn screenshots_list(
    State(state): State<Arc<AppState>>,
    AdminUser(u): AdminUser,
) -> AppResult<Response> {
    let rows = sqlx::query_as::<_, (String, String, Option<i64>, String, String)>(
        "SELECT s.id, s.captured_at, s.bytes, s.session_id, u.email
         FROM screenshots s JOIN users u ON u.id = s.user_id
         ORDER BY s.captured_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await?;

    let screenshots: Vec<ScreenshotRow> = rows
        .into_iter()
        .map(|(id, t, bytes, sid, email)| ScreenshotRow {
            id,
            captured_at: t,
            bytes: bytes.unwrap_or(0),
            session_id: sid,
            user_email: email,
        })
        .collect();

    Ok(ScreenshotsTemplate {
        user_email: u.email,
        screenshots,
    }
    .into_response())
}

#[derive(Deserialize)]
pub struct ScreenshotQuery {
    pub _v: Option<String>, // cache-buster
}

pub async fn screenshot_file(
    State(state): State<Arc<AppState>>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<String>,
    Query(_q): Query<ScreenshotQuery>,
) -> AppResult<Response> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT file_path, mime FROM screenshots WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let full: PathBuf = state.config.screenshot_dir.join(&row.0);
    let bytes = tokio::fs::read(&full)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, row.1)
        .header(header::CACHE_CONTROL, "private, max-age=300")
        .body(Body::from(bytes))
        .unwrap())
}

// ---------- Members management ----------

pub async fn members_page(
    State(state): State<Arc<AppState>>,
    AdminUser(u): AdminUser,
) -> AppResult<Response> {
    members_page_with_flash(&state, u, None).await
}

async fn members_page_with_flash(
    state: &Arc<AppState>,
    admin: crate::models::User,
    flash: Option<String>,
) -> AppResult<Response> {
    let env_members = state.config.member_emails.clone();

    let rows = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT m.email, m.note, m.created_at, m.added_by, u_added.email AS added_by_email
         FROM allowed_members m
         LEFT JOIN users u_added ON u_added.id = m.added_by
         ORDER BY m.created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let mut db_members: Vec<DbMemberRow> = Vec::with_capacity(rows.len());
    for (email, note, created_at, _added_by, added_by_email) in rows {
        let has_user: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = ?")
            .bind(&email)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
        db_members.push(DbMemberRow {
            email,
            note: note.unwrap_or_default(),
            created_at,
            added_by_email: added_by_email.unwrap_or_else(|| "—".into()),
            has_user: has_user > 0,
        });
    }

    let pending_requests = crate::handlers::membership::list_pending(state).await?;

    Ok(AdminMembersTemplate {
        user_email: admin.email,
        env_members,
        db_members,
        pending_requests,
        flash,
    }
    .into_response())
}

#[derive(Deserialize)]
pub struct AddMemberForm {
    pub email: String,
    #[serde(default)]
    pub note: String,
}

pub async fn members_add(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    axum::Form(form): axum::Form<AddMemberForm>,
) -> AppResult<Response> {
    let lc = form.email.trim().to_lowercase();
    if lc.is_empty() || !lc.contains('@') {
        return members_page_with_flash(&state, admin, Some("Email không hợp lệ".into())).await;
    }
    let note = form.note.trim();
    sqlx::query(
        "INSERT OR REPLACE INTO allowed_members (email, note, added_by, created_at)
         VALUES (?, ?, ?, COALESCE((SELECT created_at FROM allowed_members WHERE email = ?), datetime('now')))",
    )
    .bind(&lc)
    .bind(if note.is_empty() { None } else { Some(note) })
    .bind(&admin.id)
    .bind(&lc)
    .execute(&state.db)
    .await?;

    // If the user has already signed in once, flip their is_member now.
    let _ =
        sqlx::query("UPDATE users SET is_member = 1, updated_at = datetime('now') WHERE email = ?")
            .bind(&lc)
            .execute(&state.db)
            .await;

    members_page_with_flash(
        &state,
        admin,
        Some(format!("Đã thêm {} vào allow-list", lc)),
    )
    .await
}

#[derive(Deserialize)]
pub struct RemoveMemberForm {
    pub email: String,
}

pub async fn members_remove(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    axum::Form(form): axum::Form<RemoveMemberForm>,
) -> AppResult<Response> {
    let lc = form.email.trim().to_lowercase();
    if lc.is_empty() {
        return members_page_with_flash(&state, admin, Some("Email rỗng".into())).await;
    }
    sqlx::query("DELETE FROM allowed_members WHERE email = ?")
        .bind(&lc)
        .execute(&state.db)
        .await?;

    // Refresh membership for that user (might still be a member via env var).
    let user_id: Option<String> = sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
        .bind(&lc)
        .fetch_optional(&state.db)
        .await?;
    if let Some(uid) = user_id {
        let _ = crate::auth::refresh_user_membership(&state, &uid).await;
    }

    members_page_with_flash(&state, admin, Some(format!("Đã gỡ {}", lc))).await
}

//! Membership request flow.
//!
//! Guests (logged in but not on the allow-list) can submit a request via
//! /pending. Admins approve/reject from /admin/members. Approval adds the
//! email to `allowed_members` and flips `users.is_member`.

use std::sync::Arc;

use askama::Template;
use axum::extract::{Form, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::auth::{AdminUser, CurrentUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

const MAX_NOTE: usize = 1000;

#[derive(Template)]
#[template(path = "pending.html")]
pub struct PendingTemplate {
    pub user_email: String,
    pub maintainer_facebook: String,
    pub maintainer_email: String,
    pub existing: Option<RequestRow>,
    pub flash: Option<String>,
}

pub struct RequestRow {
    pub status: String,
    pub status_label: String,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
    pub decided_at: String,
}

fn status_label(s: &str) -> &'static str {
    match s {
        "pending" => "Đang chờ duyệt",
        "approved" => "Đã được duyệt",
        "rejected" => "Đã bị từ chối",
        _ => "Không rõ",
    }
}

async fn fetch_request(state: &AppState, user_id: &str) -> AppResult<Option<RequestRow>> {
    let row = sqlx::query_as::<_, (String, Option<String>, String, String, Option<String>)>(
        "SELECT status, note, created_at, updated_at, decided_at
         FROM membership_requests WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;
    Ok(row.map(|(status, note, c, u, d)| RequestRow {
        status_label: status_label(&status).to_string(),
        status,
        note: note.unwrap_or_default(),
        created_at: c,
        updated_at: u,
        decided_at: d.unwrap_or_else(|| "—".into()),
    }))
}

/// GET /pending — membership status + request form for non-members.
pub async fn pending_page(
    State(state): State<Arc<AppState>>,
    CurrentUser(u): CurrentUser,
) -> AppResult<Response> {
    pending_page_with_flash(&state, u, None).await
}

async fn pending_page_with_flash(
    state: &Arc<AppState>,
    u: crate::models::User,
    flash: Option<String>,
) -> AppResult<Response> {
    // Admins shouldn't be here.
    if u.is_admin_bool() {
        return Ok(Redirect::to("/admin").into_response());
    }
    let existing = fetch_request(state, &u.id).await?;

    Ok(PendingTemplate {
        user_email: u.email,
        maintainer_facebook: state.config.maintainer_facebook.clone(),
        maintainer_email: state.config.maintainer_email.clone(),
        existing,
        flash,
    }
    .into_response())
}

#[derive(Deserialize)]
pub struct RequestForm {
    #[serde(default)]
    pub note: String,
}

/// POST /membership/request — guest submits or updates their request.
pub async fn submit_request(
    State(state): State<Arc<AppState>>,
    CurrentUser(u): CurrentUser,
    Form(form): Form<RequestForm>,
) -> AppResult<Response> {
    if u.is_member_bool() {
        return Ok(Redirect::to("/feedback").into_response());
    }
    let note = form.note.trim();
    if note.len() > MAX_NOTE {
        return Err(AppError::BadRequest(format!(
            "Lời nhắn quá dài (tối đa {} ký tự)",
            MAX_NOTE
        )));
    }
    let note_opt: Option<&str> = if note.is_empty() { None } else { Some(note) };

    // Upsert: re-submitting resets status to pending, updates note.
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO membership_requests (id, user_id, note, status)
         VALUES (?, ?, ?, 'pending')
         ON CONFLICT(user_id) DO UPDATE SET
            note = excluded.note,
            status = 'pending',
            decided_by = NULL,
            decided_at = NULL,
            updated_at = datetime('now')",
    )
    .bind(&id)
    .bind(&u.id)
    .bind(note_opt)
    .execute(&state.db)
    .await?;

    pending_page_with_flash(
        &state,
        u,
        Some("Đã gửi yêu cầu. Quản trị viên sẽ xem xét sớm.".into()),
    )
    .await
}

// ---------- Admin actions ----------

#[derive(Deserialize)]
pub struct DecisionForm {
    pub request_id: String,
}

pub async fn approve(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    Form(form): Form<DecisionForm>,
) -> AppResult<Response> {
    decide(&state, &admin, &form.request_id, true).await
}

pub async fn reject(
    State(state): State<Arc<AppState>>,
    AdminUser(admin): AdminUser,
    Form(form): Form<DecisionForm>,
) -> AppResult<Response> {
    decide(&state, &admin, &form.request_id, false).await
}

async fn decide(
    state: &Arc<AppState>,
    admin: &crate::models::User,
    request_id: &str,
    approve: bool,
) -> AppResult<Response> {
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT r.user_id, u.email, r.status
         FROM membership_requests r JOIN users u ON u.id = r.user_id
         WHERE r.id = ?",
    )
    .bind(request_id)
    .fetch_optional(&state.db)
    .await?;
    let Some((user_id, email, current_status)) = row else {
        return Err(AppError::NotFound);
    };
    if current_status != "pending" {
        return Err(AppError::BadRequest("Yêu cầu đã được xử lý".into()));
    }

    if approve {
        let lc_email = email.to_lowercase();
        sqlx::query(
            "INSERT OR IGNORE INTO allowed_members (email, note, added_by)
             VALUES (?, 'approved via request', ?)",
        )
        .bind(&lc_email)
        .bind(&admin.id)
        .execute(&state.db)
        .await?;

        sqlx::query("UPDATE users SET is_member = 1, updated_at = datetime('now') WHERE id = ?")
            .bind(&user_id)
            .execute(&state.db)
            .await?;
    }

    let new_status = if approve { "approved" } else { "rejected" };
    sqlx::query(
        "UPDATE membership_requests
         SET status = ?, decided_by = ?, decided_at = datetime('now'), updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(new_status)
    .bind(&admin.id)
    .bind(request_id)
    .execute(&state.db)
    .await?;

    Ok(Redirect::to("/admin/members").into_response())
}

/// Convenience used by admin_members.html to list pending requests inline.
pub async fn list_pending(state: &AppState) -> AppResult<Vec<PendingRequestRow>> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>, String, String)>(
        "SELECT r.id, u.email, r.note, r.created_at, r.updated_at
         FROM membership_requests r JOIN users u ON u.id = r.user_id
         WHERE r.status = 'pending'
         ORDER BY r.created_at ASC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, email, note, created, _updated)| PendingRequestRow {
            id,
            email,
            note: note.unwrap_or_default(),
            created_at: created,
        })
        .collect())
}

pub struct PendingRequestRow {
    pub id: String,
    pub email: String,
    pub note: String,
    pub created_at: String,
}

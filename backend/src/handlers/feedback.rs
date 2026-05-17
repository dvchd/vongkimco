//! Feedback / bug-report / feature-request board.
//!
//! Members (logged-in via Google + on the allow-list) can create text-only
//! posts and see admin replies. Admins see every post and can reply + change
//! status. Image uploads are intentionally not supported — for screenshots,
//! users are directed to email / Facebook as documented in the UI.

use std::sync::Arc;

use askama::Template;
use axum::extract::{Form, Path, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::auth::{AdminUser, CurrentUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

const MAX_BODY: usize = 4000;
const MAX_TITLE: usize = 200;

#[derive(Template)]
#[template(path = "feedback.html")]
pub struct FeedbackListTemplate {
    pub user_email: String,
    pub is_admin: bool,
    pub is_member: bool,
    pub posts: Vec<PostRow>,
    pub maintainer_facebook: String,
    pub maintainer_email: String,
}

#[derive(Template)]
#[template(path = "feedback_detail.html")]
pub struct FeedbackDetailTemplate {
    pub user_email: String,
    pub is_admin: bool,
    pub post: PostRow,
    pub author_email: String,
    pub replies: Vec<ReplyRow>,
    pub can_reply: bool,
}

#[derive(Template)]
#[template(path = "admin_feedback.html")]
pub struct AdminFeedbackTemplate {
    pub user_email: String,
    pub posts: Vec<PostRow>,
    pub status_filter: String,
}


pub struct PostRow {
    pub id: String,
    pub author_email: String,
    pub category: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub status_label: String,
    pub category_label: String,
    pub created_at: String,
    pub reply_count: i64,
}

pub struct ReplyRow {
    pub author_email: String,
    pub body: String,
    pub created_at: String,
    pub is_admin_reply: bool,
}

fn category_label(c: &str) -> &'static str {
    match c {
        "bug" => "🐛 Báo lỗi",
        "feature" => "✨ Đề xuất",
        _ => "💬 Bình luận",
    }
}

fn status_label(s: &str) -> &'static str {
    match s {
        "open" => "Đang mở",
        "in_progress" => "Đang xử lý",
        "resolved" => "Đã giải quyết",
        "wontfix" => "Không xử lý",
        _ => s,
    }
}

fn truncate(body: &str, limit: usize) -> String {
    if body.chars().count() <= limit {
        body.to_string()
    } else {
        let s: String = body.chars().take(limit).collect();
        format!("{}…", s)
    }
}

pub async fn member_list(
    State(state): State<Arc<AppState>>,
    CurrentUser(u): CurrentUser,
) -> AppResult<Response> {
    let rows = sqlx::query_as::<_, (String, String, String, Option<String>, String, String, String, i64)>(
        "SELECT p.id, p.category, p.status, p.title, p.body, p.created_at, u.email,
                (SELECT COUNT(*) FROM feedback_replies r WHERE r.post_id = p.id) AS reply_count
         FROM feedback_posts p JOIN users u ON u.id = p.user_id
         WHERE p.user_id = ?
         ORDER BY p.created_at DESC",
    )
    .bind(&u.id)
    .fetch_all(&state.db)
    .await?;

    let posts = rows
        .into_iter()
        .map(|(id, category, status, title, body, created_at, email, reply_count)| PostRow {
            id,
            author_email: email,
            category_label: category_label(&category).to_string(),
            category,
            title: title.unwrap_or_default(),
            body: truncate(&body, 200),
            status_label: status_label(&status).to_string(),
            status,
            created_at,
            reply_count,
        })
        .collect();

    Ok(FeedbackListTemplate {
        user_email: u.email,
        is_admin: u.is_admin_bool(),
        is_member: u.is_member_bool(),
        posts,
        maintainer_facebook: state.config.maintainer_facebook.clone(),
        maintainer_email: state.config.maintainer_email.clone(),
    }
    .into_response())
}

#[derive(Deserialize)]
pub struct NewPostForm {
    pub category: String,
    pub title: String,
    pub body: String,
}

pub async fn create_post(
    State(state): State<Arc<AppState>>,
    CurrentUser(u): CurrentUser,
    Form(form): Form<NewPostForm>,
) -> AppResult<Response> {
    let body = form.body.trim();
    if body.is_empty() {
        return Err(AppError::BadRequest("Nội dung không được để trống".into()));
    }
    if body.len() > MAX_BODY {
        return Err(AppError::BadRequest(format!("Nội dung tối đa {} ký tự", MAX_BODY)));
    }
    let title = form.title.trim();
    if title.len() > MAX_TITLE {
        return Err(AppError::BadRequest(format!("Tiêu đề tối đa {} ký tự", MAX_TITLE)));
    }
    let category = match form.category.as_str() {
        "bug" | "feature" | "comment" => form.category,
        _ => "comment".to_string(),
    };

    // Naive rate-limit: max 8 posts/hour per user
    let recent: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM feedback_posts
         WHERE user_id = ? AND created_at > datetime('now', '-1 hour')",
    )
    .bind(&u.id)
    .fetch_one(&state.db)
    .await?;
    if recent >= 8 {
        return Err(AppError::BadRequest(
            "Bạn đã gửi quá nhiều bài trong 1 giờ. Vui lòng thử lại sau.".into(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO feedback_posts (id, user_id, category, title, body) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&u.id)
    .bind(&category)
    .bind(if title.is_empty() { None } else { Some(title) })
    .bind(body)
    .execute(&state.db)
    .await?;

    Ok(Redirect::to(&format!("/feedback/{}", id)).into_response())
}

pub async fn detail(
    State(state): State<Arc<AppState>>,
    CurrentUser(u): CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Response> {
    detail_impl(&state, &u, &id).await
}

pub async fn admin_detail(
    State(state): State<Arc<AppState>>,
    AdminUser(u): AdminUser,
    Path(id): Path<String>,
) -> AppResult<Response> {
    detail_impl(&state, &u, &id).await
}

async fn detail_impl(
    state: &Arc<AppState>,
    viewer: &crate::models::User,
    id: &str,
) -> AppResult<Response> {
    let row = sqlx::query_as::<_, (String, String, String, Option<String>, String, String, String, String, String)>(
        "SELECT p.id, p.user_id, p.category, p.title, p.body, p.status, p.created_at, p.updated_at, u.email
         FROM feedback_posts p JOIN users u ON u.id = p.user_id WHERE p.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let (post_id, owner_id, category, title, body, status, created_at, _updated, author_email) = row;

    // Visibility: owner or admin
    if owner_id != viewer.id && !viewer.is_admin_bool() {
        return Err(AppError::Forbidden);
    }

    let reply_rows = sqlx::query_as::<_, (String, i64, String, String)>(
        "SELECT u.email, r.is_admin_reply, r.body, r.created_at
         FROM feedback_replies r JOIN users u ON u.id = r.user_id
         WHERE r.post_id = ? ORDER BY r.created_at ASC",
    )
    .bind(&post_id)
    .fetch_all(&state.db)
    .await?;

    let replies = reply_rows
        .into_iter()
        .map(|(email, is_admin, body, created)| ReplyRow {
            author_email: email,
            body,
            created_at: created,
            is_admin_reply: is_admin != 0,
        })
        .collect();

    let post = PostRow {
        id: post_id,
        author_email: author_email.clone(),
        category_label: category_label(&category).to_string(),
        category,
        title: title.unwrap_or_default(),
        body,
        status_label: status_label(&status).to_string(),
        status,
        created_at,
        reply_count: 0,
    };

    Ok(FeedbackDetailTemplate {
        user_email: viewer.email.clone(),
        is_admin: viewer.is_admin_bool(),
        post,
        author_email,
        replies,
        // Owner or admin can reply
        can_reply: owner_id == viewer.id || viewer.is_admin_bool(),
    }
    .into_response())
}

#[derive(Deserialize)]
pub struct ReplyForm {
    pub body: String,
    #[serde(default)]
    pub status: Option<String>,
}

pub async fn reply_member(
    State(state): State<Arc<AppState>>,
    CurrentUser(u): CurrentUser,
    Path(id): Path<String>,
    Form(form): Form<ReplyForm>,
) -> AppResult<Response> {
    reply_impl(&state, &u, &id, &form, false).await
}

pub async fn reply_admin(
    State(state): State<Arc<AppState>>,
    AdminUser(u): AdminUser,
    Path(id): Path<String>,
    Form(form): Form<ReplyForm>,
) -> AppResult<Response> {
    reply_impl(&state, &u, &id, &form, true).await
}

async fn reply_impl(
    state: &Arc<AppState>,
    user: &crate::models::User,
    post_id: &str,
    form: &ReplyForm,
    as_admin: bool,
) -> AppResult<Response> {
    let body = form.body.trim();
    if body.is_empty() {
        return Err(AppError::BadRequest("Phản hồi không được để trống".into()));
    }
    if body.len() > MAX_BODY {
        return Err(AppError::BadRequest(format!("Phản hồi tối đa {} ký tự", MAX_BODY)));
    }

    // Verify post exists; non-admin can only reply on own post
    let owner_id: Option<String> =
        sqlx::query_scalar("SELECT user_id FROM feedback_posts WHERE id = ?")
            .bind(post_id)
            .fetch_optional(&state.db)
            .await?;
    let Some(owner_id) = owner_id else { return Err(AppError::NotFound) };
    if !as_admin && owner_id != user.id {
        return Err(AppError::Forbidden);
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO feedback_replies (id, post_id, user_id, body, is_admin_reply)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(post_id)
    .bind(&user.id)
    .bind(body)
    .bind(as_admin as i64)
    .execute(&state.db)
    .await?;

    sqlx::query("UPDATE feedback_posts SET updated_at = datetime('now') WHERE id = ?")
        .bind(post_id)
        .execute(&state.db)
        .await?;

    if as_admin {
        if let Some(new_status) = form.status.as_deref() {
            if matches!(new_status, "open" | "in_progress" | "resolved" | "wontfix") {
                sqlx::query("UPDATE feedback_posts SET status = ? WHERE id = ?")
                    .bind(new_status)
                    .bind(post_id)
                    .execute(&state.db)
                    .await?;
            }
        }
        Ok(Redirect::to(&format!("/admin/feedback/{}", post_id)).into_response())
    } else {
        Ok(Redirect::to(&format!("/feedback/{}", post_id)).into_response())
    }
}

#[derive(Deserialize)]
pub struct AdminFeedbackQuery {
    pub status: Option<String>,
}

pub async fn admin_list(
    State(state): State<Arc<AppState>>,
    AdminUser(u): AdminUser,
    axum::extract::Query(q): axum::extract::Query<AdminFeedbackQuery>,
) -> AppResult<Response> {
    let status_filter = q.status.unwrap_or_else(|| "all".to_string());
    let (sql, bind_status) = match status_filter.as_str() {
        "open" | "in_progress" | "resolved" | "wontfix" => (
            "SELECT p.id, p.category, p.status, p.title, p.body, p.created_at, u.email,
                    (SELECT COUNT(*) FROM feedback_replies r WHERE r.post_id = p.id) AS reply_count
             FROM feedback_posts p JOIN users u ON u.id = p.user_id
             WHERE p.status = ?
             ORDER BY p.updated_at DESC LIMIT 200",
            Some(status_filter.clone()),
        ),
        _ => (
            "SELECT p.id, p.category, p.status, p.title, p.body, p.created_at, u.email,
                    (SELECT COUNT(*) FROM feedback_replies r WHERE r.post_id = p.id) AS reply_count
             FROM feedback_posts p JOIN users u ON u.id = p.user_id
             ORDER BY p.updated_at DESC LIMIT 200",
            None,
        ),
    };

    let mut q = sqlx::query_as::<_, (String, String, String, Option<String>, String, String, String, i64)>(sql);
    if let Some(s) = &bind_status {
        q = q.bind(s);
    }
    let rows = q.fetch_all(&state.db).await?;

    let posts = rows
        .into_iter()
        .map(|(id, category, status, title, body, created_at, email, reply_count)| PostRow {
            id,
            author_email: email,
            category_label: category_label(&category).to_string(),
            category,
            title: title.unwrap_or_default(),
            body: truncate(&body, 240),
            status_label: status_label(&status).to_string(),
            status,
            created_at,
            reply_count,
        })
        .collect();

    Ok(AdminFeedbackTemplate {
        user_email: u.email,
        posts,
        status_filter,
    }
    .into_response())
}

/// Route after successful login: admin → /admin, everyone else (member or
/// guest) → /feedback. Guests see a banner on /feedback prompting them to
/// request membership.
pub async fn home_redirect(crate::auth::CurrentUser(u): crate::auth::CurrentUser) -> Response {
    if u.is_admin_bool() {
        Redirect::to("/admin").into_response()
    } else {
        Redirect::to("/feedback").into_response()
    }
}

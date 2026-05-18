use std::sync::Arc;
use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

use crate::handlers::{
    admin, auth_api, desktop_auth, device_api, feedback, health, home, membership, oauth, policy,
};
use crate::state::AppState;

pub async fn build_router(state: Arc<AppState>) -> anyhow::Result<Router> {
    let session_store = SqliteStore::new(state.db.clone());
    session_store.migrate().await?;

    let session_layer = SessionManagerLayer::new(session_store.clone())
        .with_name("vkc_sid")
        .with_secure(state.config.public_url.starts_with("https://"))
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(30)));

    // Sweep expired sessions in background
    {
        let store = session_store.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = store.delete_expired().await {
                    tracing::warn!("session sweep failed: {e}");
                }
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        });
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        .route("/health", get(health::health))
        .route("/server-info", get(health::server_info))
        .route("/auth/desktop/start", post(desktop_auth::start))
        .route("/auth/desktop/poll/:flow_id", get(desktop_auth::poll))
        .route("/auth/refresh", post(auth_api::refresh))
        .route("/auth/verify", get(auth_api::verify))
        .route("/whoami", get(device_api::whoami))
        .route("/config", get(policy::get_config))
        .route("/sessions", post(device_api::upsert_session))
        .route("/activity", post(device_api::ingest_activity))
        .route("/app-snapshots", post(device_api::ingest_app_snapshots))
        .route(
            "/screenshots",
            post(device_api::upload_screenshot).layer(DefaultBodyLimit::max(
                state.config.max_screenshot_bytes + 16 * 1024,
            )),
        );

    let admin_routes = Router::new()
        .route("/", get(admin::admin_root))
        .route("/login", get(admin::login_page))
        .route("/oauth/start", get(oauth::admin_login_start))
        .route("/oauth/callback", get(oauth::admin_oauth_callback))
        .route("/logout", get(oauth::admin_logout))
        .route("/users", get(admin::users_list))
        .route("/users/:id", get(admin::user_detail))
        .route("/sessions/:id", get(admin::session_detail))
        .route("/screenshots", get(admin::screenshots_list))
        .route("/screenshots/:id/image", get(admin::screenshot_file))
        .route(
            "/members",
            get(admin::members_page).post(admin::members_add),
        )
        .route("/members/remove", post(admin::members_remove))
        .route("/members/approve-request", post(membership::approve))
        .route("/members/reject-request", post(membership::reject))
        .route("/policy", get(policy::admin_page).post(policy::admin_save))
        .route("/feedback", get(feedback::admin_list))
        .route("/feedback/:id", get(feedback::admin_detail))
        .route("/feedback/:id/reply", post(feedback::reply_admin));

    let feedback_routes = Router::new()
        .route("/", get(feedback::member_list).post(feedback::create_post))
        .route("/:id", get(feedback::detail))
        .route("/:id/reply", post(feedback::reply_member));

    // Browser-facing pages for the desktop login flow. /authorize is the
    // landing URL the desktop app opens; /done is shown after successful
    // login.
    let auth_pages = Router::new()
        .route("/desktop/authorize", get(desktop_auth::authorize))
        .route("/desktop/done", get(desktop_auth::done));

    let root = Router::new()
        .route("/", get(home::home_page))
        .route("/pending", get(membership::pending_page))
        .route("/membership/request", post(membership::submit_request))
        .nest("/api/v1", api)
        .nest("/admin", admin_routes)
        .nest("/feedback", feedback_routes)
        .nest("/auth", auth_pages)
        .nest_service("/static", ServeDir::new("./static"))
        .layer(session_layer)
        .layer(cors)
        .with_state(state);

    Ok(root)
}

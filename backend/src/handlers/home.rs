//! Public landing page at `/`.
//!
//! Unlike everything else under `/admin` and `/feedback`, this page renders
//! without requiring a session — it's the first thing a stranger sees, so it
//! introduces the project before asking them to log in. Logged-in users get
//! a personalized CTA pointing at their normal landing page.

use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use tower_sessions::Session;

use crate::auth::current_user_opt;
use crate::state::AppState;

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    /// `Some` when a session cookie resolves to an existing user. Drives the
    /// hero CTA: a returning user sees "Vào trang của tôi" instead of the
    /// generic login button.
    pub viewer: Option<HomeViewer>,
    pub maintainer_facebook: String,
    pub maintainer_email: String,
    pub version: String,
}

pub struct HomeViewer {
    pub email: String,
    pub role_label: String,
    /// Role-aware landing page used by the CTA button.
    pub destination: String,
}

pub async fn home_page(State(state): State<Arc<AppState>>, session: Session) -> Response {
    let viewer = current_user_opt(&state, &session).await.map(|u| {
        let (role_label, destination) = if u.is_admin_bool() {
            ("Quản trị viên", "/admin")
        } else if u.is_member_bool() {
            ("Thành viên", "/feedback")
        } else {
            ("Khách", "/pending")
        };
        HomeViewer {
            email: u.email,
            role_label: role_label.to_string(),
            destination: destination.to_string(),
        }
    });

    HomeTemplate {
        viewer,
        maintainer_facebook: state.config.maintainer_facebook.clone(),
        maintainer_email: state.config.maintainer_email.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
    .into_response()
}

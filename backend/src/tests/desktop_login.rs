//! End-to-end test of the desktop browser-OAuth + polling login flow.
//!
//! The Google round-trip is simulated by calling
//! [`complete_desktop_flow_if_match`] directly (the same function the real
//! /admin/oauth/callback calls once Google returns the matching state).
//! Everything else is exercised through the real Axum router.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt; // for oneshot

use crate::auth::ensure_user_from_google;
use crate::handlers::desktop_auth::complete_desktop_flow_if_match;
use crate::state::{AppState, Config};

fn unique_tmp_path(suffix: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let name = format!("vkc-test-{}-{}-{}", pid, nanos, suffix);
    let mut p = std::env::temp_dir();
    p.push(name);
    p
}

async fn build_test_app() -> (Router, Arc<AppState>) {
    let dir = unique_tmp_path("dir");
    std::fs::create_dir_all(&dir).unwrap();
    let db_file = dir.join("test.db");
    let db_url = format!(
        "sqlite://{}?mode=rwc",
        db_file.display().to_string().replace('\\', "/")
    );

    let pool = crate::db::init_pool(&db_url).await.expect("init pool");
    crate::db::run_migrations(&pool).await.expect("migrate");

    let config = Config {
        port: 0,
        database_url: db_url,
        public_url: "http://localhost".into(),
        google_client_id: "test-client-id".into(),
        google_client_secret: "test-client-secret".into(),
        jwt_secret: "test-jwt-secret-please-make-it-long-enough!".into(),
        desktop_device_limit: 5,
        admin_emails: vec![],
        member_emails: vec!["test@example.com".into(), "guest@example.com".into()],
        screenshot_dir: dir.join("screenshots"),
        max_screenshot_bytes: 1024 * 1024,
        maintainer_facebook: "".into(),
        maintainer_email: "".into(),
        app_timezone: chrono_tz::Asia::Ho_Chi_Minh,
    };

    let state = Arc::new(AppState::new(config, pool));
    let app = crate::routes::build_router(state.clone())
        .await
        .expect("build router");
    (app, state)
}

struct Got {
    status: StatusCode,
    json: Value,
}

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> Got {
    let mut req = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(b) => {
            req = req.header("content-type", "application/json");
            Body::from(serde_json::to_vec(&b).unwrap())
        }
        None => Body::empty(),
    };
    let resp = app
        .clone()
        .oneshot(req.body(body).unwrap())
        .await
        .expect("router oneshot");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let json: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into_owned()))
    };
    Got { status, json }
}

async fn call_with_bearer(
    app: &Router,
    method: Method,
    uri: &str,
    token: &str,
    body: Option<Value>,
) -> Got {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {}", token));
    let body = match body {
        Some(b) => {
            req = req.header("content-type", "application/json");
            Body::from(serde_json::to_vec(&b).unwrap())
        }
        None => Body::empty(),
    };
    let resp = app
        .clone()
        .oneshot(req.body(body).unwrap())
        .await
        .expect("router oneshot");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let json: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into_owned()))
    };
    Got { status, json }
}

const FP_A: &str = "fingerprint-a-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const STATE_A: &str = "csrf-state-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[tokio::test]
async fn full_login_lifecycle() {
    let (app, state) = build_test_app().await;

    // 1. /start — desktop registers a flow.
    let resp = call(
        &app,
        Method::POST,
        "/api/v1/auth/desktop/start",
        Some(json!({
            "device_fingerprint": FP_A,
            "device_name": "test-pc",
            "os": "windows",
            "state": STATE_A,
        })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "start: {:?}", resp.json);
    let flow_id = resp.json["flow_id"].as_str().unwrap().to_string();
    let auth_url = resp.json["auth_url"].as_str().unwrap();
    assert!(auth_url.contains("/auth/desktop/authorize?flow_id="));

    // 2. Poll while pending.
    let resp = call(
        &app,
        Method::GET,
        &format!("/api/v1/auth/desktop/poll/{}", flow_id),
        None,
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.json["status"], "pending");

    // 3. Simulate Google callback: ensure_user_from_google + complete_desktop_flow_if_match.
    //    This is exactly what /admin/oauth/callback does on a real desktop callback.
    let user = ensure_user_from_google(
        &state,
        "google-sub-test",
        "test@example.com",
        Some("Test User"),
        None,
    )
    .await
    .expect("ensure user");
    let outcome = complete_desktop_flow_if_match(&state, STATE_A, &user)
        .await
        .expect("complete flow");
    assert!(outcome.matched, "flow should have been matched");

    // 4. Poll — completed with tokens.
    let resp = call(
        &app,
        Method::GET,
        &format!("/api/v1/auth/desktop/poll/{}", flow_id),
        None,
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "poll body: {:?}", resp.json);
    assert_eq!(resp.json["status"], "completed");
    let access = resp.json["access_token"].as_str().unwrap().to_string();
    let refresh = resp.json["refresh_token"].as_str().unwrap().to_string();
    assert!(!access.is_empty());
    assert!(!refresh.is_empty());
    assert_eq!(resp.json["user"]["email"], "test@example.com");
    assert_eq!(resp.json["subscription"]["tier"], "member");

    // 5. Poll again — tokens are gone (one-shot).
    let resp = call(
        &app,
        Method::GET,
        &format!("/api/v1/auth/desktop/poll/{}", flow_id),
        None,
    )
    .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "second poll should not re-deliver tokens"
    );

    // 6. /verify with the access token.
    let resp = call_with_bearer(&app, Method::GET, "/api/v1/auth/verify", &access, None).await;
    assert_eq!(resp.status, StatusCode::OK, "verify: {:?}", resp.json);
    assert_eq!(resp.json["valid"], true);
    assert_eq!(resp.json["user"]["email"], "test@example.com");
    assert_eq!(resp.json["user"]["is_member"], true);
    assert_eq!(resp.json["subscription"]["tier"], "member");

    // 7. /refresh — rotates the refresh token and returns a fresh access token.
    let resp = call(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        Some(json!({
            "refresh_token": refresh,
            "device_fingerprint": FP_A,
        })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "refresh: {:?}", resp.json);
    let new_access = resp.json["access_token"].as_str().unwrap().to_string();
    let new_refresh = resp.json["refresh_token"].as_str().unwrap().to_string();
    assert_ne!(new_access, access, "access token must rotate");
    assert_ne!(new_refresh, refresh, "refresh token must rotate");

    // 8. Old refresh token must be rejected (rotation invalidates predecessor).
    let resp = call(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        Some(json!({
            "refresh_token": refresh,
            "device_fingerprint": FP_A,
        })),
    )
    .await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "rotated-out token must not work"
    );

    // 9. Wrong fingerprint must be rejected (device binding).
    let resp = call(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        Some(json!({
            "refresh_token": new_refresh,
            "device_fingerprint": "fingerprint-b-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        })),
    )
    .await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "fingerprint mismatch must be rejected"
    );

    // 10. New access token still works against /verify.
    let resp = call_with_bearer(&app, Method::GET, "/api/v1/auth/verify", &new_access, None).await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.json["valid"], true);
}

#[tokio::test]
async fn non_member_cannot_pair() {
    let (app, state) = build_test_app().await;
    let state_csrf = "csrf-non-member-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let resp = call(
        &app,
        Method::POST,
        "/api/v1/auth/desktop/start",
        Some(json!({
            "device_fingerprint": FP_A,
            "device_name": "test-pc",
            "os": "windows",
            "state": state_csrf,
        })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    let flow_id = resp.json["flow_id"].as_str().unwrap().to_string();

    // Create a non-member user (not on env list, not in allowed_members).
    let user = ensure_user_from_google(
        &state,
        "google-non-member",
        "outsider@example.com",
        Some("Outsider"),
        None,
    )
    .await
    .unwrap();
    assert!(!user.is_member_bool());

    let outcome = complete_desktop_flow_if_match(&state, state_csrf, &user)
        .await
        .unwrap();
    assert!(outcome.matched);

    let resp = call(
        &app,
        Method::GET,
        &format!("/api/v1/auth/desktop/poll/{}", flow_id),
        None,
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.json["status"], "not_member");
}

#[tokio::test]
async fn device_limit_enforced() {
    let (app, state) = build_test_app().await;

    // Pair the first 5 devices (limit). Each uses a distinct fingerprint.
    let user = ensure_user_from_google(
        &state,
        "google-limit-test",
        "test@example.com",
        Some("Limit"),
        None,
    )
    .await
    .unwrap();

    for i in 0..5 {
        let fp = format!("fingerprint-{:062}", i);
        let csrf = format!("csrf-state-{:049}", i);
        let resp = call(
            &app,
            Method::POST,
            "/api/v1/auth/desktop/start",
            Some(json!({
                "device_fingerprint": fp,
                "device_name": format!("pc-{}", i),
                "os": "windows",
                "state": csrf,
            })),
        )
        .await;
        assert_eq!(resp.status, StatusCode::OK, "start #{}", i);
        let outcome = complete_desktop_flow_if_match(&state, &csrf, &user)
            .await
            .unwrap();
        assert!(outcome.matched);
    }

    // 6th device must hit the device_limit_exceeded path.
    let fp = "fingerprint-overflowoverflowoverflowoverflowoverflowoverflowxx";
    let csrf = "csrf-overflow-overflow-overflow-overflow-overflow-aaaa";
    let resp = call(
        &app,
        Method::POST,
        "/api/v1/auth/desktop/start",
        Some(json!({
            "device_fingerprint": fp,
            "device_name": "overflow",
            "os": "windows",
            "state": csrf,
        })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    let flow_id = resp.json["flow_id"].as_str().unwrap().to_string();

    let outcome = complete_desktop_flow_if_match(&state, csrf, &user)
        .await
        .unwrap();
    assert!(outcome.matched);

    let resp = call(
        &app,
        Method::GET,
        &format!("/api/v1/auth/desktop/poll/{}", flow_id),
        None,
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.json["status"], "device_limit_exceeded");
}

//! Desktop release info proxy.
//!
//! Fetches `latest.json` from GitHub Releases and enriches it with
//! direct download URLs for installer bundles (.dmg, .exe, .AppImage)
//! so the website can show platform-specific download buttons without
//! CORS issues or GitHub API rate limits.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::state::AppState;

/// Cached release info — refreshed at most once every 5 minutes.
struct CachedRelease {
    data: Value,
    fetched_at: Instant,
}

static CACHE: RwLock<Option<CachedRelease>> = RwLock::const_new(None);

const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(5 * 60);
const GITHUB_REPO: &str = "dvchd/vongkimco";
const LATEST_JSON_URL: &str =
    "https://github.com/dvchd/vongkimco/releases/latest/download/latest.json";
const API_LATEST_URL: &str = "https://api.github.com/repos/dvchd/vongkimco/releases/latest";

pub async fn desktop_latest(State(state): State<Arc<AppState>>) -> Response {
    // Check cache first (read lock)
    {
        let guard = CACHE.read().await;
        if let Some(cached) = guard.as_ref() {
            if cached.fetched_at.elapsed() < CACHE_TTL {
                return axum::Json(cached.data.clone()).into_response();
            }
        }
    }

    // Try fetching latest.json (lightweight, direct download)
    let result = fetch_latest_json(&state).await;

    // Fallback: GitHub API
    let enriched = match result {
        Ok(v) => v,
        Err(_) => match fetch_github_api(&state).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to fetch desktop release info: {e}");
                return axum::Json(json!({
                    "error": "Failed to fetch release info",
                    "fallback_url": format!("https://github.com/{GITHUB_REPO}/releases/latest")
                }))
                .into_response();
            }
        },
    };

    // Update cache (write lock)
    {
        let mut guard = CACHE.write().await;
        *guard = Some(CachedRelease {
            data: enriched.clone(),
            fetched_at: Instant::now(),
        });
    }

    axum::Json(enriched).into_response()
}

/// Fetch and enrich `latest.json` from GitHub Releases.
async fn fetch_latest_json(state: &AppState) -> anyhow::Result<Value> {
    let resp = state
        .http_client
        .get(LATEST_JSON_URL)
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;

    Ok(enrich_release_info(resp))
}

/// Fetch release info from GitHub API (heavier but always available).
async fn fetch_github_api(state: &AppState) -> anyhow::Result<Value> {
    let resp: Value = state
        .http_client
        .get(API_LATEST_URL)
        .header("User-Agent", "vongkimco-backend")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let version = resp["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches("desktop-v");

    let assets = resp["assets"].as_array();

    let mut platforms = serde_json::Map::new();

    if let Some(assets) = assets {
        for a in assets {
            let name = a["name"].as_str().unwrap_or("");
            let url = a["browser_download_url"].as_str().unwrap_or("");

            if name.ends_with(".app.tar.gz")
                || name.ends_with(".AppImage")
                || name.ends_with("-setup.exe")
                || name.ends_with(".msi")
            {
                // Determine platform key
                let key = if name.contains(".app.tar.gz") {
                    "darwin-aarch64".to_string()
                } else if name.ends_with(".AppImage") {
                    "linux-x86_64".to_string()
                } else if name.ends_with("-setup.exe") || name.ends_with(".msi") {
                    "windows-x86_64".to_string()
                } else {
                    continue;
                };

                // Only take the first match per platform (prefer .exe over .msi)
                if platforms.contains_key(&key) {
                    continue;
                }

                platforms.insert(key, json!({ "url": url }));
            }
        }
    }

    let mut result = serde_json::Map::new();
    result.insert("version".into(), json!(version));
    result.insert(
        "pub_date".into(),
        json!(resp["published_at"].as_str().unwrap_or("")),
    );
    result.insert("platforms".into(), Value::Object(platforms));

    Ok(enrich_release_info(Value::Object(result)))
}

/// Add installer download URLs (.dmg, .exe, .AppImage) to the
/// response so the frontend can show platform-specific buttons.
fn enrich_release_info(mut data: Value) -> Value {
    let platforms = data.get_mut("platforms").and_then(|p| p.as_object_mut());
    if platforms.is_none() {
        return data;
    }
    let platforms = platforms.unwrap();

    // For each platform entry, derive the installer URL from the updater
    // bundle URL. The updater uses .app.tar.gz (macOS) but the user-visible
    // download should be .dmg. Add "installer_url" to each platform and
    // build a top-level "download_urls" map for easy frontend access.
    let mut download_urls = serde_json::Map::new();

    // Collect updates first to avoid borrowing `platforms` mutably and
    // immutably at the same time (E0502).
    let mut updates: Vec<(String, Value)> = Vec::new();

    for (key, val) in platforms.iter() {
        let url = val["url"].as_str().unwrap_or("");

        let installer_url = if key.starts_with("darwin") {
            // Updater uses .app.tar.gz; installer is .dmg
            url.replace(".app.tar.gz", "_aarch64.dmg")
        } else {
            // Linux (.AppImage) and Windows (.exe) — same URL
            url.to_string()
        };

        // Store installer_url back into the platform entry
        if let Some(obj) = val.as_object() {
            let mut enriched = obj.clone();
            enriched.insert("installer_url".into(), json!(installer_url));
            updates.push((key.clone(), Value::Object(enriched)));
        }

        // Top-level convenience key (macos / linux / windows)
        let dl_key = if key.starts_with("darwin") {
            "macos"
        } else if key.starts_with("linux") {
            "linux"
        } else {
            "windows"
        };

        if !download_urls.contains_key(dl_key) {
            download_urls.insert(
                dl_key.to_string(),
                json!({
                    "url": installer_url,
                    "platform_key": key,
                }),
            );
        }
    }

    // Apply collected updates
    for (key, val) in updates {
        platforms.insert(key, val);
    }

    // Add download_urls to the top level
    if let Some(obj) = data.as_object_mut() {
        obj.insert("download_urls".into(), Value::Object(download_urls));
    }

    data
}

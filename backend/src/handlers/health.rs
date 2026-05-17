use axum::Json;
use serde_json::{json, Value};

pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "app": "vongkimco",
        "time": chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn server_info() -> Json<Value> {
    Json(json!({
        "name": "Vòng Kim Cô",
        "version": env!("CARGO_PKG_VERSION"),
        "auth": ["google"],
        "api": "/api/v1"
    }))
}

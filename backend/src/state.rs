use std::env;
use std::path::PathBuf;

use chrono_tz::Tz;
use sqlx::SqlitePool;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub public_url: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    /// HS256 signing key for desktop access-token JWTs. Required.
    pub jwt_secret: String,
    /// Max number of active devices a non-admin user may pair. Admins are
    /// unlimited. 0 means "no limit for members either".
    pub desktop_device_limit: u32,
    pub admin_emails: Vec<String>,
    pub member_emails: Vec<String>,
    pub screenshot_dir: PathBuf,
    pub max_screenshot_bytes: usize,
    pub maintainer_facebook: String,
    pub maintainer_email: String,
    /// Timezone used to render timestamps in the admin web UI. Storage stays
    /// UTC; this only affects display. Set via env `APP_TIMEZONE` (IANA name).
    pub app_timezone: Tz,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()?;
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://./data/vongkimco.db?mode=rwc".to_string());
        let public_url = env::var("PUBLIC_URL")
            .unwrap_or_else(|_| "https://vongkimco.hoctuthien.com".to_string());
        let google_client_id = env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
        let google_client_secret = env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default();
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_default();
        if jwt_secret.len() < 32 {
            anyhow::bail!(
                "JWT_SECRET must be set to a random string of at least 32 chars. \
                 Generate one with `openssl rand -hex 32` and add it to your .env."
            );
        }
        let desktop_device_limit = env::var("DESKTOP_DEVICE_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5u32);
        let admin_emails: Vec<String> = env::var("ADMIN_EMAILS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let member_emails: Vec<String> = env::var("MEMBER_EMAILS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let maintainer_facebook = env::var("MAINTAINER_FACEBOOK")
            .unwrap_or_else(|_| "https://fb.com/dvcuong.hust".to_string());
        let maintainer_email =
            env::var("MAINTAINER_EMAIL").unwrap_or_else(|_| "dvcuong.hust@gmail.com".to_string());
        let screenshot_dir = PathBuf::from(
            env::var("SCREENSHOT_DIR").unwrap_or_else(|_| "./data/screenshots".to_string()),
        );
        let max_screenshot_bytes = env::var("MAX_SCREENSHOT_BYTES")
            .unwrap_or_else(|_| "2097152".to_string()) // 2 MiB
            .parse()
            .unwrap_or(2 * 1024 * 1024);
        let app_timezone: Tz = env::var("APP_TIMEZONE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(chrono_tz::Asia::Ho_Chi_Minh);

        Ok(Self {
            port,
            database_url,
            public_url,
            google_client_id,
            google_client_secret,
            jwt_secret,
            desktop_device_limit,
            admin_emails,
            member_emails,
            screenshot_dir,
            max_screenshot_bytes,
            maintainer_facebook,
            maintainer_email,
            app_timezone,
        })
    }

    pub fn is_admin_email(&self, email: &str) -> bool {
        let lc = email.to_lowercase();
        self.admin_emails.iter().any(|e| e == &lc)
    }

    /// Whether the email is on the env-based MEMBER_EMAILS list. Does not check
    /// the DB allow-list — use `is_member_email_now` for the merged answer.
    pub fn is_member_email_env(&self, email: &str) -> bool {
        let lc = email.to_lowercase();
        self.member_emails.iter().any(|e| e == &lc)
    }
}

pub struct AppState {
    pub config: Config,
    pub db: SqlitePool,
}

impl AppState {
    pub fn new(config: Config, db: SqlitePool) -> Self {
        Self { config, db }
    }
}

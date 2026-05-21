-- Users (Google OAuth)
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    picture TEXT,
    is_admin INTEGER NOT NULL DEFAULT 0,
    google_sub TEXT UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- Device tokens for desktop app (long-lived bearer tokens)
CREATE TABLE IF NOT EXISTS device_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    device_name TEXT,
    platform TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen TEXT,
    revoked_at TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_device_tokens_user ON device_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_device_tokens_hash ON device_tokens(token_hash);

-- Work sessions
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    device_id TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    note TEXT,
    client_session_id TEXT UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at);

-- Activity samples (idle/active heartbeats)
CREATE TABLE IF NOT EXISTS activity_samples (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    sampled_at TEXT NOT NULL,
    state TEXT NOT NULL, -- 'active' | 'idle'
    idle_seconds INTEGER NOT NULL DEFAULT 0,
    keyboard_events INTEGER NOT NULL DEFAULT 0,
    mouse_events INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_activity_session ON activity_samples(session_id);
CREATE INDEX IF NOT EXISTS idx_activity_sampled ON activity_samples(sampled_at);

-- Running applications snapshot
CREATE TABLE IF NOT EXISTS app_snapshots (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    sampled_at TEXT NOT NULL,
    foreground_app TEXT,
    foreground_title TEXT,
    apps_json TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_appsnap_session ON app_snapshots(session_id);
CREATE INDEX IF NOT EXISTS idx_appsnap_sampled ON app_snapshots(sampled_at);

-- Screenshots
CREATE TABLE IF NOT EXISTS screenshots (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    file_path TEXT NOT NULL,
    width INTEGER,
    height INTEGER,
    bytes INTEGER,
    mime TEXT NOT NULL DEFAULT 'image/jpeg',
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_screenshots_session ON screenshots(session_id);
CREATE INDEX IF NOT EXISTS idx_screenshots_captured ON screenshots(captured_at);

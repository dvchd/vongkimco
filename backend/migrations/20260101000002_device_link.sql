-- Device-link pending requests (OAuth-style device flow)
CREATE TABLE IF NOT EXISTS device_links (
    device_code TEXT PRIMARY KEY,
    user_code TEXT NOT NULL UNIQUE,
    user_id TEXT,           -- set when user approves
    device_name TEXT,
    platform TEXT,
    issued_token_id TEXT,   -- set after exchange
    approved_at TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_device_links_user_code ON device_links(user_code);

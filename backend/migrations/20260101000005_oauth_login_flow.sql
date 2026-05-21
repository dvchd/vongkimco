-- Replace the old "device link" code-typing flow with a browser-OAuth +
-- polling flow. No production users yet, so the old tables are dropped
-- outright.

DROP TABLE IF EXISTS device_links;
DROP TABLE IF EXISTS device_tokens;

-- Short-lived rows tracking a desktop login attempt. Created when the
-- desktop app POSTs /api/v1/auth/desktop/start, completed by the
-- /admin/oauth/callback handler once Google returns matching `state`.
--
-- Timestamps are unix seconds so TTL comparisons stay trivial in SQL and
-- match the `exp` claim shape used in JWT access tokens.
CREATE TABLE login_flows (
    id                  TEXT PRIMARY KEY,
    state               TEXT NOT NULL UNIQUE,
    device_fingerprint  TEXT NOT NULL,
    device_name         TEXT NOT NULL,
    device_os           TEXT NOT NULL,
    status              TEXT NOT NULL,
        -- 'pending' | 'completed' | 'expired'
        -- | 'device_limit_exceeded' | 'not_member'
    user_id             TEXT,
    device_id           TEXT,
    auth_session_id     TEXT,
    access_token        TEXT,
    refresh_token       TEXT,
    expires_at          INTEGER NOT NULL,
    completed_at        INTEGER,
    polled_at           INTEGER,
    created_at          INTEGER NOT NULL
);

CREATE INDEX idx_login_flows_state  ON login_flows(state);
CREATE INDEX idx_login_flows_status ON login_flows(status);
CREATE INDEX idx_login_flows_expiry ON login_flows(status, expires_at);

-- A physical device the user has paired with this server. Identified by
-- its (user_id, fingerprint) pair so the same machine logging in twice
-- updates one row instead of accumulating.
CREATE TABLE devices (
    id                  TEXT PRIMARY KEY,
    user_id             TEXT NOT NULL,
    fingerprint         TEXT NOT NULL,
    name                TEXT NOT NULL,
    os                  TEXT NOT NULL,
    created_at          INTEGER NOT NULL,
    last_seen_at        INTEGER,
    revoked_at          INTEGER,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE (user_id, fingerprint)
);

CREATE INDEX idx_devices_user        ON devices(user_id);
CREATE INDEX idx_devices_fingerprint ON devices(fingerprint);

-- One row per active desktop login. Holds the hashed refresh token; the
-- access token is a JWT and is stateless.
CREATE TABLE auth_sessions (
    id                  TEXT PRIMARY KEY,
    user_id             TEXT NOT NULL,
    device_id           TEXT NOT NULL,
    refresh_token_hash  TEXT NOT NULL UNIQUE,
    issued_at           INTEGER NOT NULL,
    expires_at          INTEGER NOT NULL,
    rotated_at          INTEGER,
    revoked_at          INTEGER,
    FOREIGN KEY (user_id)  REFERENCES users(id)    ON DELETE CASCADE,
    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
);

CREATE INDEX idx_auth_sessions_user   ON auth_sessions(user_id);
CREATE INDEX idx_auth_sessions_device ON auth_sessions(device_id);

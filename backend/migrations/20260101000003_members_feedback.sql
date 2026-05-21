-- Member flag on users. Set on login based on the merged allow-list (env +
-- DB table below).
ALTER TABLE users ADD COLUMN is_member INTEGER NOT NULL DEFAULT 0;

-- Allow-list of member emails managed by admins via the admin web UI.
-- This is unioned with the env var MEMBER_EMAILS. Admins are implicitly
-- members regardless.
CREATE TABLE IF NOT EXISTS allowed_members (
    email TEXT PRIMARY KEY,
    note TEXT,
    added_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Feedback / bug reports / feature requests posted by members on the web UI.
-- Text only — no image uploads (NSFW risk). Users with images contact the
-- maintainer via Facebook / email per README.
CREATE TABLE IF NOT EXISTS feedback_posts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'comment', -- 'comment'|'bug'|'feature'
    title TEXT,
    body TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',      -- 'open'|'in_progress'|'resolved'|'wontfix'
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_feedback_user ON feedback_posts(user_id);
CREATE INDEX IF NOT EXISTS idx_feedback_created ON feedback_posts(created_at);
CREATE INDEX IF NOT EXISTS idx_feedback_status ON feedback_posts(status);

CREATE TABLE IF NOT EXISTS feedback_replies (
    id TEXT PRIMARY KEY,
    post_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    body TEXT NOT NULL,
    is_admin_reply INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (post_id) REFERENCES feedback_posts(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_feedback_replies_post ON feedback_replies(post_id);

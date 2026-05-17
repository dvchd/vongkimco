-- Membership requests: guests (logged in but not on the allow-list) can ask
-- to be added. Admin approves/rejects from /admin/members.
CREATE TABLE IF NOT EXISTS membership_requests (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE,
    note TEXT,
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending'|'approved'|'rejected'
    decided_by TEXT,
    decided_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_membership_requests_status ON membership_requests(status);

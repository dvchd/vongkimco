-- Singleton policy row controlling desktop data-collection behavior. The
-- desktop app fetches this on boot and periodically; admins edit it from
-- /admin/policy. id is fixed to 1 so we don't accidentally end up with
-- multiple rows.
CREATE TABLE IF NOT EXISTS device_policy (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    capture_screenshots INTEGER NOT NULL DEFAULT 1,
    screenshot_interval_secs INTEGER NOT NULL DEFAULT 180,
    activity_sample_interval_secs INTEGER NOT NULL DEFAULT 30,
    app_snapshot_interval_secs INTEGER NOT NULL DEFAULT 60,
    idle_threshold_secs INTEGER NOT NULL DEFAULT 120,
    screenshot_quality INTEGER NOT NULL DEFAULT 50,
    screenshot_max_width INTEGER NOT NULL DEFAULT 1280,
    refresh_interval_secs INTEGER NOT NULL DEFAULT 300,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_by TEXT
);

INSERT OR IGNORE INTO device_policy (id) VALUES (1);

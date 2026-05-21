//! Background monitoring: idle/active detection, app list snapshots, screenshots, hotkeys.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use device_query::{DeviceQuery, DeviceState};
use sysinfo::{ProcessesToUpdate, System};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tokio::time::sleep;

use crate::state::{emit_status, AppState};

pub async fn start_monitors(state: AppState) {
    // device_query's DeviceState holds an X11 Rc on Linux, so its handle isn't
    // Send. Park the activity poller on its own OS thread to avoid pulling that
    // non-Send value into the tokio runtime.
    let s1 = state.clone();
    std::thread::Builder::new()
        .name("vkc-activity".into())
        .spawn(move || activity_loop(s1))
        .expect("spawn activity thread");

    let s2 = state.clone();
    tokio::spawn(async move { app_snapshot_loop(s2).await });

    let s3 = state.clone();
    tokio::spawn(async move { screenshot_loop(s3).await });
}

fn activity_loop(state: AppState) {
    let device_state = DeviceState::new();
    let mut last_keys: HashSet<device_query::Keycode> = HashSet::new();
    let mut last_mouse = device_state.get_mouse().coords;
    let mut last_activity_ts = Instant::now();
    let mut counters_kb = 0i64;
    let mut counters_mo = 0i64;

    loop {
        // Read the sample interval up front — it determines the polling
        // cycle length.  idle_threshold is read AFTER the cycle so config
        // changes take effect on the very next classification.
        let sample_interval = state.policy.read().activity_sample_interval_secs.max(5);

        // Polling tick: every 1s, sample input deltas. Aggregate sample every `sample_interval`s.
        for _ in 0..sample_interval {
            let keys: HashSet<device_query::Keycode> =
                device_state.get_keys().into_iter().collect();
            let new_keys = keys.difference(&last_keys).count() as i64;
            counters_kb += new_keys;

            let mouse = device_state.get_mouse().coords;
            let buttons = device_state.get_mouse().button_pressed;
            let moved = mouse != last_mouse;
            let button_now_down = buttons.iter().filter(|b| **b).count() > 0;
            if moved || button_now_down {
                counters_mo += if moved { 1 } else { 0 } + (button_now_down as i64);
            }

            if new_keys > 0 || moved || button_now_down {
                last_activity_ts = Instant::now();
            }

            last_keys = keys;
            last_mouse = mouse;
            std::thread::sleep(Duration::from_secs(1));
        }

        // Read idle_threshold right before classification so config changes
        // are not delayed by one full sample interval.
        let idle_threshold = state.policy.read().idle_threshold_secs.max(15) as f64;

        let idle_secs = last_activity_ts.elapsed().as_secs_f64();
        let state_str = if idle_secs >= idle_threshold {
            "idle"
        } else {
            "active"
        };

        let session_id = state.session.read().session_id.clone();
        if let Some(sid) = session_id {
            let now = chrono::Utc::now().to_rfc3339();
            let _ = state.db.insert_activity(
                &sid,
                &now,
                state_str,
                idle_secs as i64,
                counters_kb,
                counters_mo,
            );

            {
                let mut s = state.session.write();
                s.keyboard_events += counters_kb;
                s.mouse_events += counters_mo;
                s.last_activity = state_str.into();
            }
            emit_status(&state);
        }
        counters_kb = 0;
        counters_mo = 0;
    }
}

async fn app_snapshot_loop(state: AppState) {
    let mut sys = System::new();

    loop {
        let interval = state.policy.read().app_snapshot_interval_secs.max(15);
        sleep(Duration::from_secs(interval)).await;

        let session_id = state.session.read().session_id.clone();
        let Some(sid) = session_id else { continue };

        // Foreground window
        let (fg_app, fg_title) = match active_win_pos_rs::get_active_window() {
            Ok(w) => (Some(w.app_name), Some(w.title)),
            Err(_) => (None, None),
        };

        // Running user-visible processes (de-duped by name; top N by CPU)
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let mut names: Vec<String> = sys
            .processes()
            .values()
            .filter_map(|p| p.name().to_str().map(|s| s.to_string()))
            .collect();
        names.sort();
        names.dedup();
        let apps_json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".into());

        let now = chrono::Utc::now().to_rfc3339();
        let _ = state.db.insert_app_snapshot(
            &sid,
            &now,
            fg_app.as_deref(),
            fg_title.as_deref(),
            &apps_json,
        );
    }
}

async fn screenshot_loop(state: AppState) {
    loop {
        // Only read the sleep interval up front — it determines how long we
        // wait before the next screenshot.  Quality/max_width are read AFTER
        // the sleep so that an in-flight config change takes effect on the
        // very next capture rather than being delayed by one full interval.
        let interval = state.policy.read().screenshot_interval_secs.max(30);
        sleep(Duration::from_secs(interval)).await;

        // Read the latest policy right before capturing so we never use a
        // stale quality/max_width that was snapshot before the sleep.
        let (enabled, quality, max_width) = {
            let p = state.policy.read();
            (
                p.capture_screenshots,
                p.screenshot_quality.clamp(20, 95) as u8,
                p.screenshot_max_width.max(320),
            )
        };

        let session_id = state.session.read().session_id.clone();
        let Some(sid) = session_id else { continue };
        if !enabled {
            continue;
        }

        let out_dir = state.screenshot_dir.clone();
        let state_clone = state.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::screenshot::capture_primary_jpeg(&out_dir, quality, max_width)
        })
        .await;

        match result {
            Ok(Ok((path, bytes, _w, _h))) => {
                let rel = path
                    .strip_prefix(&state_clone.screenshot_dir)
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|_| path.to_string_lossy().to_string());
                let now = chrono::Utc::now().to_rfc3339();
                if state_clone
                    .db
                    .insert_screenshot(&sid, &now, &rel, bytes)
                    .is_ok()
                {
                    let mut s = state_clone.session.write();
                    s.screenshots_taken += 1;
                }
                emit_status(&state_clone);
            }
            Ok(Err(e)) => log::warn!("screenshot failed: {e:?}"),
            Err(e) => log::warn!("screenshot task panicked: {e:?}"),
        }
    }
}

pub async fn register_hotkeys(state: AppState) {
    let app = state.app.clone();
    let (start_key, stop_key) = {
        let s = state.settings.read();
        (s.hotkey_start.clone(), s.hotkey_stop.clone())
    };

    let manager = app.global_shortcut();
    let _ = manager.unregister_all();

    let s_state = state.clone();
    let _ = manager.on_shortcut(start_key.as_str(), move |_app, _sc, _ev| {
        let st = s_state.clone();
        tauri::async_runtime::spawn(async move {
            let _ = crate::commands::do_start_session(&st, None).await;
        });
    });

    let e_state = state.clone();
    let _ = manager.on_shortcut(stop_key.as_str(), move |_app, _sc, _ev| {
        let st = e_state.clone();
        tauri::async_runtime::spawn(async move {
            let _ = crate::commands::do_stop_session(&st).await;
        });
    });
}

import { writable, derived } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface Settings {
    server_url: string;
    capture_screenshots: boolean;
    screenshot_interval_secs: number;
    activity_sample_interval_secs: number;
    app_snapshot_interval_secs: number;
    idle_threshold_secs: number;
    hotkey_start: string;
    hotkey_stop: string;
    autostart: boolean;
}

export interface UserInfo {
    id: string;
    email: string;
    name: string | null;
    picture: string | null;
}

export interface SessionState {
    running: boolean;
    session_id: string | null;
    started_at: string | null;
    last_activity: string;
    keyboard_events: number;
    mouse_events: number;
    screenshots_taken: number;
    pending_sync: number;
    online: boolean;
}

export const settings = writable<Settings | null>(null);
export const user = writable<UserInfo | null>(null);
export const sessionState = writable<SessionState>({
    running: false,
    session_id: null,
    started_at: null,
    last_activity: "active",
    keyboard_events: 0,
    mouse_events: 0,
    screenshots_taken: 0,
    pending_sync: 0,
    online: false
});
export const route = writable<string>("home");

export const isAuthed = derived(user, ($u) => $u !== null);

export async function loadSettings() {
    const s = await invoke<Settings>("get_settings");
    settings.set(s);
}

export async function saveSettings(s: Settings) {
    await invoke("save_settings", { settings: s });
    settings.set(s);
}

export async function loadUser() {
    try {
        const u = await invoke<UserInfo | null>("get_current_user");
        user.set(u);
    } catch (e) {
        user.set(null);
    }
}

export async function refreshStatus() {
    const s = await invoke<SessionState>("get_session_state");
    sessionState.set(s);
}

let unlistenStatus: (() => void) | null = null;

export async function startStatusListener() {
    if (unlistenStatus) return;
    const off = await listen<SessionState>("vkc://status", (e) => {
        sessionState.set(e.payload);
    });
    unlistenStatus = off;
}

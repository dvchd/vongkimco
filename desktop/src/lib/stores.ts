import { writable, derived } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type ThemePref = "auto" | "light" | "dark";

export interface Settings {
    server_url: string;
    hotkey_start: string;
    hotkey_stop: string;
    autostart: boolean;
    theme: ThemePref;
}

const THEME_LS_KEY = "vkc_theme";

function resolveTheme(pref: ThemePref): "light" | "dark" {
    if (pref === "auto") {
        return window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches
            ? "light"
            : "dark";
    }
    return pref;
}

/// Mirrors the chosen preference into localStorage and applies the resolved
/// theme to the document. We mirror to localStorage so the inline boot script
/// in index.html can paint the right palette before App.svelte mounts, avoiding
/// a flash of dark UI when the user has chosen light.
export function applyTheme(pref: ThemePref) {
    try { localStorage.setItem(THEME_LS_KEY, pref); } catch {}
    const root = document.documentElement;
    root.setAttribute("data-theme-pref", pref);
    root.setAttribute("data-theme", resolveTheme(pref));
}

let themeMqInstalled = false;
export function installThemeAutoListener() {
    if (themeMqInstalled || !window.matchMedia) return;
    themeMqInstalled = true;
    const mq = window.matchMedia("(prefers-color-scheme: light)");
    const handler = () => {
        let pref: ThemePref = "auto";
        try { pref = (localStorage.getItem(THEME_LS_KEY) as ThemePref) || "auto"; } catch {}
        if (pref === "auto") {
            document.documentElement.setAttribute("data-theme", resolveTheme("auto"));
        }
    };
    if (mq.addEventListener) mq.addEventListener("change", handler);
    else if ((mq as any).addListener) (mq as any).addListener(handler);
}

export interface Policy {
    capture_screenshots: boolean;
    screenshot_interval_secs: number;
    activity_sample_interval_secs: number;
    app_snapshot_interval_secs: number;
    idle_threshold_secs: number;
    screenshot_quality: number;
    screenshot_max_width: number;
    refresh_interval_secs: number;
    version: string;
    from_server: boolean;
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
export const policy = writable<Policy | null>(null);
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
    applyTheme(s.theme || "auto");
}

export async function loadPolicy() {
    const p = await invoke<Policy>("get_policy");
    policy.set(p);
}

export async function refreshPolicy() {
    const p = await invoke<Policy>("refresh_policy");
    policy.set(p);
    return p;
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

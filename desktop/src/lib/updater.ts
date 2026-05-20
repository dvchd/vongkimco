import { writable } from "svelte/store";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export interface UpdateState {
    status: "idle" | "checking" | "available" | "downloading" | "ready" | "uptodate" | "error";
    version?: string;
    notes?: string;
    progress?: number;
    error?: string;
}

export const updateState = writable<UpdateState>({ status: "idle" });
let current: Update | null = null;

// ── Periodic check ──────────────────────────────────────────────────
let periodicTimer: ReturnType<typeof setInterval> | null = null;
const DEFAULT_CHECK_INTERVAL_MS = 4 * 60 * 60 * 1000; // 4 hours

export function startPeriodicCheck(options: {
    intervalMs?: number;
    autoDownload?: boolean;
} = {}) {
    const { intervalMs = DEFAULT_CHECK_INTERVAL_MS, autoDownload = false } = options;
    stopPeriodicCheck();

    // Check immediately on start
    checkForUpdate({ silent: true, autoDownload });

    periodicTimer = setInterval(() => {
        checkForUpdate({ silent: true, autoDownload });
    }, intervalMs);
}

export function stopPeriodicCheck() {
    if (periodicTimer !== null) {
        clearInterval(periodicTimer);
        periodicTimer = null;
    }
}

// ── Check for update ────────────────────────────────────────────────
export async function checkForUpdate(options: {
    silent?: boolean;
    autoDownload?: boolean;
} = {}) {
    const { silent = false, autoDownload = false } = options;
    if (!silent) updateState.set({ status: "checking" });
    try {
        const update = await check();
        if (update?.available) {
            current = update;
            updateState.set({
                status: "available",
                version: update.version,
                notes: update.body ?? undefined
            });
            // Auto-download if enabled
            if (autoDownload) {
                await downloadAndInstall();
            }
            return true;
        } else {
            current = null;
            updateState.set({ status: "uptodate" });
            return false;
        }
    } catch (e: any) {
        updateState.set({ status: "error", error: e?.toString?.() ?? "unknown" });
        return false;
    }
}

// ── Download & install ──────────────────────────────────────────────
export async function downloadAndInstall() {
    if (!current) return;
    let downloaded = 0;
    let total = 0;
    updateState.update((s) => ({ ...s, status: "downloading", progress: 0 }));
    try {
        await current.downloadAndInstall((event) => {
            switch (event.event) {
                case "Started":
                    total = event.data.contentLength ?? 0;
                    break;
                case "Progress":
                    downloaded += event.data.chunkLength;
                    if (total > 0) {
                        updateState.update((s) => ({
                            ...s,
                            progress: Math.min(100, Math.round((downloaded / total) * 100))
                        }));
                    }
                    break;
                case "Finished":
                    updateState.update((s) => ({ ...s, status: "ready", progress: 100 }));
                    break;
            }
        });
        await relaunch();
    } catch (e: any) {
        updateState.set({ status: "error", error: e?.toString?.() ?? "unknown" });
    }
}

export function dismissUpdate() {
    current = null;
    updateState.set({ status: "idle" });
}

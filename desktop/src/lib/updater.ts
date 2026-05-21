import { writable } from "svelte/store";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { invoke } from "@tauri-apps/api/core";

export interface UpdateState {
    status: "idle" | "checking" | "available" | "downloading" | "ready" | "uptodate" | "error";
    version?: string;
    notes?: string;
    progress?: number;
    error?: string;
    /** When true, auto-update is not possible (e.g. .deb install on Linux)
     *  and the user must download the update manually. */
    manualOnly?: boolean;
    /** Direct download URL for the manual update (e.g. .deb file on GitHub). */
    downloadUrl?: string;
}

export const updateState = writable<UpdateState>({ status: "idle" });
let current: Update | null = null;

// Whether this binary is an AppImage (only true when APPIMAGE env is set).
// null = not yet checked, true/false = result.
export const isAppImage = writable<boolean | null>(null);

// ── Detect installation format ─────────────────────────────────────
let _cachedIsAppImage: boolean | null = null;
async function checkIsAppImage(): Promise<boolean> {
    if (_cachedIsAppImage !== null) return _cachedIsAppImage;
    try {
        const result = await invoke<boolean>("is_appimage");
        _cachedIsAppImage = result;
        isAppImage.set(result);
        return result;
    } catch {
        _cachedIsAppImage = false;
        isAppImage.set(false);
        return false;
    }
}

// ── Build a manual download URL for .deb ─────────────────────────
function buildDebDownloadUrl(version: string): string {
    return `https://github.com/dvchd/vongkimco/releases/download/desktop-v${version}/VongKimCo_${version}_amd64.deb`;
}

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

            // On Linux, if not an AppImage, auto-update will fail with
            // "invalid updater binary format". Show manual download instead.
            const appImage = await checkIsAppImage();
            const isLinux = navigator.userAgent.includes("Linux") ||
                (window as any).__TAURI_PLATFORM__ === "linux";

            if (isLinux && !appImage) {
                updateState.set({
                    status: "available",
                    version: update.version,
                    notes: update.body ?? undefined,
                    manualOnly: true,
                    downloadUrl: buildDebDownloadUrl(update.version),
                });
            } else {
                updateState.set({
                    status: "available",
                    version: update.version,
                    notes: update.body ?? undefined,
                });
                // Auto-download if enabled
                if (autoDownload) {
                    await downloadAndInstall();
                }
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

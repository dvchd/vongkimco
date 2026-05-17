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

export async function checkForUpdate(options: { silent?: boolean } = {}) {
    const { silent = false } = options;
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

<script lang="ts">
    import { updateState, downloadAndInstall, dismissUpdate, checkForUpdate } from "./updater";
    import { open } from "@tauri-apps/plugin-shell";
</script>

{#if $updateState.status === "available"}
    <div class="update-banner">
        <div class="update-info">
            <strong>🆕 Bản cập nhật mới: v{$updateState.version}</strong>
            {#if $updateState.manualOnly}
                <p class="muted small manual-note">
                    Ứng dụng được cài bằng .deb nên không tự cập nhật được.
                    Tải file .deb mới và cài đặt thủ công:
                </p>
            {/if}
            {#if $updateState.notes}
                <details>
                    <summary>Ghi chú phát hành</summary>
                    <pre>{$updateState.notes}</pre>
                </details>
            {/if}
        </div>
        <div class="update-actions">
            {#if $updateState.manualOnly}
                <button class="primary" on:click={() => $updateState.downloadUrl && open($updateState.downloadUrl)}>
                    Tải .deb v{$updateState.version}
                </button>
            {:else}
                <button class="primary" on:click={() => downloadAndInstall()}>Cài đặt và khởi động lại</button>
            {/if}
            <button on:click={() => dismissUpdate()}>Để sau</button>
        </div>
    </div>
{:else if $updateState.status === "downloading"}
    <div class="update-banner">
        <div class="update-info">
            <strong>⬇ Đang tải bản cập nhật v{$updateState.version}</strong>
            <div class="progress-track">
                <div class="progress-fill" style="width: {$updateState.progress ?? 0}%"></div>
            </div>
            <div class="muted small">{$updateState.progress ?? 0}%</div>
        </div>
    </div>
{:else if $updateState.status === "ready"}
    <div class="update-banner ok">
        <div class="update-info"><strong>✅ Đã tải xong, đang khởi động lại…</strong></div>
    </div>
{:else if $updateState.status === "error"}
    <div class="update-banner err">
        <div class="update-info">
            <strong>⚠ Lỗi khi cập nhật:</strong> {$updateState.error}
        </div>
        <div class="update-actions">
            <button on:click={() => checkForUpdate()}>Thử lại</button>
            <button on:click={() => dismissUpdate()}>Đóng</button>
        </div>
    </div>
{/if}

<style>
    .update-banner {
        background: var(--surface);
        border: 1px solid var(--border);
        border-left: 4px solid var(--primary);
        border-radius: 8px;
        padding: 12px 16px;
        margin: 0 0 16px;
        display: flex;
        gap: 12px;
        align-items: center;
        justify-content: space-between;
        flex-wrap: wrap;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
    }
    .update-banner.ok  { border-left-color: var(--ok); }
    .update-banner.err { border-left-color: var(--danger); }
    .update-info { flex: 1; min-width: 240px; }
    .update-actions { display: flex; gap: 8px; flex-wrap: wrap; }
    .manual-note { margin-top: 6px; }
    details summary { cursor: pointer; color: var(--muted); margin-top: 4px; user-select: none; }
    details pre {
        max-height: 160px; overflow: auto;
        background: var(--surface-2); padding: 8px; border-radius: 4px;
        font-size: 12px; white-space: pre-wrap; word-wrap: break-word;
        margin: 6px 0 0;
        border: 1px solid var(--border);
    }
    .progress-track {
        height: 6px;
        background: var(--surface-2);
        border-radius: 3px;
        margin-top: 8px;
        overflow: hidden;
        border: 1px solid var(--border);
    }
    .progress-fill {
        height: 100%;
        background: var(--primary);
        transition: width 0.2s ease;
    }
</style>

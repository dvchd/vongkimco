<script lang="ts">
    import { updateState, downloadAndInstall, dismissUpdate, checkForUpdate } from "./updater";
</script>

{#if $updateState.status === "available"}
    <div class="update-banner">
        <div class="update-info">
            <strong>🆕 Bản cập nhật mới: v{$updateState.version}</strong>
            {#if $updateState.notes}
                <details>
                    <summary>Ghi chú phát hành</summary>
                    <pre>{$updateState.notes}</pre>
                </details>
            {/if}
        </div>
        <div class="update-actions">
            <button class="primary" on:click={() => downloadAndInstall()}>Cài đặt và khởi động lại</button>
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
        border: 1px solid var(--primary);
        border-left-width: 4px;
        border-radius: 8px;
        padding: 12px 16px;
        margin: 12px 24px;
        display: flex;
        gap: 12px;
        align-items: center;
        justify-content: space-between;
        flex-wrap: wrap;
    }
    .update-banner.ok { border-color: var(--ok); }
    .update-banner.err { border-color: var(--danger); }
    .update-info { flex: 1; min-width: 240px; }
    .update-actions { display: flex; gap: 8px; }
    details summary { cursor: pointer; color: var(--muted); margin-top: 4px; }
    details pre {
        max-height: 160px; overflow: auto;
        background: var(--surface-2); padding: 8px; border-radius: 4px;
        font-size: 12px; white-space: pre-wrap; word-wrap: break-word;
        margin: 6px 0 0;
    }
    .progress-track {
        height: 6px;
        background: var(--surface-2);
        border-radius: 3px;
        margin-top: 8px;
        overflow: hidden;
    }
    .progress-fill {
        height: 100%;
        background: var(--primary);
        transition: width 0.2s;
    }
</style>

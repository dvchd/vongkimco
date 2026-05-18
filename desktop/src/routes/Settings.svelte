<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { getVersion } from "@tauri-apps/api/app";
    import { settings, saveSettings } from "../lib/stores";
    import { updateState, checkForUpdate, downloadAndInstall } from "../lib/updater";
    import { onMount } from "svelte";

    let local = { ...($settings ?? {} as any) };
    let saved = false;
    let error: string | null = null;
    let appVersion = "";

    onMount(async () => {
        try { appVersion = await getVersion(); } catch {}
    });

    async function save() {
        saved = false;
        error = null;
        try {
            await saveSettings(local);
            saved = true;
            setTimeout(() => (saved = false), 2000);
        } catch (e: any) {
            error = e?.toString?.() ?? "Lỗi";
        }
    }

    async function logout() {
        if (!confirm("Đăng xuất và xoá token thiết bị?")) return;
        await invoke("logout");
        location.reload();
    }
</script>

<h1>Cài đặt</h1>
<p>Điều chỉnh chu kỳ thu thập, phím tắt và các tuỳ chọn cập nhật.</p>

{#if saved}<div class="banner ok">✓ Đã lưu cài đặt</div>{/if}
{#if error}<div class="banner error">{error}</div>{/if}

<div class="card">
    <h2 style="margin-top: 0;">🌐 Server</h2>
    <div class="field">
        <label>
            URL server backend
            <input type="url" bind:value={local.server_url} />
        </label>
        <div class="hint">Đổi server sẽ huỷ token thiết bị hiện tại.</div>
    </div>
</div>

<div class="card">
    <h2 style="margin-top: 0;">📊 Thu thập dữ liệu</h2>
    <div class="field">
        <label>
            <input type="checkbox" bind:checked={local.capture_screenshots} />
            <span style="text-transform: none; letter-spacing: normal; color: var(--text);">Chụp màn hình định kỳ</span>
        </label>
        <div class="hint">Ảnh JPEG nén ~50%, resize ≤ 1280px. Tắt nếu chỉ cần theo dõi keystroke/mouse.</div>
    </div>
    <div class="row" style="gap: 12px; flex-wrap: wrap;">
        <div class="field" style="flex: 1; min-width: 200px;">
            <label>
                Chu kỳ chụp màn hình (giây)
                <input type="number" min="30" max="3600" bind:value={local.screenshot_interval_secs} />
            </label>
        </div>
        <div class="field" style="flex: 1; min-width: 200px;">
            <label>
                Chu kỳ đo activity (giây)
                <input type="number" min="5" max="300" bind:value={local.activity_sample_interval_secs} />
            </label>
        </div>
    </div>
    <div class="row" style="gap: 12px; flex-wrap: wrap;">
        <div class="field" style="flex: 1; min-width: 200px;">
            <label>
                Chu kỳ snapshot ứng dụng (giây)
                <input type="number" min="10" max="600" bind:value={local.app_snapshot_interval_secs} />
            </label>
        </div>
        <div class="field" style="flex: 1; min-width: 200px;">
            <label>
                Ngưỡng idle (giây)
                <input type="number" min="30" max="1800" bind:value={local.idle_threshold_secs} />
            </label>
            <div class="hint">Không có thao tác trong khoảng này = idle.</div>
        </div>
    </div>
</div>

<div class="card">
    <h2 style="margin-top: 0;">⌨ Phím tắt</h2>
    <div class="row" style="gap: 12px; flex-wrap: wrap;">
        <div class="field" style="flex: 1; min-width: 200px;">
            <label>
                Bắt đầu phiên
                <input type="text" bind:value={local.hotkey_start} placeholder="CmdOrCtrl+Alt+S" />
            </label>
        </div>
        <div class="field" style="flex: 1; min-width: 200px;">
            <label>
                Kết thúc phiên
                <input type="text" bind:value={local.hotkey_stop} placeholder="CmdOrCtrl+Alt+E" />
            </label>
        </div>
    </div>
    <p class="muted small" style="margin: 0;">
        Cú pháp: <code>CmdOrCtrl+Alt+S</code>. Áp dụng ngay sau khi lưu.
    </p>
</div>

<div class="card">
    <h2 style="margin-top: 0;">🚀 Khởi động cùng hệ thống</h2>
    <div class="field" style="margin-bottom: 0;">
        <label>
            <input type="checkbox" bind:checked={local.autostart} />
            <span style="text-transform: none; letter-spacing: normal; color: var(--text);">Tự khởi động khi đăng nhập máy</span>
        </label>
    </div>
</div>

<div class="card">
    <h2 style="margin-top: 0;">🔄 Cập nhật ứng dụng</h2>
    <div class="row between" style="margin-bottom: 12px;">
        <div>
            <div class="muted small" style="text-transform: uppercase; letter-spacing: 0.06em;">Phiên bản hiện tại</div>
            <div style="font-weight: 600;">v{appVersion || "?"}</div>
        </div>
        <button on:click={() => checkForUpdate()} disabled={$updateState.status === "checking"}>
            {$updateState.status === "checking" ? "Đang kiểm tra…" : "Kiểm tra cập nhật"}
        </button>
    </div>

    {#if $updateState.status === "uptodate"}
        <div class="banner ok small">✓ Bạn đã ở phiên bản mới nhất.</div>
    {:else if $updateState.status === "available"}
        <div class="banner info small">
            Có bản cập nhật mới: <strong>v{$updateState.version}</strong>
        </div>
        <button class="primary" on:click={() => downloadAndInstall()}>Cài đặt và khởi động lại</button>
    {:else if $updateState.status === "error"}
        <div class="banner error small">{$updateState.error}</div>
    {/if}

    <p class="muted small" style="margin: 12px 0 0;">
        Cập nhật được tải qua kênh phát hành chính thức và verify chữ ký Ed25519 trước khi cài.
    </p>
</div>

<div class="row" style="margin-top: 18px;">
    <button class="primary" on:click={save}>💾 Lưu cài đặt</button>
    <button class="danger" on:click={logout} style="margin-left: auto;">Đăng xuất khỏi thiết bị</button>
</div>

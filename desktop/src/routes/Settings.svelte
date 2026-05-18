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

{#if saved}<div class="banner ok">Đã lưu cài đặt</div>{/if}
{#if error}<div class="banner error">{error}</div>{/if}

<div class="card">
    <h2 style="margin-top: 0;">Server</h2>
    <div class="field">
        <label>
            URL server backend
            <input type="url" bind:value={local.server_url} />
        </label>
    </div>
</div>

<div class="card">
    <h2 style="margin-top: 0;">Thu thập dữ liệu</h2>
    <div class="field">
        <label><input type="checkbox" bind:checked={local.capture_screenshots} /> Chụp màn hình định kỳ</label>
    </div>
    <div class="field">
        <label>
            Chu kỳ chụp màn hình (giây)
            <input type="number" min="30" max="3600" bind:value={local.screenshot_interval_secs} />
        </label>
    </div>
    <div class="field">
        <label>
            Chu kỳ đo activity (giây)
            <input type="number" min="5" max="300" bind:value={local.activity_sample_interval_secs} />
        </label>
    </div>
    <div class="field">
        <label>
            Chu kỳ chụp danh sách ứng dụng (giây)
            <input type="number" min="10" max="600" bind:value={local.app_snapshot_interval_secs} />
        </label>
    </div>
    <div class="field">
        <label>
            Ngưỡng idle (giây)
            <input type="number" min="30" max="1800" bind:value={local.idle_threshold_secs} />
        </label>
    </div>
</div>

<div class="card">
    <h2 style="margin-top: 0;">Phím tắt</h2>
    <div class="field">
        <label>
            Bắt đầu phiên
            <input type="text" bind:value={local.hotkey_start} placeholder="CmdOrCtrl+Alt+S" />
        </label>
    </div>
    <div class="field">
        <label>
            Kết thúc phiên
            <input type="text" bind:value={local.hotkey_stop} placeholder="CmdOrCtrl+Alt+E" />
        </label>
    </div>
    <p class="muted small">
        Dùng tổ hợp như <code>CmdOrCtrl+Alt+S</code>. Hệ thống global shortcut sẽ áp dụng ngay sau khi lưu.
    </p>
</div>

<div class="card">
    <h2 style="margin-top: 0;">Khởi động cùng hệ thống</h2>
    <div class="field">
        <label><input type="checkbox" bind:checked={local.autostart} /> Tự khởi động khi đăng nhập máy</label>
    </div>
</div>

<div class="card">
    <h2 style="margin-top: 0;">Cập nhật ứng dụng</h2>
    <div class="field">
        <div class="muted small">Phiên bản hiện tại</div>
        <div>v{appVersion || "?"}</div>
    </div>

    {#if $updateState.status === "checking"}
        <p class="muted">Đang kiểm tra…</p>
    {:else if $updateState.status === "uptodate"}
        <div class="banner ok small">Bạn đã ở phiên bản mới nhất.</div>
    {:else if $updateState.status === "available"}
        <div class="banner small">
            Có bản cập nhật mới: <strong>v{$updateState.version}</strong>
        </div>
        <button class="primary" on:click={() => downloadAndInstall()}>Cài đặt và khởi động lại</button>
    {:else if $updateState.status === "error"}
        <div class="banner error small">{$updateState.error}</div>
    {/if}

    <div class="row" style="margin-top: 8px;">
        <button on:click={() => checkForUpdate()}>Kiểm tra cập nhật</button>
    </div>
    <p class="muted small">
        Cập nhật được tải tự động qua kênh phát hành chính thức và chữ ký số được xác minh trước khi cài.
    </p>
</div>

<div class="row">
    <button class="primary" on:click={save}>Lưu cài đặt</button>
    <button class="danger" on:click={logout}>Đăng xuất khỏi thiết bị</button>
</div>

<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { getVersion } from "@tauri-apps/api/app";
    import {
        settings,
        saveSettings,
        policy,
        refreshPolicy,
        applyTheme,
        type ThemePref,
    } from "../lib/stores";
    import { updateState, checkForUpdate, downloadAndInstall } from "../lib/updater";
    import { onMount } from "svelte";

    let local = { ...($settings ?? {} as any) };
    if (!local.theme) local.theme = "auto";
    let saved = false;
    let error: string | null = null;
    let appVersion = "";
    let policyRefreshing = false;

    function pickTheme(t: ThemePref) {
        local.theme = t;
        applyTheme(t);
    }

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

    async function pullPolicy() {
        policyRefreshing = true;
        try {
            await refreshPolicy();
        } finally {
            policyRefreshing = false;
        }
    }

    function fmtBool(b: boolean) {
        return b ? "Bật" : "Tắt";
    }
</script>

<h1>Cài đặt</h1>

{#if saved}<div class="banner ok">✓ Đã lưu cài đặt</div>{/if}
{#if error}<div class="banner error">{error}</div>{/if}

<!-- Data collection policy — read-only, server-controlled -->
<div class="card">
    <div class="row between" style="align-items: baseline;">
        <h2 style="margin-top: 0;">📊 Thu thập dữ liệu</h2>
        <span class="muted small">🔒 Do admin quản lý</span>
    </div>
    {#if $policy}
        <div class="row" style="gap: 12px; flex-wrap: wrap;">
            <div class="field" style="flex: 1; min-width: 140px; margin-bottom: 0;">
                <div class="muted small" style="text-transform: uppercase; letter-spacing: 0.06em;">Chụp màn hình</div>
                <div style="font-weight: 600;">{fmtBool($policy.capture_screenshots)}</div>
            </div>
            <div class="field" style="flex: 1; min-width: 140px; margin-bottom: 0;">
                <div class="muted small" style="text-transform: uppercase; letter-spacing: 0.06em;">Chu kỳ chụp</div>
                <div style="font-weight: 600;">{$policy.screenshot_interval_secs}s</div>
            </div>
            <div class="field" style="flex: 1; min-width: 140px; margin-bottom: 0;">
                <div class="muted small" style="text-transform: uppercase; letter-spacing: 0.06em;">Chu kỳ activity</div>
                <div style="font-weight: 600;">{$policy.activity_sample_interval_secs}s</div>
            </div>
        </div>
        <div class="row" style="gap: 12px; flex-wrap: wrap; margin-top: 12px;">
            <div class="field" style="flex: 1; min-width: 140px; margin-bottom: 0;">
                <div class="muted small" style="text-transform: uppercase; letter-spacing: 0.06em;">Snapshot ứng dụng</div>
                <div style="font-weight: 600;">{$policy.app_snapshot_interval_secs}s</div>
            </div>
            <div class="field" style="flex: 1; min-width: 140px; margin-bottom: 0;">
                <div class="muted small" style="text-transform: uppercase; letter-spacing: 0.06em;">Ngưỡng idle</div>
                <div style="font-weight: 600;">{$policy.idle_threshold_secs}s</div>
            </div>
            <div class="field" style="flex: 1; min-width: 140px; margin-bottom: 0;">
                <div class="muted small" style="text-transform: uppercase; letter-spacing: 0.06em;">Chất lượng ảnh</div>
                <div style="font-weight: 600;">{$policy.screenshot_quality}% · {$policy.screenshot_max_width}px</div>
            </div>
        </div>
        <div class="row between" style="margin-top: 14px;">
            <p class="muted small" style="margin: 0;">
                {#if $policy.from_server}
                    Cập nhật: <strong>{$policy.version || "—"}</strong>
                {:else}
                    Đang dùng giá trị mặc định.
                {/if}
            </p>
            <button on:click={pullPolicy} disabled={policyRefreshing}>
                {policyRefreshing ? "Đang lấy…" : "↻ Lấy cấu hình mới"}
            </button>
        </div>
    {/if}
</div>

<!-- Theme -->
<div class="card">
    <h2 style="margin-top: 0;">🎨 Giao diện</h2>
    <div class="theme-picker" role="group" aria-label="Chế độ giao diện">
        <button type="button" class:active={local.theme === "auto"} on:click={() => pickTheme("auto")}>
            🖥 Hệ thống
        </button>
        <button type="button" class:active={local.theme === "light"} on:click={() => pickTheme("light")}>
            ☀ Sáng
        </button>
        <button type="button" class:active={local.theme === "dark"} on:click={() => pickTheme("dark")}>
            🌙 Tối
        </button>
    </div>
</div>

<!-- Hotkeys -->
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

<!-- Autostart -->
<div class="card">
    <h2 style="margin-top: 0;">🚀 Khởi động cùng hệ thống</h2>
    <div class="field" style="margin-bottom: 0;">
        <label>
            <input type="checkbox" bind:checked={local.autostart} />
            <span style="text-transform: none; letter-spacing: normal; color: var(--text);">Tự khởi động khi đăng nhập máy</span>
        </label>
    </div>
</div>

<!-- Updates -->
<div class="card">
    <h2 style="margin-top: 0;">🔄 Cập nhật ứng dụng</h2>
    <div class="row between" style="margin-bottom: 12px;">
        <div>
            <div class="muted small" style="text-transform: uppercase; letter-spacing: 0.06em;">Phiên bản</div>
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
        Cập nhật verify chữ ký Ed25519 trước khi cài.
    </p>
</div>

<div class="row" style="margin-top: 18px;">
    <button class="primary" on:click={save}>💾 Lưu cài đặt</button>
    <button class="danger" on:click={logout} style="margin-left: auto;">Đăng xuất</button>
</div>

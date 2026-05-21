<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { refreshStatus, sessionState, user, settings, policy } from "../lib/stores";
    import { onMount, onDestroy } from "svelte";

    let busy = false;
    let error: string | null = null;
    let note = "";
    let tick: any = null;

    async function start() {
        busy = true;
        error = null;
        try {
            await invoke("start_session", { note });
            note = "";
            await refreshStatus();
        } catch (e: any) {
            error = e?.toString?.() ?? "Lỗi";
        }
        busy = false;
    }

    async function stop() {
        busy = true;
        error = null;
        try {
            await invoke("stop_session");
            await refreshStatus();
        } catch (e: any) {
            error = e?.toString?.() ?? "Lỗi";
        }
        busy = false;
    }

    async function syncNow() {
        busy = true;
        try {
            await invoke("sync_now");
            await refreshStatus();
        } catch (e: any) {
            error = e?.toString?.() ?? "Lỗi";
        }
        busy = false;
    }

    function fmtDuration(iso: string | null): string {
        if (!iso) return "00:00:00";
        const start = new Date(iso).getTime();
        const now = Date.now();
        const sec = Math.max(0, Math.floor((now - start) / 1000));
        const h = Math.floor(sec / 3600);
        const m = Math.floor((sec % 3600) / 60);
        const s = sec % 60;
        const pad = (n: number) => n.toString().padStart(2, "0");
        return `${pad(h)}:${pad(m)}:${pad(s)}`;
    }

    let elapsed = "00:00:00";
    onMount(() => {
        elapsed = fmtDuration($sessionState.started_at);
        tick = setInterval(() => {
            elapsed = fmtDuration($sessionState.started_at);
        }, 1000);
    });
    onDestroy(() => clearInterval(tick));

    $: statusText = !$sessionState.running
        ? "Chưa bắt đầu"
        : $sessionState.last_activity === "active"
            ? "Đang hoạt động"
            : "Idle";
    $: statusClass = !$sessionState.running
        ? ""
        : $sessionState.last_activity === "active"
            ? "active"
            : "idle";

    $: screenshotLabel = !$policy?.capture_screenshots
        ? "Tắt"
        : `Mỗi ${$policy?.screenshot_interval_secs ?? 180}s`;

    $: idleLabel = `Sau ${$policy?.idle_threshold_secs ?? 120}s`;
</script>

<h1>Phiên làm việc</h1>
<p>Xin chào <strong>{$user?.name ?? $user?.email}</strong>. Bắt đầu phiên để hệ thống ghi nhận hoạt động.</p>

{#if error}
    <div class="banner error">{error}</div>
{/if}

<div class="card">
    <div class="row between" style="align-items: flex-start;">
        <div>
            <div class="muted small" style="text-transform: uppercase; letter-spacing: 0.06em;">Trạng thái</div>
            <div class="row" style="margin-top: 8px;">
                <span class="status-pill {statusClass}">
                    <span class="dot"></span>
                    {statusText}
                </span>
                <span class="status-pill {$sessionState.online ? 'online' : 'offline'}">
                    <span class="dot"></span>
                    {$sessionState.online ? "Đã kết nối" : "Offline"}
                </span>
            </div>
        </div>
        <div style="text-align: right;">
            <div class="elapsed-display">{elapsed}</div>
            <div class="muted small">Thời gian phiên</div>
        </div>
    </div>

    <div class="session-controls" style="margin-top: 20px;">
        {#if !$sessionState.running}
            <input type="text" bind:value={note} placeholder="Ghi chú (tuỳ chọn)" style="max-width: 320px; flex: 1;" />
            <button class="primary" on:click={start} disabled={busy}>
                {busy ? "Đang bắt đầu…" : "▶ Bắt đầu phiên"}
            </button>
        {:else}
            <button class="danger" on:click={stop} disabled={busy}>
                {busy ? "Đang dừng…" : "■ Kết thúc phiên"}
            </button>
            <button on:click={syncNow} disabled={busy}>
                {busy ? "Đang đồng bộ…" : "↻ Đồng bộ ngay"}
            </button>
        {/if}
    </div>
</div>

<h2>Hoạt động phiên</h2>
<div class="kpis">
    <div class="kpi {$sessionState.last_activity === 'active' && $sessionState.running ? 'ok' : 'info'}">
        <div class="kpi-label">Trạng thái</div>
        <div class="kpi-value">{#if !$sessionState.running}—{:else}{$sessionState.last_activity === "active" ? "Hoạt động" : "Idle"}{/if}</div>
    </div>
    <div class="kpi info">
        <div class="kpi-label">Sự kiện bàn phím</div>
        <div class="kpi-value">{$sessionState.keyboard_events}</div>
    </div>
    <div class="kpi info">
        <div class="kpi-label">Sự kiện chuột</div>
        <div class="kpi-value">{$sessionState.mouse_events}</div>
    </div>
    <div class="kpi ok">
        <div class="kpi-label">Ảnh chụp</div>
        <div class="kpi-value">{$sessionState.screenshots_taken}</div>
    </div>
    <div class="kpi {$sessionState.pending_sync > 0 ? 'warn' : ''}">
        <div class="kpi-label">Chờ đồng bộ</div>
        <div class="kpi-value">{$sessionState.pending_sync}</div>
    </div>
</div>

<div class="info-grid">
    <div class="info-item">
        <span class="info-label">📸 Chụp màn hình</span>
        <span class="info-value">{screenshotLabel}</span>
    </div>
    <div class="info-item">
        <span class="info-label">💤 Ngưỡng idle</span>
        <span class="info-value">{idleLabel}</span>
    </div>
    <div class="info-item">
        <span class="info-label">⌨ Phím tắt</span>
        <span class="info-value"><kbd>{$settings?.hotkey_start ?? "—"}</kbd> bắt đầu · <kbd>{$settings?.hotkey_stop ?? "—"}</kbd> dừng</span>
    </div>
</div>

<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { refreshStatus, sessionState, user, settings } from "../lib/stores";
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
        if (!iso) return "—";
        const start = new Date(iso).getTime();
        const now = Date.now();
        const sec = Math.floor((now - start) / 1000);
        const h = Math.floor(sec / 3600);
        const m = Math.floor((sec % 3600) / 60);
        const s = sec % 60;
        if (h > 0) return `${h}h ${m}m ${s}s`;
        if (m > 0) return `${m}m ${s}s`;
        return `${s}s`;
    }

    let elapsed = "—";
    onMount(() => {
        tick = setInterval(() => {
            elapsed = fmtDuration($sessionState.started_at);
        }, 1000);
    });
    onDestroy(() => clearInterval(tick));
</script>

<h1>Phiên làm việc</h1>
<p class="muted">Xin chào {$user?.name ?? $user?.email}. Bắt đầu phiên để hệ thống ghi nhận hoạt động của bạn.</p>

{#if error}
    <div class="banner error">{error}</div>
{/if}

<div class="card">
    <div class="row" style="justify-content: space-between; align-items: flex-start;">
        <div>
            <h2 style="margin: 0;">Trạng thái</h2>
            <div class="row" style="margin-top: 8px;">
                {#if $sessionState.running}
                    <span class="status-pill {$sessionState.last_activity}">
                        <span class="dot"></span>
                        {$sessionState.last_activity === "active" ? "Đang hoạt động" : "Idle"}
                    </span>
                {:else}
                    <span class="status-pill">
                        <span class="dot"></span> Chưa bắt đầu
                    </span>
                {/if}
                <span class="status-pill {$sessionState.online ? '' : 'offline'}">
                    <span class="dot"></span>
                    {$sessionState.online ? "Online" : "Offline"}
                </span>
            </div>
        </div>
        <div style="text-align: right;">
            <div style="font-size: 28px; font-weight: 700;">{elapsed}</div>
            <div class="muted small">Thời gian phiên</div>
        </div>
    </div>

    <div class="session-controls" style="margin-top: 18px;">
        {#if !$sessionState.running}
            <input type="text" bind:value={note} placeholder="Ghi chú (tuỳ chọn)" style="max-width: 320px;" />
            <button class="primary" on:click={start} disabled={busy}>Bắt đầu phiên</button>
        {:else}
            <button class="danger" on:click={stop} disabled={busy}>Kết thúc phiên</button>
            <button on:click={syncNow} disabled={busy}>Đồng bộ ngay</button>
        {/if}
    </div>
</div>

<h2>Thống kê phiên</h2>
<div class="kpis">
    <div class="kpi"><div class="kpi-value">{$sessionState.keyboard_events}</div><div class="kpi-label">Sự kiện bàn phím</div></div>
    <div class="kpi"><div class="kpi-value">{$sessionState.mouse_events}</div><div class="kpi-label">Sự kiện chuột</div></div>
    <div class="kpi"><div class="kpi-value">{$sessionState.screenshots_taken}</div><div class="kpi-label">Ảnh chụp</div></div>
    <div class="kpi"><div class="kpi-value">{$sessionState.pending_sync}</div><div class="kpi-label">Chờ đồng bộ</div></div>
</div>

<p class="muted small" style="margin-top: 16px;">
    Phím tắt: <kbd>{$settings?.hotkey_start}</kbd> bắt đầu · <kbd>{$settings?.hotkey_stop}</kbd> dừng
</p>

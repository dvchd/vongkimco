<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";

    interface LocalSession {
        id: string;
        started_at: string;
        ended_at: string | null;
        note: string | null;
        synced: boolean;
        keyboard_events: number;
        mouse_events: number;
    }

    let rows: LocalSession[] = [];
    let loading = true;
    let error: string | null = null;
    let filter: "all" | "synced" | "pending" = "all";

    async function load() {
        loading = true;
        error = null;
        try {
            rows = await invoke<LocalSession[]>("list_local_sessions");
        } catch (e: any) {
            error = e?.toString?.() ?? "Lỗi";
        }
        loading = false;
    }

    function fmt(s: string) {
        return new Date(s).toLocaleString();
    }

    function durationLabel(r: LocalSession): string {
        if (!r.ended_at) return "đang chạy…";
        const ms = new Date(r.ended_at).getTime() - new Date(r.started_at).getTime();
        const sec = Math.max(0, Math.floor(ms / 1000));
        const h = Math.floor(sec / 3600);
        const m = Math.floor((sec % 3600) / 60);
        if (h > 0) return `${h}h ${m}m`;
        return `${m}m`;
    }

    $: filtered = rows.filter(r =>
        filter === "all" ? true :
        filter === "synced" ? r.synced :
        !r.synced
    );
    $: pendingCount = rows.filter(r => !r.synced).length;

    onMount(load);
</script>

<h1>Lịch sử phiên (cục bộ)</h1>
<p>Tất cả phiên được lưu trên thiết bị, kể cả khi đang offline. Khi có mạng sẽ được đồng bộ lên server.</p>

{#if error}<div class="banner error">{error}</div>{/if}

<div class="row between" style="margin-bottom: 12px;">
    <div class="row" style="gap: 6px;">
        <button class:primary={filter === "all"} on:click={() => filter = "all"}>Tất cả ({rows.length})</button>
        <button class:primary={filter === "synced"} on:click={() => filter = "synced"}>Đã đồng bộ</button>
        <button class:primary={filter === "pending"} on:click={() => filter = "pending"}>
            Chờ đồng bộ{#if pendingCount > 0} ({pendingCount}){/if}
        </button>
    </div>
    <button on:click={load} disabled={loading}>
        {loading ? "Đang tải…" : "↻ Làm mới"}
    </button>
</div>

<div class="card" style="padding: 0; overflow: hidden;">
    {#if loading}
        <p class="muted center" style="padding: 32px;">Đang tải…</p>
    {:else if filtered.length === 0}
        <div class="empty" style="border: none; padding: 36px 20px;">
            <div class="icon">🗂</div>
            <p>{rows.length === 0 ? "Chưa có phiên nào." : "Không có phiên phù hợp bộ lọc."}</p>
        </div>
    {:else}
        <table>
            <thead>
                <tr>
                    <th>Bắt đầu</th>
                    <th>Kết thúc</th>
                    <th>Thời lượng</th>
                    <th>Bàn phím</th>
                    <th>Chuột</th>
                    <th>Ghi chú</th>
                    <th>Sync</th>
                </tr>
            </thead>
            <tbody>
                {#each filtered as r}
                    <tr>
                        <td>{fmt(r.started_at)}</td>
                        <td>{r.ended_at ? fmt(r.ended_at) : "—"}</td>
                        <td>{durationLabel(r)}</td>
                        <td>{r.keyboard_events}</td>
                        <td>{r.mouse_events}</td>
                        <td class="muted">{r.note ?? ""}</td>
                        <td>
                            {#if r.synced}
                                <span class="status-pill online" style="padding: 2px 8px; font-size: 11px;">
                                    <span class="dot"></span> đã sync
                                </span>
                            {:else}
                                <span class="status-pill idle" style="padding: 2px 8px; font-size: 11px;">
                                    <span class="dot"></span> chờ
                                </span>
                            {/if}
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>

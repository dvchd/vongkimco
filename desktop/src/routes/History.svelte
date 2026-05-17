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

    async function load() {
        loading = true;
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

    onMount(load);
</script>

<h1>Lịch sử phiên (cục bộ)</h1>
<p class="muted">Tất cả phiên được lưu trên thiết bị, kể cả khi đang offline. Khi có mạng sẽ được đồng bộ lên server.</p>

{#if error}<div class="banner error">{error}</div>{/if}

<div class="card">
    {#if loading}
        <p class="muted">Đang tải…</p>
    {:else if rows.length === 0}
        <p class="muted center">Chưa có phiên nào.</p>
    {:else}
        <table>
            <thead>
                <tr><th>Bắt đầu</th><th>Kết thúc</th><th>Bàn phím</th><th>Chuột</th><th>Ghi chú</th><th>Sync</th></tr>
            </thead>
            <tbody>
                {#each rows as r}
                    <tr>
                        <td>{fmt(r.started_at)}</td>
                        <td>{r.ended_at ? fmt(r.ended_at) : "—"}</td>
                        <td>{r.keyboard_events}</td>
                        <td>{r.mouse_events}</td>
                        <td class="muted">{r.note ?? ""}</td>
                        <td>{r.synced ? "✓" : "⏳"}</td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>

<button on:click={load}>Làm mới</button>

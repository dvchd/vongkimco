<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { loadSettings, settings } from "../lib/stores";

    const dispatch = createEventDispatcher();

    let url = $settings?.server_url ?? "https://vongkimco.hoctuthtien.com";
    let testing = false;
    let info: any = null;
    let error: string | null = null;

    async function testServer() {
        testing = true;
        error = null;
        info = null;
        try {
            const norm = url.replace(/\/+$/, "");
            info = await invoke("test_server", { url: norm });
        } catch (e: any) {
            error = e?.toString?.() ?? "Lỗi kết nối";
        }
        testing = false;
    }

    async function saveAndContinue() {
        const norm = url.replace(/\/+$/, "");
        await invoke("set_server_url", { url: norm });
        await loadSettings();
        dispatch("done");
    }
</script>

<div class="layout">
    <main class="main" style="grid-column: 1 / -1; max-width: 520px; margin: 0 auto; padding-top: 48px;">
        <h1>Chọn server</h1>
        <p class="muted">Chọn server backend mà ứng dụng sẽ gửi dữ liệu hoạt động lên. Mặc định là Vòng Kim Cô chính thức.</p>

        <div class="card">
            <div class="field">
                <label>URL server</label>
                <input type="url" bind:value={url} placeholder="https://vongkimco.hoctuthtien.com" />
            </div>

            <div class="row">
                <button on:click={testServer} disabled={testing}>{testing ? "Đang kiểm tra…" : "Kiểm tra"}</button>
                <button class="primary" on:click={saveAndContinue} disabled={!info}>Tiếp tục</button>
            </div>

            {#if info}
                <div class="banner ok small" style="margin-top: 12px;">
                    ✓ Kết nối tới <strong>{info.name}</strong> v{info.version}
                </div>
            {/if}
            {#if error}
                <div class="banner error small" style="margin-top: 12px;">{error}</div>
            {/if}
        </div>

        <p class="muted small">
            Bạn có thể đổi server bất kỳ lúc nào trong Cài đặt. Mã nguồn mở:
            <code>github.com/.../vongkimco</code>.
        </p>
    </main>
</div>

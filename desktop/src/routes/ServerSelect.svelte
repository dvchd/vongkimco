<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { loadSettings, settings } from "../lib/stores";

    const dispatch = createEventDispatcher();

    let url = $settings?.server_url ?? "https://vongkimco.hoctuthien.com";
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

    function useDefault() {
        url = "https://vongkimco.hoctuthien.com";
        info = null;
        error = null;
    }
</script>

<div class="layout">
    <main class="main" style="grid-column: 1 / -1; max-width: 560px; margin: 0 auto; padding-top: 56px;">
        <div style="display: flex; flex-direction: column; align-items: center; gap: 10px; margin-bottom: 18px;">
            <span style="width: 56px; height: 56px; background: var(--primary); border-radius: 50%; display: inline-flex; align-items: center; justify-content: center; font-size: 28px; color: #111; box-shadow: 0 0 0 1px rgba(212, 160, 23, 0.35), 0 0 16px rgba(212, 160, 23, 0.25);">🌐</span>
            <h1 style="margin: 0;">Chọn server</h1>
            <p class="muted small" style="margin: 0; text-align: center;">
                Server backend mà ứng dụng sẽ gửi dữ liệu hoạt động lên.
            </p>
        </div>

        <div class="card">
            <div class="field">
                <label>
                    URL server
                    <input type="url" bind:value={url} placeholder="https://vongkimco.hoctuthien.com" />
                </label>
                <div class="hint">
                    Dùng <code>https://vongkimco.hoctuthien.com</code> (mặc định) hoặc instance riêng của bạn.
                    <button class="link-btn" on:click={useDefault}>Dùng mặc định</button>
                </div>
            </div>

            <div class="row">
                <button on:click={testServer} disabled={testing || !url}>
                    {testing ? "Đang kiểm tra…" : "Kiểm tra kết nối"}
                </button>
                <button class="primary" on:click={saveAndContinue} disabled={!info}>
                    Tiếp tục →
                </button>
            </div>

            {#if info}
                <div class="banner ok small" style="margin-top: 12px;">
                    ✓ Đã kết nối tới <strong>{info.name}</strong> · API v{info.version}
                </div>
            {/if}
            {#if error}
                <div class="banner error small" style="margin-top: 12px;">⚠ {error}</div>
            {/if}
        </div>

        <p class="muted small center">
            Bạn có thể đổi server bất kỳ lúc nào trong <strong>Cài đặt</strong>.<br>
            Mã nguồn mở: <code>github.com/dvcuong-hust/vongkimco</code>
        </p>
    </main>
</div>

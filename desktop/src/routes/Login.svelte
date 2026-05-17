<script lang="ts">
    import { createEventDispatcher, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { open as openUrl } from "@tauri-apps/plugin-shell";
    import { loadUser, settings } from "../lib/stores";

    const dispatch = createEventDispatcher<{ done: void; "change-server": void }>();

    let starting = false;
    let userCode: string | null = null;
    let verificationUrl: string | null = null;
    let polling = false;
    let error: string | null = null;
    let pollTimer: any = null;

    async function startLink() {
        starting = true;
        error = null;
        try {
            const res = await invoke<any>("device_link_start");
            userCode = res.user_code;
            verificationUrl = res.verification_url;
            await openUrl(res.verification_url);
            polling = true;
            pollTimer = setInterval(poll, 3000);
        } catch (e: any) {
            error = e?.toString?.() ?? "Lỗi";
        }
        starting = false;
    }

    async function poll() {
        try {
            const res = await invoke<any>("device_link_poll");
            if (res.status === "approved") {
                clearInterval(pollTimer);
                pollTimer = null;
                polling = false;
                await loadUser();
                dispatch("done");
            } else if (res.status === "expired") {
                clearInterval(pollTimer);
                pollTimer = null;
                polling = false;
                error = "Mã đã hết hạn. Vui lòng thử lại.";
                userCode = null;
            }
        } catch (e: any) {
            // keep polling unless fatal
        }
    }

    function reopen() {
        if (verificationUrl) openUrl(verificationUrl);
    }

    onDestroy(() => {
        if (pollTimer) clearInterval(pollTimer);
    });
</script>

<div class="layout">
    <main class="main" style="grid-column: 1 / -1; max-width: 520px; margin: 0 auto; padding-top: 48px;">
        <h1>Đăng nhập</h1>
        <p class="muted">Đăng nhập bằng tài khoản Google đã được quản trị viên cấp quyền.</p>
        <p class="muted small">
            Server: {$settings?.server_url}
            <button class="link-btn" on:click={() => dispatch("change-server")}>Đổi</button>
        </p>

        <div class="card">
            {#if !userCode}
                <button class="primary" on:click={startLink} disabled={starting}>
                    {starting ? "Đang khởi tạo…" : "Đăng nhập bằng Google"}
                </button>
            {:else}
                <p>Mở trình duyệt và nhập mã sau:</p>
                <div class="code-box">{userCode}</div>
                <div class="row">
                    <button on:click={reopen}>Mở lại trình duyệt</button>
                    <span class="muted small">Đang chờ phê duyệt…</span>
                </div>
            {/if}

            {#if error}
                <div class="banner error small" style="margin-top: 12px;">{error}</div>
            {/if}
        </div>
    </main>
</div>

<style>
.link-btn {
    background: none;
    border: none;
    padding: 0;
    margin-left: 4px;
    color: var(--primary, #f5c518);
    cursor: pointer;
    font-size: inherit;
    text-decoration: underline;
}
</style>

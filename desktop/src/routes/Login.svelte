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

    function cancel() {
        if (pollTimer) clearInterval(pollTimer);
        pollTimer = null;
        polling = false;
        userCode = null;
        verificationUrl = null;
    }

    onDestroy(() => {
        if (pollTimer) clearInterval(pollTimer);
    });
</script>

<div class="layout">
    <main class="main" style="grid-column: 1 / -1; max-width: 560px; margin: 0 auto; padding-top: 56px;">
        <div style="display: flex; flex-direction: column; align-items: center; gap: 10px; margin-bottom: 18px;">
            <span style="width: 56px; height: 56px; background: var(--primary); border-radius: 50%; display: inline-flex; align-items: center; justify-content: center; font-size: 28px; color: #111; box-shadow: 0 0 0 1px rgba(212, 160, 23, 0.35), 0 0 16px rgba(212, 160, 23, 0.25);">⚙</span>
            <h1 style="margin: 0;">Đăng nhập</h1>
            <p class="muted small" style="margin: 0; text-align: center;">
                Đăng nhập bằng tài khoản Google đã được quản trị viên cấp quyền.
            </p>
        </div>

        <div class="card">
            <div class="row between" style="margin-bottom: 12px; align-items: center;">
                <div class="small muted">
                    Server: <strong style="color: var(--text);">{$settings?.server_url ?? "—"}</strong>
                </div>
                <button class="ghost small" on:click={() => dispatch("change-server")}>Đổi server</button>
            </div>

            {#if !userCode}
                <button class="primary" on:click={startLink} disabled={starting} style="width: 100%; padding: 12px;">
                    {starting ? "Đang khởi tạo…" : "Đăng nhập bằng Google"}
                </button>
                <p class="muted small" style="margin: 12px 0 0;">
                    Trình duyệt sẽ mở ra. Đăng nhập Google rồi nhập mã hiển thị tại đây để liên kết thiết bị này.
                </p>
            {:else}
                <p style="margin: 0 0 4px;">Nhập mã sau trên trình duyệt:</p>
                <div class="code-box">{userCode}</div>
                <div class="row between">
                    <span class="status-pill idle">
                        <span class="dot"></span> Đang chờ phê duyệt…
                    </span>
                    <div class="row" style="gap: 8px;">
                        <button on:click={reopen}>Mở lại trình duyệt</button>
                        <button class="ghost" on:click={cancel}>Huỷ</button>
                    </div>
                </div>
            {/if}

            {#if error}
                <div class="banner error small" style="margin-top: 12px;">{error}</div>
            {/if}
        </div>

        <p class="muted small center">
            Mã nguồn mở: <code>github.com/dvcuong-hust/vongkimco</code>
        </p>
    </main>
</div>

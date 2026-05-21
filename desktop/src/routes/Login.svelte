<script lang="ts">
    import { createEventDispatcher, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { open as openUrl } from "@tauri-apps/plugin-shell";
    import { loadUser, settings } from "../lib/stores";

    const dispatch = createEventDispatcher<{ done: void; "change-server": void }>();

    type StartResp = { flow_id: string; auth_url: string };

    let starting = false;
    let waiting = false;
    let authUrl: string | null = null;
    let error: string | null = null;
    let pollTimer: any = null;
    let attempts = 0;
    let copied = false;

    /// 2 seconds between polls, 120 attempts → 4 minutes total. The flow
    /// itself expires after 8 minutes server-side, but the user is unlikely
    /// to still be staring at the app if they didn't approve in 4.
    const POLL_INTERVAL_MS = 2000;
    const POLL_MAX_ATTEMPTS = 120;

    async function startLink() {
        starting = true;
        error = null;
        try {
            const res = await invoke<StartResp>("auth_start");
            authUrl = res.auth_url;
            await openUrl(res.auth_url);
            waiting = true;
            attempts = 0;
            pollTimer = setInterval(poll, POLL_INTERVAL_MS);
        } catch (e: any) {
            error = e?.toString?.() ?? "Lỗi khi khởi tạo đăng nhập";
        }
        starting = false;
    }

    async function poll() {
        attempts += 1;
        if (attempts > POLL_MAX_ATTEMPTS) {
            stopPolling();
            error = "Hết thời gian chờ. Vui lòng thử lại.";
            return;
        }
        try {
            const res = await invoke<any>("auth_poll");
            if (res.status === "completed") {
                stopPolling();
                await loadUser();
                dispatch("done");
            } else if (res.status === "expired") {
                stopPolling();
                error = "Phiên đăng nhập đã hết hạn. Vui lòng thử lại.";
            } else if (res.status === "device_limit_exceeded") {
                stopPolling();
                error =
                    "Bạn đã đạt giới hạn số thiết bị. Hãy gỡ một thiết bị cũ trong trang quản trị rồi thử lại.";
            } else if (res.status === "not_member") {
                stopPolling();
                error =
                    "Tài khoản của bạn chưa được duyệt làm thành viên. Vui lòng đăng ký yêu cầu thành viên trên web và liên hệ quản trị viên.";
            }
            // status === "pending" → keep polling silently
        } catch (e: any) {
            // Network error: keep polling. The server may be slow or offline
            // momentarily; the periodic retry handles transient blips.
        }
    }

    function stopPolling() {
        if (pollTimer) clearInterval(pollTimer);
        pollTimer = null;
        waiting = false;
    }

    function reopen() {
        if (authUrl) openUrl(authUrl);
    }

    async function copyLink() {
        if (!authUrl) return;
        try {
            await navigator.clipboard.writeText(authUrl);
            copied = true;
            setTimeout(() => (copied = false), 2000);
        } catch {
            selectUrl();
        }
    }

    function selectUrl() {
        const el = document.getElementById("auth-url-input");
        if (el && el instanceof HTMLInputElement) {
            el.select();
        }
    }

    async function cancel() {
        stopPolling();
        authUrl = null;
        try {
            await invoke("auth_cancel");
        } catch {}
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

            {#if !waiting}
                <button class="primary" on:click={startLink} disabled={starting} style="width: 100%; padding: 12px;">
                    {starting ? "Đang mở trình duyệt…" : "Đăng nhập bằng Google"}
                </button>
                <p class="muted small" style="margin: 12px 0 0;">
                    Trình duyệt sẽ mở ra. Đăng nhập Google trên trình duyệt và quay lại — ứng dụng sẽ tự nhận diện đăng nhập.
                </p>
            {:else}
                <div class="row" style="align-items: center; gap: 12px;">
                    <span class="status-pill idle">
                        <span class="dot"></span> Đang chờ đăng nhập…
                    </span>
                </div>

                <!-- Copyable auth URL for cases where browser doesn't open correctly -->
                <div class="auth-url-box">
                    <div class="muted small" style="margin-bottom: 6px;">
                        Nếu trình duyệt không mở đúng link, hãy copy link bên dưới và mở thủ công:
                    </div>
                    <div class="auth-url-row">
                        <input
                            id="auth-url-input"
                            type="text"
                            readonly
                            value={authUrl ?? ""}
                            class="auth-url-input"
                            on:click={selectUrl}
                        />
                        <button class="primary small" on:click={copyLink} style="flex-shrink: 0; white-space: nowrap;">
                            {copied ? "✓ Đã copy" : "Copy link"}
                        </button>
                    </div>
                </div>

                <div class="row" style="gap: 8px; margin-top: 12px;">
                    <button on:click={reopen}>Mở lại trình duyệt</button>
                    <button class="ghost" on:click={cancel}>Huỷ</button>
                </div>
            {/if}

            {#if error}
                <div class="banner error small" style="margin-top: 12px;">{error}</div>
            {/if}
        </div>

        <p class="muted small center">
            Mã nguồn mở: <code>github.com/dvchd/vongkimco</code>
        </p>
    </main>
</div>

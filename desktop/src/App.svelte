<script lang="ts">
    import { onMount } from "svelte";
    import { getVersion } from "@tauri-apps/api/app";
    import {
        loadSettings,
        loadPolicy,
        loadUser,
        refreshStatus,
        startStatusListener,
        settings,
        user,
        sessionState,
        route,
        applyTheme,
        installThemeAutoListener,
    } from "./lib/stores";
    import Home from "./routes/Home.svelte";
    import Login from "./routes/Login.svelte";
    import Settings from "./routes/Settings.svelte";
    import ServerSelect from "./routes/ServerSelect.svelte";
    import History from "./routes/History.svelte";
    import UpdateBanner from "./lib/UpdateBanner.svelte";
    import { checkForUpdate, startPeriodicCheck } from "./lib/updater";

    let booted = false;
    let appVersion = "";

    onMount(async () => {
        await loadSettings();
        applyTheme($settings?.theme || "auto");
        installThemeAutoListener();
        await loadPolicy();
        await loadUser();
        await refreshStatus();
        await startStatusListener();
        if (!$settings) {
            $route = "server";
        } else if (!$user) {
            $route = "login";
        } else {
            $route = "home";
        }
        booted = true;

        checkForUpdate({ silent: true }).catch(() => {});
        startPeriodicCheck({ intervalMs: 4 * 60 * 60 * 1000, autoDownload: false });

        try { appVersion = await getVersion(); } catch {}
    });

    function go(r: string) {
        $route = r;
    }

    function serverHost(url: string | undefined | null): string {
        if (!url) return "—";
        try {
            return new URL(url).host;
        } catch {
            return url;
        }
    }

    function changeServer() {
        $route = "server";
    }
</script>

{#if !booted}
    <div class="layout">
        <div class="main">
            <div class="empty" style="margin-top: 80px;">
                <div class="icon">⚙</div>
                <p class="muted">Đang khởi tạo Vòng Kim Cô…</p>
            </div>
        </div>
    </div>
{:else if $route === "server"}
    <ServerSelect on:done={() => go($user ? "home" : "login")} />
{:else if $route === "login"}
    <Login on:done={() => go("home")} on:change-server={() => go("server")} />
{:else}
    <div class="layout">
        <aside class="sidebar">
            <div class="brand">
                <span class="logo">⚙</span>
                <span>Vòng Kim Cô</span>
            </div>
            <button type="button" class="nav-link" class:active={$route === "home"} on:click={() => go("home")}>
                <span class="icon">▶</span><span>Phiên làm việc</span>
            </button>
            <button type="button" class="nav-link" class:active={$route === "history"} on:click={() => go("history")}>
                <span class="icon">🕒</span><span>Lịch sử</span>
            </button>
            <button type="button" class="nav-link" class:active={$route === "settings"} on:click={() => go("settings")}>
                <span class="icon">⚙</span><span>Cài đặt</span>
            </button>
            <div class="footer">
                {#if $user}
                    <div class="user-email">{$user.email}</div>
                {/if}
                <div class="server-section">
                    <div class="server-row">
                        <span class="status-pill {$sessionState.online ? 'online' : 'offline'}">
                            <span class="dot"></span>
                            {$sessionState.online ? "Online" : "Offline"}
                        </span>
                        <span class="server-host-label">{serverHost($settings?.server_url)}</span>
                    </div>
                    <button class="server-change-btn" on:click={changeServer}>
                        🔄 Đổi server
                    </button>
                </div>
                <div class="muted small" style="margin-top: 6px;">v{appVersion || "?"}</div>
            </div>
        </aside>
        <main class="main">
            <UpdateBanner />
            {#if $route === "home"}<Home />
            {:else if $route === "history"}<History />
            {:else if $route === "settings"}<Settings />
            {/if}
        </main>
    </div>
{/if}

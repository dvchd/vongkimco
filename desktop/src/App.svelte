<script lang="ts">
    import { onMount } from "svelte";
    import {
        loadSettings,
        loadUser,
        refreshStatus,
        startStatusListener,
        settings,
        user,
        sessionState,
        route
    } from "./lib/stores";
    import Home from "./routes/Home.svelte";
    import Login from "./routes/Login.svelte";
    import Settings from "./routes/Settings.svelte";
    import ServerSelect from "./routes/ServerSelect.svelte";
    import History from "./routes/History.svelte";
    import UpdateBanner from "./lib/UpdateBanner.svelte";
    import { checkForUpdate } from "./lib/updater";

    let booted = false;

    onMount(async () => {
        await loadSettings();
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

        // Silent update check on startup. Errors stay silent here; the
        // Settings page has an explicit "Check for update" button.
        checkForUpdate({ silent: true }).catch(() => {});
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
            <button type="button" class="nav-link" class:active={$route === "server"} on:click={() => go("server")}>
                <span class="icon">🌐</span><span>Server</span>
            </button>
            <div class="footer">
                {#if $user}
                    <div class="user-email">{$user.email}</div>
                {/if}
                <div class="muted small">{serverHost($settings?.server_url)}</div>
                <div class="muted small" style="margin-top: 6px;">
                    <span class="status-pill {$sessionState.online ? 'online' : 'offline'}" style="padding: 2px 8px; font-size: 10px;">
                        <span class="dot"></span>
                        {$sessionState.online ? "Online" : "Offline"}
                    </span>
                </div>
                <div class="muted small" style="margin-top: 8px;">v0.1.0 · OSS</div>
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

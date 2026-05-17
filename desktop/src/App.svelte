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
</script>

{#if !booted}
    <div class="layout">
        <div class="main">
            <p class="muted">Đang khởi tạo…</p>
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
            <a class:active={$route === "home"} on:click={() => go("home")}>Phiên làm việc</a>
            <a class:active={$route === "history"} on:click={() => go("history")}>Lịch sử</a>
            <a class:active={$route === "settings"} on:click={() => go("settings")}>Cài đặt</a>
            <a class:active={$route === "server"} on:click={() => go("server")}>Server</a>
            <div class="footer">
                {#if $user}
                    <div>{$user.email}</div>
                {/if}
                <div class="muted small">v0.1.0 · OSS</div>
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

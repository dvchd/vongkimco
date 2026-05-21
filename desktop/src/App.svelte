<script lang="ts">
    import { onMount } from "svelte";
    import { getVersion } from "@tauri-apps/api/app";
    import { listen } from "@tauri-apps/api/event";
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

        // Fade out and remove the static splash from index.html
        const splash = document.getElementById("splash");
        if (splash) {
            splash.classList.add("fade-out");
            setTimeout(() => splash.remove(), 350);
        }

        // Listen for boot completion from Rust — session may have been restored
        // from keyring after the initial loadUser() call. If user is now present,
        // switch away from the login screen.
        const unlisten = await listen("vkc://booted", async () => {
            await loadUser();
            if ($user && $route === "login") {
                $route = "home";
            }
        });

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
    <!-- Shown only briefly if the index.html splash was removed too early
         or on very slow loads. Uses the same logo + spinner. -->
    <div class="splash-screen">
        <svg class="splash-logo" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
            <defs>
                <linearGradient id="app-gold" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" stop-color="#fbbf24" />
                    <stop offset="50%" stop-color="#f59e0b" />
                    <stop offset="100%" stop-color="#b45309" />
                </linearGradient>
                <linearGradient id="app-highlight" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" stop-color="#fef08a" stop-opacity="0.8" />
                    <stop offset="50%" stop-color="#fcd34d" stop-opacity="0.2" />
                    <stop offset="100%" stop-color="#fef08a" stop-opacity="0.8" />
                </linearGradient>
                <filter id="app-glow" x="-50%" y="-50%" width="200%" height="200%">
                    <feGaussianBlur stdDeviation="6" result="coloredBlur"/>
                    <feMerge><feMergeNode in="coloredBlur"/><feMergeNode in="SourceGraphic"/></feMerge>
                </filter>
            </defs>
            <rect x="16" y="16" width="480" height="480" rx="112" fill="var(--bg)" />
            <circle cx="256" cy="256" r="120" fill="none" stroke="#f59e0b" stroke-width="2" stroke-opacity="0.2" stroke-dasharray="8 12" />
            <circle cx="256" cy="256" r="80" fill="none" stroke="#f59e0b" stroke-width="2" stroke-opacity="0.5" stroke-dasharray="4 8" />
            <circle cx="256" cy="256" r="40" fill="none" stroke="#fcd34d" stroke-width="1.5" stroke-opacity="0.8" stroke-dasharray="2 4" />
            <circle class="focus-dot" cx="256" cy="256" r="12" fill="#22d3ee" filter="url(#app-glow)" />
            <circle cx="256" cy="256" r="4" fill="#ffffff" />
            <g class="headband-spin" transform="translate(0, -24)">
                <path d="M 256 150 C 290 100, 380 120, 340 180 C 360 220, 400 310, 256 390 C 112 310, 152 220, 172 180 C 132 120, 222 100, 256 150 Z"
                    fill="none" stroke="url(#app-gold)" stroke-width="32" stroke-linecap="round" stroke-linejoin="round" />
                <path d="M 256 150 C 290 100, 380 120, 340 180 C 360 220, 400 310, 256 390 C 112 310, 152 220, 172 180 C 132 120, 222 100, 256 150 Z"
                    fill="none" stroke="url(#app-highlight)" stroke-width="10" stroke-linecap="round" stroke-linejoin="round" />
            </g>
        </svg>
        <p class="muted" style="margin-top: 14px; font-size: 14px;">Đang khởi tạo Vòng Kim Cô…</p>
    </div>
{:else if $route === "server"}
    <ServerSelect on:done={() => go($user ? "home" : "login")} />
    <UpdateBanner />
{:else if $route === "login"}
    <Login on:done={() => go("home")} on:change-server={() => go("server")} />
    <UpdateBanner />
{:else}
    <div class="layout">
        <aside class="sidebar">
            <div class="brand">
                <svg class="brand-logo" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
                    <defs>
                        <linearGradient id="bl-gold" x1="0%" y1="0%" x2="100%" y2="100%">
                            <stop offset="0%" stop-color="#fbbf24" />
                            <stop offset="50%" stop-color="#f59e0b" />
                            <stop offset="100%" stop-color="#b45309" />
                        </linearGradient>
                    </defs>
                    <rect x="16" y="16" width="480" height="480" rx="112" fill="var(--surface)" />
                    <circle cx="256" cy="256" r="80" fill="none" stroke="url(#bl-gold)" stroke-width="2" stroke-opacity="0.5" stroke-dasharray="4 8" />
                    <circle cx="256" cy="256" r="12" fill="#22d3ee" />
                    <circle cx="256" cy="256" r="4" fill="#ffffff" />
                    <g transform="translate(0, -24)">
                        <path d="M 256 150 C 290 100, 380 120, 340 180 C 360 220, 400 310, 256 390 C 112 310, 152 220, 172 180 C 132 120, 222 100, 256 150 Z"
                            fill="none" stroke="url(#bl-gold)" stroke-width="32" stroke-linecap="round" stroke-linejoin="round" />
                    </g>
                </svg>
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

<style>
    /* ── Splash / Loading screen ──────────────────────────────────── */
    .splash-screen {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        height: 100vh;
        background: var(--bg);
    }
    .splash-logo {
        width: 80px;
        height: 80px;
    }
    /* Vòng kim cô tightening animation */
    .headband-spin {
        transform-origin: center;
        animation: headband-tighten 2s ease-in-out infinite;
    }
    @keyframes headband-tighten {
        0%   { transform: translate(0, -24) scale(1);    opacity: 0.7; }
        50%  { transform: translate(0, -24) scale(0.86); opacity: 1;   }
        100% { transform: translate(0, -24) scale(1);    opacity: 0.7; }
    }
    .focus-dot {
        animation: dot-glow 2s ease-in-out infinite;
    }
    @keyframes dot-glow {
        0%, 100% { opacity: 0.5; }
        50%      { opacity: 1;   }
    }

    /* ── Sidebar brand logo ───────────────────────────────────────── */
    .brand-logo {
        width: 28px;
        height: 28px;
        border-radius: 50%;
        box-shadow: 0 0 0 1px rgba(212, 160, 23, 0.35), 0 0 12px rgba(212, 160, 23, 0.2);
    }
</style>

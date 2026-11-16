<script>
  import { t } from "./lib/i18n.js";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Sidebar from "./lib/Sidebar.svelte";
  import DevicePanel from "./lib/DevicePanel.svelte";
  import UdevModal from "./lib/UdevModal.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { locale, selectLocale } from "./lib/i18n.js";
  import { snapshot, setSetting, setTxSetting, udevHelp } from "./lib/api.js";

  // Custom window chrome. macOS keeps its native traffic lights (via the
  // Overlay title bar); Windows/Linux use our own frameless controls.
  const appWindow = getCurrentWindow();
  const isMac = /Mac/i.test(navigator.userAgent || navigator.platform || "");
  let maximized = $state(false);
  async function refreshMax() {
    try {
      maximized = await appWindow.isMaximized();
    } catch {
      /* ignore */
    }
  }
  const minimizeWin = () => appWindow.minimize();
  const toggleMaxWin = () => appWindow.toggleMaximize();
  const closeWin = () => appWindow.close();

  // Frameless windows lose the compositor's edge-resize borders, so we add our
  // own invisible handles (Linux/Windows; macOS keeps its native resize).
  const startResize = (dir) => (e) => {
    if (e.button === 0) appWindow.startResizeDragging(dir);
  };

  let snap = $state(null);
  let selected = $state(null);
  // Distinguishes "nothing selected yet" (auto-select the first device once
  // one shows up) from "the user deliberately cleared the selection" (leave
  // it cleared until they pick one again).
  let userDeselected = $state(false);
  let sidebarOpen = $state(true);
  let pending = $state({});
  let optimistic = $state({}); // values shown immediately, before the device confirms
  // Per-TX-targeted settings (currently just Voice Tone) don't fit the flat
  // `settings`/`optimistic` maps above, since the same setting id can hold a
  // different value per transmitter slot — keyed by 0-based TX index instead.
  let pendingTx = $state({});
  let optimisticVoiceTone = $state({});
  let view = $state("compact");
  let error = $state(null);

  let showUdev = $state(false);
  let help = $state(null);

  const devices = $derived(snap?.devices ?? []);
  const selectedDevice = $derived(devices.find((d) => d.id === selected) ?? null);
  const rawStatus = $derived(selectedDevice ? snap?.status ?? null : null);
  // Merge live values with any not-yet-confirmed local changes for instant feedback.
  const values = $derived({ ...(rawStatus?.settings ?? {}), ...optimistic });
  // Same, for each TX slot's Voice Tone.
  const status = $derived(
    rawStatus && {
      ...rawStatus,
      tx: rawStatus.tx.map((t, i) =>
        t && optimisticVoiceTone[i] !== undefined ? { ...t, voice_tone: optimisticVoiceTone[i] } : t,
      ),
    },
  );
  const linuxAccessIssue = $derived(
    snap?.os === "linux" && snap?.probe?.permission_issue && devices.length === 0,
  );

  async function refresh() {
    try {
      const next = await snapshot(selected);
      snap = next;
      // Auto-select once devices show up, unless the user deliberately
      // cleared the selection (clicking empty space in the sidebar).
      if (!selected && !userDeselected && next.devices.length > 0) {
        selected = next.devices[0].id;
      }
      // Drop a stale selection.
      if (selected && !next.devices.some((d) => d.id === selected)) {
        selected = next.devices[0]?.id ?? null;
      }
      // Retire optimistic values the device has now confirmed.
      const confirmed = next.status?.settings ?? {};
      let changed = false;
      const remaining = { ...optimistic };
      for (const k of Object.keys(remaining)) {
        if (confirmed[k] === remaining[k]) {
          delete remaining[k];
          changed = true;
        }
      }
      if (changed) optimistic = remaining;

      // Same, for each TX slot's Voice Tone.
      const confirmedTx = next.status?.tx ?? [];
      let changedTx = false;
      const remainingTx = { ...optimisticVoiceTone };
      for (const k of Object.keys(remainingTx)) {
        if (confirmedTx[k]?.voice_tone === remainingTx[k]) {
          delete remainingTx[k];
          changedTx = true;
        }
      }
      if (changedTx) optimisticVoiceTone = remainingTx;
    } catch (e) {
      error = String(e);
    }
  }

  function select(id) {
    selected = id;
    userDeselected = id === null;
    refresh();
  }

  async function change(settingId, value) {
    if (!selected) return;
    optimistic = { ...optimistic, [settingId]: value }; // reflect the flip instantly
    pending = { ...pending, [settingId]: true };
    error = null;
    try {
      await setSetting(selected, settingId, value);
      await refresh();
    } catch (e) {
      error = String(e);
      const { [settingId]: _drop, ...rest } = optimistic;
      optimistic = rest;
    } finally {
      const { [settingId]: _p, ...rest } = pending;
      pending = rest;
    }
  }

  async function changeTx(tx, settingId, value) {
    if (!selected) return;
    // Only Voice Tone exists today; extend this map if a second per-TX
    // setting is added.
    if (settingId === "voice-tone") {
      optimisticVoiceTone = { ...optimisticVoiceTone, [tx]: value }; // reflect instantly
    }
    pendingTx = { ...pendingTx, [tx]: true };
    error = null;
    try {
      await setTxSetting(selected, tx, settingId, value);
      await refresh();
    } catch (e) {
      error = String(e);
      const { [tx]: _drop, ...rest } = optimisticVoiceTone;
      optimisticVoiceTone = rest;
    } finally {
      const { [tx]: _p, ...rest } = pendingTx;
      pendingTx = rest;
    }
  }

  async function openUdev() {
    if (!help) help = await udevHelp();
    showUdev = true;
  }

  $effect(() => {
    refresh();
    const timer = setInterval(refresh, 150);
    return () => clearInterval(timer);
  });

  // Keep the maximize/restore icon in sync with the window state.
  $effect(() => {
    refreshMax();
    const onResize = () => refreshMax();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  });

  $effect(() => {
    document.documentElement.lang = $locale;
    invoke("set_locale", { locale: $locale }).catch((e) => { error = String(e); });
  });
</script>

<div class="app" class:mac={isMac}>
  {#if !isMac}
    <div class="rz rz-n" onmousedown={startResize("North")}></div>
    <div class="rz rz-s" onmousedown={startResize("South")}></div>
    <div class="rz rz-e" onmousedown={startResize("East")}></div>
    <div class="rz rz-w" onmousedown={startResize("West")}></div>
    <div class="rz rz-ne" onmousedown={startResize("NorthEast")}></div>
    <div class="rz rz-nw" onmousedown={startResize("NorthWest")}></div>
    <div class="rz rz-se" onmousedown={startResize("SouthEast")}></div>
    <div class="rz rz-sw" onmousedown={startResize("SouthWest")}></div>
  {/if}

  <div class="topbar" class:mac={isMac} data-tauri-drag-region>
    <button class="icon-btn" onclick={() => (sidebarOpen = !sidebarOpen)} aria-label={$t("Toggle devices")}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
        <path d="M3.5 6h17M3.5 12h17M3.5 18h17" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
      </svg>
    </button>
    <span class="brand" data-tauri-drag-region>DJI Mic Control</span>
    <div class="drag-fill" data-tauri-drag-region></div>
    <select class="language-select" aria-label={$t("Language")} value={$locale}
      onchange={(event) => selectLocale(event.currentTarget.value)}>
      <option value="en">English</option>
      <option value="zh-CN">简体中文</option>
    </select>

    {#if !isMac}
      <div class="win-controls">
        <button class="win-btn" onclick={minimizeWin} aria-label={$t("Minimize")}>
          <svg width="11" height="11" viewBox="0 0 12 12"><path d="M2 6h8" stroke="currentColor" stroke-width="1.2" /></svg>
        </button>
        <button class="win-btn" onclick={toggleMaxWin} aria-label={maximized ? $t("Restore") : $t("Maximize")}>
          {#if maximized}
            <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.1">
              <rect x="3.2" y="1.6" width="6.2" height="6.2" rx="1" /><rect x="1.6" y="3.6" width="6.2" height="6.8" rx="1" fill="var(--bg-panel)" />
            </svg>
          {:else}
            <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.1"><rect x="2" y="2" width="8" height="8" rx="1" /></svg>
          {/if}
        </button>
        <button class="win-btn close" onclick={closeWin} aria-label={$t("Close")}>
          <svg width="11" height="11" viewBox="0 0 12 12"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" /></svg>
        </button>
      </div>
    {/if}
  </div>

  {#if linuxAccessIssue}
    <button class="banner" onclick={openUdev}>
      {$t("A microphone is connected but not accessible — click to fix USB permissions.")}
    </button>
  {/if}

  {#if error}
    <div class="banner err" role="alert">{error}</div>
  {/if}

  <div class="body">
    <div class="sidebar-wrap" class:open={sidebarOpen}>
      <Sidebar {devices} {selected} onselect={select} />
    </div>

    <main class="main">
      {#if selectedDevice && status}
        <DevicePanel
          device={selectedDevice}
          {status}
          settings={snap.settings}
          {values}
          {pending}
          {pendingTx}
          {view}
          onchange={change}
          onchangeTx={changeTx}
          onview={(v) => (view = v)}
        />
      {:else}
        <div class="placeholder">
          <div class="ph-card">
            {#if linuxAccessIssue}
              <h2>{$t("Permission needed")}</h2>
              <p>{$t("A supported microphone is connected but can't be accessed.")}</p>
              <button class="cta" onclick={openUdev}>{$t("Show setup instructions")}</button>
            {:else if devices.length > 0}
              <h2>{$t("No device selected")}</h2>
              <p>{$t("Choose a device from the sidebar.")}</p>
            {:else}
              <h2>{$t("No microphone connected")}</h2>
              <p>{$t("Connect a supported DJI microphone over USB to get started.")}</p>
            {/if}
          </div>
        </div>
      {/if}
    </main>
  </div>
</div>

{#if showUdev && help}
  <UdevModal {help} onclose={() => (showUdev = false)} />
{/if}

<style>
  .language-select {
    position: relative;
    max-width: 120px;
    margin-right: 8px;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-panel);
    color: var(--text);
    font-size: 12px;
  }

  .app {
    height: 100%;
    display: flex;
    flex-direction: column;
    /* The window is frameless on some platforms (no native border/shadow),
       so a solid-color desktop behind it can make the edge disappear
       entirely without this. Not needed on macOS, which already has native
       decorations (see .mac below). */
    border: 1px solid var(--border);
  }
  .app.mac {
    border: none;
  }
  /* Invisible resize handles pinned to the window edges/corners. */
  .rz {
    position: fixed;
    z-index: 100;
  }
  .rz-n {
    top: 0;
    left: 8px;
    right: 8px;
    height: 5px;
    cursor: ns-resize;
  }
  .rz-s {
    bottom: 0;
    left: 8px;
    right: 8px;
    height: 5px;
    cursor: ns-resize;
  }
  .rz-e {
    top: 8px;
    bottom: 8px;
    right: 0;
    width: 5px;
    cursor: ew-resize;
  }
  .rz-w {
    top: 8px;
    bottom: 8px;
    left: 0;
    width: 5px;
    cursor: ew-resize;
  }
  .rz-ne {
    top: 0;
    right: 0;
    width: 11px;
    height: 11px;
    cursor: nesw-resize;
  }
  .rz-nw {
    top: 0;
    left: 0;
    width: 11px;
    height: 11px;
    cursor: nwse-resize;
  }
  .rz-se {
    bottom: 0;
    right: 0;
    width: 11px;
    height: 11px;
    cursor: nwse-resize;
  }
  .rz-sw {
    bottom: 0;
    left: 0;
    width: 11px;
    height: 11px;
    cursor: nesw-resize;
  }
  .topbar {
    position: relative;
    flex: 0 0 auto;
    height: 30px;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 2px 0 4px;
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border);
    user-select: none;
  }
  /* Leave room for the macOS traffic lights (top-left) under the overlay title bar. */
  .topbar.mac {
    padding-left: 76px;
  }
  .drag-fill {
    flex: 1 1 auto;
    align-self: stretch;
  }
  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    line-height: 0;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-dim);
  }
  .icon-btn:hover {
    background: var(--bg-elev);
    color: var(--text);
  }
  /* Kill inline-svg baseline gap so icons sit dead-center in their buttons. */
  .topbar button svg {
    display: block;
  }
  /* Absolutely centered so it stays put regardless of the side controls. */
  .brand {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    font-weight: 600;
    font-size: 13px;
    letter-spacing: 0.01em;
    color: var(--text-dim);
    white-space: nowrap;
  }
  .win-controls {
    display: flex;
    align-self: stretch;
    margin-left: 4px;
  }
  .win-btn {
    width: 40px;
    align-self: stretch;
    display: grid;
    place-items: center;
    border: none;
    background: transparent;
    color: var(--text-dim);
    transition: background 0.12s, color 0.12s;
  }
  .win-btn:hover {
    background: var(--bg-elev);
    color: var(--text);
  }
  .win-btn.close:hover {
    background: #e5484d;
    color: #fff;
  }
  .banner {
    flex: 0 0 auto;
    text-align: left;
    border: none;
    padding: 10px 16px;
    background: color-mix(in srgb, var(--warn) 16%, var(--bg-panel));
    color: var(--text);
    border-bottom: 1px solid color-mix(in srgb, var(--warn) 40%, transparent);
    font-size: 13px;
  }
  .banner.err {
    background: color-mix(in srgb, var(--danger) 16%, var(--bg-panel));
    border-bottom-color: color-mix(in srgb, var(--danger) 40%, transparent);
  }
  .body {
    flex: 1 1 auto;
    display: flex;
    min-height: 0;
  }
  /* Collapsible sidebar: animate width so the panel slides in/out. The inner
     Sidebar keeps a fixed width and is clipped, giving a smooth slide. */
  .sidebar-wrap {
    flex: 0 0 auto;
    width: 260px;
    overflow: hidden;
    border-right: 1px solid var(--border);
    transition: width 0.22s ease, border-color 0.22s ease;
  }
  .sidebar-wrap:not(.open) {
    width: 0;
    border-right-color: transparent;
  }
  .main {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
  }
  .placeholder {
    flex: 1 1 auto;
    display: grid;
    place-items: center;
    padding: 24px;
  }
  .ph-card {
    text-align: center;
    max-width: 340px;
    color: var(--text-dim);
  }
  .ph-card h2 {
    margin: 0 0 8px;
    color: var(--text);
    font-size: 18px;
  }
  .ph-card p {
    margin: 0 0 16px;
    line-height: 1.5;
  }
  .cta {
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--accent-contrast);
    padding: 8px 16px;
    border-radius: var(--radius-sm);
    font-weight: 600;
  }
</style>

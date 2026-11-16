<script>
  import { t } from "./i18n.js";
  import DevicePicto from "./DevicePicto.svelte";
  import BatteryIcon from "./BatteryIcon.svelte";

  let { devices = [], selected = null, onselect } = $props();
</script>

<aside
  class="sidebar"
  onclick={(e) => {
    /* Clicking anywhere that isn't a device row (the header, gaps between
       rows, or empty space below the list) deselects the current device. */
    if (!e.target.closest(".device")) onselect?.(null);
  }}
>
  <div class="sidebar-head">{$t("Devices")}</div>

  {#if devices.length === 0}
    <div class="empty">{$t("No devices")}</div>
  {:else}
    <ul class="device-list">
      {#each devices as d (d.id)}
        <li>
          <button
            class="device"
            class:active={d.id === selected}
            onclick={() => onselect?.(d.id)}
          >
            <span class="picto-wrap">
              <DevicePicto pictogram={`${d.pictogram_key}-rx`} size={40} />
            </span>
            <span class="labels">
              <span class="name">{d.model_name}</span>
              <span class="serial">{d.rx_serial ?? d.id}</span>
              {#if d.tx?.some((t) => t)}
                <span class="tx-batteries">
                  {#each d.tx as tx, i}
                    {#if tx}
                      <span class="tx-battery">
                        <span class="tx-battery-label">TX{i + 1}</span>
                        <BatteryIcon value={tx.battery} charging={!!tx.charging} size={16} />
                      </span>
                    {/if}
                  {/each}
                </span>
              {/if}
            </span>
            <span
              class="dot"
              class:connected={d.connected}
              title={d.connected ? $t("Connected") : $t("Connecting\u2026")}
            ></span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</aside>

<style>
  .sidebar {
    width: 260px;
    height: 100%;
    background: var(--bg-panel);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }
  .sidebar-head {
    padding: 16px 16px 8px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-faint);
  }
  .empty {
    padding: 12px 16px;
    color: var(--text-faint);
  }
  .device-list {
    list-style: none;
    margin: 0;
    padding: 6px 10px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .device {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    text-align: left;
    transition: background 0.12s, border-color 0.12s;
  }
  .device:hover {
    background: var(--bg-elev);
  }
  .device.active {
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
  }
  .picto-wrap {
    flex: 0 0 40px;
    height: 40px;
    display: grid;
    place-items: center;
  }
  .labels {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }
  .name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .serial {
    font-size: 12px;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tx-batteries {
    display: flex;
    gap: 12px;
    margin-top: 4px;
  }
  .tx-battery {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .tx-battery-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: var(--text-faint);
  }
  .dot {
    flex: 0 0 auto;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--text-faint);
  }
  .dot.connected {
    background: var(--good);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--good) 22%, transparent);
  }
</style>

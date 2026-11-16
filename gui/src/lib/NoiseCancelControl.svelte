<script>
  import { t } from "./i18n.js";
  // Noise Cancel's three settings (power, mode, and the TX-button toggle) are
  // closely related, so they render as one combined element instead of three
  // separate rows: a main switch for "Noise Cancelling", with the Basic/Strong
  // mode picker and the TX-button toggle nested underneath it.
  let {
    power,
    mode,
    button,
    values = {},
    pending = {},
    lockReason,
    compact = false,
    onchange,
  } = $props();

  const powerValue = $derived(values[power.id] ?? null);
  const modeValue = $derived(values[mode.id] ?? null);
  const buttonValue = $derived(values[button.id] ?? null);

  const powerReason = $derived(lockReason(power));
  const modeReason = $derived(lockReason(mode));
  const buttonReason = $derived(lockReason(button));

  const powerLocked = $derived(!!powerReason);
  const modeLocked = $derived(!!modeReason);
  const buttonLocked = $derived(!!buttonReason);

  const powerDisabled = $derived(powerLocked || !!pending[power.id]);
  const modeDisabled = $derived(modeLocked || !!pending[mode.id]);
  const buttonDisabled = $derived(buttonLocked || !!pending[button.id]);

  const powerChecked = $derived(powerValue === power.options[1]?.value);
  const buttonChecked = $derived(buttonValue === button.options[1]?.value);

  function togglePower() {
    if (powerDisabled || powerValue === null) return;
    onchange?.(power.id, powerChecked ? power.options[0].value : power.options[1].value);
  }
  function toggleButton() {
    if (buttonDisabled || buttonValue === null) return;
    onchange?.(button.id, buttonChecked ? button.options[0].value : button.options[1].value);
  }
  function chooseMode(v) {
    if (modeDisabled || v === modeValue) return;
    onchange?.(mode.id, v);
  }
</script>

<div class="ncgroup" class:compact>
  <div class="ncmain" class:busy={!!pending[power.id]} title={powerLocked ? powerReason : null}>
    <div class="label">
      <span class="title">{$t(power.label)}</span>
      {#if powerLocked}
        <span class="lock" title={powerReason} aria-label={powerReason}>🔒</span>
      {/if}
    </div>
    <div class="control" class:dim={powerLocked}>
      {#if powerValue === null}
        <span class="unknown">—</span>
      {:else}
        <button
          class="switch"
          class:on={powerChecked}
          role="switch"
          aria-checked={powerChecked}
          aria-label={$t(power.label)}
          onclick={togglePower}
        >
          <span class="knob"></span>
        </button>
      {/if}
    </div>
  </div>

  <div class="ncsub" class:busy={!!pending[mode.id]} title={modeLocked ? modeReason : null}>
    <div class="label">
      <span class="subtitle">{$t("Mode")}</span>
      {#if modeLocked}
        <span class="lock" title={modeReason} aria-label={modeReason}>🔒</span>
      {/if}
    </div>
    <div class="control" class:dim={modeLocked}>
      {#if modeValue === null}
        <span class="unknown">—</span>
      {:else}
        <div class="segmented" role="group" aria-label={$t(mode.label)}>
          {#each mode.options as opt (opt.value)}
            <button class="segment" class:active={modeValue === opt.value} onclick={() => chooseMode(opt.value)}>
              {$t(opt.label)}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <div class="ncsub" class:busy={!!pending[button.id]} title={buttonLocked ? buttonReason : null}>
    <div class="label">
      <span class="subtitle">{$t(button.label)}</span>
      {#if buttonLocked}
        <span class="lock" title={buttonReason} aria-label={buttonReason}>🔒</span>
      {/if}
    </div>
    <div class="control" class:dim={buttonLocked}>
      {#if buttonValue === null}
        <span class="unknown">—</span>
      {:else}
        <button
          class="switch small"
          class:on={buttonChecked}
          role="switch"
          aria-checked={buttonChecked}
          aria-label={$t(button.label)}
          onclick={toggleButton}
        >
          <span class="knob"></span>
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .ncgroup {
    padding: 10px 4px 13px;
    border-bottom: 1px solid var(--border);
  }
  .ncgroup:last-child {
    border-bottom: none;
  }
  .ncgroup.compact {
    padding: 8px 10px;
    border-bottom: none;
    background: var(--bg-elev);
    border-radius: var(--radius-sm);
  }

  .ncmain {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 3px 0;
  }
  .ncmain.busy {
    cursor: progress;
  }
  .ncsub {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 6px 0 0 14px;
  }
  .ncsub.busy {
    cursor: progress;
  }
  .compact .ncsub {
    padding-left: 10px;
  }

  .label {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }
  .title {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .compact .title {
    font-size: 13px;
  }
  .subtitle {
    font-size: 12px;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .lock {
    font-size: 11px;
    opacity: 0.7;
    cursor: help;
  }
  .control.dim {
    opacity: 0.4;
    pointer-events: none;
  }
  .unknown {
    color: var(--text-faint);
  }

  /* Switch (mirrors SettingControl's, plus a smaller variant for the nested
     TX-button toggle) */
  .switch {
    width: 42px;
    height: 25px;
    border-radius: 999px;
    border: none;
    background: var(--border-strong);
    padding: 0;
    position: relative;
    transition: background 0.16s;
    flex: 0 0 auto;
  }
  .compact .switch {
    width: 36px;
    height: 21px;
  }
  .switch.small {
    width: 32px;
    height: 19px;
  }
  .compact .switch.small {
    width: 28px;
    height: 17px;
  }
  .switch.on {
    background: var(--accent);
  }
  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 19px;
    height: 19px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
    transition: transform 0.16s;
  }
  .compact .knob {
    width: 15px;
    height: 15px;
  }
  .switch.on .knob {
    transform: translateX(17px);
  }
  .compact .switch.on .knob {
    transform: translateX(15px);
  }
  .switch.small .knob {
    width: 13px;
    height: 13px;
  }
  .switch.small.on .knob {
    transform: translateX(13px);
  }
  .compact .switch.small .knob {
    width: 11px;
    height: 11px;
  }
  .compact .switch.small.on .knob {
    transform: translateX(11px);
  }

  /* Segmented (mirrors SettingControl's) */
  .segmented {
    display: inline-flex;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px;
    flex: 0 0 auto;
  }
  .segment {
    border: none;
    background: transparent;
    color: var(--text-dim);
    padding: 5px 14px;
    border-radius: 999px;
    font-weight: 500;
    transition: background 0.14s, color 0.14s;
  }
  .compact .segment {
    padding: 3px 10px;
    font-size: 12px;
  }
  .segment.active {
    background: var(--accent);
    color: var(--accent-contrast);
  }
</style>

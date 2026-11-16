<script>
  import { t } from "./i18n.js";
  // Renders one setting from its descriptor: a switch for toggles, a segmented
  // picker for enums. Options are ordered off→on, so the second option is the
  // "on" state. `pending` blocks input while a write is in flight; `compact`
  // tightens the layout for the grouped view.
  let {
    setting,
    value = null,
    pending = false,
    compact = false,
    locked = false,
    lockReason = "",
    onchange,
  } = $props();

  const isToggle = $derived(setting.kind === "toggle");
  const onValue = $derived(setting.options[1]?.value);
  const checked = $derived(value === onValue);
  const disabled = $derived(pending || locked);

  function choose(v) {
    if (disabled || v === value) return;
    onchange?.(setting.id, v);
  }

  function toggle() {
    if (disabled) return;
    const next = checked ? setting.options[0].value : setting.options[1].value;
    onchange?.(setting.id, next);
  }
</script>

<div class="row" class:compact class:busy={pending} class:locked title={locked ? lockReason : null}>
  <div class="label">
    <span class="title">{$t(setting.label)}</span>
    {#if locked}
      <span class="lock" title={lockReason} aria-label={lockReason}>🔒</span>
    {:else if setting.note}
      <span class="note" title={$t(setting.note)}>{compact ? "⚠" : $t(setting.note)}</span>
    {/if}
  </div>

  <div class="control" class:dim={locked}>
    {#if value === null}
      <span class="unknown">—</span>
    {:else if isToggle}
      <button
        class="switch"
        class:on={checked}
        role="switch"
        aria-checked={checked}
        aria-label={$t(setting.label)}
        onclick={toggle}
      >
        <span class="knob"></span>
      </button>
    {:else}
      <div class="segmented" role="group" aria-label={$t(setting.label)}>
        {#each setting.options as opt (opt.value)}
          <button class="segment" class:active={value === opt.value} onclick={() => choose(opt.value)}>
            {$t(opt.label)}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 13px 4px;
    border-bottom: 1px solid var(--border);
  }
  .row:last-child {
    border-bottom: none;
  }
  .row.compact {
    padding: 8px 10px;
    border-bottom: none;
    background: var(--bg-elev);
    border-radius: var(--radius-sm);
    gap: 8px;
  }
  .row.busy {
    cursor: progress;
  }
  .row.locked .title {
    color: var(--text-dim);
  }
  .control.dim {
    opacity: 0.4;
    pointer-events: none;
  }
  .lock {
    font-size: 11px;
    opacity: 0.7;
    cursor: help;
  }
  .label {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }
  .compact .label {
    flex-direction: row;
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
  .note {
    font-size: 12px;
    color: var(--warn);
    cursor: help;
  }
  .unknown {
    color: var(--text-faint);
  }

  /* Switch */
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

  /* Segmented */
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

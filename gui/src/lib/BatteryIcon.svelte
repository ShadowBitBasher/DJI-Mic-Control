<script>
  import { t } from "./i18n.js";
  // `value` is the raw 1-7 gauge from `TxInfo.battery` (see PROTOCOL.md) --
  // 1 is full, 7 is the terminal reading immediately before auto-shutoff.
  // 0/null (not yet known, or the practically-unreachable gauge floor)
  // renders nothing rather than guessing.
  let { value = null, charging = false, size = 20 } = $props();

  // 1-4 green, 5 orange (caution), 6-7 red — the transmitter's own LED
  // turns red starting at 6, not 7, so both share the danger color; 7 is
  // additionally the flashing/terminal reading.
  const tier = $derived(
    value == null || value === 0
      ? null
      : value === 7
        ? "critical"
        : value === 6
          ? "warning"
          : value === 5
            ? "caution"
            : "normal",
  );
  // Not a linear 1-7 gauge: the green tier (1-4) ramps down to the same
  // fill amount caution (5) uses, so the last green step and the first
  // orange step read as the same fill level, just a different color —
  // then warning/critical (6-7) are always empty.
  const CAUTION_FILL = 0.25;
  const fillFrac = $derived(
    tier === "normal"
      ? 1 - ((value - 1) * (1 - CAUTION_FILL)) / 3
      : tier === "caution"
        ? CAUTION_FILL
        : 0,
  );
  const color = $derived(
    tier === "critical" || tier === "warning"
      ? "var(--danger)"
      : tier === "caution"
        ? "var(--warn)"
        : "var(--good)",
  );
  // At critical, a charging bolt takes priority over the flashing
  // exclamation mark rather than the two competing for attention.
  const showExclaim = $derived(tier === "critical" && !charging);

  // Display labels, distinct from `tier`: 1 gets its own name rather than
  // sharing "Good" with 2-4.
  const label = $derived(
    value === 1
      ? "Fully Charged"
      : tier === "normal"
        ? "Good"
        : tier === "caution"
          ? "Low"
          : tier === "warning"
            ? "Very Low"
            : tier === "critical"
              ? "Critical"
              : null,
  );
</script>

{#if tier}
  <span
    class="batt"
    class:flash={showExclaim}
    style="--batt-color:{color}"
    title={$t("Battery: {label}", { label: $t(label) }) + (charging ? $t(" (charging)") : "")}
    aria-label={$t("Battery: {label}", { label: $t(label) }) + (charging ? $t(" (charging)") : "")}
  >
    <svg width={size} height={size * 0.55} viewBox="0 0 22 12" fill="none" aria-hidden="true">
      <rect x="1" y="1" width="17" height="10" rx="2.5" stroke="var(--batt-color)" stroke-width="1.5" />
      <rect x="19" y="4" width="2" height="4" rx="1" fill="var(--batt-color)" />
      {#if fillFrac > 0}
        <rect x="3" y="3" width={13 * fillFrac} height="6" rx="1.2" fill="var(--batt-color)" />
      {/if}
      {#if showExclaim}
        <text x="9.5" y="8.8" text-anchor="middle" font-size="7.5" font-weight="700" fill="var(--batt-color)" class="excl">!</text>
      {/if}
      {#if charging}
        <!-- Drawn in the same 22x12 viewBox as the body/nub above, rather
             than as a separately-positioned element, so it can't drift
             relative to the body — CSS percentage centering was landing on
             the wrong point because the nub makes the icon's own bounding
             box asymmetric. The path's natural bounding box is x:1-10,
             y:0.5-11.5 (9x11); this transform scales and translates that
             box to sit centered on the body rect (x:1-18, y:1-11, center
             9.5,6), independent of the nub entirely. -->
        <g transform="translate(6.475 2.7) scale(0.55)">
          <path
            d="M6.6 0.5 1 7h3.6l-1.2 4.5L10 4.5H6.4z"
            fill="var(--batt-color)"
            stroke="var(--bg-panel)"
            stroke-width="3.5"
            stroke-linejoin="round"
            paint-order="stroke fill"
          />
        </g>
      {/if}
    </svg>
  </span>
{/if}

<style>
  .batt {
    position: relative;
    display: inline-flex;
    align-items: center;
    line-height: 0;
  }
  .flash .excl {
    /* No steps() function at all -- the keyframes' own percentages already
       give a hard cut (0-50% then a near-instant drop right at 50%), so a
       custom timing function is unnecessary and, as tried twice above, easy
       to get subtly wrong. 1s duration, split exactly in half = 500ms on,
       500ms off. */
    animation: batt-flash 1s infinite;
  }
  @keyframes batt-flash {
    0%,
    50% {
      opacity: 1;
    }
    50.01%,
    100% {
      opacity: 0;
    }
  }
</style>

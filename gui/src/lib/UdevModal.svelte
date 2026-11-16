<script>
  import { t } from "./i18n.js";
  let { help, onclose } = $props();
  let copied = $state(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(help.rule);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      copied = false;
    }
  }
</script>

<div class="overlay" onclick={onclose} role="presentation">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
    <header>
      <h2>{$t("Enable USB access")}</h2>
      <button class="x" onclick={onclose} aria-label={$t("Close")}>×</button>
    </header>

    <p class="lead">
      {$t("A microphone is connected but this app can't access it. On Linux, USB devices need a udev rule granting your user permission.")}
    </p>

    <ol class="steps">
      <li>{$t("Save the rule below to {file} (needs sudo).", { file: help.file })}</li>
      <li>{$t("Reload the rules: sudo udevadm control --reload-rules")}</li>
      <li>{$t("Re-trigger: sudo udevadm trigger")}</li>
      <li>{$t("Unplug and replug the receiver.")}</li>
    </ol>

    <div class="rule">
      <div class="rule-head">
        <code>{help.file}</code>
        <button class="copy" onclick={copy}>{copied ? $t("Copied") : $t("Copy rule")}</button>
      </div>
      <pre>{help.rule}</pre>
    </div>

    <p class="foot">
      {$t("The Linux .deb and .rpm packages install this rule for you.")}
    </p>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(8, 10, 14, 0.5);
    display: grid;
    place-items: center;
    padding: 24px;
    z-index: 50;
  }
  .modal {
    width: min(560px, 100%);
    max-height: 90vh;
    overflow-y: auto;
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    padding: 20px 22px 22px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    margin: 0;
    font-size: 17px;
  }
  .x {
    border: none;
    background: transparent;
    color: var(--text-dim);
    font-size: 22px;
    line-height: 1;
  }
  .lead {
    color: var(--text-dim);
    line-height: 1.5;
  }
  .steps {
    margin: 4px 0 16px;
    padding-left: 20px;
    line-height: 1.7;
  }
  .rule {
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--bg);
  }
  .rule-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }
  .rule-head code {
    font-size: 12px;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .copy {
    flex: 0 0 auto;
    border: 1px solid var(--border-strong);
    background: var(--bg-panel);
    color: var(--text);
    border-radius: 6px;
    padding: 4px 10px;
    font-size: 12px;
  }
  pre {
    margin: 0;
    padding: 12px;
    overflow-x: auto;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre;
  }
  .foot {
    margin-bottom: 0;
    color: var(--text-faint);
    font-size: 13px;
  }
  code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
</style>

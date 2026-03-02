<script lang="ts">
  import {
    setStartupEnabled,
    startAll,
    startupEnabled,
    stopAll,
  } from "../stores/portManager";
</script>

<div class="action-bar">
  <div class="tunnel-buttons">
    <button class="primary" on:click={startAll}>▶ Start All</button>
    <button on:click={stopAll}>■ Stop All</button>
  </div>
  <label class="startup-toggle">
    <input
      type="checkbox"
      checked={$startupEnabled}
      on:change={(e) => setStartupEnabled(e.currentTarget.checked)}
    />
    Run on startup
  </label>
</div>

<style>
  .action-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .tunnel-buttons {
    display: flex;
    gap: 8px;
  }

  button {
    padding: 9px 20px;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    background: #f0f0f0;
    cursor: pointer;
    font-size: 13px;
    transition: background 0.15s, border-color 0.15s;
  }

  button.primary {
    background: #0078d4;
    color: white;
    border-color: #0078d4;
  }

  button.primary:hover {
    background: #106ebe;
    border-color: #106ebe;
  }

  button:not(.primary):hover {
    background: #e5e5e5;
    border-color: #0078d4;
  }

  .startup-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: #555;
    cursor: pointer;
  }
</style>

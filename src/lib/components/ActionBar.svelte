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
    <button class="btn-primary" on:click={startAll}>Start All</button>
    <button class="btn-secondary" on:click={stopAll}>Stop All</button>
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
  }

  .tunnel-buttons {
    display: flex;
    gap: 6px;
  }

  .btn-primary {
    padding: 7px 16px;
    background: #0078d4;
    color: white;
    border: 1px solid transparent;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
    transition: background 0.15s;
  }

  .btn-primary:hover {
    background: #106ebe;
  }

  .btn-secondary {
    padding: 7px 16px;
    background: white;
    color: #374151;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
    transition: background 0.15s, border-color 0.15s;
  }

  .btn-secondary:hover {
    background: #f9fafb;
    border-color: #d1d5db;
  }

  .startup-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: #6b7280;
    cursor: pointer;
    user-select: none;
  }
</style>

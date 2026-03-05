<script lang="ts">
  import {
    aggregateStatus,
    autoReconnect,
    setAutoReconnect,
  } from "../stores/portManager";

  $: statusText =
    {
      "all-forwarding": "All ports forwarding",
      partial: "Some ports forwarding",
      inactive: "No ports forwarding",
      "no-ports": "No ports configured",
    }[$aggregateStatus] ?? "Ready";

  $: statusColor =
    {
      "all-forwarding": "#16a34a",
      partial: "#d97706",
      inactive: "#9ca3af",
      "no-ports": "#9ca3af",
    }[$aggregateStatus] ?? "#9ca3af";
</script>

<div class="header">
  <div class="title-row">
    <h1>Port Manager</h1>
    <label class="toggle-label">
      <input
        type="checkbox"
        checked={$autoReconnect}
        on:change={(e) => setAutoReconnect(e.currentTarget.checked)}
      />
      Auto-reconnect
    </label>
  </div>
  <div class="status-row">
    <span class="status-dot" style="background: {statusColor}"></span>
    <span class="status-text" style="color: {statusColor}">{statusText}</span>
  </div>
</div>

<style>
  .header {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .header h1 {
    font-size: 18px;
    font-weight: 600;
    color: #111827;
    letter-spacing: -0.3px;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: #6b7280;
    cursor: pointer;
    user-select: none;
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-text {
    font-size: 12px;
    font-weight: 500;
  }
</style>

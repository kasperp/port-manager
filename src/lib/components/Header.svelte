<script lang="ts">
  import {
    aggregateStatus,
    autoReconnect,
    setAutoReconnect,
  } from "../stores/portManager";

  $: statusText =
    {
      "all-active": "All ports active",
      partial: "Some ports active",
      inactive: "No ports active",
      "no-ports": "No ports configured",
    }[$aggregateStatus] ?? "Ready";

  $: statusColor =
    {
      "all-active": "#16a34a",
      partial: "#ea580c",
      inactive: "#6b7280",
      "no-ports": "#6b7280",
    }[$aggregateStatus] ?? "#6b7280";
</script>

<div class="header">
  <h1>Port Manager</h1>
  <div class="status-row">
    <span class="connection-status" style="color: {statusColor}">
      {statusText}
    </span>
    <span class="separator">|</span>
    <label class="checkbox-label">
      <input
        type="checkbox"
        checked={$autoReconnect}
        on:change={(e) => setAutoReconnect(e.currentTarget.checked)}
      />
      Auto-reconnect
    </label>
  </div>
</div>

<style>
  .header h1 {
    font-size: 22px;
    font-weight: 600;
    margin: 0 0 4px;
    color: #1a1a1a;
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .connection-status {
    font-weight: 500;
  }

  .separator {
    color: #ccc;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
    color: #666;
  }
</style>

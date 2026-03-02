<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { PortStatusInfo } from "../types";

  export let portInfo: PortStatusInfo;
  export let pending: boolean = false;
  export let selected: boolean = false;

  const dispatch = createEventDispatcher();

  $: displayStatus = pending ? "Pending" : portInfo.status;
  $: dotColor = pending
    ? "#f59e0b"
    : portInfo.status === "Active"
      ? "#16a34a"
      : "#dc2626";
  $: textColor = pending
    ? "#f59e0b"
    : portInfo.status === "Active"
      ? "#16a34a"
      : "#6b7280";
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="port-row"
  class:selected
  on:click={() => dispatch("select")}
>
  <span class="port-number">{portInfo.port}</span>
  <span class="status-cell">
    <span class="dot" style="background: {dotColor}"></span>
    <span style="color: {textColor}">{displayStatus}</span>
  </span>
  <span class="pid">{portInfo.pid ?? "—"}</span>
</div>

<style>
  .port-row {
    display: grid;
    grid-template-columns: 80px 130px 1fr;
    padding: 9px 12px;
    border-bottom: 1px solid #ebebeb;
    cursor: pointer;
    font-size: 13px;
    user-select: none;
    transition: background 0.1s;
  }

  .port-row:hover {
    background: #f5f5f5;
  }

  .port-row.selected {
    background: #e5f1fb;
  }

  .status-cell {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .pid {
    color: #888;
    font-size: 12px;
  }
</style>

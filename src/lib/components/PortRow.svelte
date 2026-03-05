<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { PortStatusInfo } from "../types";
  import { killPortProcess, startPort, stopPort } from "../stores/portManager";

  export let portInfo: PortStatusInfo;
  export let pending: boolean = false;
  export let selected: boolean = false;

  const dispatch = createEventDispatcher();

  let killing = false;

  // Context menu state
  let showMenu = false;
  let menuX = 0;
  let menuY = 0;

  $: displayStatus = pending
    ? "Pending"
    : {
        Forwarding: "Forwarding",
        RemoteDown: "Not Listening",
        Reconnecting: "Reconnecting",
        TunnelDown: "Tunnel Down",
        PortInUse: "Port In Use",
        Stopped: "Stopped",
      }[portInfo.status] ?? portInfo.status;

  $: dotColor = pending
    ? "#f59e0b"
    : {
        Forwarding: "#16a34a",
        RemoteDown: "#ea580c",
        Reconnecting: "#f59e0b",
        TunnelDown: "#dc2626",
        PortInUse: "#3b82f6",
        Stopped: "#d1d5db",
      }[portInfo.status] ?? "#d1d5db";

  $: textColor = pending
    ? "#f59e0b"
    : {
        Forwarding: "#16a34a",
        RemoteDown: "#ea580c",
        Reconnecting: "#f59e0b",
        TunnelDown: "#dc2626",
        PortInUse: "#3b82f6",
        Stopped: "#9ca3af",
      }[portInfo.status] ?? "#9ca3af";

  $: statusTooltip = pending
    ? "Waiting for SSH connection to establish"
    : {
        Forwarding: "Tunnel is up and remote service is accepting connections",
        RemoteDown: "Tunnel is up but nothing is listening on this port",
        Reconnecting: "Tunnel died — automatically reconnecting",
        TunnelDown: "Tunnel died — auto-reconnect is off or in cooldown",
        PortInUse: "Another process is already listening on this port",
        Stopped: "Not forwarding — right-click to start",
      }[portInfo.status] ?? "";

  $: isForwarding = !pending && portInfo.status === "Forwarding";
  $: isReconnecting = !pending && portInfo.status === "Reconnecting";
  $: isPortInUse = portInfo.status === "PortInUse";
  $: ownerLabel = portInfo.process_name
    ? `${portInfo.process_name} (${portInfo.owner_pid})`
    : portInfo.owner_pid
      ? `PID ${portInfo.owner_pid}`
      : null;

  // Determine which context menu actions are available
  $: canStart =
    !pending &&
    (portInfo.status === "Stopped" || portInfo.status === "TunnelDown");
  $: canStop =
    !pending &&
    (portInfo.status === "Forwarding" || portInfo.status === "RemoteDown" || portInfo.status === "Reconnecting");
  $: canKill = !pending && portInfo.status === "PortInUse";

  async function handleKill() {
    killing = true;
    await killPortProcess(portInfo.port);
    killing = false;
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    // Only show if there's a meaningful action
    if (!canStart && !canStop && !canKill) return;
    menuX = e.clientX;
    menuY = e.clientY;
    showMenu = true;
  }

  function closeMenu() {
    showMenu = false;
  }

  async function handleMenuStart() {
    showMenu = false;
    await startPort(portInfo.port);
  }

  async function handleMenuStop() {
    showMenu = false;
    await stopPort(portInfo.port);
  }

  async function handleMenuKill() {
    showMenu = false;
    killing = true;
    await killPortProcess(portInfo.port);
    killing = false;
  }
</script>

<svelte:window on:click={closeMenu} />

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="port-row"
  class:selected
  on:click={() => dispatch("select")}
  on:contextmenu={handleContextMenu}
>
  <span class="port-number">{portInfo.port}</span>
  <span class="status-cell" title={statusTooltip}>
    <span
      class="dot"
      class:dot-forwarding={isForwarding}
      class:dot-reconnecting={isReconnecting}
      style="background: {dotColor}"
    ></span>
    <span style="color: {textColor}">{displayStatus}</span>
  </span>
  {#if isPortInUse && ownerLabel}
    <span class="owner-cell">
      <span class="owner-name" title={ownerLabel}>{ownerLabel}</span>
      <button
        class="kill-btn"
        on:click|stopPropagation={handleKill}
        disabled={killing}
        title="Kill this process"
      >
        {killing ? "..." : "Kill"}
      </button>
    </span>
  {:else}
    <span class="pid">{portInfo.pid ?? "—"}</span>
  {/if}
</div>

{#if showMenu}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="ctx-backdrop" on:click|stopPropagation={closeMenu}>
    <div
      class="ctx-menu"
      style="left: {menuX}px; top: {menuY}px"
      on:click|stopPropagation
    >
      {#if canStart}
        <button class="ctx-item" on:click={handleMenuStart}>
          Start
        </button>
      {/if}
      {#if canStop}
        <button class="ctx-item" on:click={handleMenuStop}>
          Stop
        </button>
      {/if}
      {#if canKill}
        <button class="ctx-item ctx-item--danger" on:click={handleMenuKill}>
          Kill Process
        </button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .port-row {
    display: grid;
    grid-template-columns: 80px 130px 1fr;
    padding: 9px 12px;
    border-bottom: 1px solid #f3f4f6;
    cursor: pointer;
    font-size: 13px;
    user-select: none;
    transition: background 0.1s;
  }

  .port-row:last-child {
    border-bottom: none;
  }

  .port-row:hover {
    background: #f9fafb;
  }

  .port-row.selected {
    background: #eff6ff;
  }

  .port-number {
    font-weight: 500;
    color: #374151;
  }

  .status-cell {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  @keyframes glow-green {
    0%, 100% { box-shadow: 0 0 0 0 rgba(22, 163, 74, 0.5); }
    50% { box-shadow: 0 0 0 4px rgba(22, 163, 74, 0); }
  }

  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.25; }
  }

  .dot-forwarding {
    animation: glow-green 2.5s ease-out infinite;
  }

  .dot-reconnecting {
    animation: blink 1s ease-in-out infinite;
  }

  .pid {
    color: #9ca3af;
    font-size: 12px;
  }

  .owner-cell {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    justify-content: flex-end;
  }

  .owner-name {
    color: #3b82f6;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .kill-btn {
    flex-shrink: 0;
    padding: 1px 8px;
    font-size: 11px;
    border: 1px solid #fca5a5;
    border-radius: 4px;
    background: white;
    color: #dc2626;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .kill-btn:hover:not(:disabled) {
    background: #dc2626;
    border-color: #dc2626;
    color: white;
  }

  .kill-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .ctx-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    z-index: 1000;
  }

  .ctx-menu {
    position: fixed;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12), 0 2px 4px rgba(0, 0, 0, 0.06);
    padding: 4px 0;
    min-width: 120px;
    z-index: 1001;
  }

  .ctx-item {
    display: block;
    width: 100%;
    padding: 6px 14px;
    font-size: 13px;
    text-align: left;
    border: none;
    background: none;
    cursor: pointer;
    color: #111827;
  }

  .ctx-item:hover {
    background: #f0f7ff;
  }

  .ctx-item--danger {
    color: #dc2626;
  }

  .ctx-item--danger:hover {
    background: #fef2f2;
  }
</style>

<script lang="ts">
  import {
    addPort,
    isPending,
    portStatuses,
    removePort,
    statusMessage,
  } from "../stores/portManager";
  import PortRow from "./PortRow.svelte";

  let newPort = "";
  let selectedPort: number | null = null;

  async function handleAdd() {
    const port = parseInt(String(newPort).trim(), 10);
    if (isNaN(port) || port < 1 || port > 65535) {
      statusMessage.set("Enter a valid port number (1–65535)");
      return;
    }
    const err = await addPort(port);
    if (err) {
      statusMessage.set(err);
    } else {
      newPort = "";
    }
  }

  async function handleRemove() {
    if (selectedPort !== null) {
      await removePort(selectedPort);
      selectedPort = null;
    } else {
      statusMessage.set("Select a port to remove");
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") handleAdd();
  }
</script>

<div class="port-section">
  <h2>Managed Ports</h2>
  <div class="list-area">
    <div class="port-table">
      <div class="table-header">
        <span>Port</span>
        <span>Status</span>
        <span>PID</span>
      </div>
      <div class="table-body">
        {#each $portStatuses as portInfo (portInfo.port)}
          <PortRow
            {portInfo}
            pending={$isPending}
            selected={selectedPort === portInfo.port}
            on:select={() => (selectedPort = portInfo.port)}
          />
        {:else}
          <p class="empty">No ports configured. Add one below.</p>
        {/each}
      </div>
    </div>
  </div>
  <div class="add-row">
    <input
      bind:value={newPort}
      placeholder="Port number"
      type="text"
      inputmode="numeric"
      pattern="[0-9]*"
      on:keydown={handleKeydown}
    />
    <button on:click={handleAdd} class="add-btn">+ Add</button>
    <button
      title="Remove selected port"
      on:click={handleRemove}
      disabled={selectedPort === null}
      class="icon-btn"
    >
      &minus;
    </button>
  </div>
</div>

<style>
  h2 {
    font-size: 11px;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0 0 6px;
    flex-shrink: 0;
  }

  .port-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .list-area {
    flex: 1;
    min-height: 0;
  }

  .port-table {
    height: 100%;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06), 0 1px 2px rgba(0, 0, 0, 0.04);
  }

  .table-header {
    display: grid;
    grid-template-columns: 80px 130px 1fr;
    padding: 7px 12px;
    background: #f9fafb;
    border-bottom: 1px solid #e5e7eb;
    font-size: 11px;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    flex-shrink: 0;
  }

  .table-body {
    flex: 1;
    min-height: 60px;
    overflow-y: auto;
    background: white;
  }

  .empty {
    color: #9ca3af;
    font-size: 13px;
    padding: 24px;
    text-align: center;
  }

  .icon-btn {
    width: 34px;
    height: 34px;
    padding: 0;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    background: white;
    color: #374151;
    font-size: 20px;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s, border-color 0.15s;
    flex-shrink: 0;
  }

  .icon-btn:hover:not(:disabled) {
    background: #fef2f2;
    border-color: #fca5a5;
    color: #dc2626;
  }

  .icon-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .add-row {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }

  .add-row input {
    flex: 1;
    padding: 7px 10px;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    transition: border-color 0.15s;
  }

  .add-row input:focus {
    outline: none;
    border-color: #0078d4;
    box-shadow: 0 0 0 3px rgba(0, 120, 212, 0.1);
  }

  .add-btn {
    padding: 7px 14px;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
    color: #374151;
    transition: background 0.15s, border-color 0.15s;
    white-space: nowrap;
  }

  .add-btn:hover {
    background: #f0f7ff;
    border-color: #0078d4;
    color: #0078d4;
  }
</style>

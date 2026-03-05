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
    font-size: 13px;
    font-weight: 600;
    color: #444;
    text-transform: uppercase;
    letter-spacing: 0.04em;
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
    flex: 1;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .table-header {
    display: grid;
    grid-template-columns: 80px 130px 1fr;
    padding: 7px 12px;
    background: #f5f5f5;
    border-bottom: 1px solid #e0e0e0;
    font-size: 11px;
    font-weight: 600;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }

  .table-body {
    flex: 1;
    min-height: 60px;
    overflow-y: auto;
  }

  .empty {
    color: #aaa;
    font-size: 13px;
    padding: 20px;
    text-align: center;
  }

  .icon-btn {
    width: 34px;
    height: 34px;
    padding: 0;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    background: #f0f0f0;
    font-size: 20px;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s;
  }

  .icon-btn:hover:not(:disabled) {
    background: #e5e5e5;
    border-color: #0078d4;
  }

  .icon-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .add-row {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  .add-row input {
    flex: 1;
    padding: 7px 10px;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    font-size: 13px;
  }

  .add-row input:focus {
    outline: none;
    border-color: #0078d4;
  }

  .add-btn {
    padding: 7px 16px;
    background: #f0f0f0;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
    transition: background 0.15s;
  }

  .add-btn:hover {
    background: #e5e5e5;
    border-color: #0078d4;
  }
</style>

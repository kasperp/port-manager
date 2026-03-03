<script lang="ts">
  import { activeProfile, saveSettings, statusMessage } from "../stores/portManager";

  let host = "";
  let user = "";
  let sshPort = 22;

  // Track the last store values we applied so we can detect user edits.
  // Only update local vars when the store changes AND the user hasn't diverged.
  let prevHost = "";
  let prevUser = "";
  let prevSshPort = 22;

  $: {
    const sh = $activeProfile.host;
    const su = $activeProfile.user;
    const sp = $activeProfile.ssh_port;
    if (host === prevHost) host = sh;
    if (user === prevUser) user = su;
    if (sshPort === prevSshPort) sshPort = sp;
    prevHost = sh;
    prevUser = su;
    prevSshPort = sp;
  }

  async function handleSave() {
    const port = Number(sshPort);
    if (!port || port < 1 || port > 65535) {
      statusMessage.set("Invalid SSH port number");
      return;
    }
    await saveSettings(host.trim(), user.trim(), port);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") handleSave();
  }
</script>

<div class="settings-card">
  <div class="field-group">
    <div class="field">
      <label for="host-input">Host</label>
      <input
        id="host-input"
        bind:value={host}
        placeholder="example.com"
        autofocus
        on:keydown={handleKeydown}
      />
    </div>
    <div class="field field-narrow">
      <label for="ssh-port-input">SSH Port</label>
      <input
        id="ssh-port-input"
        bind:value={sshPort}
        type="number"
        min="1"
        max="65535"
        on:keydown={handleKeydown}
      />
    </div>
    <div class="field">
      <label for="user-input">User</label>
      <input
        id="user-input"
        bind:value={user}
        placeholder="username"
        on:keydown={handleKeydown}
      />
    </div>
    <button on:click={handleSave}>Save</button>
  </div>
</div>

<style>
  .settings-card {
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    padding: 14px;
    background: #fafafa;
  }

  .field-group {
    display: flex;
    gap: 8px;
    align-items: flex-end;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }

  .field-narrow {
    flex: 0 0 90px;
  }

  label {
    font-size: 11px;
    color: #666;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  input {
    padding: 7px 10px;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    font-size: 13px;
    background: white;
    transition: border-color 0.15s;
  }

  input:focus {
    outline: none;
    border-color: #0078d4;
  }

  button {
    padding: 8px 16px;
    background: #f0f0f0;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
    white-space: nowrap;
    align-self: flex-end;
    transition: background 0.15s, border-color 0.15s;
  }

  button:hover {
    background: #e5e5e5;
    border-color: #0078d4;
  }
</style>

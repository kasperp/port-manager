<script lang="ts">
  import { activeProfile, saveProfileSettings, statusMessage } from "../stores/portManager";

  let host = "";
  let user = "";
  let sshPort = 22;
  let rateLimitMax = 6;
  let rateLimitWindowSecs = 30;

  // Track the last store values we applied so we can detect user edits.
  // Only update local vars when the store changes AND the user hasn't diverged.
  let prevHost = "";
  let prevUser = "";
  let prevSshPort = 22;
  let prevRateLimitMax = 6;
  let prevRateLimitWindowSecs = 30;

  $: {
    const sh = $activeProfile.host;
    const su = $activeProfile.user;
    const sp = $activeProfile.ssh_port;
    const rlm = $activeProfile.rate_limit_max;
    const rlw = $activeProfile.rate_limit_window_secs;
    if (host === prevHost) host = sh;
    if (user === prevUser) user = su;
    if (sshPort === prevSshPort) sshPort = sp;
    if (rateLimitMax === prevRateLimitMax) rateLimitMax = rlm;
    if (rateLimitWindowSecs === prevRateLimitWindowSecs) rateLimitWindowSecs = rlw;
    prevHost = sh;
    prevUser = su;
    prevSshPort = sp;
    prevRateLimitMax = rlm;
    prevRateLimitWindowSecs = rlw;
  }

  async function handleSave() {
    const port = Number(sshPort);
    if (!port || port < 1 || port > 65535) {
      statusMessage.set("Invalid SSH port number");
      return;
    }
    const max = Number(rateLimitMax);
    const window = Number(rateLimitWindowSecs);
    if (!max || max < 1) {
      statusMessage.set("Max connections must be at least 1");
      return;
    }
    if (!window || window < 1) {
      statusMessage.set("Window must be at least 1 second");
      return;
    }
    await saveProfileSettings(host.trim(), user.trim(), port, max, window);
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
  </div>

  <div class="bottom-row">
    <div class="rate-limit-row">
      <span class="rate-limit-label">Rate limit</span>
      <div class="rate-limit-fields">
        <input
          id="rate-limit-max"
          bind:value={rateLimitMax}
          type="number"
          min="1"
          class="rate-limit-input"
          on:keydown={handleKeydown}
        />
        <span class="rate-limit-text">conn /</span>
        <input
          id="rate-limit-window"
          bind:value={rateLimitWindowSecs}
          type="number"
          min="1"
          class="rate-limit-input"
          on:keydown={handleKeydown}
        />
        <span class="rate-limit-text">sec</span>
      </div>
    </div>
    <button on:click={handleSave}>Save</button>
  </div>
</div>

<style>
  .settings-card {
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 14px;
    background: white;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06), 0 1px 2px rgba(0, 0, 0, 0.04);
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
    color: #6b7280;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  input {
    padding: 7px 10px;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    background: white;
    transition: border-color 0.15s;
  }

  input:focus {
    outline: none;
    border-color: #0078d4;
    box-shadow: 0 0 0 3px rgba(0, 120, 212, 0.1);
  }

  button {
    padding: 7px 16px;
    background: #0078d4;
    color: white;
    border: 1px solid transparent;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
    white-space: nowrap;
    transition: background 0.15s;
  }

  button:hover {
    background: #106ebe;
  }

  .bottom-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid #f3f4f6;
  }

  .rate-limit-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .rate-limit-label {
    font-size: 11px;
    color: #6b7280;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }

  .rate-limit-fields {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .rate-limit-input {
    width: 60px;
    padding: 5px 8px;
    text-align: center;
  }

  .rate-limit-text {
    font-size: 12px;
    color: #6b7280;
    white-space: nowrap;
  }
</style>

<script lang="ts">
  import {
    config,
    sshHosts,
    switchProfile,
    createProfile,
    deleteProfile,
    importSshProfile,
    statusMessage,
  } from "../stores/portManager";

  let showNewForm = false;
  let showImportMenu = false;
  let newName = "";
  let confirmDelete = false;

  $: profiles = $config.profiles;
  $: activeProfileName = $config.active_profile;
  $: canDelete = profiles.length > 1;
  $: availableSshHosts = $sshHosts.filter(
    (h) => !profiles.some((p) => p.name === h.name)
  );

  async function handleSwitch(e: Event) {
    const target = e.target as HTMLSelectElement;
    const name = target.value;
    if (name !== activeProfileName) {
      await switchProfile(name);
    }
  }

  async function handleCreate() {
    const name = newName.trim();
    if (!name) {
      statusMessage.set("Profile name cannot be empty");
      return;
    }
    const err = await createProfile(name, "", "", 22);
    if (err) {
      statusMessage.set(err);
    } else {
      newName = "";
      showNewForm = false;
    }
  }

  async function handleDelete() {
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    const err = await deleteProfile(activeProfileName);
    if (err) {
      statusMessage.set(err);
    }
    confirmDelete = false;
  }

  function cancelDelete() {
    confirmDelete = false;
  }

  async function handleImport(sshHostName: string) {
    const err = await importSshProfile(sshHostName);
    if (err) {
      statusMessage.set(err);
    }
    showImportMenu = false;
  }

  function handleNewKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") handleCreate();
    if (e.key === "Escape") {
      showNewForm = false;
      newName = "";
    }
  }
</script>

<div class="profile-bar">
  <div class="profile-select-row">
    <label for="profile-select">Profile</label>
    <select
      id="profile-select"
      value={activeProfileName}
      on:change={handleSwitch}
    >
      {#each profiles as profile (profile.name)}
        <option value={profile.name}>{profile.name}</option>
      {/each}
    </select>

    <button
      class="icon-btn"
      title="New profile"
      on:click={() => {
        showNewForm = !showNewForm;
        showImportMenu = false;
        confirmDelete = false;
      }}
    >
      +
    </button>

    {#if availableSshHosts.length > 0}
      <button
        class="icon-btn import-btn"
        title="Import from SSH config"
        on:click={() => {
          showImportMenu = !showImportMenu;
          showNewForm = false;
          confirmDelete = false;
        }}
      >
        SSH
      </button>
    {/if}

    {#if canDelete}
      {#if confirmDelete}
        <button class="icon-btn delete-confirm" on:click={handleDelete}>
          Delete?
        </button>
        <button class="icon-btn" on:click={cancelDelete}>Cancel</button>
      {:else}
        <button
          class="icon-btn delete-btn"
          title="Delete current profile"
          on:click={handleDelete}
        >
          &times;
        </button>
      {/if}
    {/if}
  </div>

  {#if showNewForm}
    <div class="new-profile-form">
      <input
        bind:value={newName}
        placeholder="Profile name"
        on:keydown={handleNewKeydown}
        autofocus
      />
      <button on:click={handleCreate}>Create</button>
      <button
        class="cancel-btn"
        on:click={() => {
          showNewForm = false;
          newName = "";
        }}>Cancel</button
      >
    </div>
  {/if}

  {#if showImportMenu}
    <div class="import-list">
      <p class="import-header">Import from ~/.ssh/config</p>
      {#each availableSshHosts as host (host.name)}
        <button
          class="import-item"
          on:click={() => handleImport(host.name)}
        >
          <span class="host-name">{host.name}</span>
          <span class="host-detail">
            {host.user ? host.user + "@" : ""}{host.hostname}{host.port !== 22
              ? ":" + host.port
              : ""}
          </span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .profile-bar {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .profile-select-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  label {
    font-size: 11px;
    color: #6b7280;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }

  select {
    flex: 1;
    padding: 6px 10px;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    font-size: 13px;
    background: white;
    color: #111827;
    cursor: pointer;
    transition: border-color 0.15s;
  }

  select:focus {
    outline: none;
    border-color: #0078d4;
    box-shadow: 0 0 0 3px rgba(0, 120, 212, 0.1);
  }

  .icon-btn {
    padding: 5px 10px;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    background: white;
    color: #374151;
    cursor: pointer;
    font-size: 13px;
    white-space: nowrap;
    transition: background 0.15s, border-color 0.15s;
  }

  .icon-btn:hover {
    background: #f9fafb;
    border-color: #d1d5db;
  }

  .import-btn {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: #6b7280;
  }

  .delete-btn {
    color: #dc2626;
    font-size: 16px;
    line-height: 1;
    padding: 3px 8px;
    border-color: #fecaca;
  }

  .delete-btn:hover {
    background: #fef2f2;
    border-color: #fca5a5;
  }

  .delete-confirm {
    color: #dc2626;
    background: #fef2f2;
    border-color: #fca5a5;
    font-size: 12px;
  }

  .delete-confirm:hover {
    background: #fee2e2;
  }

  .new-profile-form {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .new-profile-form input {
    flex: 1;
    padding: 7px 10px;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    font-size: 13px;
    transition: border-color 0.15s;
  }

  .new-profile-form input:focus {
    outline: none;
    border-color: #0078d4;
    box-shadow: 0 0 0 3px rgba(0, 120, 212, 0.1);
  }

  .new-profile-form button {
    padding: 7px 14px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: #0078d4;
    color: white;
    cursor: pointer;
    font-weight: 500;
    transition: background 0.15s;
  }

  .new-profile-form button:hover {
    background: #106ebe;
  }

  .cancel-btn {
    background: white !important;
    color: #374151 !important;
    border-color: #e5e7eb !important;
  }

  .cancel-btn:hover {
    background: #f9fafb !important;
  }

  .import-list {
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    background: white;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
    overflow: hidden;
  }

  .import-header {
    font-size: 11px;
    color: #6b7280;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 8px 12px 4px;
  }

  .import-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 8px 12px;
    border: none;
    border-top: 1px solid #f3f4f6;
    background: transparent;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
  }

  .import-item:hover {
    background: #f0f7ff;
  }

  .host-name {
    font-weight: 500;
    color: #111827;
  }

  .host-detail {
    color: #9ca3af;
    font-size: 12px;
  }
</style>

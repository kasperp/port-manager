<script lang="ts">
  import { tick } from "svelte";
  import {
    config,
    sshHosts,
    switchProfile,
    createProfile,
    deleteProfile,
    importSshProfile,
    statusMessage,
  } from "../stores/portManager";

  import PlusIcon from "@lucide/svelte/icons/plus";
  import Trash2Icon from "@lucide/svelte/icons/trash-2";
  import { Button } from "$lib/components/ui/button";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
  import * as Field from "$lib/components/ui/field";
  import { Input } from "$lib/components/ui/input";
  import * as NativeSelect from "$lib/components/ui/native-select";

  let showNewForm = false;
  let showImportMenu = false;
  let newName = "";
  let newNameInput: HTMLInputElement | null = null;
  let confirmDelete = false;

  $: profiles = $config.profiles;
  $: activeProfileName = $config.active_profile;
  $: canDelete = profiles.length > 1;
  $: availableSshHosts = $sshHosts.filter(
    (h) => !profiles.some((p) => p.name === h.name)
  );

  async function toggleNewForm() {
    showNewForm = !showNewForm;
    showImportMenu = false;
    confirmDelete = false;
    if (showNewForm) {
      await tick();
      newNameInput?.focus();
    }
  }

  async function handleSwitch(name: string) {
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

<div class="flex flex-col gap-2">
  <div class="flex items-center gap-2">
    <Field.Label for="profile-select" class="shrink-0">Profile</Field.Label>
    <NativeSelect.Root
      id="profile-select"
      class="min-w-0 flex-1"
      value={activeProfileName}
      onchange={(event) => handleSwitch(event.currentTarget.value)}
    >
      {#each profiles as profile (profile.name)}
        <NativeSelect.Option value={profile.name}>{profile.name}</NativeSelect.Option>
      {/each}
    </NativeSelect.Root>
    <Button
      variant="outline"
      size="icon-sm"
      title="New profile"
      aria-label="New profile"
      onclick={toggleNewForm}
    >
      <PlusIcon />
    </Button>
    {#if availableSshHosts.length > 0}
      <DropdownMenu.Root
        bind:open={showImportMenu}
        onOpenChange={(open) => {
          if (open) {
            showNewForm = false;
            confirmDelete = false;
          }
        }}
      >
        <DropdownMenu.Trigger>
          {#snippet child({ props })}
            <Button {...props} variant="outline" size="sm" title="Import from SSH config">SSH</Button>
          {/snippet}
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="end" class="max-h-64 min-w-64">
          <DropdownMenu.Group>
            <DropdownMenu.GroupHeading>Import from ~/.ssh/config</DropdownMenu.GroupHeading>
            {#each availableSshHosts as host (host.name)}
              <DropdownMenu.Item onclick={() => handleImport(host.name)} class="flex-col items-start gap-0.5">
                <span>{host.name}</span>
                <span class="text-xs text-muted-foreground">
                  {host.user ? host.user + "@" : ""}{host.hostname}{host.port !== 22 ? ":" + host.port : ""}
                </span>
              </DropdownMenu.Item>
            {/each}
          </DropdownMenu.Group>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    {/if}
    {#if canDelete}
      {#if confirmDelete}
        <Button variant="destructive" size="sm" onclick={handleDelete}>Delete?</Button>
        <Button variant="outline" size="sm" onclick={cancelDelete}>Cancel</Button>
      {:else}
        <Button variant="outline" size="icon-sm" title="Delete current profile" aria-label="Delete current profile" onclick={handleDelete}>
          <Trash2Icon />
        </Button>
      {/if}
    {/if}
  </div>
  {#if showNewForm}
    <Field.Group class="flex-row items-end gap-2">
      <Field.Field class="min-w-0 flex-1 gap-1.5">
        <Field.Label for="new-profile-name" class="sr-only">Profile name</Field.Label>
        <Input id="new-profile-name" bind:ref={newNameInput} bind:value={newName} placeholder="Profile name" onkeydown={handleNewKeydown} />
      </Field.Field>
      <Button size="sm" onclick={handleCreate}>Create</Button>
      <Button variant="outline" size="sm" onclick={() => { showNewForm = false; newName = ""; }}>Cancel</Button>
    </Field.Group>
  {/if}
</div>

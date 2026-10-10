<script lang="ts">
  import {
    addPort,
    activeProfile,
    isPending,
    portStatuses,
    pendingPorts,
    removeAllPorts,
    removePorts,
    statusMessage,
  } from "../stores/portManager";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import * as Table from "$lib/components/ui/table";
  import * as Empty from "$lib/components/ui/empty";
  import * as Field from "$lib/components/ui/field";
  import * as AlertDialog from "$lib/components/ui/alert-dialog";
  import { Switch } from "$lib/components/ui/switch";
  import { Label } from "$lib/components/ui/label";
  import { tick } from "svelte";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
  import * as Collapsible from "$lib/components/ui/collapsible";
  import { MoreHorizontal, Plus, X } from "@lucide/svelte";
  import PortRow from "./PortRow.svelte";

  let newPort = "";
  let newName = "";
  let search = "";
  let needsAttention = false;
  let confirmRemoveAll = false;
  let removingAll = false;
  let adding = false;
  let addOpen = false;
  let portInput: HTMLInputElement | null = null;
  let addButton: HTMLButtonElement | null = null;

  async function toggleAdd() {
    addOpen = !addOpen;
    await tick();
    if (addOpen) portInput?.focus();
    else addButton?.focus();
  }
  let confirmDisconnected = false;
  let removalProfile = "";
  let removalPorts: number[] = [];

  $: disconnected = $portStatuses.filter((entry) => entry.status !== "Forwarding");
  $: if ($activeProfile.name !== removalProfile) confirmDisconnected = false;

  function reviewDisconnected() {
    removalProfile = $activeProfile.name;
    removalPorts = disconnected.map((entry) => entry.port);
    confirmDisconnected = true;
  }

  async function handleRemoveDisconnected() {
    removingAll = true;
    const error = await removePorts(removalProfile, removalPorts);
    removingAll = false;
    if (error) statusMessage.set(`Could not remove disconnected ports: ${error}`);
    else confirmDisconnected = false;
  }

  $: if ($activeProfile) confirmRemoveAll = false;
  $: query = search.trim().toLowerCase();
  $: filteredPorts = $portStatuses.filter((entry) =>
    (String(entry.port).includes(query) || entry.name.toLowerCase().includes(query)) &&
    (!needsAttention || (entry.status !== "Forwarding" && entry.status !== "Stopped"))
  ).sort((a, b) => a.port - b.port);

  async function handleAdd() {
    if ($isPending || removingAll || adding) return;
    const port = Number(newPort.trim());
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      statusMessage.set("Enter a valid port number (1–65535)");
      return;
    }
    adding = true;
    const err = await addPort(port, newName.trim());
    adding = false;
    if (err) {
      statusMessage.set(err);
    } else {
      newPort = "";
      newName = "";
      addOpen = false;
      await tick();
      addButton?.focus();
    }
  }

  async function handleRemoveAll() {
    removingAll = true;
    const err = await removeAllPorts();
    removingAll = false;
    confirmRemoveAll = false;
    if (err) {
      statusMessage.set(err);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") handleAdd();
  }
</script>

<section class="flex min-h-52 flex-1 flex-col gap-2" aria-labelledby="ports-heading">
  <div class="flex items-center justify-between gap-2">
    <h2 id="ports-heading" class="text-sm font-semibold">Ports <span class="font-normal text-muted-foreground">{filteredPorts.length}/{$portStatuses.length}</span></h2>
    <div class="flex items-center gap-1">
      <Button bind:ref={addButton} size="sm" onclick={toggleAdd} aria-expanded={addOpen} aria-controls="add-port-form"><Plus data-icon="inline-start" />Add port</Button>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          {#snippet child({ props })}<Button {...props} variant="ghost" size="icon-sm" aria-label="Manage ports"><MoreHorizontal /></Button>{/snippet}
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="end">
          <DropdownMenu.Group>
            <DropdownMenu.GroupHeading>Manage ports</DropdownMenu.GroupHeading>
            <DropdownMenu.Item variant="destructive" onclick={reviewDisconnected} disabled={disconnected.length === 0 || $isPending || $pendingPorts.size > 0 || removingAll || adding}>Remove disconnected ({disconnected.length})</DropdownMenu.Item>
            <DropdownMenu.Item variant="destructive" onclick={() => (confirmRemoveAll = true)} disabled={$activeProfile.ports.length === 0 || $isPending || $pendingPorts.size > 0 || removingAll || adding}>Remove all ports</DropdownMenu.Item>
          </DropdownMenu.Group>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
      <AlertDialog.Root bind:open={confirmDisconnected}>
        <AlertDialog.Content>
          <AlertDialog.Header>
            <AlertDialog.Title>Remove {removalPorts.length} disconnected ports?</AlertDialog.Title>
            <AlertDialog.Description>
              Stop their tunnels and remove these ports from profile "{removalProfile}": {removalPorts.join(", ")}.
              Includes stopped ports and ports hidden by the current filter. Forwarding ports are kept.
            </AlertDialog.Description>
          </AlertDialog.Header>
          <AlertDialog.Footer>
            <AlertDialog.Cancel disabled={removingAll}>Cancel</AlertDialog.Cancel>
            <Button variant="destructive" disabled={removingAll} onclick={handleRemoveDisconnected}>{removingAll ? "Removing…" : `Remove ${removalPorts.length} ports`}</Button>
          </AlertDialog.Footer>
        </AlertDialog.Content>
      </AlertDialog.Root>
    <AlertDialog.Root bind:open={confirmRemoveAll}>
      <AlertDialog.Content>
        <AlertDialog.Header>
          <AlertDialog.Title>Remove all {$activeProfile.ports.length} ports?</AlertDialog.Title>
          <AlertDialog.Description>Stop forwarding and remove all ports from profile "{$activeProfile.name}", including ports hidden by the current filter.</AlertDialog.Description>
        </AlertDialog.Header>
        <AlertDialog.Footer>
          <AlertDialog.Cancel disabled={removingAll}>Cancel</AlertDialog.Cancel>
          <Button variant="destructive" onclick={handleRemoveAll} disabled={removingAll}>{removingAll ? "Removing…" : `Remove ${$activeProfile.ports.length} ports`}</Button>
        </AlertDialog.Footer>
      </AlertDialog.Content>
    </AlertDialog.Root>
    </div>
  </div>
  <Field.Group class="flex-row items-center gap-3">
    <Field.Field class="min-w-0 flex-1">
      <Field.Label for="port-search" class="sr-only">Search ports</Field.Label>
      <Input id="port-search" bind:value={search} placeholder="Search by port or name" type="search" />
    </Field.Field>
    <div class="flex shrink-0 items-center gap-2">
      <Switch id="needs-attention" bind:checked={needsAttention} />
      <Label for="needs-attention">Needs attention</Label>
    </div>
  </Field.Group>
  <div class="min-h-16 flex-1 overflow-auto rounded-xl border bg-card [&_[data-slot=table-container]]:overflow-visible">
    {#if filteredPorts.length > 0}
      <Table.Root>
        <Table.Header class="sticky top-0 bg-card">
          <Table.Row>
            <Table.Head>Service</Table.Head>
            <Table.Head class="w-16">Port</Table.Head>
            <Table.Head>Status</Table.Head>
            <Table.Head class="w-16"><span class="sr-only">Actions</span></Table.Head>
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {#each filteredPorts as portInfo (portInfo.port)}
            <PortRow {portInfo} pending={$pendingPorts.has(portInfo.port)} busy={$isPending || removingAll} />
          {/each}
        </Table.Body>
      </Table.Root>
    {:else}
      <Empty.Root class="p-4">
        <Empty.Header>
          <Empty.Title>{$portStatuses.length === 0 ? "No ports configured" : "No matching ports"}</Empty.Title>
          <Empty.Description>{$portStatuses.length === 0 ? "Add a port to start forwarding." : "Try another search or turn off the attention filter."}</Empty.Description>
        </Empty.Header>
      </Empty.Root>
    {/if}
  </div>
  <Collapsible.Root bind:open={addOpen}>
    <Collapsible.Content forceMount class={addOpen ? undefined : "hidden"}>
      <div id="add-port-form" class="rounded-xl border bg-card p-3">
        <div class="mb-2 flex items-center justify-between">
          <span class="text-sm font-medium">Add a port</span>
          <Button variant="ghost" size="icon-xs" onclick={toggleAdd} aria-label="Close add port"><X /></Button>
        </div>
        <Field.Group class="flex-row items-end gap-2">
    <Field.Field class="w-24 shrink-0">
      <Field.Label for="new-port" class="sr-only">Port number</Field.Label>
      <Input id="new-port" bind:ref={portInput} bind:value={newPort} placeholder="Port" inputmode="numeric" pattern="[0-9]*" onkeydown={handleKeydown} />
    </Field.Field>
    <Field.Field class="min-w-0">
      <Field.Label for="new-port-name" class="sr-only">Name (optional)</Field.Label>
      <Input id="new-port-name" bind:value={newName} placeholder="Name (optional)" onkeydown={handleKeydown} />
    </Field.Field>
    <Button onclick={handleAdd} disabled={$isPending || removingAll || adding}><Plus data-icon="inline-start" />Add &amp; Start</Button>
        </Field.Group>
      </div>
    </Collapsible.Content>
  </Collapsible.Root>
</section>

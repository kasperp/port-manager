<script lang="ts">
  import Settings from "@lucide/svelte/icons/settings";
  import { Badge } from "$lib/components/ui/badge";
  import { Button } from "$lib/components/ui/button";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
  import {
    activeProfile,
    aggregateStatus,
    autoReconnect,
    portStatuses,
    setAutoReconnect,
    setStartupEnabled,
    startupEnabled,
  } from "../stores/portManager";

  $: forwarding = $portStatuses.filter((port) => port.status === "Forwarding").length;
  $: statusText = {
    "all-forwarding": "All ports connected",
    partial: "Some ports need attention",
    inactive: "No ports connected",
    "no-ports": "Add your first port to get started",
  }[$aggregateStatus];
</script>

<header class="flex shrink-0 items-center justify-between gap-3 px-1 py-1.5">
  <div class="flex min-w-0 flex-col gap-1">
    <h1 class="app-title text-foreground">My ports</h1>
    <p class="truncate text-xs text-muted-foreground" title={$activeProfile.host}>
      {$activeProfile.host || "Connect a server to get started"}
    </p>
  </div>
  <div class="flex shrink-0 items-center gap-2">
      <Badge variant={$aggregateStatus === "all-forwarding" ? "success" : $aggregateStatus === "partial" ? "warning" : $aggregateStatus === "inactive" ? "destructive" : "secondary"} title={statusText}>
        <span class="size-1.5 rounded-full bg-current" aria-hidden="true"></span>
        {#if $portStatuses.length > 0}
          <span class="tabular-nums">{forwarding}</span> connected
        {:else}
          No ports yet
        {/if}
      </Badge>
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        {#snippet child({ props: triggerProps })}
          <Button {...triggerProps} variant="ghost" size="icon-xs" aria-label="App settings" title="App settings"><Settings /></Button>
        {/snippet}
      </DropdownMenu.Trigger>
      <DropdownMenu.Content align="end">
        <DropdownMenu.Group>
          <DropdownMenu.GroupHeading>Settings</DropdownMenu.GroupHeading>
          <DropdownMenu.CheckboxItem checked={$autoReconnect} onCheckedChange={setAutoReconnect}>Auto-reconnect</DropdownMenu.CheckboxItem>
          <DropdownMenu.CheckboxItem checked={$startupEnabled} onCheckedChange={setStartupEnabled}>Run on startup</DropdownMenu.CheckboxItem>
        </DropdownMenu.Group>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  </div>
</header>

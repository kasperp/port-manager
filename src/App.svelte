<script lang="ts">
  import * as Collapsible from "$lib/components/ui/collapsible";
  import { Button } from "$lib/components/ui/button";
  import { Badge } from "$lib/components/ui/badge";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ActionBar from "./lib/components/ActionBar.svelte";
  import ConnectionSettings from "./lib/components/ConnectionSettings.svelte";
  import Header from "./lib/components/Header.svelte";
  import PortList from "./lib/components/PortList.svelte";
  import ProfileSelector from "./lib/components/ProfileSelector.svelte";
  import {
    activeProfile,
    loadConfig,
    loadSshHosts,
    loadStartupStatus,
    loadStatuses,
    startAll,
    startListening,
    statusMessage,
    stopListening,
  } from "./lib/stores/portManager";

  let connectionOpen = false;

  onMount(async () => {
    const appWindow = getCurrentWindow();

    // Hide to tray on minimize
    await appWindow.onResized(async () => {
      if (await appWindow.isMinimized()) {
        await appWindow.hide();
      }
    });

    await loadConfig();
    await loadSshHosts();
    await loadStartupStatus();
    await startListening();

    // Kick off initial tunnel start then load status
    await startAll();
    await loadStatuses();
  });

  onDestroy(() => {
    stopListening();
  });
</script>

<main class="flex h-full min-h-[480px] flex-col gap-2.5 p-3.5">
  <div class="shrink-0"><Header /></div>
  <Collapsible.Root bind:open={connectionOpen} class="shrink-0">
    <Collapsible.Trigger>
      {#snippet child({ props })}
        <Button {...props} variant="ghost" size="sm" class="w-full justify-between gap-3 px-2" aria-label="Connection and profile">
          <span class="text-muted-foreground">Server settings</span>
          <span class="flex shrink-0 items-center gap-2">
            <Badge variant="secondary" class="max-w-28 truncate" title={$activeProfile.name}>{$activeProfile.name}</Badge>
            <ChevronDown class={connectionOpen ? "rotate-180" : undefined} />
          </span>
        </Button>
      {/snippet}
    </Collapsible.Trigger>
    <Collapsible.Content forceMount class={connectionOpen ? undefined : "hidden"}>
      <div class="flex flex-col gap-2.5 pt-2.5">
        <ProfileSelector />
        <ConnectionSettings />
      </div>
    </Collapsible.Content>
  </Collapsible.Root>
  <PortList />
  <div class="shrink-0"><ActionBar /></div>
  <p class="app-status shrink-0 text-xs text-muted-foreground" role="status">{$statusMessage}</p>
</main>

<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import ActionBar from "./lib/components/ActionBar.svelte";
  import ConnectionSettings from "./lib/components/ConnectionSettings.svelte";
  import Header from "./lib/components/Header.svelte";
  import PortList from "./lib/components/PortList.svelte";
  import ProfileSelector from "./lib/components/ProfileSelector.svelte";
  import {
    loadConfig,
    loadSshHosts,
    loadStartupStatus,
    loadStatuses,
    startAll,
    startListening,
    statusMessage,
    stopListening,
  } from "./lib/stores/portManager";

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

<main>
  <Header />
  <ProfileSelector />
  <ConnectionSettings />
  <PortList />
  <ActionBar />
  <p class="status-bar">{$statusMessage}</p>
</main>

<style>
  main {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
    box-sizing: border-box;
  }

  .status-bar {
    font-size: 11px;
    color: #9ca3af;
    margin: 0;
    padding-top: 2px;
    flex-shrink: 0;
  }
</style>

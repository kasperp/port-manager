import { derived, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AggregateStatus, Config, PortStatusInfo } from "../types";

// Core stores
export const config = writable<Config>({
  host: "",
  user: "",
  ssh_port: 22,
  ports: [],
});
export const portStatuses = writable<PortStatusInfo[]>([]);
export const autoReconnect = writable(true);
export const startupEnabled = writable(false);
export const statusMessage = writable("Ready");
export const isPending = writable(false);

// Derived aggregate status for the header
export const aggregateStatus = derived<
  typeof portStatuses,
  AggregateStatus
>(portStatuses, ($statuses) => {
  if ($statuses.length === 0) return "no-ports";
  const active = $statuses.filter((s) => s.status === "Active").length;
  if (active === $statuses.length) return "all-active";
  if (active > 0) return "partial";
  return "inactive";
});

// Actions
export async function loadConfig() {
  const cfg = await invoke<Config>("get_config");
  config.set(cfg);
}

export async function loadStatuses() {
  const statuses = await invoke<PortStatusInfo[]>("get_port_statuses");
  portStatuses.set(statuses);
}

export async function loadStartupStatus() {
  const enabled = await invoke<boolean>("get_startup_enabled");
  startupEnabled.set(enabled);
}

export async function saveSettings(
  host: string,
  user: string,
  sshPort: number
) {
  await invoke("save_settings", { host, user, sshPort });
  config.update((c) => ({ ...c, host, user, ssh_port: sshPort }));
  statusMessage.set("Settings saved!");
}

export async function addPort(port: number): Promise<string | null> {
  try {
    await invoke("add_port", { port });
    config.update((c) => ({ ...c, ports: [...c.ports, port] }));
    await loadStatuses();
    statusMessage.set(`Added port ${port}`);
    return null;
  } catch (e) {
    return String(e);
  }
}

export async function removePort(port: number) {
  await invoke("remove_port", { port });
  config.update((c) => ({ ...c, ports: c.ports.filter((p) => p !== port) }));
  portStatuses.update((s) => s.filter((ps) => ps.port !== port));
  statusMessage.set(`Removed port ${port}`);
}

export async function startAll() {
  isPending.set(true);
  statusMessage.set("Starting port forwards...");
  const errors = await invoke<string[]>("start_all");
  // Give SSH ~2s to establish connections before refreshing status
  setTimeout(async () => {
    await loadStatuses();
    isPending.set(false);
    statusMessage.set(
      errors.length === 0
        ? "Port forwards started!"
        : `Started with errors: ${errors.join(", ")}`
    );
  }, 2000);
}

export async function stopAll() {
  await invoke("stop_all");
  await loadStatuses();
  statusMessage.set("Port forwards stopped");
}

export async function setAutoReconnect(enabled: boolean) {
  autoReconnect.set(enabled);
  await invoke("set_auto_reconnect", { enabled });
}

export async function setStartupEnabled(enabled: boolean) {
  try {
    await invoke("set_startup_enabled", { enabled });
    startupEnabled.set(enabled);
  } catch (e) {
    statusMessage.set(`Startup toggle failed: ${e}`);
  }
}

// Listen for background status updates emitted by Rust every 10s
let unlisten: UnlistenFn | null = null;

export async function startListening() {
  unlisten = await listen<PortStatusInfo[]>("port-status-update", (event) => {
    portStatuses.set(event.payload);
  });
}

export function stopListening() {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
}

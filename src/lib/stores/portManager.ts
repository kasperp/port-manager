import { derived, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AggregateStatus,
  Config,
  PortlessDiscoveryResult,
  PortlessGateway,
  PortStatusInfo,
  Profile,
  ProfileMode,
  SshHostEntry,
} from "../types";

// Core stores
export const config = writable<Config>({
  active_profile: "Default",
  profiles: [
    {
      name: "Default",
      host: "",
      user: "",
      ssh_port: 22,
      mode: "ports",
      portless: {
        base_local_port: 8100,
        scheme: "https",
        hosts: [],
        gateways: [],
      },
      ports: [],
      rate_limit_max: 6,
      rate_limit_window_secs: 30,
    },
  ],
});
export const portStatuses = writable<PortStatusInfo[]>([]);
export const autoReconnect = writable(true);
export const startupEnabled = writable(false);
export const statusMessage = writable("Ready");
export const isPending = writable(false);
export const sshHosts = writable<SshHostEntry[]>([]);

// Derived: the currently active profile
export const activeProfile = derived<typeof config, Profile>(
  config,
  ($config) => {
    const found = $config.profiles.find(
      (p) => p.name === $config.active_profile
    );
    return (
      found ?? $config.profiles[0] ?? {
        name: "Default",
        host: "",
        user: "",
        ssh_port: 22,
        mode: "ports",
        portless: {
          base_local_port: 8100,
          scheme: "https",
          hosts: [],
          gateways: [],
        },
        ports: [],
        rate_limit_max: 6,
        rate_limit_window_secs: 30,
      }
    );
  }
);

// Derived aggregate status for the header
export const aggregateStatus = derived<
  typeof portStatuses,
  AggregateStatus
>(portStatuses, ($statuses) => {
  if ($statuses.length === 0) return "no-ports";
  const forwarding = $statuses.filter((s) => s.status === "Forwarding").length;
  if (forwarding === $statuses.length) return "all-forwarding";
  if (forwarding > 0) return "partial";
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

export async function loadSshHosts() {
  const hosts = await invoke<SshHostEntry[]>("get_ssh_hosts");
  sshHosts.set(hosts);
}

export async function saveProfileSettings(
  host: string,
  user: string,
  sshPort: number,
  rateLimitMax: number,
  rateLimitWindowSecs: number
) {
  await invoke("save_profile_settings", {
    host,
    user,
    sshPort,
    rateLimitMax,
    rateLimitWindowSecs,
  });
  config.update((c) => ({
    ...c,
    profiles: c.profiles.map((p) =>
      p.name === c.active_profile
        ? {
            ...p,
            host,
            user,
            ssh_port: sshPort,
            rate_limit_max: rateLimitMax,
            rate_limit_window_secs: rateLimitWindowSecs,
          }
        : p
    ),
  }));
  statusMessage.set("Settings saved!");
}

export async function saveProfileMode(mode: ProfileMode) {
  const cfg = await invoke<Config>("save_profile_mode", { mode });
  config.set(cfg);
  portStatuses.set([]);
  statusMessage.set(mode === "portless" ? "Portless mode enabled" : "Ports mode enabled");
}

export async function savePortlessSettings(
  baseLocalPort: number,
  scheme: "http" | "https",
  hosts: string[],
  gateways: PortlessGateway[]
): Promise<string | null> {
  try {
    const cfg = await invoke<Config>("save_portless_settings", {
      baseLocalPort,
      scheme,
      hosts,
      gateways,
    });
    config.set(cfg);
    await loadStatuses();
    statusMessage.set("Portless gateways saved");
    return null;
  } catch (e) {
    return String(e);
  }
}

export async function discoverPortlessPorts(
  pattern: string,
  startPort: number,
  endPort: number
): Promise<PortlessDiscoveryResult | string> {
  try {
    return await invoke<PortlessDiscoveryResult>("discover_portless_ports", {
      pattern,
      startPort,
      endPort,
    });
  } catch (e) {
    return String(e);
  }
}

export async function addPort(
  port: number,
  name: string
): Promise<string | null> {
  try {
    await invoke("add_port", { port, name });
    config.update((c) => ({
      ...c,
      profiles: c.profiles.map((p) =>
        p.name === c.active_profile
          ? { ...p, ports: [...p.ports, { port, name }] }
          : p
      ),
    }));
    await loadStatuses();
    statusMessage.set(name ? `Added port ${port} (${name})` : `Added port ${port}`);
    return null;
  } catch (e) {
    return String(e);
  }
}

export async function removePort(port: number) {
  await invoke("remove_port", { port });
  config.update((c) => ({
    ...c,
    profiles: c.profiles.map((p) =>
      p.name === c.active_profile
        ? { ...p, ports: p.ports.filter((entry) => entry.port !== port) }
        : p
    ),
  }));
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

export async function startPort(port: number): Promise<string | null> {
  try {
    isPending.set(true);
    statusMessage.set(`Starting port ${port}...`);
    await invoke("start_port", { port });
    // Give SSH ~2s to establish the connection before refreshing
    setTimeout(async () => {
      await loadStatuses();
      isPending.set(false);
      statusMessage.set(`Port ${port} started`);
    }, 2000);
    return null;
  } catch (e) {
    isPending.set(false);
    const msg = String(e);
    statusMessage.set(`Start failed: ${msg}`);
    return msg;
  }
}

export async function stopPort(port: number): Promise<string | null> {
  try {
    await invoke("stop_port", { port });
    await loadStatuses();
    statusMessage.set(`Port ${port} stopped`);
    return null;
  } catch (e) {
    const msg = String(e);
    statusMessage.set(`Stop failed: ${msg}`);
    return msg;
  }
}

export async function killPortProcess(port: number): Promise<string | null> {
  try {
    await invoke("kill_port_process", { port });
    statusMessage.set(`Killed process on port ${port}`);
    await loadStatuses();
    return null;
  } catch (e) {
    const msg = String(e);
    statusMessage.set(`Kill failed: ${msg}`);
    return msg;
  }
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

// ---- Profile Actions ----

export async function switchProfile(name: string) {
  isPending.set(true);
  statusMessage.set(`Switching to profile "${name}"...`);
  try {
    const cfg = await invoke<Config>("switch_profile", { name });
    config.set(cfg);
    portStatuses.set([]);
    await loadStatuses();
    statusMessage.set(`Switched to "${name}"`);
  } catch (e) {
    statusMessage.set(`Switch failed: ${e}`);
  } finally {
    isPending.set(false);
  }
}

export async function createProfile(
  name: string,
  host: string,
  user: string,
  sshPort: number
): Promise<string | null> {
  try {
    const cfg = await invoke<Config>("create_profile", {
      name,
      host,
      user,
      sshPort,
    });
    config.set(cfg);
    portStatuses.set([]);
    statusMessage.set(`Created and switched to profile "${name}"`);
    return null;
  } catch (e) {
    return String(e);
  }
}

export async function deleteProfile(name: string): Promise<string | null> {
  try {
    const cfg = await invoke<Config>("delete_profile", { name });
    config.set(cfg);
    portStatuses.set([]);
    await loadStatuses();
    statusMessage.set(`Deleted profile "${name}"`);
    return null;
  } catch (e) {
    return String(e);
  }
}

export async function importSshProfile(
  sshHostName: string
): Promise<string | null> {
  try {
    const cfg = await invoke<Config>("import_ssh_profile", { sshHostName });
    config.set(cfg);
    portStatuses.set([]);
    statusMessage.set(`Imported and switched to profile "${sshHostName}"`);
    return null;
  } catch (e) {
    return String(e);
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

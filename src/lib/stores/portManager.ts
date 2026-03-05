import { derived, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AggregateStatus,
  Config,
  PortStatusInfo,
  Profile,
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

export async function addPort(port: number): Promise<string | null> {
  try {
    await invoke("add_port", { port });
    config.update((c) => ({
      ...c,
      profiles: c.profiles.map((p) =>
        p.name === c.active_profile
          ? { ...p, ports: [...p.ports, port] }
          : p
      ),
    }));
    await loadStatuses();
    statusMessage.set(`Added port ${port}`);
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
        ? { ...p, ports: p.ports.filter((pp) => pp !== port) }
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

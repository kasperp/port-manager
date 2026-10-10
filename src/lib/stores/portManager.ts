import { derived, get, writable } from "svelte/store";
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
export const pendingPorts = writable<Set<number>>(new Set());
export const sshHosts = writable<SshHostEntry[]>([]);

let pendingProfile = get(config).active_profile;
let profileVersion = 0;
config.subscribe((value) => {
  if (value.active_profile === pendingProfile) return;
  pendingProfile = value.active_profile;
  profileVersion += 1;
  pendingPorts.set(new Set());
});

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
  const profileName = get(config).active_profile;
  const statuses = await invoke<PortStatusInfo[]>("get_port_statuses");
  if (get(config).active_profile === profileName) portStatuses.set(statuses);
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

export async function addPort(
  port: number,
  name: string
): Promise<string | null> {
  const profileName = get(config).active_profile;
  const version = profileVersion;
  try {
    await invoke("add_port", { port, name });
    config.update((c) => ({
      ...c,
      profiles: c.profiles.map((p) =>
        p.name === profileName
          ? { ...p, ports: [...p.ports, { port, name }] }
          : p
      ),
    }));
    if (version !== profileVersion) return null;
    await loadStatuses();
    if (version !== profileVersion) return null;
    const startError = await startPort(port);
    if (startError && version === profileVersion) {
      statusMessage.set(`Added port ${port}, but could not start: ${startError}`);
    }
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

export async function removeAllPorts(): Promise<string | null> {
  try {
    const cfg = await invoke<Config>("remove_all_ports");
    config.set(cfg);
    portStatuses.set([]);
    statusMessage.set("Removed all ports from the active profile");
    return null;
  } catch (e) {
    return String(e);
  }
}

export async function removePorts(profileName: string, ports: number[]): Promise<string | null> {
  isPending.set(true);
  try {
    const cfg = await invoke<Config>("remove_ports", { profileName, ports });
    if (get(config).active_profile === profileName) {
      config.set(cfg);
      portStatuses.update((statuses) => statuses.filter((entry) => !ports.includes(entry.port)));
      statusMessage.set(`Removed ${ports.length} disconnected ports from "${profileName}"`);
    }
    return null;
  } catch (e) {
    return String(e);
  } finally {
    isPending.set(false);
  }
}

const statusLabels = {
  Forwarding: "forwarding",
  RemoteDown: "not listening",
  Reconnecting: "reconnecting or queued",
  TunnelDown: "tunnel down",
  PortInUse: "port in use",
  Stopped: "stopped",
};

function forwardingSummary(statuses: PortStatusInfo[]) {
  const counts = Object.entries(statusLabels).flatMap(([status, label]) => {
    const count = statuses.filter((entry) => entry.status === status).length;
    return count ? [`${count} ${label}`] : [];
  });
  return counts.length ? counts.join(", ") : "No ports configured";
}

export async function startAll() {
  const version = profileVersion;
  const message = "Starting port forwards...";
  isPending.set(true);
  statusMessage.set(message);
  try {
    const errors = await invoke<string[]>("start_all");
    // SSH needs time to establish connections before an observed result is useful.
    await new Promise((resolve) => setTimeout(resolve, 2000));
    if (version !== profileVersion) return;
    await loadStatuses();
    if (version === profileVersion && get(statusMessage) === message) {
      const summary = forwardingSummary(get(portStatuses));
      statusMessage.set(errors.length ? `${summary}. Errors: ${errors.join(", ")}` : summary);
    }
  } catch (e) {
    if (version === profileVersion && get(statusMessage) === message) {
      statusMessage.set(`Start failed: ${e}`);
    }
  } finally {
    isPending.set(false);
  }
}

export async function stopAll() {
  isPending.set(true);
  try {
    await invoke("stop_all");
    await loadStatuses();
    statusMessage.set("Port forwards stopped");
  } catch (e) {
    statusMessage.set(`Stop failed: ${e}`);
  } finally {
    isPending.set(false);
  }
}

export async function startPort(port: number): Promise<string | null> {
  const version = profileVersion;
  const message = `Starting port ${port}...`;
  pendingPorts.update((ports) => new Set([...ports, port]));
  statusMessage.set(message);
  try {
    await invoke("start_port", { port });
    await new Promise((resolve) => setTimeout(resolve, 2000));
    if (version !== profileVersion) return null;
    await loadStatuses();
    if (version === profileVersion && get(statusMessage) === message) {
      const status = get(portStatuses).find((entry) => entry.port === port);
      statusMessage.set(status ? `Port ${port}: ${statusLabels[status.status]}` : `Port ${port} is no longer configured`);
    }
    return null;
  } catch (e) {
    const msg = String(e);
    if (version === profileVersion && get(statusMessage) === message) {
      statusMessage.set(`Start failed: ${msg}`);
    }
    return msg;
  } finally {
    if (version === profileVersion) {
      pendingPorts.update((ports) => {
        const remaining = new Set(ports);
        remaining.delete(port);
        return remaining;
      });
    }
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

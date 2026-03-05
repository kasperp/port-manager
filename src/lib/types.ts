export type PortStatus = "Forwarding" | "RemoteDown" | "Reconnecting" | "TunnelDown" | "PortInUse" | "Stopped";

export interface PortStatusInfo {
  port: number;
  status: PortStatus;
  pid: number | null;
  owner_pid: number | null;
  process_name: string | null;
}

export interface Profile {
  name: string;
  host: string;
  user: string;
  ssh_port: number;
  ports: number[];
  rate_limit_max: number;
  rate_limit_window_secs: number;
}

export interface Config {
  active_profile: string;
  profiles: Profile[];
}

export interface SshHostEntry {
  name: string;
  hostname: string;
  user: string;
  port: number;
}

export type AggregateStatus =
  | "all-forwarding"
  | "partial"
  | "inactive"
  | "no-ports";

export type PortStatus = "Active" | "Inactive";

export interface PortStatusInfo {
  port: number;
  status: PortStatus;
  pid: number | null;
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
  | "all-active"
  | "partial"
  | "inactive"
  | "no-ports";

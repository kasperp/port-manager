export type PortStatus = "Forwarding" | "RemoteDown" | "Reconnecting" | "TunnelDown" | "PortInUse" | "Stopped";

export interface PortStatusInfo {
  port: number;
  name: string;
  status: PortStatus;
  pid: number | null;
  owner_pid: number | null;
  process_name: string | null;
}

export interface ForwardedPort {
  port: number;
  name: string;
}

export type ProfileMode = "ports" | "portless";

export interface PortlessGateway {
  name: string;
  host: string | null;
  local_port: number;
  remote_port: number;
}

export interface PortlessConfig {
  base_local_port: number;
  scheme: "http" | "https";
  hosts: string[];
  gateways: PortlessGateway[];
}

export interface Profile {
  name: string;
  host: string;
  user: string;
  ssh_port: number;
  mode: ProfileMode;
  portless: PortlessConfig;
  ports: ForwardedPort[];
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

export interface PortlessDiscoveryResult {
  scheme: "http" | "https";
  host: string;
  ports: number[];
}

export type AggregateStatus =
  | "all-forwarding"
  | "partial"
  | "inactive"
  | "no-ports";

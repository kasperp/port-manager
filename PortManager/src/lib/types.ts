export type PortStatus = "Active" | "Inactive";

export interface PortStatusInfo {
  port: number;
  status: PortStatus;
  pid: number | null;
}

export interface Config {
  host: string;
  user: string;
  ssh_port: number;
  ports: number[];
}

export type AggregateStatus =
  | "all-active"
  | "partial"
  | "inactive"
  | "no-ports";

<script lang="ts">
  import { cn } from "$lib/utils";
  import type { PortStatusInfo } from "../types";
  import { killPortProcess, removePort, startPort, statusMessage, stopPort } from "../stores/portManager";
  import { MoreHorizontal, Trash2 } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button";
  import * as Table from "$lib/components/ui/table";
  import * as ContextMenu from "$lib/components/ui/context-menu";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu";

  export let portInfo: PortStatusInfo;
  export let pending = false;
  export let busy = false;
  let working = false;

  $: displayStatus = pending ? "Connecting" : {
    Forwarding: "Connected", RemoteDown: "Service offline", Reconnecting: "Connecting",
    TunnelDown: "Disconnected", PortInUse: "Port in use", Stopped: "Stopped",
  }[portInfo.status];
  $: statusTooltip = pending ? "Waiting for the SSH connection to establish" : {
    Forwarding: "Tunnel is up and the remote service is accepting connections",
    RemoteDown: "Tunnel is up but nothing is listening on this port",
    Reconnecting: "Tunnel stopped, automatically reconnecting",
    TunnelDown: "Tunnel stopped, auto-reconnect is off or in cooldown",
    PortInUse: "Another process is already listening on this port",
    Stopped: "Not forwarding. Use the actions menu to start this port",
  }[portInfo.status];
  $: statusClass = pending ? "text-warning" : {
    Forwarding: "text-success", RemoteDown: "text-destructive", Reconnecting: "text-warning",
    TunnelDown: "text-destructive", PortInUse: "text-destructive", Stopped: "text-muted-foreground",
  }[portInfo.status];
  $: actionsDisabled = pending || busy || working;
  $: canStart = portInfo.status === "Stopped" || portInfo.status === "TunnelDown";
  $: canStop = ["Forwarding", "RemoteDown", "Reconnecting"].includes(portInfo.status);
  $: canKill = portInfo.status === "PortInUse";
  $: ownerLabel = portInfo.process_name ? `${portInfo.process_name}${portInfo.owner_pid ? ` (${portInfo.owner_pid})` : ""}`
    : portInfo.owner_pid ? `PID ${portInfo.owner_pid}` : null;
  $: processLabel = ownerLabel ?? (portInfo.pid ? `Tunnel PID ${portInfo.pid}` : null);

  async function handleStop() {
    working = true;
    try {
      await stopPort(portInfo.port);
    } finally {
      working = false;
    }
  }

  async function handleKill() {
    working = true;
    try {
      await killPortProcess(portInfo.port);
    } finally {
      working = false;
    }
  }

  async function handleRemove() {
    working = true;
    try {
      await removePort(portInfo.port);
    } catch (error) {
      statusMessage.set(`Could not remove port ${portInfo.port}: ${error}`);
    } finally {
      working = false;
    }
  }
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger disabled={actionsDisabled}>
    {#snippet child({ props })}
      <Table.Row {...props}>
        <Table.Cell class="max-w-40 truncate font-medium" title={portInfo.name || `Port ${portInfo.port}`}>{portInfo.name || `Port ${portInfo.port}`}</Table.Cell>
        <Table.Cell class="font-mono text-muted-foreground tabular-nums">{portInfo.port}</Table.Cell>
        <Table.Cell title={statusTooltip}>
          <span class={cn("inline-flex items-center gap-1.5 whitespace-nowrap text-xs", statusClass)}>
            <span class="size-1.5 shrink-0 rounded-full bg-current" aria-hidden="true"></span>
            {displayStatus}
          </span>
        </Table.Cell>
        <Table.Cell class="w-16 px-1 text-right">
          <div class="flex items-center justify-end gap-1">
          <Button variant="ghost" size="icon-xs" class="text-muted-foreground hover:bg-destructive/10 hover:text-destructive" disabled={actionsDisabled} onclick={handleRemove} aria-label={`Remove port ${portInfo.port}`} title={`Remove port ${portInfo.port}`}><Trash2 /></Button>
          <DropdownMenu.Root>
            <DropdownMenu.Trigger disabled={actionsDisabled}>
              {#snippet child({ props: triggerProps })}
                <Button {...triggerProps} variant="ghost" size="icon-xs" aria-label={`Actions for port ${portInfo.port}`}><MoreHorizontal /></Button>
              {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content align="end">
              <DropdownMenu.Group>
                <DropdownMenu.GroupHeading>Port {portInfo.port}</DropdownMenu.GroupHeading>
                {#if processLabel}<DropdownMenu.Item disabled>{processLabel}</DropdownMenu.Item>{/if}
                {#if canStart}<DropdownMenu.Item disabled={actionsDisabled} onclick={() => startPort(portInfo.port)}>Start</DropdownMenu.Item>{/if}
                {#if canStop}<DropdownMenu.Item disabled={actionsDisabled} onclick={handleStop}>Stop</DropdownMenu.Item>{/if}
                {#if canKill}<DropdownMenu.Item variant="destructive" disabled={actionsDisabled} onclick={handleKill}>Kill process</DropdownMenu.Item>{/if}
              </DropdownMenu.Group>
              <DropdownMenu.Separator />
              <DropdownMenu.Group>
                <DropdownMenu.Item variant="destructive" disabled={actionsDisabled} onclick={handleRemove}>Remove port</DropdownMenu.Item>
              </DropdownMenu.Group>
            </DropdownMenu.Content>
          </DropdownMenu.Root>
          </div>
        </Table.Cell>
      </Table.Row>
    {/snippet}
  </ContextMenu.Trigger>
  <ContextMenu.Content>
    <ContextMenu.Group>
      <ContextMenu.GroupHeading>Port {portInfo.port}</ContextMenu.GroupHeading>
      {#if processLabel}<ContextMenu.Item disabled>{processLabel}</ContextMenu.Item>{/if}
      {#if canStart}<ContextMenu.Item disabled={actionsDisabled} onclick={() => startPort(portInfo.port)}>Start</ContextMenu.Item>{/if}
      {#if canStop}<ContextMenu.Item disabled={actionsDisabled} onclick={handleStop}>Stop</ContextMenu.Item>{/if}
      {#if canKill}<ContextMenu.Item variant="destructive" disabled={actionsDisabled} onclick={handleKill}>Kill process</ContextMenu.Item>{/if}
    </ContextMenu.Group>
    <ContextMenu.Separator />
    <ContextMenu.Group>
      <ContextMenu.Item variant="destructive" disabled={actionsDisabled} onclick={handleRemove}>Remove port</ContextMenu.Item>
    </ContextMenu.Group>
  </ContextMenu.Content>
</ContextMenu.Root>

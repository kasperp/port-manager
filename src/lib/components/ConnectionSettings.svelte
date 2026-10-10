<script lang="ts">
  import {
    activeProfile,
    saveProfileSettings,
    statusMessage,
  } from "../stores/portManager";

  import { Button } from "$lib/components/ui/button";
  import * as Card from "$lib/components/ui/card";
  import * as Field from "$lib/components/ui/field";
  import { Input } from "$lib/components/ui/input";

  let host = "";
  let user = "";
  let sshPort = 22;
  let rateLimitMax = 6;
  let rateLimitWindowSecs = 30;

  // Track the last store values we applied so we can detect user edits.
  // Only update local vars when the store changes AND the user hasn't diverged.
  let prevHost = "";
  let prevUser = "";
  let prevSshPort = 22;
  let prevRateLimitMax = 6;
  let prevRateLimitWindowSecs = 30;

  $: {
    const sh = $activeProfile.host;
    const su = $activeProfile.user;
    const sp = $activeProfile.ssh_port;
    const rlm = $activeProfile.rate_limit_max;
    const rlw = $activeProfile.rate_limit_window_secs;
    if (host === prevHost) host = sh;
    if (user === prevUser) user = su;
    if (sshPort === prevSshPort) sshPort = sp;
    if (rateLimitMax === prevRateLimitMax) rateLimitMax = rlm;
    if (rateLimitWindowSecs === prevRateLimitWindowSecs) rateLimitWindowSecs = rlw;
    prevHost = sh;
    prevUser = su;
    prevSshPort = sp;
    prevRateLimitMax = rlm;
    prevRateLimitWindowSecs = rlw;
  }

  async function handleSave() {
    const port = Number(sshPort);
    if (!port || port < 1 || port > 65535) {
      statusMessage.set("Invalid SSH port number");
      return;
    }
    const max = Number(rateLimitMax);
    const window = Number(rateLimitWindowSecs);
    if (!max || max < 1) {
      statusMessage.set("Max connections must be at least 1");
      return;
    }
    if (!window || window < 1) {
      statusMessage.set("Window must be at least 1 second");
      return;
    }
    await saveProfileSettings(host.trim(), user.trim(), port, max, window);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") handleSave();
  }
</script>

<Card.Root size="sm" class="gap-3 py-3">
  <Card.Header class="px-3">
    <Card.Title class="text-sm">Connection</Card.Title>
  </Card.Header>
  <Card.Content class="px-3">
    <Field.Group class="grid grid-cols-[minmax(0,1fr)_80px_minmax(0,1fr)] gap-2">
      <Field.Field class="gap-1.5">
        <Field.Label for="host-input">Host</Field.Label>
        <Input id="host-input" bind:value={host} placeholder="example.com" onkeydown={handleKeydown} />
      </Field.Field>
      <Field.Field class="gap-1.5">
        <Field.Label for="ssh-port-input">SSH port</Field.Label>
        <Input id="ssh-port-input" bind:value={sshPort} type="number" min="1" max="65535" onkeydown={handleKeydown} />
      </Field.Field>
      <Field.Field class="gap-1.5">
        <Field.Label for="user-input">User</Field.Label>
        <Input id="user-input" bind:value={user} placeholder="username" onkeydown={handleKeydown} />
      </Field.Field>
    </Field.Group>
  </Card.Content>
  <Card.Footer class="justify-between gap-3 px-3">
    <Field.Group class="flex-row items-center gap-2">
      <span class="shrink-0 text-xs text-muted-foreground">Rate limit</span>
      <Field.Field class="w-14 shrink-0 gap-0">
        <Field.Label for="rate-limit-max" class="sr-only">Maximum connections</Field.Label>
        <Input id="rate-limit-max" bind:value={rateLimitMax} type="number" min="1" onkeydown={handleKeydown} />
      </Field.Field>
      <span class="shrink-0 text-xs text-muted-foreground">conn /</span>
      <Field.Field class="w-14 shrink-0 gap-0">
        <Field.Label for="rate-limit-window" class="sr-only">Rate limit window in seconds</Field.Label>
        <Input id="rate-limit-window" bind:value={rateLimitWindowSecs} type="number" min="1" onkeydown={handleKeydown} />
      </Field.Field>
      <span class="text-xs text-muted-foreground">sec</span>
    </Field.Group>
    <Button size="sm" onclick={handleSave}>Save</Button>
  </Card.Footer>
</Card.Root>

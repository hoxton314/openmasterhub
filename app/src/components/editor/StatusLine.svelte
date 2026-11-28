<!-- Under the hub: what is selected, whether this is a preview, what an integration draws. -->
<script lang="ts">
  import { ChevronLeft, ChevronRight } from 'lucide-svelte';
  import { currentPage, currentProfile, hub, isEditingLive, showPage } from '../../lib/hub.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import MarkerLegend from '../ui/MarkerLegend.svelte';
  import IntegrationBanner from './IntegrationBanner.svelte';

  const sel = $derived(hub.selection);
  const text = $derived.by(() => {
    switch (sel.kind) {
      case 'none':
        return 'Nothing selected · click a control, or press Tab to enter the hub';
      case 'key':
        return `Key ${sel.index + 1} selected · arrow keys move between controls`;
      case 'dial':
        return 'Dial selected · arrow keys move between controls';
      case 'knob':
        return `Knob ${sel.index + 1} selected · arrow keys move between controls`;
      case 'fader':
        return `Fader ${sel.index + 1} selected · arrow keys move between controls`;
      case 'roller':
        return `${sel.side === 'left' ? 'Left' : 'Right'} roller selected · arrow keys move between controls`;
      case 'module': {
        const m = hub.modules.find((x) => x.serial === sel.serial);
        return `${m ? m.kind[0].toUpperCase() + m.kind.slice(1) : 'Module'} selected`;
      }
    }
  });

  const profile = $derived(currentProfile());
  const pages = $derived(profile?.page ?? []);
  const page = $derived(currentPage());
  const index = $derived(pages.findIndex((p) => p.name === page?.name));
  const go = (d: number) => {
    const next = pages[(index + d + pages.length) % pages.length];
    if (next) showPage(next.name);
  };
</script>

<div class="line">
  <div class="text" aria-live="polite">
    {#if !isEditingLive()}<span>Preview · not on the hub</span>{:else}<span>{text}</span>{/if}
    <IntegrationBanner />
  </div>
  <div class="bar">
    {#if pages.length > 1 && index >= 0}
      <div class="pager">
        <IconButton icon={ChevronLeft} size="sm" label="Previous page" onclick={() => go(-1)} />
        <span>Page {index + 1} of {pages.length} · <strong>{page?.name}</strong></span>
        <IconButton icon={ChevronRight} size="sm" label="Next page" onclick={() => go(1)} />
      </div>
    {:else}<span></span>{/if}
    <MarkerLegend />
  </div>
</div>

<style>
  .line {
    display: grid;
    gap: 10px;
    padding: 0 16px 12px;
  }
  .text {
    display: grid;
    justify-items: center;
    gap: 4px;
    text-align: center;
    color: var(--color-text-2);
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  .pager {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    font-size: var(--type-small-size);
    color: var(--color-text-2);
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .pager span {
    padding: 0 6px;
  }
  .pager strong {
    color: var(--color-text-1);
  }
</style>

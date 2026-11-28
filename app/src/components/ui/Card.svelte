<!-- Surface card (radius 12, hairline): setups overview, library, settings sections. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children, selected = false, onclick, padding = 16, label }: { children: Snippet; selected?: boolean; onclick?: () => void; padding?: number; label?: string } = $props();
</script>

{#if onclick}
  <button type="button" class="card interactive" class:selected style="padding:{padding}px" aria-label={label} aria-pressed={selected} {onclick}>{@render children()}</button>
{:else}
  <div class="card" class:selected style="padding:{padding}px" role={label ? 'group' : undefined} aria-label={label}>{@render children()}</div>
{/if}

<style>
  .card {
    display: grid;
    gap: 10px;
    text-align: left;
    color: var(--color-text-1);
    background: var(--color-surface);
    border: 0;
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-hairline);
  }
  .interactive {
    cursor: pointer;
    transition: box-shadow var(--motion-fast);
  }
  .interactive:hover {
    box-shadow: 0 0 0 var(--panel-border-width) var(--color-border-2);
  }
  .selected {
    box-shadow: 0 0 0 2px var(--color-accent);
  }
</style>

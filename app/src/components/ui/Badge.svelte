<!-- Count / index badge: "+1", "2 of 4", rule numbers. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let { tone = 'neutral', children }: { tone?: 'neutral' | 'accent' | 'warning' | 'danger'; children: Snippet } = $props();
</script>

<span class="badge {tone}">{@render children()}</span>

<style>
  .badge {
    display: inline-grid;
    place-items: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    font: 600 var(--type-micro-size) / 1 var(--type-mono-family);
    color: var(--color-text-2);
    background: var(--chip-neutral-bg);
    border-radius: var(--radius-pill);
  }
  .accent {
    color: var(--color-on-accent);
    background: var(--color-accent-fill);
  }
  .warning {
    color: var(--chip-warning-fg);
    background: var(--chip-warning-bg);
  }
  .danger {
    color: var(--chip-danger-fg);
    background: var(--chip-danger-bg);
  }
</style>

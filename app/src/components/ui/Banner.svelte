<!-- Inline full-width notice (top of the canvas: not live, integration running): icon + text + actions. -->
<script lang="ts">
  import { CircleAlert, Info, Plug, TriangleAlert } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import type { Icon } from './icon';

  let { kind = 'info', icon, children, actions }: { kind?: 'info' | 'accent' | 'warning' | 'danger' | 'claim'; icon?: Icon; children: Snippet; actions?: Snippet } = $props();
  const Ic = $derived(icon ?? { info: Info, accent: Info, warning: TriangleAlert, danger: CircleAlert, claim: Plug }[kind]);
</script>

<div class="banner {kind}" role={kind === 'danger' ? 'alert' : 'status'}>
  <span class="icon"><Ic size={16} /></span>
  <div class="text">{@render children()}</div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</div>

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px 8px 12px;
    color: var(--color-text-1);
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-lg);
  }
  .icon {
    display: inline-flex;
    flex: none;
    color: var(--color-text-2);
  }
  .accent {
    border-color: var(--color-accent);
  }
  .accent .icon {
    color: var(--color-accent);
  }
  .warning {
    border-color: var(--color-warning);
  }
  .warning .icon {
    color: var(--color-warning);
  }
  .danger {
    border-color: var(--color-danger);
  }
  .danger .icon {
    color: var(--color-danger);
  }
  .claim {
    border: 2px dashed var(--color-claim);
  }
  .claim .icon {
    color: var(--color-claim);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .actions {
    display: flex;
    gap: 6px;
    flex: none;
  }
</style>

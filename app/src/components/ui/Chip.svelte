<!--
  Chip: always icon + text. neutral (filled), outline, glob (mono, app icon), linked (link icon),
  integration (plug), accent, and status success / warning / danger / off.
-->
<script lang="ts">
  import { AppWindow, CircleAlert, CircleCheck, CircleOff, Link2, Plug, TriangleAlert } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import type { Icon } from './icon';

  type Variant = 'neutral' | 'outline' | 'glob' | 'linked' | 'integration' | 'accent' | 'success' | 'warning' | 'danger' | 'off';
  let { variant = 'neutral', icon, title, children }: { variant?: Variant; icon?: Icon | null; title?: string; children: Snippet } = $props();

  const DEFAULT: Partial<Record<Variant, Icon>> = {
    glob: AppWindow,
    linked: Link2,
    integration: Plug,
    success: CircleCheck,
    warning: TriangleAlert,
    danger: CircleAlert,
    off: CircleOff,
  };
  const Ic = $derived(icon === null ? undefined : (icon ?? DEFAULT[variant]));
</script>

<span class="chip {variant}" {title}>
  {#if Ic}<Ic size={12} />{/if}
  <span class="text">{@render children()}</span>
</span>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: var(--size-chip);
    max-width: 100%;
    padding: 0 8px;
    font: var(--type-chip-weight) var(--type-chip-size) / 1 var(--type-chip-family);
    white-space: nowrap;
    color: var(--color-text-2);
    background: var(--chip-neutral-bg);
    border: 1px solid transparent;
    border-radius: var(--radius-chip);
  }
  .chip :global(svg) {
    flex: none;
  }
  .text {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .outline,
  .linked {
    background: transparent;
    border: var(--chip-outline-border);
  }
  .glob {
    font-family: var(--type-mono-family);
    background: var(--color-sunken);
    border: 1px solid var(--color-border);
    color: var(--color-text-1);
  }
  .integration,
  .accent {
    background: var(--chip-accent-bg);
    color: var(--chip-accent-fg);
  }
  .success {
    background: var(--chip-success-bg);
    color: var(--chip-success-fg);
    border: var(--chip-success-border);
  }
  .warning {
    background: var(--chip-warning-bg);
    color: var(--chip-warning-fg);
    border: var(--chip-warning-border);
  }
  .danger {
    background: var(--chip-danger-bg);
    color: var(--chip-danger-fg);
    border: var(--chip-danger-border);
  }
</style>

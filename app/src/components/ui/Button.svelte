<!-- Button: primary / secondary / ghost / danger, md (control height) or sm. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import type { Icon } from './icon';

  let {
    variant = 'secondary',
    size = 'md',
    icon: IconC,
    iconRight: IconR,
    full = false,
    children,
    ...rest
  }: HTMLButtonAttributes & {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    size?: 'md' | 'sm';
    icon?: Icon;
    iconRight?: Icon;
    full?: boolean;
    children?: Snippet;
  } = $props();
</script>

<button type="button" class="btn-{variant} {size}" class:full {...rest}>
  {#if IconC}<IconC size={size === 'sm' ? 14 : 15} />{/if}
  {@render children?.()}
  {#if IconR}<IconR size={size === 'sm' ? 14 : 15} />{/if}
</button>

<style>
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: var(--size-control);
    padding: 0 12px;
    font-family: var(--type-body-family);
    font-size: var(--type-body-size);
    font-weight: var(--button-weight);
    white-space: nowrap;
    border-radius: var(--radius-control);
    border: 1px solid transparent;
    cursor: pointer;
    transition: background var(--motion-fast), color var(--motion-fast), filter var(--motion-fast);
  }
  button.sm {
    height: var(--size-control-sm);
    padding: 0 10px;
  }
  button.full {
    width: 100%;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  button :global(svg) {
    flex-shrink: 0;
  }
  .btn-primary {
    background: var(--color-accent-fill);
    color: var(--color-on-accent);
  }
  .btn-primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }
  .btn-secondary {
    background: var(--button-secondary-bg);
    color: var(--color-text-1);
    border: var(--button-secondary-border);
  }
  .btn-secondary:hover:not(:disabled) {
    background: var(--color-selection);
  }
  .btn-ghost {
    background: transparent;
    color: var(--color-text-2);
  }
  .btn-ghost:hover:not(:disabled) {
    background: var(--color-selection);
    color: var(--color-text-1);
  }
  .btn-danger {
    background: var(--button-danger-bg);
    color: var(--button-danger-fg);
    border: var(--button-danger-border);
  }
  .btn-danger:hover:not(:disabled) {
    filter: brightness(1.1);
  }
</style>

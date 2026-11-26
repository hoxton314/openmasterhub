<!-- Square icon-only button; `label` is the accessible name and tooltip. -->
<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import type { Icon } from './icon';

  let {
    icon: IconC,
    label,
    size = 'md',
    variant = 'ghost',
    active = false,
    ...rest
  }: HTMLButtonAttributes & { icon: Icon; label: string; size?: 'sm' | 'md'; variant?: 'ghost' | 'secondary'; active?: boolean } = $props();
</script>

<button type="button" class="{variant} {size}" class:active aria-label={label} title={label} {...rest}>
  <IconC size={size === 'sm' ? 14 : 16} />
</button>

<style>
  button {
    display: inline-grid;
    place-items: center;
    flex: none;
    width: var(--size-icon-button);
    height: var(--size-icon-button);
    padding: 0;
    color: var(--color-text-2);
    background: transparent;
    border: 0;
    border-radius: var(--radius-row);
    cursor: pointer;
    transition: background var(--motion-fast), color var(--motion-fast);
  }
  button.sm {
    width: 26px;
    height: 26px;
  }
  button.secondary {
    background: var(--button-secondary-bg);
    border: var(--button-secondary-border);
  }
  button:hover:not(:disabled),
  button.active {
    background: var(--color-selection);
    color: var(--color-text-1);
  }
  button:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>

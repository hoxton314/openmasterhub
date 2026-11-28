<!-- Segmented control (aria-pressed buttons). `full` stretches the items (inspector). -->
<script lang="ts" generics="T extends string | number">
  import type { Icon } from './icon';

  let {
    options,
    value,
    onchange,
    label,
    full = false,
    disabled = false,
  }: {
    options: { value: T; label: string; icon?: Icon; disabled?: boolean; title?: string }[];
    value: T | undefined;
    onchange: (v: T) => void;
    label?: string;
    full?: boolean;
    disabled?: boolean;
  } = $props();
</script>

<div class="track" class:full role="group" aria-label={label}>
  {#each options as o (o.value)}
    <button type="button" aria-pressed={o.value === value} disabled={disabled || o.disabled} title={o.title} onclick={() => onchange(o.value)}>
      {#if o.icon}<o.icon size={13} />{/if}{o.label}
    </button>
  {/each}
</div>

<style>
  .track {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    background: var(--color-sunken);
    border: var(--field-border);
    border-radius: var(--radius-control);
  }
  .full {
    display: flex;
    width: 100%;
  }
  .full button {
    flex: 1;
  }
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    height: var(--size-segment);
    padding: 0 10px;
    font-family: var(--type-small-family);
    font-size: var(--type-small-size);
    font-weight: var(--segment-idle-weight);
    white-space: nowrap;
    color: var(--segment-idle-fg);
    background: transparent;
    border: 0;
    border-radius: var(--radius-segment);
    cursor: pointer;
    transition: background var(--motion-fast), color var(--motion-fast);
  }
  button:hover:not(:disabled) {
    color: var(--color-text-1);
  }
  button[aria-pressed='true'] {
    background: var(--segment-selected-bg);
    color: var(--segment-selected-fg);
    box-shadow: var(--segment-selected-shadow);
    font-weight: var(--segment-selected-weight);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>

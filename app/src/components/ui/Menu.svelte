<!-- A keyboard-navigable menu (arrows, Home/End, Enter); put it in a Popover. -->
<script lang="ts" module>
  import type { Icon } from './icon';

  export type MenuEntry =
    | { label: string; icon?: Icon; danger?: boolean; disabled?: boolean; checked?: boolean; shortcut?: string; hint?: string; onselect: () => void }
    | { separator: true }
    | { heading: string };
</script>

<script lang="ts">
  import { Check } from 'lucide-svelte';
  import { prettyKeys } from '../../lib/keys';

  let { items, onclose, label }: { items: MenuEntry[]; onclose?: () => void; label?: string } = $props();
  let root: HTMLElement | undefined = $state();

  const buttons = () => [...(root?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? [])];
  $effect(() => {
    buttons()[0]?.focus();
  });

  function key(e: KeyboardEvent) {
    const all = buttons();
    const i = all.indexOf(document.activeElement as HTMLButtonElement);
    const go = (n: number) => {
      e.preventDefault();
      all[(n + all.length) % all.length]?.focus();
    };
    if (e.key === 'ArrowDown') go(i + 1);
    else if (e.key === 'ArrowUp') go(i - 1);
    else if (e.key === 'Home') go(0);
    else if (e.key === 'End') go(all.length - 1);
    else if (e.key === 'Tab') onclose?.();
  }
</script>

<div class="menu-list" role="menu" aria-label={label} tabindex="-1" bind:this={root} onkeydown={key}>
  {#each items as item, i (i)}
    {#if 'separator' in item}
      <hr />
    {:else if 'heading' in item}
      <div class="heading t-label">{item.heading}</div>
    {:else}
      <button
        type="button"
        role={item.checked === undefined ? 'menuitem' : 'menuitemcheckbox'}
        aria-checked={item.checked}
        class:danger={item.danger}
        disabled={item.disabled}
        onclick={() => {
          onclose?.();
          item.onselect();
        }}
      >
        {#if item.checked !== undefined}<span class="check">{#if item.checked}<Check size={14} />{/if}</span>{:else if item.icon}<item.icon size={14} />{/if}
        <span class="label">{item.label}{#if item.hint}<span class="hint">{item.hint}</span>{/if}</span>
        {#if item.shortcut}<span class="shortcut">{prettyKeys(item.shortcut).join(' ')}</span>{/if}
      </button>
    {/if}
  {/each}
</div>

<style>
  .menu-list {
    display: grid;
    min-width: 180px;
    outline: none;
  }
  button {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    padding: 5px 8px;
    font-family: var(--type-body-family);
    font-size: var(--type-body-size);
    text-align: left;
    color: var(--color-text-1);
    background: none;
    border: 0;
    border-radius: var(--radius-segment);
    cursor: pointer;
  }
  button :global(svg) {
    flex: none;
    color: var(--color-text-2);
  }
  button:hover:not(:disabled),
  button:focus-visible {
    background: var(--color-selection);
    outline: none;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .danger,
  .danger :global(svg) {
    color: var(--color-danger);
  }
  .check {
    display: inline-grid;
    width: 14px;
    color: var(--color-accent);
  }
  .label {
    flex: 1;
    display: grid;
  }
  .hint {
    font-size: var(--type-small-size);
    color: var(--color-text-2);
  }
  .shortcut {
    font: var(--type-mono-weight) var(--type-chip-size) var(--type-mono-family);
    color: var(--color-text-3);
  }
  .heading {
    padding: 8px 8px 4px;
  }
  hr {
    border: 0;
    border-top: 1px solid var(--color-border);
    margin: 4px 0;
  }
</style>

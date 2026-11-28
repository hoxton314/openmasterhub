<!-- A page or folder under the selected profile: icon, name, trailing number / "folder". -->
<script lang="ts">
  import { Folder, LayoutGrid, Plus } from 'lucide-svelte';
  import type { Snippet } from 'svelte';

  let {
    name,
    kind = 'page',
    selected = false,
    meta,
    onclick,
    trailing,
  }: { name: string; kind?: 'page' | 'folder' | 'add'; selected?: boolean; meta?: string | number; onclick?: () => void; trailing?: Snippet } = $props();
  const Ic = $derived(kind === 'folder' ? Folder : kind === 'add' ? Plus : LayoutGrid);
</script>

<div class="row {kind}" class:selected>
  <button type="button" aria-current={selected ? 'page' : undefined} {onclick}>
    <span class="icon"><Ic size={14} /></span>
    <span class="name">{name}</span>
    {#if meta !== undefined}<span class="meta">{meta}</span>{/if}
  </button>
  {#if trailing}{@render trailing()}{/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    border-radius: var(--radius-row);
    color: var(--color-text-2);
  }
  .row:hover:not(.selected) {
    background: var(--color-selection);
    color: var(--color-text-1);
  }
  .selected {
    background: var(--row-selected-bg);
    color: var(--row-selected-fg);
  }
  button {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 7px;
    height: var(--size-row-tree);
    padding: 0 8px 0 12px;
    font: var(--type-tree-weight) var(--type-tree-size) / 1 var(--type-tree-family);
    text-align: left;
    color: inherit;
    background: none;
    border: 0;
    border-radius: inherit;
    cursor: pointer;
  }
  .selected button {
    font-weight: 600;
  }
  .icon {
    display: inline-flex;
    color: var(--color-text-3);
  }
  .selected .icon {
    color: var(--row-selected-icon);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    font-size: var(--type-label-size);
    font-weight: 400;
    color: var(--color-text-3);
  }
  .selected .meta {
    color: var(--row-selected-meta);
  }
  .add button {
    height: 26px;
    font-size: var(--type-small-size);
    color: var(--color-text-3);
  }
</style>

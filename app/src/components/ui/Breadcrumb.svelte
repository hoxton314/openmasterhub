<!-- setup › profile › page, with a live dot on what is on the hub. Items with onclick are buttons. -->
<script lang="ts" module>
  import type { Icon } from './icon';

  export interface Crumb {
    label: string;
    icon?: Icon;
    live?: boolean;
    /** Shows a chevron: the item opens a menu. */
    menu?: boolean;
    title?: string;
    onclick?: (e: MouseEvent) => void;
  }
</script>

<script lang="ts">
  import { ChevronDown, ChevronRight } from 'lucide-svelte';
  import StatusDot from './StatusDot.svelte';

  let { items }: { items: Crumb[] } = $props();
</script>

<nav aria-label="Breadcrumb">
  <ol>
    {#each items as c, i}
      {#if i > 0}<li class="sep" aria-hidden="true"><ChevronRight size={13} /></li>{/if}
      <li>
        {#if c.onclick}
          <button type="button" class:last={i === items.length - 1} title={c.title} aria-haspopup={c.menu ? 'menu' : undefined} onclick={c.onclick}>
            {#if c.icon}<c.icon size={13} />{/if}{c.label}{#if c.live}<StatusDot size={6} label="Live on the hub" />{/if}{#if c.menu}<ChevronDown size={13} class="chev" />{/if}
          </button>
        {:else}
          <span class="item" class:last={i === items.length - 1} title={c.title}>
            {#if c.icon}<c.icon size={13} />{/if}{c.label}{#if c.live}<StatusDot size={6} label="Live on the hub" />{/if}
          </span>
        {/if}
      </li>
    {/each}
  </ol>
</nav>

<style>
  ol {
    display: flex;
    align-items: center;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
    min-width: 0;
  }
  li {
    display: inline-flex;
    min-width: 0;
  }
  .sep {
    color: var(--color-text-3);
  }
  button,
  .item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 6px;
    font-weight: 500;
    white-space: nowrap;
    color: var(--color-text-2);
    background: none;
    border: 0;
    border-radius: var(--radius-row);
  }
  button {
    cursor: pointer;
  }
  button:hover {
    background: var(--color-selection);
    color: var(--color-text-1);
  }
  .last {
    color: var(--color-text-1);
    font-weight: 600;
  }
  button :global(.chev) {
    color: var(--color-text-3);
  }
</style>

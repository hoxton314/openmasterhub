<!-- The library's filter column: search, then kind, needed module, category and state as checkboxes (S22). -->
<script lang="ts" module>
  import type { ModuleKind } from '../../lib/types';

  export type Kind = 'profile' | 'setup' | 'control' | 'integration';
  export interface LibraryFilters {
    kinds: Kind[];
    needs: ModuleKind[];
    categories: string[];
    states: ('installed' | 'available')[];
    query: string;
  }
  export const emptyFilters = (): LibraryFilters => ({ kinds: [], needs: [], categories: [], states: [], query: '' });
  export const anyFilter = (f: LibraryFilters) => f.kinds.length + f.needs.length + f.categories.length + f.states.length > 0 || !!f.query.trim();
</script>

<script lang="ts">
  import { Search } from 'lucide-svelte';
  import Button from '../ui/Button.svelte';

  type Option = { value: string; label: string; count: number };
  let {
    filters,
    kinds,
    needs,
    categories,
    states,
    onchange,
  }: {
    filters: LibraryFilters;
    kinds: Option[];
    needs: Option[];
    categories: Option[];
    states: Option[];
    onchange: (f: LibraryFilters) => void;
  } = $props();

  function toggle<K extends keyof LibraryFilters>(key: K, value: string) {
    const cur = filters[key] as string[];
    const next = cur.includes(value) ? cur.filter((v) => v !== value) : [...cur, value];
    onchange({ ...filters, [key]: next });
  }

  const groups = $derived([
    { key: 'kinds' as const, title: 'Kind', options: kinds },
    { key: 'needs' as const, title: 'Needs module', options: needs },
    { key: 'categories' as const, title: 'Category', options: categories },
    { key: 'states' as const, title: 'State', options: states },
  ]);
</script>

<nav class="filters" aria-label="Library filters">
  <div class="search">
    <Search size={14} aria-hidden="true" />
    <input type="search" placeholder="Search" aria-label="Search the library" value={filters.query} oninput={(e) => onchange({ ...filters, query: e.currentTarget.value })} />
  </div>
  {#each groups as g (g.key)}
    {#if g.options.length}
      <section aria-label={g.title}>
        <h2 class="t-label faint">{g.title}</h2>
        {#each g.options as o (o.value)}
          {@const on = (filters[g.key] as string[]).includes(o.value)}
          <label class="item" class:on>
            <input type="checkbox" checked={on} onchange={() => toggle(g.key, o.value)} />
            <span class="t-body">{o.label}</span>
            <span class="t-small count">{o.count}</span>
          </label>
        {/each}
      </section>
    {/if}
  {/each}
  {#if anyFilter(filters)}
    <div class="clear"><Button variant="ghost" size="sm" onclick={() => onchange(emptyFilters())}>Clear filters</Button></div>
  {/if}
</nav>

<style>
  .filters {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    color: var(--color-text-3);
  }
  .search :global(svg) {
    position: absolute;
    left: 9px;
    pointer-events: none;
  }
  .search input {
    width: 100%;
    min-width: 0;
    height: var(--size-control);
    padding: 0 10px 0 28px;
    font-family: var(--type-body-family);
    font-size: var(--type-body-size);
    color: var(--color-text-1);
    background: var(--color-sunken);
    border: var(--field-border);
    border-radius: var(--radius-control);
    outline: none;
  }
  .search input:focus-visible {
    outline: var(--focus-width) solid var(--focus-color);
    outline-offset: var(--focus-offset);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  h2 {
    margin: 0;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 26px;
    color: var(--idle-item-fg);
    cursor: pointer;
  }
  .item:hover,
  .item.on {
    color: var(--color-text-1);
  }
  .item input {
    margin: 0;
    accent-color: var(--color-accent-fill);
  }
  .item input:focus-visible {
    outline: var(--focus-width) solid var(--focus-color);
    outline-offset: var(--focus-offset);
  }
  .count {
    margin-left: auto;
    color: var(--color-text-3);
  }
  .clear {
    padding: 0;
  }
</style>

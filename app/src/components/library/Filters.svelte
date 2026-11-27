<!-- The library's filter column: kind, needed module, category and state (S22). -->
<script lang="ts" module>
  import type { ModuleKind } from '../../lib/types';

  export type Kind = 'profile' | 'setup' | 'control' | 'integration';
  export interface LibraryFilters {
    kinds: Kind[];
    needs: ModuleKind[];
    categories: string[];
    states: ('installed' | 'available')[];
  }
  export const emptyFilters = (): LibraryFilters => ({ kinds: [], needs: [], categories: [], states: [] });
  export const anyFilter = (f: LibraryFilters) => f.kinds.length + f.needs.length + f.categories.length + f.states.length > 0;
</script>

<script lang="ts">
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
  {#each groups as g (g.key)}
    {#if g.options.length}
      <section aria-label={g.title}>
        <h2 class="t-label faint">{g.title}</h2>
        {#each g.options as o (o.value)}
          {@const on = (filters[g.key] as string[]).includes(o.value)}
          <button class="item" class:on aria-pressed={on} onclick={() => toggle(g.key, o.value)}>
            <span class="t-body">{o.label}</span>
            <span class="t-small count">{o.count}</span>
          </button>
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
    padding: 16px 10px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  h2 {
    margin: 0 0 4px;
    padding: 0 8px;
  }
  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    min-height: 30px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius-sidebar-row);
    background: none;
    color: var(--idle-item-fg);
    cursor: pointer;
    text-align: left;
  }
  .item:hover {
    background: var(--color-selection);
  }
  .item.on {
    background: var(--active-item-bg);
    color: var(--active-item-fg);
  }
  .count {
    color: var(--color-text-3);
  }
  .item.on .count {
    color: inherit;
  }
  .clear {
    padding: 0 4px;
  }
</style>

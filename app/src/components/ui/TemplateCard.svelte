<!-- Template: mini hub preview (dark hardware, from `preview`), title, description, module chips, warning. -->
<script lang="ts">
  import { TriangleAlert } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import Chip from './Chip.svelte';

  let {
    title,
    description,
    modules = [],
    warning,
    selected = false,
    onclick,
    preview,
  }: { title: string; description?: string; modules?: string[]; warning?: string; selected?: boolean; onclick?: () => void; preview?: Snippet } = $props();
</script>

<button type="button" class="tpl" class:selected aria-pressed={selected} {onclick}>
  <div class="well" aria-hidden="true">{@render preview?.()}</div>
  <b>{title}</b>
  {#if description}<span class="desc">{description}</span>{/if}
  {#if modules.length}<span class="chips">{#each modules as m}<Chip icon={null}>{m}</Chip>{/each}</span>{/if}
  {#if warning}<span class="warn"><TriangleAlert size={13} />{warning}</span>{/if}
</button>

<style>
  .tpl {
    display: grid;
    align-content: start;
    gap: 8px;
    padding: 10px;
    text-align: left;
    color: var(--color-text-1);
    background: var(--color-surface);
    border: 2px solid var(--color-border);
    border-radius: calc(var(--radius-card) - 1px);
    cursor: pointer;
  }
  .tpl:hover {
    border-color: var(--color-border-2);
  }
  .selected,
  .selected:hover {
    border-color: var(--color-accent);
  }
  .well {
    display: grid;
    place-items: center;
    height: 128px;
    overflow: hidden;
    background: var(--color-canvas);
    border-radius: var(--radius-control);
  }
  b {
    font-weight: 600;
  }
  .desc {
    font-size: var(--type-small-size);
    color: var(--color-text-2);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .warn {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--type-chip-size);
    color: var(--color-warning);
  }
</style>

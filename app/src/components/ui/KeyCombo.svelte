<!-- A combo as caps joined by "+": "ctrl+s" → [Ctrl] + [S]. -->
<script lang="ts">
  import { prettyKeys } from '../../lib/keys';
  import Kbd from './Kbd.svelte';

  let { combo, empty = 'Nothing' }: { combo: string | undefined; empty?: string } = $props();
  const caps = $derived(combo ? prettyKeys(combo) : []);
</script>

<span class="combo">
  {#each caps as cap, i}
    {#if i > 0}<span class="plus">+</span>{/if}<Kbd>{cap}</Kbd>
  {:else}
    <span class="empty">{empty}</span>
  {/each}
</span>

<style>
  .combo {
    display: inline-flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }
  .plus,
  .empty {
    font-size: var(--type-small-size);
    color: var(--color-text-3);
  }
</style>

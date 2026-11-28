<!-- Legend under the canvas: what the marker badges mean. -->
<script lang="ts">
  import CanvasMarker, { type MarkerKind } from './CanvasMarker.svelte';

  let { kinds = ['pinned', 'integration', 'inherited'] }: { kinds?: MarkerKind[] } = $props();
  const TEXT: Record<MarkerKind, string> = { pinned: 'Pinned everywhere', integration: 'Integration', inherited: 'Inherited from base' };
</script>

<ul class="legend" aria-label="Marker legend">
  {#each kinds as k}
    <li><CanvasMarker kind={k} size={16} inline />{TEXT[k]}</li>
  {/each}
</ul>

<style>
  .legend {
    display: flex;
    gap: 16px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--type-chip-size);
    color: var(--color-text-2);
  }
</style>

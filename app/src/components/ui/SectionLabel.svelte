<!-- Caps section label (11/600 text3; Signal: Archivo 12/800 white), optional trailing action. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let { children, action, as = 'div' }: { children: Snippet; action?: Snippet; as?: 'div' | 'h2' | 'h3' } = $props();
</script>

<div class="row">
  <svelte:element this={as} class="t-label">{@render children()}</svelte:element>
  {@render action?.()}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-height: 26px;
  }
</style>

<!-- Integration claim: 2 px dashed outline around a module + a tag outside the hardware. -->
<script lang="ts">
  import { Plug } from 'lucide-svelte';

  let { label, inset = -6 }: { label: string; inset?: number } = $props();
</script>

<span class="claim" style="--i:{inset}px" aria-hidden="true">
  <span class="tag"><Plug size={11} />{label}</span>
</span>

<style>
  .claim {
    position: absolute;
    inset: var(--i);
    z-index: 2;
    border: 2px dashed var(--color-claim);
    border-radius: calc(var(--hub-chassis-radius) + 4px);
    pointer-events: none;
  }
  .tag {
    position: absolute;
    left: -2px;
    bottom: calc(100% + 4px);
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 18px;
    padding: 0 6px;
    font: 600 var(--type-micro-size) / 1 var(--type-micro-family);
    white-space: nowrap;
    color: var(--color-marker-fg);
    background: var(--color-claim);
    border-radius: var(--radius-xs);
  }
</style>

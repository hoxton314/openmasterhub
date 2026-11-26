<!--
  Badge outside a control's screen (never over its pixels): pinned, integration or inherited.
  Place it inside a `position: relative` control. Selected / pressed / applied are the
  `.marker-selected`, `.marker-pressed`, `.marker-applied` classes (app.css); claims use ClaimOutline.
-->
<script lang="ts" module>
  export type MarkerKind = 'pinned' | 'integration' | 'inherited';
</script>

<script lang="ts">
  import { CornerLeftUp, Pin, Plug } from 'lucide-svelte';

  let { kind, title, size = 18, inline = false }: { kind: MarkerKind; title?: string; size?: number; inline?: boolean } = $props();
  const Ic = $derived({ pinned: Pin, integration: Plug, inherited: CornerLeftUp }[kind]);
  const label = $derived(title ?? { pinned: 'Pinned everywhere', integration: 'Integration', inherited: 'Inherited from base' }[kind]);
</script>

<span class="marker" class:inline style="--m:{size}px" title={label} aria-label={label} role="img">
  <Ic size={Math.round(size * 0.6)} strokeWidth={2.5} />
</span>

<style>
  .marker {
    position: absolute;
    top: calc(var(--m) / -2);
    right: calc(var(--m) / -2);
    z-index: 3;
    display: grid;
    place-items: center;
    width: var(--m);
    height: var(--m);
    border-radius: 50%;
    color: var(--color-marker-fg);
    background: var(--color-marker-bg);
    box-shadow: 0 0 0 2px var(--color-marker-fg);
    pointer-events: auto;
  }
  .inline {
    position: relative;
    top: auto;
    right: auto;
  }
</style>

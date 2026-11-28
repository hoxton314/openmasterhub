<!-- On-device preview at 1:1 (72×72 key, 240 round dial), pixelated, in the device frame. -->
<script lang="ts">
  let { src, label, dial = false, size, alt = 'On-device preview' }: { src?: string; label?: string; dial?: boolean; size?: number; alt?: string } = $props();
  const px = $derived(size ?? (dial ? 240 : 72));
</script>

<figure class="face" class:dial style="--s:{px}px">
  {#if src}<img {src} {alt} width={px} height={px} draggable="false" />{:else}<span class="blank"></span>{/if}
  {#if label}<figcaption>{label}</figcaption>{/if}
</figure>

<style>
  .face {
    display: inline-grid;
    justify-items: center;
    gap: 6px;
    margin: 0;
  }
  img,
  .blank {
    display: block;
    width: var(--s);
    height: var(--s);
    image-rendering: pixelated;
    border-radius: var(--hub-key-radius);
    background: var(--hub-empty);
    box-shadow: var(--hub-preview-frame);
  }
  .dial img,
  .dial .blank {
    border-radius: 50%;
    image-rendering: auto;
  }
  figcaption {
    font-size: var(--type-chip-size);
    color: var(--color-text-3);
  }
</style>

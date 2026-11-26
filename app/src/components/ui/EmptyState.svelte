<!-- Empty state: icon, bold title, text2 body, one primary action. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Button from './Button.svelte';
  import type { Icon } from './icon';

  let {
    icon: Ic,
    title,
    body,
    actionLabel,
    onaction,
    children,
  }: { icon?: Icon; title: string; body?: string; actionLabel?: string; onaction?: () => void; children?: Snippet } = $props();
</script>

<div class="empty">
  {#if Ic}<span class="icon"><Ic size={22} /></span>{/if}
  <b>{title}</b>
  {#if body}<p>{body}</p>{/if}
  {@render children?.()}
  {#if actionLabel && onaction}<Button variant="primary" size="sm" onclick={onaction}>{actionLabel}</Button>{/if}
</div>

<style>
  .empty {
    display: grid;
    justify-items: center;
    gap: 8px;
    padding: 24px;
    text-align: center;
    border: 1.5px dashed var(--color-border-2);
    border-radius: var(--radius-well);
  }
  .icon {
    display: inline-flex;
    color: var(--color-text-3);
  }
  p {
    max-width: 360px;
    font-size: var(--type-small-size);
    color: var(--color-text-2);
  }
</style>

<!-- A check list row (first run, system checks): title, detail, status chip, optional action. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Chip from './Chip.svelte';

  let { title, detail, status, statusLabel, action }: { title: string; detail?: string; status: 'success' | 'warning' | 'danger' | 'off'; statusLabel: string; action?: Snippet } = $props();
</script>

<div class="check">
  <div class="text">
    <b>{title}</b>
    {#if detail}<span>{detail}</span>{/if}
  </div>
  <Chip variant={status}>{statusLabel}</Chip>
  {@render action?.()}
</div>

<style>
  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 0;
    border-bottom: 1px solid var(--color-border);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 2px;
  }
  b {
    font-weight: 600;
  }
  span {
    font-size: var(--type-small-size);
    color: var(--color-text-2);
  }
</style>

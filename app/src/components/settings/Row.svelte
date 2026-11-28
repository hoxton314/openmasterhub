<!-- A labelled setting: text on the left, the control on the right. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let { label, help, children }: { label: string; help?: string; children: Snippet } = $props();
</script>

<div class="row">
  <div class="text">
    <span class="t-body">{label}</span>
    {#if help}<span class="t-small muted">{help}</span>{/if}
  </div>
  <div class="control">{@render children()}</div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-3) 0;
    border-bottom: 1px solid var(--color-border);
  }
  .text {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .control {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
</style>

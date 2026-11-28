<!-- Title, one line of context and the area's actions (Setups overview and detail). -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let { title, subtitle, actions }: { title: string; subtitle?: string; actions?: Snippet } = $props();
</script>

<header>
  <div class="titles">
    <h1 class="t-title">{title}</h1>
    {#if subtitle}<p class="muted">{subtitle}</p>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 20px 32px 16px;
    border-bottom: var(--panel-border-width) solid var(--color-border);
  }
  .titles {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }
</style>

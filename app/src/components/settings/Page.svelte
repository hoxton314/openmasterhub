<!-- A settings page: heading, one-line description and the page's groups. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let { title, about, children }: { title: string; about?: string; children: Snippet } = $props();
</script>

<div class="page">
  <header>
    <h2 class="t-title">{title}</h2>
    {#if about}<p class="t-small muted">{about}</p>{/if}
  </header>
  {@render children()}
</div>

<style>
  .page {
    display: grid;
    align-content: start;
    gap: var(--space-6);
    max-width: 720px;
  }
  header {
    display: grid;
    gap: 2px;
  }
  p {
    margin: 0;
  }
</style>

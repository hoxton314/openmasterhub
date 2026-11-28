<!-- An inspector block: padding 14×16, gap 10, hairline below; header with label + action. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import SectionLabel from './SectionLabel.svelte';

  let { title, action, children, divider = true }: { title?: string; action?: Snippet; children: Snippet; divider?: boolean } = $props();
</script>

<section class:divider>
  {#if title}<SectionLabel as="h3" {action}>{title}</SectionLabel>{/if}
  {@render children()}
</section>

<style>
  section {
    display: grid;
    gap: 10px;
    padding: 14px var(--size-panel-pad);
  }
  .divider {
    border-bottom: var(--panel-border-width) solid var(--color-border);
  }
</style>

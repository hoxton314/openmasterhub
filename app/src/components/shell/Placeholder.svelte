<!-- Stand-in for an area or view a later work package builds. -->
<script lang="ts">
  import { Construction } from 'lucide-svelte';
  import EmptyState from '../ui/EmptyState.svelte';

  let { title, body }: { title: string; body?: string } = $props();
</script>

<div class="placeholder">
  <EmptyState icon={Construction} {title} body={body ?? 'This part of the redesign is not built yet.'} />
</div>

<style>
  .placeholder {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 32px;
    overflow: auto;
  }
</style>

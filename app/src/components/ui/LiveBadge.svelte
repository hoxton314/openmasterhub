<!-- "● LIVE": what is on the hub right now. A selected row sets --live-color to its own colours. -->
<script lang="ts">
  import StatusDot from './StatusDot.svelte';

  let { label = 'LIVE', title = 'Live on the hub' }: { label?: string; title?: string } = $props();
</script>

<span class="live" aria-label={title} {title}><StatusDot size={6} />{label}</span>

<style>
  .live {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex: none;
    font: 700 var(--type-micro-size) / 1 var(--type-micro-family);
    letter-spacing: 0.02em;
    color: var(--live-color);
  }
</style>

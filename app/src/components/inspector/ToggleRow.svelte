<!-- A labelled switch row, optionally disabled with a reason (shown as the tooltip). -->
<script lang="ts">
  import Toggle from '../ui/Toggle.svelte';

  let { label, checked, onchange, disabled = false, title }: { label: string; checked: boolean; onchange: (v: boolean) => void; disabled?: boolean; title?: string } = $props();
</script>

<div class="row" class:disabled>
  <span class="t-body">{label}</span>
  <Toggle {checked} title={title ?? label} {disabled} {onchange} />
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .row.disabled {
    color: var(--color-text-3);
  }
</style>

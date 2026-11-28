<!-- Switch (role=switch). With `label` it renders as a labelled row ("Follow live"). -->
<script lang="ts">
  let { checked, onchange, label, disabled = false, title }: { checked: boolean; onchange: (v: boolean) => void; label?: string; disabled?: boolean; title?: string } = $props();
</script>

{#if label}
  <label class="row" class:disabled {title}>
    <button type="button" role="switch" aria-checked={checked} aria-label={label} {disabled} onclick={() => onchange(!checked)}><span></span></button>
    <span class="text">{label}</span>
  </label>
{:else}
  <button type="button" role="switch" aria-checked={checked} aria-label={title} {title} {disabled} onclick={() => onchange(!checked)}><span></span></button>
{/if}

<style>
  .row {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--color-text-2);
    cursor: pointer;
  }
  .row.disabled {
    opacity: 0.5;
  }
  .text {
    font-size: var(--type-small-size);
    font-weight: 500;
    white-space: nowrap;
  }
  button {
    position: relative;
    flex: none;
    width: var(--size-toggle-width);
    height: var(--size-toggle-height);
    padding: 0;
    background: var(--toggle-off-bg);
    border: 2px solid var(--color-border-2);
    border-radius: var(--radius-pill);
    cursor: pointer;
    transition: background var(--motion-fast), border-color var(--motion-fast);
  }
  span:empty {
    position: absolute;
    top: 1px;
    left: 1px;
    width: calc(var(--size-toggle-height) - 6px);
    height: calc(var(--size-toggle-height) - 6px);
    border-radius: var(--radius-pill);
    background: var(--toggle-off-knob);
    transition: transform var(--motion-fast);
  }
  button[aria-checked='true'] {
    background: var(--color-accent-fill);
    border-color: var(--color-accent-fill);
  }
  button[aria-checked='true'] span:empty {
    background: var(--toggle-on-knob);
    transform: translateX(calc(var(--size-toggle-width) - var(--size-toggle-height)));
  }
  button:disabled {
    cursor: default;
  }
</style>

<!-- Label above (12/500 text2), control, help or error below. Wraps TextField, Select, ColorField... -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  let { label, help, error, id, inline = false, children, action }: { label?: string; help?: string; error?: string; id?: string | null; inline?: boolean; children: Snippet; action?: Snippet } = $props();
</script>

<div class="ui-field" class:inline>
  {#if label || action}
    <div class="head">
      {#if label}<label for={id}>{label}</label>{/if}
      {@render action?.()}
    </div>
  {/if}
  {@render children()}
  {#if error}<p class="error" role="alert">{error}</p>{:else if help}<p class="help">{help}</p>{/if}
</div>

<style>
  .ui-field {
    display: grid;
    gap: 5px;
    min-width: 0;
  }
  .inline {
    grid-template-columns: minmax(80px, 34%) minmax(0, 1fr);
    align-items: center;
    column-gap: 10px;
  }
  .inline .help,
  .inline .error {
    grid-column: 2;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  label {
    font-size: var(--type-small-size);
    font-weight: 500;
    color: var(--color-text-2);
  }
  .help,
  .error {
    font-size: var(--type-chip-size);
    color: var(--color-text-2);
  }
  .error {
    color: var(--color-danger);
  }
</style>

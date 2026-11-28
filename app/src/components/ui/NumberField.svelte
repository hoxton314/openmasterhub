<!-- Number input with optional unit ("ms", "%", "s"). Emits numbers clamped to min/max. -->
<script lang="ts">
  import Field from './Field.svelte';

  let {
    value,
    onchange,
    min,
    max,
    step = 1,
    unit,
    label,
    help,
    disabled = false,
    placeholder,
    id = `num-${Math.random().toString(36).slice(2, 8)}`,
  }: {
    value: number | undefined;
    onchange: (v: number | undefined) => void;
    min?: number;
    max?: number;
    step?: number;
    unit?: string;
    label?: string;
    help?: string;
    disabled?: boolean;
    placeholder?: string;
    id?: string;
  } = $props();

  function change(e: Event) {
    const raw = (e.currentTarget as HTMLInputElement).value;
    if (raw === '') return onchange(undefined);
    let n = Number(raw);
    if (!Number.isFinite(n)) return;
    if (min !== undefined) n = Math.max(min, n);
    if (max !== undefined) n = Math.min(max, n);
    onchange(n);
  }
</script>

{#snippet control()}
  <span class="wrap">
    <input {id} type="number" {min} {max} {step} {disabled} {placeholder} value={value ?? ''} onchange={change} />
    {#if unit}<span class="unit">{unit}</span>{/if}
  </span>
{/snippet}

{#if label || help}
  <Field {label} {help} {id}>{@render control()}</Field>
{:else}
  {@render control()}
{/if}

<style>
  .wrap {
    display: flex;
    align-items: center;
    width: 100%;
    min-width: 0;
    height: var(--size-control);
    background: var(--color-sunken);
    border: var(--field-border);
    border-radius: var(--radius-control);
  }
  .wrap:focus-within {
    border-color: var(--color-accent);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0 10px;
    font-family: var(--type-mono-family);
    font-size: var(--type-mono-size);
    color: var(--color-text-1);
    background: transparent;
    border: 0;
    outline: none;
    user-select: text;
  }
  input::-webkit-inner-spin-button {
    opacity: 0.4;
  }
  .unit {
    flex: none;
    padding-right: 10px;
    font-size: var(--type-small-size);
    color: var(--color-text-3);
  }
</style>

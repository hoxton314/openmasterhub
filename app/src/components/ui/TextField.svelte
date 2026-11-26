<!-- Text input (sunken, control height). `mono` for globs, commands and shortcuts. -->
<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  import Field from './Field.svelte';

  let {
    value = $bindable(''),
    label,
    help,
    error,
    mono = false,
    size = 'md',
    id = `tf-${Math.random().toString(36).slice(2, 8)}`,
    ...rest
  }: Omit<HTMLInputAttributes, 'size'> & { value?: string; label?: string; help?: string; error?: string; mono?: boolean; size?: 'md' | 'sm' } = $props();
</script>

{#if label || help || error}
  <Field {label} {help} {error} {id}>
    <input {id} class={size} class:mono class:invalid={!!error} bind:value aria-invalid={!!error} {...rest} />
  </Field>
{:else}
  <input {id} class={size} class:mono class:invalid={!!error} bind:value {...rest} />
{/if}

<style>
  input {
    width: 100%;
    min-width: 0;
    height: var(--size-control);
    padding: 0 10px;
    font-family: var(--type-body-family);
    font-size: var(--type-body-size);
    color: var(--color-text-1);
    background: var(--color-sunken);
    border: var(--field-border);
    border-radius: var(--radius-control);
    outline: none;
    user-select: text;
    transition: border-color var(--motion-fast);
  }
  input.sm {
    height: var(--size-control-sm);
  }
  input.mono {
    font-family: var(--type-mono-family);
    font-size: var(--type-mono-size);
  }
  input::placeholder {
    color: var(--color-text-3);
  }
  input:hover {
    border-color: var(--color-border-2);
  }
  input:focus {
    border-color: var(--color-accent);
  }
  input.invalid {
    border-color: var(--color-danger);
  }
  input:disabled {
    opacity: 0.55;
  }
</style>

<!-- Native select styled as a field. -->
<script lang="ts" generics="T extends string | number">
  import { ChevronDown } from 'lucide-svelte';
  import Field from './Field.svelte';

  let {
    value,
    options,
    onchange,
    label,
    help,
    disabled = false,
    size = 'md',
    id = `sel-${Math.random().toString(36).slice(2, 8)}`,
  }: {
    value: T | undefined;
    options: { value: T; label: string; disabled?: boolean; group?: string }[];
    onchange: (v: T) => void;
    label?: string;
    help?: string;
    disabled?: boolean;
    size?: 'md' | 'sm';
    id?: string;
  } = $props();

  const groups = $derived([...new Set(options.map((o) => o.group))]);
  function change(e: Event) {
    const i = (e.currentTarget as HTMLSelectElement).selectedIndex;
    const flat = groups.flatMap((g) => options.filter((o) => o.group === g));
    if (flat[i]) onchange(flat[i].value);
  }
</script>

{#snippet control()}
  <span class="wrap {size}">
    <select {id} {disabled} onchange={change}>
      {#each groups as g}
        {#if g}
          <optgroup label={g}>
            {#each options.filter((o) => o.group === g) as o}<option selected={o.value === value} disabled={o.disabled}>{o.label}</option>{/each}
          </optgroup>
        {:else}
          {#each options.filter((o) => !o.group) as o}<option selected={o.value === value} disabled={o.disabled}>{o.label}</option>{/each}
        {/if}
      {/each}
    </select>
    <ChevronDown size={14} />
  </span>
{/snippet}

{#if label || help}
  <Field {label} {help} {id}>{@render control()}</Field>
{:else}
  {@render control()}
{/if}

<style>
  .wrap {
    position: relative;
    display: flex;
    width: 100%;
    min-width: 0;
  }
  .wrap :global(svg) {
    position: absolute;
    right: 9px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--color-text-2);
    pointer-events: none;
  }
  select {
    appearance: none;
    width: 100%;
    min-width: 0;
    height: var(--size-control);
    padding: 0 30px 0 10px;
    font-family: var(--type-body-family);
    font-size: var(--type-body-size);
    color: var(--color-text-1);
    background: var(--color-sunken);
    border: var(--field-border);
    border-radius: var(--radius-control);
    outline: none;
    cursor: pointer;
  }
  .sm select {
    height: var(--size-control-sm);
  }
  select:hover {
    border-color: var(--color-border-2);
  }
  select:focus-visible {
    border-color: var(--color-accent);
  }
  select:disabled {
    opacity: 0.55;
    cursor: default;
  }
  option,
  optgroup {
    background: var(--color-surface);
    color: var(--color-text-1);
  }
</style>

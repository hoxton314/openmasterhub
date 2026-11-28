<!-- A radio group of choices, each with an optional description; the chosen one can show a detail. -->
<script lang="ts" generics="T extends string">
  import type { Snippet } from 'svelte';

  let {
    options,
    value,
    onchange,
    label,
    detail,
  }: {
    options: { value: T; label: string; desc?: string; disabled?: boolean; title?: string }[];
    value: T | undefined;
    onchange: (v: T) => void;
    label?: string;
    detail?: Snippet<[T]>;
  } = $props();
  const name = `rl-${Math.random().toString(36).slice(2, 8)}`;
</script>

<div class="radios" role="radiogroup" aria-label={label}>
  {#each options as o (o.value)}
    <div class="opt" class:on={value === o.value} class:disabled={o.disabled} title={o.title}>
      <label class="radio">
        <input type="radio" {name} checked={value === o.value} disabled={o.disabled} onchange={() => onchange(o.value)} />
        <span class="t-body">{o.label}</span>
      </label>
      {#if o.desc && value !== o.value}<span class="desc t-small">{o.desc}</span>{/if}
      {#if value === o.value && detail}<div class="detail">{@render detail(o.value)}</div>{/if}
    </div>
  {/each}
</div>

<style>
  .radios {
    display: grid;
    gap: 4px;
  }
  .opt {
    padding: 6px 8px;
    border-radius: var(--radius-control);
  }
  .opt.on {
    background: var(--color-selection);
  }
  .opt.disabled {
    color: var(--color-text-3);
  }
  .radio {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  .disabled .radio {
    cursor: default;
  }
  input {
    margin: 0;
    accent-color: var(--color-accent-fill);
  }
  .desc {
    display: block;
    margin: 2px 0 0 24px;
    color: var(--color-text-3);
  }
  .detail {
    display: grid;
    gap: 8px;
    padding: 8px 0 2px 24px;
  }
</style>

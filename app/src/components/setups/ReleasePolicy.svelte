<!-- Hybrid setups: when the app rules take back over from a pinned profile. -->
<script lang="ts">
  import type { ReleasePolicy } from '../../lib/types';
  import NumberField from '../ui/NumberField.svelte';

  let {
    value,
    minutes,
    onchange,
    onminutes,
  }: { value: ReleasePolicy; minutes: number | undefined; onchange: (v: ReleasePolicy) => void; onminutes: (m: number | undefined) => void } = $props();

  const OPTIONS: { value: ReleasePolicy; title: string; detail: string }[] = [
    { value: 'same_trigger', title: 'When I press the same trigger again', detail: 'A second press of the dial long-press or the shortcut releases the pin' },
    { value: 'timeout', title: 'After a timeout', detail: 'The rules resume by themselves' },
    { value: 'next_app', title: 'At the next app change', detail: 'Like today: the next focus change takes over again' },
  ];
</script>

<div class="group" role="radiogroup" aria-label="Release the pin">
  {#each OPTIONS as o (o.value)}
    <div class="opt" class:on={value === o.value}>
      <button type="button" role="radio" aria-checked={value === o.value} onclick={() => onchange(o.value)}>
        <span class="radio" aria-hidden="true"></span>
        <span class="text"><b>{o.title}</b><span class="detail">{o.detail}</span></span>
      </button>
      {#if o.value === 'timeout' && value === 'timeout'}
        <div class="minutes"><NumberField value={minutes ?? 10} min={1} max={1440} unit="minutes" onchange={onminutes} /></div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .group {
    display: grid;
    gap: 2px;
    padding: 6px;
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-panel);
  }
  .opt {
    border-radius: var(--radius-row);
  }
  .on {
    background: var(--color-selection);
  }
  button {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    width: 100%;
    padding: 9px 10px;
    text-align: left;
    color: var(--color-text-1);
    background: none;
    border: 0;
    border-radius: inherit;
    cursor: pointer;
  }
  .radio {
    flex: none;
    width: 16px;
    height: 16px;
    margin-top: 2px;
    border: 2px solid var(--color-border-2);
    border-radius: 50%;
  }
  [aria-checked='true'] .radio {
    border: 5px solid var(--color-accent-fill);
  }
  .text {
    display: grid;
    gap: 2px;
  }
  .detail {
    font-size: var(--type-chip-size);
    color: var(--color-text-2);
  }
  .minutes {
    padding: 0 10px 10px 36px;
    width: 180px;
  }
</style>

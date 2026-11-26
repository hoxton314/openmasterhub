<!--
  Colour field: swatch + mono hex; opens a popover with swatches, a picker and a hex field.
  The colours themselves are device colours (key faces, LEDs), not theme colours.
-->
<script lang="ts">
  import { RotateCcw } from 'lucide-svelte';
  import { FACE_BACKGROUNDS } from '../../lib/palette';
  import Button from './Button.svelte';
  import Field from './Field.svelte';
  import Popover from './Popover.svelte';

  let {
    value,
    fallback = FACE_BACKGROUNDS[0],
    swatches = FACE_BACKGROUNDS,
    resettable = true,
    label,
    help,
    onchange,
  }: { value: string | undefined; fallback?: string; swatches?: string[]; resettable?: boolean; label?: string; help?: string; onchange: (v: string | undefined) => void } = $props();

  const HEX = /^#[0-9a-f]{6}$/i;
  const current = $derived((value ?? fallback).toLowerCase());
  let open = $state(false);
  let draft = $state('');
  $effect(() => {
    draft = current;
  });
  function commit() {
    const v = draft.startsWith('#') ? draft : `#${draft}`;
    if (HEX.test(v)) onchange(v.toLowerCase());
    else draft = current;
  }
</script>

{#snippet control()}
  <Popover bind:open width={216} label={label ?? 'Colour'}>
    {#snippet trigger({ toggle, attrs })}
      <button type="button" class="field" {...attrs} aria-label="{label ?? 'Colour'}: {current}" onclick={toggle}>
        <span class="swatch" style="--c:{current}"></span>
        <span class="hex">{value === undefined ? 'Default' : current}</span>
      </button>
    {/snippet}
    {#snippet children({ close })}
      <div class="picker">
        <div class="grid">
          {#each swatches as s}
            <button type="button" class="swatch big" class:on={value?.toLowerCase() === s.toLowerCase()} style="--c:{s}" title={s} aria-label={s} onclick={() => onchange(s)}></button>
          {/each}
        </div>
        <div class="row">
          <label class="native swatch big" style="--c:{current}" title="Pick any colour">
            <input type="color" value={current} oninput={(e) => onchange(e.currentTarget.value)} />
          </label>
          <input class="hexin" bind:value={draft} maxlength="7" spellcheck="false" aria-label="Hex colour" onblur={commit} onkeydown={(e) => e.key === 'Enter' && commit()} />
        </div>
        {#if resettable && value !== undefined}
          <Button variant="ghost" size="sm" icon={RotateCcw} onclick={() => { onchange(undefined); close(); }}>Reset to default</Button>
        {/if}
      </div>
    {/snippet}
  </Popover>
{/snippet}

{#if label || help}
  <Field {label} {help}>{@render control()}</Field>
{:else}
  {@render control()}
{/if}

<style>
  .field {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: var(--size-control);
    padding: 0 8px;
    background: var(--color-sunken);
    border: var(--field-border);
    border-radius: var(--radius-control);
    cursor: pointer;
  }
  .field:hover {
    border-color: var(--color-border-2);
  }
  .hex {
    font: var(--type-mono-weight) var(--type-mono-size) var(--type-mono-family);
    color: var(--color-text-1);
  }
  .swatch {
    flex: none;
    width: 16px;
    height: 16px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-xs);
    background: var(--c);
    box-shadow: inset 0 0 0 1px var(--color-border-2);
  }
  .picker {
    display: grid;
    gap: 10px;
    padding: 8px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 6px;
  }
  .big {
    width: 20px;
    height: 20px;
    cursor: pointer;
  }
  .big.on {
    box-shadow: 0 0 0 2px var(--color-raised), 0 0 0 4px var(--color-accent);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .native {
    position: relative;
    overflow: hidden;
  }
  .native input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
  .hexin {
    flex: 1;
    min-width: 0;
    height: var(--size-control-sm);
    padding: 0 8px;
    font: var(--type-mono-weight) var(--type-mono-size) var(--type-mono-family);
    color: var(--color-text-1);
    background: var(--color-sunken);
    border: var(--field-border);
    border-radius: var(--radius-control);
    outline: none;
    user-select: text;
  }
  .hexin:focus {
    border-color: var(--color-accent);
  }
</style>

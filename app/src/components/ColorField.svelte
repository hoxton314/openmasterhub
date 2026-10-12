<script lang="ts">
  import { Pipette, RotateCcw } from 'lucide-svelte';

  let {
    value,
    fallback = '#000000',
    swatches = ['#000000', '#27272a', '#3b82f6', '#8b5cf6', '#ec4899', '#ef4444', '#f59e0b', '#22c55e'],
    resettable = true,
    onchange,
  }: { value: string | undefined; fallback?: string; swatches?: string[]; resettable?: boolean; onchange: (v: string | undefined) => void } = $props();

  const current = $derived((value ?? fallback).toLowerCase());
</script>

<div class="color">
  {#each swatches as s}
    <button class="swatch" class:on={value?.toLowerCase() === s} style="--c:{s}" title={s} aria-label={s} onclick={() => onchange(s)}></button>
  {/each}
  <label class="swatch custom" class:on={value !== undefined && !swatches.includes(current)} style="--c:{current}" title="Custom colour">
    <Pipette size={11} />
    <input type="color" value={current} oninput={(e) => onchange(e.currentTarget.value)} />
  </label>
  {#if resettable && value !== undefined}
    <button class="btn ghost sm icon" title="Reset to default" onclick={() => onchange(undefined)}><RotateCcw size={12} /></button>
  {/if}
</div>

<style>
  .color {
    display: flex;
    flex-wrap: nowrap;
    align-items: center;
    gap: 4px;
  }
  .swatch {
    position: relative;
    flex-shrink: 0;
    width: 17px;
    height: 17px;
    padding: 0;
    border-radius: 50%;
    background: var(--c);
    border: none;
    box-shadow: inset 0 0 0 1px rgb(127 127 127 / 0.35);
    cursor: pointer;
    transition: transform 0.1s;
  }
  .swatch:hover {
    transform: scale(1.12);
  }
  .swatch.on {
    box-shadow: 0 0 0 2px var(--bg-panel), 0 0 0 3.5px var(--accent);
  }
  .custom {
    display: grid;
    place-items: center;
    color: white;
    mix-blend-mode: normal;
  }
  .custom :global(svg) {
    filter: drop-shadow(0 0 1px rgb(0 0 0 / 0.8));
  }
  .custom input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
</style>

<script lang="ts">
  import { Clock, Cpu, Gpu, ImagePlus, MemoryStick, Music2, Thermometer, Volume2, X } from 'lucide-svelte';
  import { invoke, pickImage } from '../lib/backend';
  import ColorField from './ColorField.svelte';
  import type { Face, Widget } from '../lib/types';

  let {
    face,
    live,
    round = false,
    liveTurn = 0,
    onchange,
  }: { face: Face; live?: string; round?: boolean; liveTurn?: number; onchange: (f: (face: Face) => void) => void } = $props();

  const WIDGETS = [
    { k: 'clock', label: 'Clock', icon: Clock },
    { k: 'cpu_usage', label: 'CPU', icon: Cpu },
    { k: 'cpu_temp', label: 'CPU temp', icon: Thermometer },
    { k: 'ram_usage', label: 'RAM', icon: MemoryStick },
    { k: 'gpu_usage', label: 'GPU', icon: Gpu },
    { k: 'gpu_temp', label: 'GPU temp', icon: Thermometer },
    { k: 'volume', label: 'Volume', icon: Volume2 },
    { k: 'now_playing', label: 'Playing', icon: Music2 },
  ] as const;

  let preview = $state('');
  $effect(() => {
    const snapshot = $state.snapshot(face);
    invoke<string>('preview_face', { face: snapshot }).then((src) => (preview = src)).catch(() => (preview = ''));
  });

  async function chooseImage() {
    const path = await pickImage();
    if (path) onchange((f) => (f.image = path));
  }
</script>

<div class="previews">
  <figure>
    <div class="tile" class:round>{#if preview}<img src={preview} alt="" />{/if}</div>
    <figcaption>Design</figcaption>
  </figure>
  <figure>
    <div class="tile" class:round class:empty={!live}>{#if live}<img src={live} alt="" style="transform: rotate({liveTurn}deg)" />{/if}</div>
    <figcaption>On device</figcaption>
  </figure>
</div>

<div class="group">
  <div class="field">
    <span class="label">Label</span>
    <input class="input" value={face.label ?? ''} placeholder="No label" oninput={(e) => { const v = e.currentTarget.value; onchange((f) => (f.label = v || undefined)); }} />
  </div>
  <div class="field">
    <span class="label">Background</span>
    <ColorField value={face.background} onchange={(v) => onchange((f) => (f.background = v))} />
  </div>
  <div class="field">
    <span class="label">Text</span>
    <ColorField value={face.text_color} fallback="#ffffff" swatches={['#ffffff', '#d1d5db', '#000000', '#facc15', '#60a5fa', '#f87171']} onchange={(v) => onchange((f) => (f.text_color = v))} />
  </div>
  <div class="field">
    <span class="label">Image</span>
    {#if face.image}
      <div class="file">
        <span class="name" title={face.image}>{face.image.split(/[\\/]/).pop()}</span>
        <button class="btn ghost sm icon" title="Remove image" onclick={() => onchange((f) => (f.image = undefined))}><X size={13} /></button>
      </div>
    {:else}
      <button class="btn sm add" onclick={chooseImage}><ImagePlus size={13} />Choose image or GIF</button>
    {/if}
  </div>
</div>

<div class="group">
  <div class="group-title">Live widget</div>
  <div class="widgets">
    <button class="widget" class:on={!face.widget} onclick={() => onchange((f) => (f.widget = undefined))}>None</button>
    {#each WIDGETS as { k, label, icon: I }}
      <button class="widget" class:on={face.widget?.kind === k} onclick={() => onchange((f) => (f.widget = { kind: k } as Widget))}>
        <I size={13} />{label}
      </button>
    {/each}
  </div>
</div>

<style>
  .previews {
    display: flex;
    justify-content: center;
    gap: 18px;
    padding: 18px 16px 16px;
  }
  figure {
    margin: 0;
    display: grid;
    justify-items: center;
    gap: 6px;
  }
  figcaption {
    font-size: 11.5px;
    color: var(--text-3);
  }
  .tile {
    width: 92px;
    height: 92px;
    border-radius: 16px;
    overflow: hidden;
    background: #000;
    box-shadow: 0 0 0 1px var(--border-strong), 0 6px 18px rgb(0 0 0 / 0.35);
  }
  .tile.round {
    border-radius: 50%;
  }
  .tile.empty {
    background: var(--bg-input);
    box-shadow: 0 0 0 1px var(--border);
  }
  .tile img {
    width: 100%;
    height: 100%;
    display: block;
  }
  .file {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    min-width: 0;
    height: 28px;
    padding-left: 9px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .file .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .add {
    justify-self: start;
    color: var(--text-2);
  }
  .widgets {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  .widget {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 9px;
    font: inherit;
    font-size: 12px;
    color: var(--text-2);
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 99px;
    cursor: pointer;
    transition: all 0.1s;
  }
  .widget:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }
  .widget.on {
    color: var(--accent-text);
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
  }
</style>

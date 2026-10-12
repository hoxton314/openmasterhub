<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';
  import type { Face, Widget } from '../lib/types';

  let { face, onchange }: { face: Face; onchange: (f: (face: Face) => void) => void } = $props();

  const WIDGETS: [Widget['kind'] | '', string][] = [
    ['', 'None'],
    ['clock', 'Clock'],
    ['cpu_usage', 'CPU usage'],
    ['cpu_temp', 'CPU temperature'],
    ['ram_usage', 'RAM usage'],
    ['gpu_usage', 'GPU usage'],
    ['gpu_temp', 'GPU temperature'],
    ['volume', 'Volume'],
    ['now_playing', 'Now playing'],
  ];

  let preview = $state('');
  // Re-render the local preview whenever the face changes.
  $effect(() => {
    const snapshot = $state.snapshot(face);
    invoke<string>('preview_face', { face: snapshot }).then((src) => (preview = src)).catch(() => (preview = ''));
  });

  async function pickImage() {
    const path = await openDialog({ multiple: false, filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif'] }] });
    if (typeof path === 'string') onchange((f) => (f.image = path));
  }
</script>

<div class="face">
  {#if preview}<img class="preview" src={preview} alt="Preview" />{/if}

  <label class="row">
    <span>Label</span>
    <input value={face.label ?? ''} oninput={(e) => { const v = e.currentTarget.value; onchange((f) => (f.label = v || undefined)); }} />
  </label>

  <div class="row">
    <span>Background</span>
    <div class="color">
      <input type="color" value={face.background ?? '#000000'} oninput={(e) => { const v = e.currentTarget.value; onchange((f) => (f.background = v)); }} />
      {#if face.background}<button class="flat" title="Default" onclick={() => onchange((f) => (f.background = undefined))}>↺</button>{/if}
    </div>
  </div>

  <div class="row">
    <span>Text colour</span>
    <div class="color">
      <input type="color" value={face.text_color ?? '#ffffff'} oninput={(e) => { const v = e.currentTarget.value; onchange((f) => (f.text_color = v)); }} />
      {#if face.text_color}<button class="flat" title="Default" onclick={() => onchange((f) => (f.text_color = undefined))}>↺</button>{/if}
    </div>
  </div>

  <div class="row">
    <span>Image</span>
    <div class="image">
      <span class="path" title={face.image}>{face.image ? face.image.split(/[\\/]/).pop() : 'None'}</span>
      <button class="flat" onclick={pickImage}>Choose…</button>
      {#if face.image}<button class="flat" title="Remove" onclick={() => onchange((f) => (f.image = undefined))}>✕</button>{/if}
    </div>
  </div>

  <label class="row">
    <span>Live widget</span>
    <select
      value={face.widget?.kind ?? ''}
      onchange={(e) => { const k = e.currentTarget.value; onchange((f) => (f.widget = k ? ({ kind: k } as Widget) : undefined)); }}
    >
      {#each WIDGETS as [k, label]}<option value={k}>{label}</option>{/each}
    </select>
  </label>
</div>

<style>
  .preview {
    display: block;
    width: 96px;
    height: 96px;
    margin: 4px auto 14px;
    border-radius: 14px;
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.45);
  }
  .color,
  .image {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .path {
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg-dim);
  }
</style>

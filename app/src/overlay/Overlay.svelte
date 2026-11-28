<!-- The switch toast (S08): which profile or setup just became live, near the tray, 1.5 s (5 s with
  Undo for a setup). The Rust side shows and hides the window (it never takes focus) and sends
  `overlay:show`; the window is exactly the card, so the page measures it and answers with
  `overlay_fit` (the window is shown then). In a browser (`overlay.html?demo=setup|profile`) it
  shows a sample. -->
<script lang="ts">
  import { Hand, Layers, Undo2 } from 'lucide-svelte';
  import { onMount, tick } from 'svelte';
  import { invoke, listen } from '../lib/backend';
  import { adoptAppSettings } from '../lib/theme/theme.svelte';
  import type { AppSettings } from '../lib/types';

  interface Toast {
    /** Echoed in `overlay_fit`. */
    generation?: number;
    kind: 'profile' | 'setup';
    title: string;
    /** The line above the title. */
    line: string;
    /** The profile's or setup's own colour. */
    color?: string | null;
    /** The setup to go back to. */
    undo?: string | null;
    secs: number;
    app?: AppSettings | null;
  }

  let toast = $state<Toast | undefined>();
  /** Bumped per toast so the card replays its entrance. */
  let shown = $state(0);

  const demo = new URLSearchParams(location.search).get('demo');
  const SAMPLES: Record<string, Toast> = {
    profile: { kind: 'profile', title: 'Blender', line: 'Work · because blender is focused', secs: 1.5 },
    setup: { kind: 'setup', title: 'Stream', line: 'Setup switched · Ctrl Alt F12', undo: 'Work', secs: 5, color: '#8B90FF' },
  };

  let card = $state<HTMLElement | undefined>();
  let sent = '';

  /** Tell the Rust side how big the card is (logical px) so the window matches it. */
  function fit(force = false) {
    if (!card || !toast || toast.generation === undefined) return;
    const w = card.offsetWidth;
    const h = card.offsetHeight;
    const key = `${toast.generation}:${w}x${h}`;
    if (!force && key === sent) return;
    sent = key;
    invoke('overlay_fit', { generation: toast.generation, width: w, height: h }).catch(() => undefined);
  }

  async function show(t: Toast) {
    if (t.app) adoptAppSettings(t.app);
    toast = t;
    shown++;
    await tick();
    fit(true);
  }

  // The card can change size after fonts or a theme arrive.
  $effect(() => {
    if (!card) return;
    const ro = new ResizeObserver(() => fit());
    ro.observe(card);
    return () => ro.disconnect();
  });

  onMount(() => {
    void listen<Toast>('overlay:show', (e) => show(e.payload));
    if (demo && SAMPLES[demo]) show(SAMPLES[demo]);
  });

  const Icon = $derived(toast?.kind === 'setup' ? Hand : Layers);
  const undo = () => toast?.undo && invoke('overlay_undo', { setup: toast.undo }).catch(() => undefined);
</script>

{#if toast}
  {#key shown}
    <div class="card" bind:this={card} role="status" aria-live="polite" style:--tile={toast.color || undefined}>
      <span class="tile"><Icon size={18} /></span>
      <div class="text">
        <div class="line t-chip">{toast.line}</div>
        <div class="title t-heading">{toast.title}</div>
      </div>
      {#if toast.undo}
        <button class="undo t-chip" onclick={undo}><Undo2 size={12} />Undo</button>
      {/if}
    </div>
  {/key}
{/if}

<style>
  :global(html.overlay),
  :global(html.overlay body) {
    background: transparent;
  }
  /* The window is exactly the card: no padding, no shadow (it would be cut off at the edge). */
  :global(html.overlay body) {
    margin: 0;
    overflow: hidden;
  }
  /* The surface can stay taller than the card (WebKitGTK's minimum height on a Wayland layer
     surface), so keep the card in the anchored bottom-right corner. */
  :global(html.overlay #app) {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    justify-content: flex-end;
    height: 100vh;
  }
  .card {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 320px;
    padding: 12px 14px;
    color: var(--color-text-1);
    background: var(--color-raised);
    border-radius: var(--radius-toast);
    outline: 1px solid var(--color-border-2);
    outline-offset: -1px;
    animation: rise var(--motion-toast-in);
  }
  .tile {
    display: grid;
    place-items: center;
    flex: none;
    width: 36px;
    height: 36px;
    color: var(--color-on-accent);
    background: var(--tile, var(--color-accent-fill));
    border-radius: var(--radius-avatar);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .line {
    color: var(--color-text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .title {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .undo {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex: none;
    height: var(--size-control-sm);
    padding: 0 8px;
    color: var(--color-accent);
    background: transparent;
    border: 0;
    border-radius: var(--radius-control);
    cursor: pointer;
  }
  .undo:hover {
    background: var(--color-selection);
  }
  .undo:focus-visible {
    outline: var(--focus-width) solid var(--focus-color);
    outline-offset: var(--focus-offset);
  }
</style>

<!-- The face of a key or the dial: Design and On device previews, then Icon & label / Image / Widget. -->
<script lang="ts">
  import { untrack } from 'svelte';
  import { ImagePlus, TriangleAlert as AlertTriangle, X } from 'lucide-svelte';
  import { invoke, pickImage } from '../../lib/backend';
  import { hasCapability } from '../../lib/hub.svelte';
  import type { Face, Widget } from '../../lib/types';
  import Button from '../ui/Button.svelte';
  import ColorField from '../ui/ColorField.svelte';
  import Field from '../ui/Field.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import InspectorSection from '../ui/InspectorSection.svelte';
  import KeyFace from '../ui/KeyFace.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import Select from '../ui/Select.svelte';
  import TextField from '../ui/TextField.svelte';
  import { FACE_TEXT } from '../../lib/palette';
  import Hint from './Hint.svelte';
  import ToggleRow from './ToggleRow.svelte';
  import { WIDGETS } from './widgets';

  let {
    face,
    live,
    liveTurn = 0,
    round = false,
    readonly = false,
    caption,
    onchange,
  }: {
    face: Face;
    /** The picture on the device now (only when this is the live profile). */
    live?: string;
    liveTurn?: number;
    round?: boolean;
    /** Shown but not editable (inherited, pinned): the owner must be edited instead. */
    readonly?: boolean;
    /** Right of the section title: "Key 5". */
    caption?: string;
    onchange: (f: (face: Face) => void) => void;
  } = $props();

  type Tab = 'label' | 'image' | 'widget';
  const tabOf = (f: Face): Tab => (f.widget ? 'widget' : f.image ? 'image' : 'label');
  let tab = $state<Tab>(untrack(() => tabOf(face)));
  // Follows the data when it changes under us (undo, another control), not on every keystroke.
  let lastKind = untrack(() => tabOf(face));
  $effect(() => {
    const k = tabOf(face);
    if (k !== lastKind) {
      lastKind = k;
      tab = k;
    }
  });

  const px = $derived(round ? 240 : 72);
  const WINDOWS = navigator.userAgent.includes('Windows');
  const fit = $derived(hasCapability('face_fit'));
  const refresh = $derived(hasCapability('widget_refresh'));

  let preview = $state('');
  $effect(() => {
    const snapshot = $state.snapshot(face);
    let stale = false;
    invoke<string>('preview_face', { face: snapshot }).then((src) => !stale && (preview = src)).catch(() => !stale && (preview = ''));
    return () => (stale = true);
  });

  async function chooseImage() {
    const path = await pickImage();
    if (path) onchange((f) => (f.image = path));
  }

  function setTab(t: Tab) {
    tab = t;
    if (t !== 'widget' && face.widget) onchange((f) => (f.widget = undefined));
  }
  const pickWidget = (kind: Widget['kind']) =>
    onchange((f) => (f.widget = { kind, ...(f.widget?.refresh ? { refresh: f.widget.refresh } : {}) } as Widget));
</script>

<InspectorSection title="Face">
  {#snippet action()}{#if caption}<span class="t-small muted">{caption}</span>{/if}{/snippet}

  <div class="previews">
    <figure class="card design">
      <KeyFace src={preview} dial={round} size={round ? 92 : 72} />
      <figcaption>Design</figcaption>
    </figure>
    <figure class="card">
      <span class="turn" style="transform: rotate({liveTurn}deg)"><KeyFace src={live} dial={round} size={round ? 92 : 72} /></span>
      <figcaption>{live ? `On device · ${px}×${px} live` : 'On device · not live'}</figcaption>
    </figure>
  </div>

  <fieldset class="body" disabled={readonly}>
    <Segmented
      full
      label="Face content"
      value={tab}
      onchange={setTab}
      options={[{ value: 'label', label: 'Icon & label' }, { value: 'image', label: 'Image' }, { value: 'widget', label: 'Widget' }]}
    />

    {#if tab === 'label'}
      <TextField label="Label" value={face.label ?? ''} placeholder="No label" oninput={(e) => { const v = e.currentTarget.value; onchange((f) => (f.label = v || undefined)); }} />
      <div class="two">
        <Field label="Background" id={null}><ColorField value={face.background} onchange={(v) => onchange((f) => (f.background = v))} /></Field>
        <Field label="Text" id={null}><ColorField value={face.text_color} fallback="#ffffff" swatches={FACE_TEXT} onchange={(v) => onchange((f) => (f.text_color = v))} /></Field>
      </div>
      {#if round}<Hint>Round 240×240. Corners are cut, keep text in the middle 170 px.</Hint>{/if}
      {#if face.image}<Hint>An image is set: it shows behind the label (Image tab).</Hint>{/if}
    {:else if tab === 'image'}
      {#if face.image}
        <div class="file">
          <ImagePlus size={15} />
          <span class="t-body name" title={face.image}>{face.image.split(/[\\/]/).pop()}</span>
          <Button size="sm" variant="ghost" onclick={chooseImage}>Replace</Button>
          <IconButton icon={X} size="sm" label="Remove image" onclick={() => onchange((f) => (f.image = undefined))} />
        </div>
      {:else}
        <button type="button" class="drop" onclick={chooseImage}>
          <ImagePlus size={20} />
          <span class="t-body">Choose a PNG, JPEG or GIF</span>
          <span class="t-small muted">Cropped to {px}×{px} · GIFs play on the {round ? 'dial' : 'key'}</span>
        </button>
      {/if}
      <Field label="Fit" id={null}>
        <Segmented
          full
          label="Image fit"
          value={face.fit ?? 'fit'}
          disabled={!fit}
          onchange={(v) => onchange((f) => (f.fit = v === 'fit' ? undefined : v))}
          options={[{ value: 'fill', label: 'Fill' }, { value: 'fit', label: 'Fit' }, { value: 'center', label: 'Center' }]}
        />
      </Field>
      <ToggleRow label="Draw the label over the image" checked={!!face.label_over_image} disabled={!fit} title={fit ? undefined : 'Needs a newer service'} onchange={(v) => onchange((f) => (f.label_over_image = v || undefined))} />
      {#if !fit}<Hint warn>Fit and label over image need a newer service.</Hint>{/if}
    {:else}
      <div class="widgets" role="radiogroup" aria-label="Widget">
        {#each WIDGETS as w (w.kind)}
          <button type="button" class="widget" class:on={face.widget?.kind === w.kind} role="radio" aria-checked={face.widget?.kind === w.kind} onclick={() => pickWidget(w.kind)}>
            <w.icon size={16} />
            <span class="t-chip">{w.label}</span>
            {#if w.lhm && WINDOWS}<span class="lhm"><AlertTriangle size={11} />Needs LHM</span>{/if}
          </button>
        {/each}
      </div>
      {#if face.widget}
        {@const w = face.widget}
        {#if w.kind === 'clock'}
          <TextField label="Format" mono size="sm" value={w.format ?? ''} placeholder="%H:%M" onchange={(e) => { const v = e.currentTarget.value.trim(); onchange((f) => (f.widget = { ...f.widget!, kind: 'clock', format: v || undefined } as Widget)); }} />
        {:else if w.kind === 'claude_session'}
          <Select label="Session" size="sm" value={w.slot ?? 0} onchange={(n) => onchange((f) => (f.widget = { ...f.widget!, kind: 'claude_session', slot: n } as Widget))} options={Array.from({ length: 15 }, (_, n) => ({ value: n, label: `Session ${n + 1}` }))} />
          <Hint>Shows the Claude Code session in that slot (waiting for you first, or as set by the sort order): its name and status colour. Pair it with the Claude action to bring the session up.</Hint>
        {/if}
        <div class="refresh">
          <span class="t-body">Refresh every</span>
          <Segmented
            label="Refresh interval"
            value={w.refresh ?? 2}
            disabled={!refresh}
            onchange={(n) => onchange((f) => (f.widget = { ...f.widget!, refresh: n } as Widget))}
            options={[{ value: 1, label: '1 s' }, { value: 2, label: '2 s' }, { value: 5, label: '5 s' }]}
          />
        </div>
        {#if !refresh}<Hint warn>A custom refresh interval needs a newer service.</Hint>{/if}
      {/if}
    {/if}
  </fieldset>
</InspectorSection>

<style>
  .previews {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .card {
    margin: 0;
    display: grid;
    justify-items: center;
    align-content: center;
    gap: 8px;
    padding: 14px 8px 10px;
    background: var(--color-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-well);
  }
  .card.design {
    border-color: var(--color-accent);
  }
  figcaption {
    font-size: var(--type-chip-size);
    color: var(--color-text-2);
    text-align: center;
  }
  .turn {
    display: inline-flex;
  }
  .body {
    display: grid;
    gap: 10px;
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }
  .body:disabled {
    opacity: 0.6;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 6px 6px 10px;
    color: var(--color-text-2);
    background: var(--color-sunken);
    border: var(--field-border);
    border-radius: var(--radius-control);
  }
  .file .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text-1);
  }
  .drop {
    display: grid;
    justify-items: center;
    gap: 4px;
    padding: 16px 10px;
    color: var(--color-text-2);
    background: transparent;
    border: 1px dashed var(--color-border-2);
    border-radius: var(--radius-well);
    cursor: pointer;
  }
  .drop:hover {
    border-color: var(--color-accent);
    color: var(--color-text-1);
  }
  .widgets {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 6px;
  }
  .widget {
    display: grid;
    justify-items: center;
    align-content: center;
    gap: 4px;
    min-height: 56px;
    padding: 8px 4px;
    color: var(--color-text-2);
    background: var(--color-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-control);
    cursor: pointer;
  }
  .widget:hover {
    color: var(--color-text-1);
    border-color: var(--color-border-2);
  }
  .widget.on {
    color: var(--color-accent);
    border-color: var(--color-accent);
    background: var(--color-selection);
  }
  .lhm {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: var(--type-micro-size);
    color: var(--color-warning);
  }
  .refresh {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
</style>

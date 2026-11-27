<!-- Backlight and orientation of the displays. -->
<script lang="ts">
  import { edit, hub } from '../../lib/hub.svelte';
  import type { Orientation } from '../../lib/types';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import Toggle from '../ui/Toggle.svelte';
  import Row from '../settings/Row.svelte';

  const orientation = $derived(hub.config.orientation ?? 'auto');
  const base = $derived(orientation.replace('_flipped', '') as 'auto' | 'portrait' | 'landscape');
  const flipped = $derived(orientation.endsWith('_flipped'));

  function setOrientation(base: 'auto' | 'portrait' | 'landscape', flip: boolean) {
    const value: Orientation = base === 'auto' ? 'auto' : flip ? (`${base}_flipped` as Orientation) : base;
    edit((c) => (c.orientation = value === 'auto' ? undefined : value), { label: 'orientation' });
  }
</script>

<section>
  <SectionLabel as="h2">Backlight and orientation</SectionLabel>
  <Row label="Backlight" help="Brightness of the key and dial screens">
    <input
      type="range"
      min="0"
      max="100"
      aria-label="Backlight"
      value={hub.config.backlight}
      oninput={(e) => edit((c) => (c.backlight = Number((e.currentTarget as HTMLInputElement).value)), { label: 'backlight', coalesce: 'backlight' })} />
    <output class="t-mono">{hub.config.backlight}</output>
  </Row>
  <Row label="Orientation" help="Auto follows the hub's tilt sensor where it has one">
    <Segmented
      label="Orientation"
      value={base}
      onchange={(v) => setOrientation(v, flipped)}
      options={[
        { value: 'auto', label: 'Auto' },
        { value: 'portrait', label: 'Portrait' },
        { value: 'landscape', label: 'Landscape' },
      ]} />
  </Row>
  <Row label="Flipped" help="Turn the layout 180°">
    <Toggle title="Flipped" checked={flipped} onchange={(v) => setOrientation(base, v)} />
  </Row>
</section>

<style>
  section {
    display: grid;
  }
  input[type='range'] {
    width: 160px;
    accent-color: var(--color-accent-fill);
  }
  output {
    width: 3ch;
    text-align: right;
    color: var(--color-text-2);
  }
</style>

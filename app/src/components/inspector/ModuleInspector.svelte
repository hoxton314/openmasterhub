<!-- A connected module: identity, rotation, identify, and the base's placement and lights. -->
<script lang="ts">
  import { Zap } from 'lucide-svelte';
  import { invoke } from '../../lib/backend';
  import { edit, hasCapability, hub, toast } from '../../lib/hub.svelte';
  import type { ModuleState, Orientation } from '../../lib/types';
  import Banner from '../ui/Banner.svelte';
  import Button from '../ui/Button.svelte';
  import Field from '../ui/Field.svelte';
  import InspectorSection from '../ui/InspectorSection.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import Select from '../ui/Select.svelte';
  import Hint from './Hint.svelte';
  import LedColor from './LedColor.svelte';
  import ToggleRow from './ToggleRow.svelte';

  let { module }: { module: ModuleState } = $props();

  const ORIENTATIONS: { value: Orientation; label: string }[] = [
    { value: 'auto', label: 'Automatic' },
    { value: 'landscape', label: 'Landscape' },
    { value: 'portrait', label: 'Portrait' },
    { value: 'landscape_flipped', label: 'Landscape, upside down' },
    { value: 'portrait_flipped', label: 'Portrait, upside down' },
  ];
  const mask = (s: string) => `•••• ${s.slice(-4)}`;
  const pl = $derived(module.placement);
  const span = (from: number, n: number) => (n > 1 ? `${from + 1}–${from + n}` : `${from + 1}`);
  const position = $derived(pl ? `Columns ${span(pl.col, pl.cols)}, rows ${span(pl.row, pl.rows)}` : 'unknown');
  const orientation = $derived(hub.config.orientation ?? 'auto');
  const tilt = $derived(hub.state.horizontal ? 'landscape' : 'portrait');
  const hasLeds = $derived(module.kind === 'base' || module.kind === 'fader' || module.kind === 'knob');
  const hasScreen = $derived(module.kind === 'base' || module.kind === 'keypad' || module.kind === 'dial');

  async function identify() {
    try {
      await invoke('identify', { serial: module.serial });
      toast({ text: 'Flashing the module for 3 seconds', timeout: 3000 });
    } catch (e) {
      toast({ text: `Could not identify: ${e}`, kind: 'warning' });
    }
  }
</script>

{#if module.needs_replug}
  <div class="pad">
    <Banner kind="warning">
      <strong class="t-body">Replug this module</strong>
      <p class="t-small muted">It ignores the service until it is unplugged and plugged in again. Use the app's hardware page for the steps.</p>
    </Banner>
  </div>
{/if}

<InspectorSection title="Module">
  <dl class="facts">
    <dt>Serial</dt><dd class="t-mono">{mask(module.serial)}</dd>
    <dt>Firmware</dt><dd class="t-mono">{module.firmware ?? 'unknown'}</dd>
    {#if module.socket}<dt>Socket</dt><dd class="t-mono">{module.socket}</dd>{/if}
    <dt>Position</dt><dd>{position}</dd>
    {#if pl}<dt>Rotation</dt><dd>{pl.rotation}°</dd>{/if}
  </dl>
  {#if pl}<Hint>Detected from the Base. Rotation is not editable yet.</Hint>{/if}
</InspectorSection>

{#if hasCapability('identify')}
  <InspectorSection title="Identify">
    <Hint>Flashes this module's lights white for 3 seconds so you can find it on the desk.</Hint>
    <div class="row"><Button size="sm" icon={Zap} onclick={identify}>Flash this module</Button></div>
  </InspectorSection>
{/if}

{#if module.kind === 'base'}
  <InspectorSection title="Base">
    <Select label="Orientation" value={orientation} options={ORIENTATIONS} onchange={(v) => edit((c) => (c.orientation = v === 'auto' ? undefined : v), { label: 'change orientation' })} />
    {#if orientation === 'auto'}<Hint>Auto uses the tilt sensor. Now: {tilt}.</Hint>{/if}
    <ToggleRow label="Upright screens" checked={hub.config.upright !== false} onchange={(on) => edit((c) => (c.upright = on ? undefined : false), { label: 'change upright screens' })} />
    <Hint>Turns key and dial images to read upright from your side.</Hint>
  </InspectorSection>
  <InspectorSection title="Base light">
    <LedColor kind="base" />
  </InspectorSection>
{/if}

{#if hasLeds}
  <InspectorSection title="Lights">
    <Field label="LED brightness" id="led-b">
      <input id="led-b" class="range" type="range" min="1" max="100" value={hub.config.leds?.brightness ?? 100} onchange={(e) => { const v = +e.currentTarget.value; edit((c) => ((c.leds ??= {}).brightness = v), { label: 'change LED brightness' }); }} />
    </Field>
  </InspectorSection>
{/if}
{#if hasScreen}
  <InspectorSection title="Screens" divider={false}>
    <Field label="Screen brightness" id="screen-b">
      <input id="screen-b" class="range" type="range" min="0" max="100" value={hub.config.backlight} onchange={(e) => { const v = +e.currentTarget.value; edit((c) => (c.backlight = v), { label: 'change screen brightness' }); }} />
    </Field>
  </InspectorSection>
{/if}

<style>
  .pad {
    padding: 14px var(--size-panel-pad) 0;
  }
  .facts {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 8px 16px;
    margin: 0;
  }
  dt {
    color: var(--color-text-2);
  }
  dd {
    margin: 0;
    min-width: 0;
    text-align: right;
    color: var(--color-text-1);
    user-select: text;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row {
    display: flex;
  }
  .range {
    width: 100%;
    accent-color: var(--color-accent-fill);
  }
</style>

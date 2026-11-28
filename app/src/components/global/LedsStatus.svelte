<!-- LEDs and status: Base ring and knob ring colours, Claude and setup blinks, volume on the rings. -->
<script lang="ts">
  import { edit, hasCapability, hub } from '../../lib/hub.svelte';
  import { LED_COLORS } from '../../lib/palette';
  import type { Color, Config } from '../../lib/types';
  import ColorField from '../ui/ColorField.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import Toggle from '../ui/Toggle.svelte';
  import Row from '../settings/Row.svelte';

  const c = $derived(hub.config);
  const knobColor = $derived(Array.isArray(c.leds?.knob) ? c.leds.knob[0] : c.leds?.knob);

  function led(key: 'base' | 'knob', v: Color | undefined) {
    edit((cfg: Config) => {
      cfg.leds = { ...cfg.leds, [key]: v };
    }, { label: 'LED colour', coalesce: `led-${key}` });
  }
  function flag(f: (cfg: Config) => void, label: string) {
    edit(f, { label });
  }
</script>

<section>
  <SectionLabel as="h2">LEDs and status</SectionLabel>
  <Row label="Base ring" help="Profiles can override it">
    <ColorField value={c.leds?.base} fallback={LED_COLORS[0]} swatches={LED_COLORS} onchange={(v) => led('base', v)} />
  </Row>
  <Row label="Knob rings">
    <ColorField value={knobColor} fallback={LED_COLORS[0]} swatches={LED_COLORS} onchange={(v) => led('knob', v)} />
  </Row>
  <Row label="Flash the Base when Claude waits for you" help="Uses the status colours of the Claude Code page in Library">
    <Toggle title="Flash the Base when Claude waits" checked={c.status?.enabled ?? false} onchange={(v) => flag((cfg) => (cfg.status = { ...cfg.status, enabled: v }), 'status LED')} />
  </Row>
  <Row label="Blink twice when the setup changes" help={hasCapability('blink_on_setup') ? undefined : 'Needs a newer service'}>
    <Toggle
      title="Blink twice when the setup changes"
      checked={c.status?.blink_on_setup ?? true}
      onchange={(v) => flag((cfg) => (cfg.status = { ...cfg.status, blink_on_setup: v }), 'setup blink')} />
  </Row>
  <Row label="Show volume on knob rings" help={hasCapability('knob_volume') ? undefined : 'Needs a newer service'}>
    <Toggle title="Show volume on knob rings" checked={c.leds?.knob_volume ?? false} onchange={(v) => flag((cfg) => (cfg.leds = { ...cfg.leds, knob_volume: v }), 'knob rings')} />
  </Row>
</section>

<style>
  section {
    display: grid;
  }
</style>

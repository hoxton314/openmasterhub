<!-- A module LED colour (base light, knob ring, fader track). Writes to the profile's own LED
     override when it has one, else to the global colours. -->
<script lang="ts">
  import { currentProfile, edit, hub } from '../../lib/hub.svelte';
  import { LED_COLORS } from '../../lib/palette';
  import type { Leds } from '../../lib/types';
  import ColorField from '../ui/ColorField.svelte';
  import Hint from './Hint.svelte';

  let { kind, index = 0, label = 'Colour' }: { kind: 'base' | 'fader' | 'knob'; index?: number; label?: string } = $props();

  const profile = $derived(currentProfile());
  const own = $derived(!!profile?.leds);
  const pick = (v: string | string[] | undefined) => (Array.isArray(v) ? v[index] : v);
  const value = $derived(pick(profile?.leds?.[kind]) ?? pick(hub.config.leds?.[kind]));

  function set(color: string | undefined) {
    const name = profile?.name;
    edit((c) => {
      const target: Leds | undefined = own ? c.profile?.find((p) => p.name === name)?.leds : (c.leds ??= {});
      if (!target) return;
      if (kind === 'base') {
        target.base = color;
        return;
      }
      const n = kind === 'fader' ? 5 : 3;
      const cur = target[kind];
      const list = Array.from({ length: n }, (_, k) => (Array.isArray(cur) ? cur[k] : cur) ?? '#ffffff');
      list[index] = color ?? '#ffffff';
      target[kind] = list;
    }, { label: `change ${kind} light`, coalesce: `led:${kind}:${index}` });
  }
</script>

<ColorField {label} {value} swatches={LED_COLORS} resettable={false} onchange={set} />
<Hint>{own ? `Set for ${profile?.name} only.` : 'Global colour, the same in every profile. Override it from the profile overview.'}</Hint>

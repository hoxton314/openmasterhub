<!-- A knob: turn left/right, press, and its ring light. -->
<script lang="ts">
  import { ArrowDown, ArrowUp, RotateCcw, RotateCw } from 'lucide-svelte';
  import { currentProfile, edit, hasCapability, hub, knobMut } from '../../lib/hub.svelte';
  import type { KnobConfig } from '../../lib/types';
  import InspectorSection from '../ui/InspectorSection.svelte';
  import ControlFooter from './ControlFooter.svelte';
  import { baseConfig, ownerFor } from './control.svelte';
  import Hint from './Hint.svelte';
  import LedColor from './LedColor.svelte';
  import OwnerNotice from './OwnerNotice.svelte';
  import ToggleRow from './ToggleRow.svelte';
  import Trigger from './Trigger.svelte';

  let { index }: { index: number } = $props();
  const ctl = $derived({ kind: 'knob', index } as const);
  const owner = $derived(ownerFor(ctl));
  let unlocked = $state(false);
  let open = $state<string | null>(null);
  const shown = $derived<KnobConfig>(
    (owner.kind === 'base' ? (baseConfig(owner, ctl) as KnobConfig | undefined) : currentProfile()?.knob?.find((k) => k.index === index)) ?? { index },
  );
  const readonly = $derived(owner.kind === 'base' || (owner.kind === 'claim' && !unlocked));
  const ring = $derived(hasCapability('knob_volume'));
  const label = $derived(`Knob ${index + 1}`);
  const setFlag = (key: 'knob_volume' | 'knob_mute_red', v: boolean) => edit((c) => ((c.leds ??= {})[key] = v || undefined), { label: 'change knob rings' });
</script>

<OwnerNotice {owner} {ctl} {label} {unlocked} onunlock={(on) => (unlocked = on)} />

<InspectorSection title="Turn and press">
  <div class="triggers">
    <Trigger id="ccw" label="Turn left" icon={RotateCcw} value={shown.ccw} bind:open {readonly} defaultVolume="-2" set={(c, a) => (knobMut(c, index).ccw = a)} />
    <Trigger id="cw" label="Turn right" icon={RotateCw} value={shown.cw} bind:open {readonly} defaultVolume="+2" set={(c, a) => (knobMut(c, index).cw = a)} />
    <Trigger id="press" label="Press" icon={ArrowDown} value={shown.press} bind:open {readonly} set={(c, a) => (knobMut(c, index).press = a)} />
    <Trigger id="release" label="Release" icon={ArrowUp} value={shown.release} bind:open {readonly} set={(c, a) => (knobMut(c, index).release = a)} />
  </div>
</InspectorSection>

<InspectorSection title="Ring light">
  <LedColor kind="knob" {index} />
  <ToggleRow label="Show volume level on the ring" checked={!!hub.config.leds?.knob_volume} disabled={!ring} title={ring ? undefined : 'Needs a newer service'} onchange={(v) => setFlag('knob_volume', v)} />
  <ToggleRow label="Turn red while muted" checked={!!hub.config.leds?.knob_mute_red} disabled={!ring} title={ring ? undefined : 'Needs a newer service'} onchange={(v) => setFlag('knob_mute_red', v)} />
  {#if ring}<Hint>These two apply to all knob rings.</Hint>{:else}<Hint warn>Ring level and mute colour need a newer service.</Hint>{/if}
</InspectorSection>
{#if !readonly}<ControlFooter {ctl} {label} />{/if}

<style>
  .triggers {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 8px;
    min-width: 0;
  }
</style>

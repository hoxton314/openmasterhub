<!-- The dial: its round screen, then turn, press, release and long-press. -->
<script lang="ts">
  import { ArrowDown, ArrowUp, Clock, RotateCcw, RotateCw } from 'lucide-svelte';
  import { currentProfile, dialMut, displayKey, edit, hasCapability, hub, isEditingLive } from '../../lib/hub.svelte';
  import { TARGET_DIAL, type DialConfig } from '../../lib/types';
  import InspectorSection from '../ui/InspectorSection.svelte';
  import ControlFooter from './ControlFooter.svelte';
  import { baseConfig, ownerFor } from './control.svelte';
  import FaceEditor from './FaceEditor.svelte';
  import Hint from './Hint.svelte';
  import OwnerNotice from './OwnerNotice.svelte';
  import Trigger from './Trigger.svelte';
  import { faceTurn } from './turn';

  const ctl = { kind: 'dial' } as const;
  const owner = $derived(ownerFor(ctl));
  let unlocked = $state(false);
  let open = $state<string | null>(null);
  const shown = $derived<DialConfig>((owner.kind === 'base' ? (baseConfig(owner, ctl) as DialConfig | undefined) : currentProfile()?.dial) ?? {});
  const readonly = $derived(owner.kind === 'base' || (owner.kind === 'claim' && !unlocked));
  const serial = $derived(hub.modules.find((m) => m.kind === 'dial')?.serial ?? '');
  const live = $derived(isEditingLive() ? hub.displays[displayKey(serial, TARGET_DIAL)] : undefined);
  const longPress = $derived(hasCapability('long_press'));
</script>

<OwnerNotice {owner} {ctl} label="The dial" {unlocked} onunlock={(on) => (unlocked = on)} />

<FaceEditor face={shown} round {live} liveTurn={faceTurn('dial')} {readonly} caption="Dial" onchange={(f) => edit((c) => f(dialMut(c)), { label: 'edit dial face', coalesce: 'face:dial' })} />

<InspectorSection title="Turn and press">
  <div class="triggers">
    <Trigger id="ccw" label="Turn left" icon={RotateCcw} value={shown.ccw} bind:open {readonly} defaultVolume="-2" set={(c, a) => (dialMut(c).ccw = a)} />
    <Trigger id="cw" label="Turn right" icon={RotateCw} value={shown.cw} bind:open {readonly} defaultVolume="+2" set={(c, a) => (dialMut(c).cw = a)} />
    <Trigger id="press" label="Press" icon={ArrowDown} value={shown.press} bind:open {readonly} set={(c, a) => (dialMut(c).press = a)} />
    <Trigger id="release" label="Release" icon={ArrowUp} value={shown.release} bind:open {readonly} set={(c, a) => (dialMut(c).release = a)} />
    {#if longPress || shown.long_press}
      <Trigger id="long" label="Long-press" icon={Clock} value={shown.long_press} bind:open {readonly} set={(c, a) => (dialMut(c).long_press = a)} />
    {/if}
  </div>
  {#if !longPress}<Hint warn>Long-press needs a newer service.</Hint>{/if}
</InspectorSection>
{#if !readonly}<ControlFooter {ctl} label="The dial" resetLabel="Reset dial" />{/if}

<style>
  .triggers {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 8px;
    min-width: 0;
  }
</style>

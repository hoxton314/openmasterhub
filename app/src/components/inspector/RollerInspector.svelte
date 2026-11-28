<!-- A roller: what rolling away from you and toward you does. -->
<script lang="ts">
  import { ArrowDown, ArrowUp } from 'lucide-svelte';
  import { currentProfile, rollerMut } from '../../lib/hub.svelte';
  import type { RollerConfig, Side } from '../../lib/types';
  import InspectorSection from '../ui/InspectorSection.svelte';
  import ControlFooter from './ControlFooter.svelte';
  import { baseConfig, ownerFor } from './control.svelte';
  import Hint from './Hint.svelte';
  import OwnerNotice from './OwnerNotice.svelte';
  import Trigger from './Trigger.svelte';

  let { side }: { side: Side } = $props();
  const ctl = $derived({ kind: 'roller', side } as const);
  const owner = $derived(ownerFor(ctl));
  let unlocked = $state(false);
  let open = $state<string | null>(null);
  const shown = $derived<RollerConfig>(
    (owner.kind === 'base' ? (baseConfig(owner, ctl) as RollerConfig | undefined) : currentProfile()?.roller?.find((r) => r.side === side)) ?? { side },
  );
  const readonly = $derived(owner.kind === 'base' || (owner.kind === 'claim' && !unlocked));
  const label = $derived(`${side === 'left' ? 'Left' : 'Right'} roller`);
</script>

<OwnerNotice {owner} {ctl} {label} {unlocked} onunlock={(on) => (unlocked = on)} />

<InspectorSection title="Roll">
  <div class="triggers">
    <Trigger id="away" label="Roll away" icon={ArrowUp} value={shown.away} bind:open {readonly} defaultVolume="+2" set={(c, a) => (rollerMut(c, side).away = a)} />
    <Trigger id="toward" label="Roll toward" icon={ArrowDown} value={shown.toward} bind:open {readonly} defaultVolume="-2" set={(c, a) => (rollerMut(c, side).toward = a)} />
  </div>
  <Hint>One step per notch. Fast rolls are not accelerated.</Hint>
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

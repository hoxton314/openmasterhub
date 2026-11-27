<!-- A fader: its live position, what moving it does, and its track light. -->
<script lang="ts">
  import { Move } from 'lucide-svelte';
  import { currentProfile, faderMut, hub } from '../../lib/hub.svelte';
  import type { FaderConfig } from '../../lib/types';
  import InspectorSection from '../ui/InspectorSection.svelte';
  import ControlFooter from './ControlFooter.svelte';
  import { baseConfig, ownerFor } from './control.svelte';
  import Hint from './Hint.svelte';
  import LedColor from './LedColor.svelte';
  import OwnerNotice from './OwnerNotice.svelte';
  import Trigger from './Trigger.svelte';

  let { index }: { index: number } = $props();
  const ctl = $derived({ kind: 'fader', index } as const);
  const owner = $derived(ownerFor(ctl));
  let unlocked = $state(false);
  let open = $state<string | null>('move');
  const shown = $derived<FaderConfig>(
    (owner.kind === 'base' ? (baseConfig(owner, ctl) as FaderConfig | undefined) : currentProfile()?.fader?.find((f) => f.index === index)) ?? { index },
  );
  const readonly = $derived(owner.kind === 'base' || (owner.kind === 'claim' && !unlocked));
  const serial = $derived(hub.modules.find((m) => m.kind === 'fader')?.serial ?? '');
  const position = $derived(hub.faders[`${serial}/${index}`]);
  const label = $derived(`Fader ${index + 1}`);
</script>

<OwnerNotice {owner} {ctl} {label} {unlocked} onunlock={(on) => (unlocked = on)} />

<InspectorSection title="Position">
  <div class="pos">
    <div class="bar" role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow={position ?? 0} aria-label="Fader position"><span style:width="{position ?? 0}%"></span></div>
    <span class="t-mono val">{position ?? '–'}</span>
  </div>
  <Hint>Live from the hub. Absolute 0–100, moves with your hand.</Hint>
</InspectorSection>

<InspectorSection title="When it moves">
  <Trigger id="move" label="Move" icon={Move} value={shown.change} bind:open {readonly} fader liveValue={position} set={(c, a) => (faderMut(c, index).change = a)} />
</InspectorSection>

<InspectorSection title="Track light">
  <LedColor kind="fader" {index} />
</InspectorSection>
{#if !readonly}<ControlFooter {ctl} {label} />{/if}

<style>
  .pos {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .bar {
    flex: 1;
    height: 8px;
    overflow: hidden;
    background: var(--color-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--color-accent-fill);
    transition: width var(--motion-fast);
  }
  .val {
    min-width: 28px;
    text-align: right;
    color: var(--color-text-1);
  }
</style>

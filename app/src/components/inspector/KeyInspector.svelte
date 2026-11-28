<!-- Key N: its face, then Press and Release. Inherited, pinned and claimed keys explain who provides
     them first and are read-only until overridden or unlocked. -->
<script lang="ts">
  import { ArrowDown, ArrowUp } from 'lucide-svelte';
  import { currentPage, currentProfile, displayKey, edit, hub, isEditingLive, keyMut } from '../../lib/hub.svelte';
  import type { ControlRef } from '../../lib/model/precedence';
  import type { KeyConfig } from '../../lib/types';
  import InspectorSection from '../ui/InspectorSection.svelte';
  import { describe } from '../../lib/hub.svelte';
  import ControlFooter from './ControlFooter.svelte';
  import { baseConfig, ownerFor, pinnedMut } from './control.svelte';
  import FaceEditor from './FaceEditor.svelte';
  import OwnerNotice from './OwnerNotice.svelte';
  import Trigger from './Trigger.svelte';
  import { faceTurn } from './turn';

  let { index }: { index: number } = $props();
  const ctl: ControlRef = $derived({ kind: 'key', index });
  const page = $derived(currentPage());
  const owner = $derived(ownerFor(ctl));
  const own = $derived(page?.key?.find((k) => k.index === index));
  let unlocked = $state(false);

  const shown = $derived<KeyConfig | undefined>(
    owner.kind === 'pinned' ? owner.pinned : owner.kind === 'base' ? (baseConfig(owner, ctl) as KeyConfig | undefined) : own,
  );
  const readonly = $derived(owner.kind === 'base' || ((owner.kind === 'pinned' || owner.kind === 'claim') && !unlocked));
  const mut = (c: Parameters<typeof keyMut>[0]) => (owner.kind === 'pinned' ? (pinnedMut(c, index) ?? keyMut(c, index)) : keyMut(c, index));
  const serial = $derived(hub.modules.find((m) => m.kind === 'keypad')?.serial ?? '');
  const live = $derived(isEditingLive() ? hub.displays[displayKey(serial, index)] : undefined);
  const underneath = $derived(owner.kind === 'claim' && !unlocked);
  let open = $state<string | null>(null);
  const noun = $derived(`Key ${index + 1}`);
</script>

<OwnerNotice {owner} {ctl} label={noun} {unlocked} onunlock={(on) => (unlocked = on)} />

{#if underneath}
  <InspectorSection title="Your key underneath">
    <div class="under">
      {#if own}
        <span class="tile" style:background={own.background} style:color={own.text_color}>{[...(own.label ?? '·')][0]}</span>
        <div class="what">
          <span class="t-body name">{own.label ?? 'No label'}</span>
          <span class="t-small muted">Press: {own.press ? describe(own.press) : 'nothing'}</span>
        </div>
      {:else}
        <span class="t-small muted">You have not set this key in {currentProfile()?.name}.</span>
      {/if}
    </div>
  </InspectorSection>
{:else}
  <FaceEditor
    face={shown ?? {}}
    {live}
    liveTurn={faceTurn('keypad')}
    {readonly}
    caption={noun}
    onchange={(f) => edit((c) => f(mut(c)), { label: 'edit key face', coalesce: `face:key${index}` })}
  />
  <InspectorSection title="When pressed and released">
    <div class="triggers">
      <Trigger id="press" label="Press" icon={ArrowDown} value={shown?.press} bind:open {readonly} set={(c, a) => (mut(c).press = a)} />
      <Trigger id="release" label="Release" icon={ArrowUp} value={shown?.release} bind:open {readonly} set={(c, a) => (mut(c).release = a)} />
    </div>
  </InspectorSection>
  {#if !readonly}<ControlFooter {ctl} label={noun} disabled={owner.kind === 'pinned'} />{/if}
{/if}

<style>
  .triggers {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 8px;
    min-width: 0;
  }
  .under {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .tile {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    flex: none;
    font-weight: 600;
    color: var(--hub-key-text);
    background: var(--hub-key);
    border-radius: var(--hub-key-radius);
    box-shadow: var(--hub-preview-frame);
  }
  .what {
    display: grid;
    min-width: 0;
  }
  .name {
    font-weight: 600;
  }
</style>

<!-- Who provides the selected control when it is not simply this profile's own: an integration
     claim, a pinned key from Global, or a base profile. Offers the way to change it. -->
<script lang="ts">
  import { CornerLeftUp, Pencil, Pin, Plug } from 'lucide-svelte';
  import { currentProfile, toastUndo } from '../../lib/hub.svelte';
  import type { ControlRef, Owner } from '../../lib/model/precedence';
  import { ui } from '../../lib/ui.svelte';
  import Banner from '../ui/Banner.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import { editInBase, hidePinned, overrideHere } from './control.svelte';

  let {
    owner,
    ctl,
    label,
    unlocked = false,
    onunlock,
  }: {
    owner: Owner;
    ctl: ControlRef;
    /** "Key 5", "the dial", "Knob 2". */
    label: string;
    /** Claim: the user's own control underneath is being edited; pinned: the pinned key is. */
    unlocked?: boolean;
    onunlock?: (on: boolean) => void;
  } = $props();

  const profile = $derived(currentProfile()?.name ?? '');
  const owns = (o: { keys?: number[]; dial?: boolean; knobs?: number[]; rollers?: number[]; faders?: number[] }) =>
    [o.keys?.length && 'Keypad', o.dial && 'dial', o.knobs?.length && 'Knobs', o.rollers?.length && 'Rollers', o.faders?.length && 'Faders'].filter(Boolean).join(' + ');
</script>

{#if owner.kind === 'claim' || owner.kind === 'pinned' || owner.kind === 'base'}
<div class="wrap">
{#if owner.kind === 'claim'}
  {@const name = owner.claim.name}
  <Banner kind="claim" icon={Plug}>
    <div class="note">
      <strong class="t-body title">Owned by {name}</strong>
      <p class="t-body">{name.replace(/ mode$/, '')} draws {label.toLowerCase()} and handles it while it is active. Your own comes back when you switch away.</p>
      <div class="chips">
        <Chip variant="success">{owner.claim.active ? 'Bridge connected' : 'Not connected'}</Chip>
        {#if owner.claim.priority !== undefined}<Chip variant="neutral" icon={null}>Priority {owner.claim.priority}</Chip>{/if}
        {#if owns(owner.claim.owns)}<Chip variant="neutral" icon={null}>{owns(owner.claim.owns)}</Chip>{/if}
      </div>
      <div class="buttons">
        <Button size="sm" onclick={() => ui.go('library', { view: 'integration', id: name.split(' ')[0].toLowerCase() })}>Open integration</Button>
        <Button size="sm" variant="ghost" onclick={() => onunlock?.(!unlocked)}>{unlocked ? 'Done editing mine' : `Edit my ${ctl.kind === 'dial' ? 'dial' : ctl.kind} underneath`}</Button>
      </div>
    </div>
  </Banner>
{:else if owner.kind === 'pinned'}
  {@const except = owner.pinned.except ?? []}
  <Banner kind="accent" icon={Pin}>
    <div class="note">
      <strong class="t-body title">Pinned on key {ctl.kind === 'key' ? ctl.index + 1 : ''} everywhere</strong>
      <p class="t-body">From the global layer. Shown on every page of every profile{except.length ? ` except ${except.join(', ')}` : ''}.</p>
      <div class="buttons">
        <Button size="sm" icon={Pencil} onclick={() => onunlock?.(!unlocked)}>{unlocked ? 'Done editing' : 'Edit pinned key'}</Button>
        <Button
          size="sm"
          variant="ghost"
          onclick={() => {
            if (ctl.kind === 'key') hidePinned(ctl.index);
            toastUndo(`Hidden ${label} in ${profile}`);
          }}>Hide in {profile}</Button>
      </div>
    </div>
  </Banner>
{:else if owner.kind === 'base'}
  {@const base = owner.profile}
  <Banner kind="info" icon={CornerLeftUp}>
    <div class="note">
      <strong class="t-body title">Inherited from {base}</strong>
      <p class="t-body">{profile} does not set this control, so it uses {base}’s. Changes to {base} show here too.</p>
      <div class="buttons">
        <Button size="sm" variant="primary" onclick={() => overrideHere(owner, ctl)}>Override here</Button>
        <Button size="sm" onclick={() => editInBase(owner)}>Edit in {base}</Button>
      </div>
    </div>
  </Banner>
{/if}
</div>
{/if}

<style>
  .wrap {
    padding: 14px var(--size-panel-pad) 0;
  }
  .note {
    display: grid;
    gap: 8px;
    min-width: 0;
  }
  .title {
    font-weight: 600;
  }
  p {
    margin: 0;
    color: var(--color-text-2);
  }
  .chips,
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
</style>

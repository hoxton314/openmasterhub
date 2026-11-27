<!-- Manual setups: the profiles in the order next / previous cycle through them. -->
<script lang="ts">
  import { Ellipsis, GripVertical } from 'lucide-svelte';
  import { hub } from '../../lib/hub.svelte';
  import { profileByName, type EffectiveSetup } from '../../lib/model';
  import { ui } from '../../lib/ui.svelte';
  import Avatar from '../ui/Avatar.svelte';
  import Chip from '../ui/Chip.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Menu, { type MenuEntry } from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';
  import Toggle from '../ui/Toggle.svelte';
  import AddProfileButton from './AddProfileButton.svelte';
  import { moveProfile, openInEditor, removeProfileFromSetup, updateSetup } from './ops';

  let { setup }: { setup: EffectiveSetup } = $props();
  const liveHere = $derived(hub.state.setup === setup.name);
  const last = $derived(setup.profiles.at(-1));
  const first = $derived(setup.profiles[0]);

  const menu = (name: string, i: number): MenuEntry[] => [
    { label: 'Move up', disabled: i === 0, onselect: () => moveProfile(setup.name, i, i - 1) },
    { label: 'Move down', disabled: i === setup.profiles.length - 1, onselect: () => moveProfile(setup.name, i, i + 1) },
    { separator: true },
    { label: 'Open in editor', onselect: () => openInEditor(setup.name, name) },
    { label: 'Remove from setup', danger: true, onselect: () => removeProfileFromSetup(setup.name, name) },
  ];

  let from = $state<number | undefined>();
  let slot = $state<number | undefined>();
  function over(e: DragEvent, i: number) {
    if (from === undefined) return;
    e.preventDefault();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    slot = e.clientY < r.top + r.height / 2 ? i : i + 1;
  }
  function drop(e: DragEvent) {
    e.preventDefault();
    if (from !== undefined && slot !== undefined) moveProfile(setup.name, from, slot > from ? slot - 1 : slot);
    from = slot = undefined;
  }
</script>

<section class="order" aria-label="Order">
  <header>
    <div>
      <h2 class="t-heading">Order</h2>
      <p class="t-small muted">Next and previous follow this order. Focus changes do nothing here.</p>
    </div>
    <AddProfileButton {setup} />
  </header>

  {#if !setup.profiles.length}
    <EmptyState title="No profiles in {setup.name} yet" body="Add the profiles you want to switch between." actionLabel="Add a new profile" onaction={() => ui.openNewProfile({ setup: setup.name, position: 0 })} />
  {/if}

  <div class="rows" role="list" ondragover={(e) => from !== undefined && e.preventDefault()} ondrop={drop}>
    {#each setup.profiles as name, i (name)}
      {@const p = profileByName(hub.config, name)}
      {#if p}
        <div
          class="row"
          class:dragging={from === i}
          class:before={from !== undefined && slot === i && slot !== from && slot !== from + 1}
          class:after={from !== undefined && i === setup.profiles.length - 1 && slot === i + 1 && slot !== from + 1}
          role="listitem"
          draggable="true"
          ondragstart={(e) => { from = i; e.dataTransfer?.setData('text/plain', name); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
          ondragend={() => (from = slot = undefined)}
          ondragover={(e) => over(e, i)}
        >
          <span class="grip" aria-hidden="true"><GripVertical size={14} /></span>
          <span class="idx">{i + 1}</span>
          <Avatar name={p.name} color={p.color} size={26} />
          <b class="name">{p.name}</b>
          <span class="end">
            {#if liveHere && hub.state.profile === name}<Chip variant="success" icon={null}>Live now</Chip>{/if}
            <span class="about t-small muted">{p.about ?? ''}</span>
          </span>
          <Popover placement="bottom-end" role="menu" label="{name} menu">
            {#snippet trigger({ toggle, attrs })}<IconButton icon={Ellipsis} size="sm" label="{name} menu" {...attrs} onclick={toggle} />{/snippet}
            {#snippet children({ close })}<Menu items={menu(name, i)} onclose={close} label="{name} menu" />{/snippet}
          </Popover>
        </div>
      {/if}
    {/each}
  </div>

  <div class="opts">
    <div class="opt">
      <span>Wrap from {last ?? 'the last'} back to {first ?? 'the first'}</span>
      <Toggle checked={!!setup.wrap} title="Wrap around" onchange={(v) => updateSetup(setup.name, (s) => (s.wrap = v || undefined), 'change wrap')} />
    </div>
    <div class="opt">
      <span>Show the current profile on the dial</span>
      <Toggle checked={!!setup.show_on_dial} title="Show the current profile on the dial" onchange={(v) => updateSetup(setup.name, (s) => (s.show_on_dial = v || undefined), 'change show on dial')} />
    </div>
  </div>
</section>

<style>
  .order {
    display: grid;
    gap: 8px;
    align-content: start;
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 4px;
  }
  .rows {
    display: grid;
    gap: 8px;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-panel);
  }
  .dragging {
    opacity: 0.45;
  }
  .before::before,
  .after::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    height: 3px;
    background: var(--color-accent);
    border-radius: var(--radius-pill);
  }
  .before::before {
    top: -6px;
  }
  .after::after {
    bottom: -6px;
  }
  .grip {
    display: inline-flex;
    color: var(--color-text-3);
    cursor: grab;
  }
  .idx {
    width: 18px;
    font: var(--type-mono-weight) var(--type-mono-size) var(--type-mono-family);
    color: var(--color-text-3);
  }
  .name {
    flex: none;
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .end {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
  }
  .about {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .opts {
    display: grid;
    gap: 8px;
    margin-top: 8px;
  }
  .opt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 14px;
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-panel);
    font-weight: 500;
  }
</style>

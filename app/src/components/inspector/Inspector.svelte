<!-- The inspector: a header and the editor for whatever is selected on the hub (S02, S03, S09,
     S10–11, S13). Nothing selected shows the profile overview. -->
<script lang="ts">
  import { CircleDot, Disc3, Grid3x3, Layers, MoreHorizontal, Pin, RectangleHorizontal, Settings, SlidersHorizontal, SquareDashed } from 'lucide-svelte';
  import { currentPage, currentProfile, editor, hub, isEditingLive, toastUndo } from '../../lib/hub.svelte';
  import { activeSetup, setupByName } from '../../lib/model/setups';
  import type { ModuleKind } from '../../lib/types';
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Menu, { type MenuEntry } from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';
  import { ownerFor, refOf, resetControl } from './control.svelte';
  import DialInspector from './DialInspector.svelte';
  import FaderInspector from './FaderInspector.svelte';
  import KeyInspector from './KeyInspector.svelte';
  import KnobInspector from './KnobInspector.svelte';
  import ModuleInspector from './ModuleInspector.svelte';
  import ProfileOverview from './ProfileOverview.svelte';
  import RollerInspector from './RollerInspector.svelte';
  import { WIDGETS } from './widgets';

  const s = $derived(hub.selection);
  const profile = $derived(currentProfile());
  const page = $derived(currentPage());
  const module = $derived(s.kind === 'module' ? hub.modules.find((m) => m.serial === s.serial) : undefined);
  const setup = $derived(setupByName(hub.config, editor.setup) ?? activeSetup(hub.config, hub.state));

  const MODULES: Record<ModuleKind, { name: string; about: string; icon: typeof Grid3x3 }> = {
    base: { name: 'Base', about: 'Tilt sensor · base light', icon: SquareDashed },
    keypad: { name: 'Display keypad', about: '5×3 keys with 72×72 screens', icon: Grid3x3 },
    fader: { name: 'Faders', about: '5 faders · RGB tracks', icon: SlidersHorizontal },
    knob: { name: 'Knobs', about: '3 endless knobs · RGB rings', icon: CircleDot },
    roller: { name: 'Rollers', about: '2 rollers', icon: RectangleHorizontal },
    dial: { name: 'Dial', about: '240×240 round screen', icon: Disc3 },
  };

  interface Head {
    icon: typeof Grid3x3;
    title: string;
    sub: string;
    /** What Reset does, when the control can be reset. */
    reset?: string;
  }
  const head = $derived.by((): Head => {
    const name = profile?.name ?? '';
    switch (s.kind) {
      case 'key': {
        const o = ownerFor({ kind: 'key', index: s.index });
        const widget = page?.key?.find((k) => k.index === s.index)?.widget;
        const w = widget && WIDGETS.find((x) => x.kind === widget.kind)?.label;
        const sub = o.kind === 'pinned' ? 'Pinned key' : o.kind === 'claim' ? `Keypad · ${name} profile` : w && o.kind !== 'base' ? `Widget · ${w}` : `Keypad · page ${page?.name ?? ''}`;
        return { icon: o.kind === 'pinned' ? Pin : Grid3x3, title: `Key ${s.index + 1}`, sub, reset: `Reset key ${s.index + 1}` };
      }
      case 'dial': return { icon: Disc3, title: 'Dial', sub: 'Turn, press, release', reset: 'Reset dial' };
      case 'knob': return { icon: CircleDot, title: `Knob ${s.index + 1}`, sub: `Knobs · ${s.index + 1} of 3`, reset: `Reset knob ${s.index + 1}` };
      case 'fader': return { icon: SlidersHorizontal, title: `Fader ${s.index + 1}`, sub: 'Faders', reset: `Reset fader ${s.index + 1}` };
      case 'roller': return { icon: RectangleHorizontal, title: `${s.side === 'left' ? 'Left' : 'Right'} roller`, sub: `Rollers · ${s.side}`, reset: `Reset ${s.side} roller` };
      case 'module': return module ? { icon: MODULES[module.kind].icon, title: MODULES[module.kind].name, sub: MODULES[module.kind].about } : { icon: Layers, title: 'Module', sub: 'Not connected' };
      default: return { icon: Layers, title: name || 'Profile', sub: `Profile in ${setup.name} · ${isEditingLive() ? 'live on the hub' : 'not live'}` };
    }
  });

  const none = $derived(s.kind === 'none');
  let menuOpen = $state(false);
  const menu = $derived.by((): MenuEntry[] => {
    const items: MenuEntry[] = [];
    const ctl = refOf(s);
    if (none) {
      items.push({ label: 'Profile settings', icon: Settings, onselect: () => profile && ui.go('editor', { view: 'profile-settings', profile: profile.name }) });
      return items;
    }
    if (ctl && head.reset) items.push({ label: head.reset, onselect: () => { resetControl(ctl); toastUndo(head.reset ?? 'Reset'); }, danger: true });
    items.push({ label: 'Close inspector', onselect: () => (hub.selection = { kind: 'none' }) });
    return items;
  });
</script>

<div class="inspector">
  <header>
    <span class="tile"><head.icon size={16} /></span>
    <div class="text">
      <h2 class="t-section">{head.title}</h2>
      <div class="t-small sub" title={head.sub}>{head.sub}</div>
    </div>
    {#if none && profile}
      <Button size="sm" variant="ghost" icon={Settings} onclick={() => ui.go('editor', { view: 'profile-settings', profile: profile.name })}>Settings</Button>
    {/if}
    <Popover bind:open={menuOpen} placement="bottom-end" role="menu" label="More actions">
      {#snippet trigger({ toggle, attrs })}
        <IconButton icon={MoreHorizontal} label="More actions" {...attrs} onclick={toggle} />
      {/snippet}
      {#snippet children({ close })}
        <Menu items={menu} onclose={close} label="More actions" />
      {/snippet}
    </Popover>
  </header>

  <div class="scroll">
    {#key JSON.stringify(s) + editor.profile + editor.page}
      {#if s.kind === 'key'}
        <KeyInspector index={s.index} />
      {:else if s.kind === 'dial'}
        <DialInspector />
      {:else if s.kind === 'knob'}
        <KnobInspector index={s.index} />
      {:else if s.kind === 'fader'}
        <FaderInspector index={s.index} />
      {:else if s.kind === 'roller'}
        <RollerInspector side={s.side} />
      {:else if module}
        <ModuleInspector {module} />
      {:else}
        <ProfileOverview />
      {/if}
    {/key}
  </div>
</div>

<style>
  .inspector {
    flex: 1;
    min-width: var(--size-inspector-narrow);
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--color-surface);
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px var(--size-panel-pad) 12px;
    border-bottom: var(--panel-border-width) solid var(--color-border);
  }
  .tile {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    color: var(--color-text-2);
    background: var(--color-raised);
    border-radius: var(--radius-avatar);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  h2 {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    color: var(--color-text-2);
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
  }
  .scroll > :global(*) {
    flex: none;
  }
</style>

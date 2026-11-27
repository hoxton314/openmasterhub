<!--
  Editor topbar: setup › profile › page (each opens a menu), the focused app, "Why …?", whether
  the config is applied, undo / redo, Follow live and search.
-->
<script lang="ts">
  import { AppWindow, ChevronDown, ChevronRight, Folder, LayoutGrid, Layers, PanelRight, Redo2, Search, Undo2 } from 'lucide-svelte';
  import { currentPage, currentProfile, editProfile, editor, hub, isEditingLive, redo, setFollowLive, setProfile, showPage, undo, undoState } from '../../lib/hub.svelte';
  import { profilesOf } from '../../lib/model';
  import { ui } from '../../lib/ui.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Kbd from '../ui/Kbd.svelte';
  import Menu, { type MenuEntry } from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';
  import StatusDot from '../ui/StatusDot.svelte';
  import Toggle from '../ui/Toggle.svelte';
  import WhyPopover from './WhyPopover.svelte';
  import { liveSetupName, setupMenu, shownSetup } from './setupSwitch';

  let { drawer = false }: { drawer?: boolean } = $props();

  const setup = $derived(shownSetup());
  const profile = $derived(currentProfile());
  const page = $derived(currentPage());
  const focused = $derived(hub.state.active_app?.split('.').pop());
  const isLive = $derived(isEditingLive());
  const isLiveSetup = $derived(setup.name === liveSetupName());

  const profileItems = (close: () => void): MenuEntry[] => {
    const names = [...profilesOf(hub.config, setup).map((p) => p.name), ...(setup.fallback ? [setup.fallback] : [])];
    const items: MenuEntry[] = names.map((n) => ({
      label: n,
      checked: n === profile?.name,
      hint: n === hub.state.profile && isLiveSetup ? 'live' : undefined,
      onselect: () => (close(), editProfile(n)),
    }));
    items.push({ separator: true });
    if (profile && !isLive) items.push({ label: `Make ${profile.name} live now`, onselect: () => (close(), setProfile(profile.name)) });
    items.push({ label: 'Profile settings…', onselect: () => (close(), ui.open({ view: 'profile-settings', profile: profile?.name })) });
    items.push({ label: 'Pages and folders…', onselect: () => (close(), ui.open({ view: 'pages' })) });
    return items;
  };
  const pageItems = (close: () => void): MenuEntry[] => [
    ...(profile?.page ?? []).map((p) => ({ label: p.name, icon: LayoutGrid, checked: p.name === page?.name, onselect: () => (close(), showPage(p.name)) })),
    ...(profile?.folder ?? []).map((p) => ({ label: p.name, icon: Folder, checked: p.name === page?.name, hint: 'folder', onselect: () => (close(), showPage(p.name)) })),
  ];
  const isFolder = $derived((profile?.folder ?? []).some((f) => f.name === page?.name));
</script>

{#snippet crumb(label: string, menu: (close: () => void) => MenuEntry[], opts: { icon?: typeof Layers; live?: boolean; last?: boolean; menuLabel: string })}
  <Popover role="menu" label={opts.menuLabel}>
    {#snippet trigger({ toggle, attrs })}
      <button type="button" class="crumb" class:last={opts.last} {...attrs} onclick={toggle}>
        {#if opts.icon}<opts.icon size={13} />{/if}<span class="lbl">{label}</span>{#if opts.live}<StatusDot size={6} label="Live on the hub" />{/if}<ChevronDown size={13} class="chev" />
      </button>
    {/snippet}
    {#snippet children({ close })}<Menu onclose={close} items={menu(close)} />{/snippet}
  </Popover>
{/snippet}

<nav class="crumbs" aria-label="Breadcrumb">
  {@render crumb(setup.name, (close) => setupMenu(close), { icon: Layers, live: isLiveSetup, menuLabel: 'Setups' })}
  <ChevronRight size={13} class="sep" />
  {@render crumb(profile?.name ?? 'No profile', profileItems, { live: isLive && isLiveSetup, menuLabel: 'Profiles' })}
  <ChevronRight size={13} class="sep" />
  {@render crumb(page?.name ?? 'Main', pageItems, { icon: isFolder ? Folder : LayoutGrid, last: true, menuLabel: 'Pages' })}
</nav>

{#if focused}
  <span class="focused" title="Focused application"><AppWindow size={13} /><code>{focused}</code>focused</span>
{/if}
<span class="why"><WhyPopover /></span>

<div class="tools">
  <span class="applied" title="Applied: the service runs this config"><StatusDot size={7} />Applied</span>
  <IconButton icon={Undo2} label={undoState.canUndo ? `Undo ${undoState.undoLabel}` : 'Undo'} disabled={!undoState.canUndo} onclick={() => undo()} />
  <IconButton icon={Redo2} label={undoState.canRedo ? `Redo ${undoState.redoLabel}` : 'Redo'} disabled={!undoState.canRedo} onclick={() => redo()} />
  <Toggle label="Follow live" checked={editor.followLive} onchange={setFollowLive} title="Keep the editor on whatever is live" />
  <button type="button" class="search" onclick={() => (ui.palette = true)} title="Search (Ctrl K)">
    <Search size={14} /><span class="word">Search</span><Kbd>Ctrl K</Kbd>
  </button>
  {#if drawer}<IconButton icon={PanelRight} label="Inspector" active={ui.drawer} onclick={() => (ui.drawer = !ui.drawer)} />{/if}
</div>

<style>
  .crumbs {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    flex: 0 1 auto;
    overflow: hidden;
    color: var(--color-text-3);
  }
  .crumbs :global(.sep) {
    flex: none;
  }
  .crumbs :global(.anchor) {
    flex: 0 1 auto;
    min-width: 0;
  }
  .crumb {
    min-width: 0;
  }
  .lbl {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .crumb :global(svg),
  .crumb :global(.dot) {
    flex: none;
  }
  .why {
    display: inline-flex;
    flex: none;
  }
  .crumb {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 6px;
    font-weight: 500;
    white-space: nowrap;
    color: var(--color-text-2);
    background: none;
    border: 0;
    border-radius: var(--radius-row);
    cursor: pointer;
  }
  .crumb:hover {
    background: var(--color-selection);
    color: var(--color-text-1);
  }
  .crumb.last {
    color: var(--color-text-1);
    font-weight: 600;
  }
  .crumb :global(.chev) {
    color: var(--color-text-3);
  }
  .focused {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: var(--size-chip);
    padding: 0 9px;
    white-space: nowrap;
    font-size: var(--type-chip-size);
    color: var(--color-text-2);
    background: var(--color-chip);
    border-radius: var(--radius-chip);
  }
  .focused code {
    font-weight: 500;
    color: var(--color-text-1);
  }
  .tools {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
  .applied {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    margin-right: 4px;
    font-size: var(--type-small-size);
    color: var(--color-text-2);
  }
  .search {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: var(--size-control-sm);
    padding: 0 8px;
    color: var(--color-text-2);
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-control);
    cursor: pointer;
  }
  .search:hover {
    border-color: var(--color-border-2);
  }
  .applied span {
    white-space: nowrap;
  }
  @media (max-width: 1500px) {
    .focused,
    .word {
      display: none;
    }
    .applied {
      font-size: 0;
      gap: 0;
    }
  }
</style>

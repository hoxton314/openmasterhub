<!--
  Editor sidebar: the setup switcher, that setup's profiles in priority order (LIVE, first app
  rule, integration plug), the selected profile's pages and folders, and the fallback pinned last
  under "When nothing matches". Collapsed (≤1100 px): the setup icon and one avatar per profile.
-->
<script lang="ts">
  import { ArrowDown, ChevronsUpDown, Layers, Plus } from 'lucide-svelte';
  import { editProfile, editor, hub, showPage } from '../../lib/hub.svelte';
  import { games, refreshGames } from '../../lib/games.svelte';
  import { avatarColor, fallbackOf, profilesOf } from '../../lib/model';
  import type { Profile } from '../../lib/types';
  import { ui } from '../../lib/ui.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Menu from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import SidebarRow from '../ui/SidebarRow.svelte';
  import StatusDot from '../ui/StatusDot.svelte';
  import TreeRow from '../ui/TreeRow.svelte';
  import { MODE_TEXT, liveSetupName, setupMenu, shownSetup } from './setupSwitch';

  let { collapsed = false }: { collapsed?: boolean } = $props();

  $effect(() => {
    refreshGames();
  });

  const setup = $derived(shownSetup());
  const isLiveSetup = $derived(setup.name === liveSetupName());
  const list = $derived(profilesOf(hub.config, setup));
  const fallback = $derived(fallbackOf(hub.config, setup));
  const selectedProfile = $derived(editor.profile);

  const glob = (p: Profile) => (p.apps?.length ? `${p.apps[0]}${p.apps.length > 1 ? ` +${p.apps.length - 1}` : ''}` : undefined);
  /** A profile an integration (game mode, live-surface client) draws or claims for. */
  function isIntegration(p: Profile) {
    const apps = (p.apps ?? []).map((a) => a.toLowerCase());
    if (hub.claims.some((c) => c.apps?.some((a) => apps.includes(a.toLowerCase())))) return true;
    return (games.list ?? []).some((g) => g.enabled && g.steam != null && apps.includes(`steam_app_${g.steam}`));
  }
  const live = (p: Profile) => isLiveSetup && hub.state.profile === p.name;

  const pages = $derived(hub.config.profile?.find((p) => p.name === selectedProfile)?.page ?? []);
  const folders = $derived(hub.config.profile?.find((p) => p.name === selectedProfile)?.folder ?? []);
  const currentPageName = $derived(editor.page || pages[0]?.name);
</script>

{#snippet switcher()}
  <Popover role="menu" label="Setups" placement={collapsed ? 'right-start' : 'bottom-start'} width={collapsed ? 240 : 212}>
    {#snippet trigger({ toggle, attrs })}
      {#if collapsed}
        <IconButton icon={Layers} label="Setup: {setup.name}" variant="secondary" {...attrs} onclick={toggle} />
      {:else}
        <button type="button" class="switcher" {...attrs} onclick={toggle} title="Switch the setup shown here">
          <span class="tile"><Layers size={16} /></span>
          <span class="who">
            <span class="name">{setup.name}{#if isLiveSetup}<StatusDot size={6} label="Active setup" />{/if}</span>
            <span class="mode">{MODE_TEXT[setup.mode]}</span>
          </span>
          <ChevronsUpDown size={14} />
        </button>
      {/if}
    {/snippet}
    {#snippet children({ close })}
      <Menu onclose={close} items={setupMenu(close).map((i) => ('onselect' in i ? { ...i, onselect: () => (close(), i.onselect()) } : i))} />
    {/snippet}
  </Popover>
{/snippet}

<div class="sidebar" class:collapsed>
  <div class="head">{@render switcher()}</div>

  <div class="scroll" role="listbox" aria-label="Profiles in {setup.name}">
    {#if !collapsed}
      <div class="label">
        <SectionLabel>
          Profiles in {setup.name}
          {#snippet action()}<IconButton icon={Plus} size="sm" label="New profile in {setup.name}" onclick={() => ui.openNewProfile({ setup: setup.name })} />{/snippet}
        </SectionLabel>
      </div>
    {/if}

    {#snippet entry(p: Profile, isFallback: boolean)}
      <SidebarRow
        name={p.name}
        color={avatarColor(p)}
        selected={p.name === selectedProfile}
        live={live(p)}
        glob={isFallback ? undefined : glob(p)}
        fallback={isFallback}
        locked={isFallback}
        integration={isIntegration(p)}
        {collapsed}
        onclick={() => editProfile(p.name)}
      />
      {#if !collapsed && p.name === selectedProfile && (pages.length || folders.length)}
        <div class="tree">
          {#each pages as pg, i (pg.name)}
            <TreeRow name={pg.name} selected={pg.name === currentPageName} meta={i + 1} onclick={() => showPage(pg.name)} />
          {/each}
          {#each folders as f (f.name)}
            <TreeRow name={f.name} kind="folder" selected={f.name === currentPageName} meta="folder" onclick={() => showPage(f.name)} />
          {/each}
          <TreeRow name="Add page or folder" kind="add" onclick={() => ui.open({ view: 'pages' })} />
        </div>
      {/if}
    {/snippet}

    {#each list as p (p.name)}{@render entry(p, false)}{/each}

    {#if fallback}
      {#if collapsed}
        <hr />
      {:else}
        <div class="nothing"><ArrowDown size={12} />When nothing matches</div>
      {/if}
      {@render entry(fallback, true)}
    {:else if !list.length && !collapsed}
      <p class="empty">No profiles in this setup yet.</p>
    {/if}
  </div>
</div>

<style>
  .sidebar {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 12px 8px 8px;
    gap: 14px;
  }
  .collapsed {
    align-items: center;
    padding: 12px 4px 8px;
  }
  .head {
    display: flex;
  }
  .switcher {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    text-align: left;
    color: var(--color-text-3);
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-lg);
    cursor: pointer;
  }
  .switcher:hover {
    border-color: var(--color-border-2);
  }
  .tile {
    display: grid;
    place-items: center;
    flex: none;
    width: 30px;
    height: 30px;
    color: var(--color-accent);
    background: var(--color-accent-soft);
    border-radius: var(--radius-md);
  }
  .who {
    flex: 1;
    min-width: 0;
    display: grid;
  }
  .name {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-weight: 600;
    color: var(--color-text-1);
  }
  .mode {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--type-small-size);
    color: var(--color-text-2);
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .collapsed .scroll {
    align-items: center;
    gap: 6px;
  }
  .label {
    padding: 0 4px 2px 8px;
  }
  .tree {
    display: grid;
    gap: 1px;
    margin: 2px 0 4px 18px;
    padding-left: 4px;
    border-left: var(--panel-border-width) solid var(--color-border);
  }
  .nothing {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    padding: 6px 10px;
    font-size: var(--type-small-size);
    color: var(--color-text-3);
  }
  hr {
    width: 24px;
    margin: 4px 0;
    border: 0;
    border-top: var(--panel-border-width) solid var(--color-border);
  }
  .empty {
    padding: 8px 10px;
    color: var(--color-text-3);
  }
</style>

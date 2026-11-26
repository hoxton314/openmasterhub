<!--
  WP1 owns this file. Wave 0 stand-in: the pre-redesign sidebar, topbar, hub and inspector inside
  the new Layout, so the app stays usable.
-->
<script lang="ts">
  import { AppWindow, ChevronDown, Folder, LayoutGrid, PanelRight, Redo2, RotateCw, Undo2 } from 'lucide-svelte';
  import { invoke } from '../../lib/backend';
  import { currentPage, currentProfile, hub, redo, showPage, toast, undo, undoState } from '../../lib/hub.svelte';
  import { ui } from '../../lib/ui.svelte';
  import Hub from '../Hub.svelte';
  import Sidebar from '../Sidebar.svelte';
  import Inspector from '../inspector/Inspector.svelte';
  import Layout from '../shell/Layout.svelte';
  import Breadcrumb from '../ui/Breadcrumb.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Menu from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';

  const profile = $derived(currentProfile());
  const page = $derived(currentPage());
  const focused = $derived(hub.state.active_app?.split('.').pop());
  const pageList = $derived([...(profile?.page ?? []).map((p) => ({ name: p.name, folder: false })), ...(profile?.folder ?? []).map((p) => ({ name: p.name, folder: true }))]);
  let pagesOpen = $state(false);
  let restarting = $state(false);

  async function restartService() {
    restarting = true;
    toast('Restarting the service…');
    await invoke('restart_service').catch((e) => toast(String(e)));
    restarting = false;
  }
</script>

<Layout chrome={false}>
  {#snippet sidebar()}<Sidebar />{/snippet}
  {#snippet topbar({ drawer })}
    <Breadcrumb items={[{ label: profile?.name ?? 'Default', live: profile?.name === hub.state.profile }]} />
    <Popover bind:open={pagesOpen} role="menu" label="Pages">
      {#snippet trigger({ toggle, attrs })}
        <button class="page" {...attrs} disabled={pageList.length < 2} onclick={toggle}>
          {#if pageList.find((p) => p.name === page?.name)?.folder}<Folder size={13} />{:else}<LayoutGrid size={13} />{/if}
          {page?.name ?? 'Main'}{#if pageList.length > 1}<ChevronDown size={13} />{/if}
        </button>
      {/snippet}
      {#snippet children({ close })}
        <Menu onclose={close} items={pageList.map((p) => ({ label: p.name, icon: p.folder ? Folder : LayoutGrid, checked: p.name === page?.name, onselect: () => showPage(p.name) }))} />
      {/snippet}
    </Popover>
    <div class="tools">
      {#if focused}<span class="focused" title="Focused application"><AppWindow size={12} />{focused}</span>{/if}
      <IconButton icon={Undo2} label={undoState.canUndo ? `Undo ${undoState.undoLabel}` : 'Undo'} disabled={!undoState.canUndo} onclick={() => undo()} />
      <IconButton icon={Redo2} label={undoState.canRedo ? `Redo ${undoState.redoLabel}` : 'Redo'} disabled={!undoState.canRedo} onclick={() => redo()} />
      <IconButton icon={RotateCw} label="Restart service" disabled={restarting} onclick={restartService} />
      {#if drawer}<IconButton icon={PanelRight} label="Inspector" active={ui.drawer} onclick={() => (ui.drawer = !ui.drawer)} />{/if}
    </div>
  {/snippet}
  {#snippet main()}
    <div class="stage"><div class="canvas"><Hub /></div></div>
  {/snippet}
  {#snippet inspector()}<Inspector />{/snippet}
</Layout>

<style>
  .page {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 8px;
    font-weight: 500;
    color: var(--color-text-1);
    background: none;
    border: 0;
    border-radius: var(--radius-row);
    cursor: pointer;
  }
  .page:hover:not(:disabled) {
    background: var(--color-selection);
  }
  .page:disabled {
    cursor: default;
  }
  .tools {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .focused {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: var(--size-chip);
    padding: 0 9px;
    margin-right: 6px;
    font: var(--type-mono-weight) var(--type-chip-size) var(--type-mono-family);
    color: var(--color-text-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-chip);
  }
  .stage {
    flex: 1;
    min-height: 0;
    display: flex;
    background-color: var(--color-canvas);
    background-image: radial-gradient(var(--color-canvas-dot) 1px, transparent 1px);
    background-size: 20px 20px;
    background-position: center;
  }
  .canvas {
    flex: 1;
    min-width: 0;
    padding: 36px;
  }
</style>

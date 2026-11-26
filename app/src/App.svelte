<script lang="ts">
  import { AppWindow, ChevronDown, ChevronRight, Folder, LayoutGrid, Power, RotateCw } from 'lucide-svelte';
  import Dialog from './components/Dialog.svelte';
  import GameDialog from './components/GameDialog.svelte';
  import Hub from './components/Hub.svelte';
  import Inspector from './components/Inspector.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import { invoke } from './lib/backend';
  import { connect, currentPage, currentProfile, hub, showPage, toast } from './lib/hub.svelte';
  import { games } from './lib/games.svelte';
  import { ui } from './lib/ui.svelte';

  connect();

  const profile = $derived(currentProfile());
  const page = $derived(currentPage());
  const inFolder = $derived(!!profile?.folder?.some((f) => f.name === page?.name));
  const focused = $derived(hub.state.active_app?.split('.').pop());

  const pageList = $derived([...(profile?.page ?? []).map((p) => ({ name: p.name, folder: false })), ...(profile?.folder ?? []).map((p) => ({ name: p.name, folder: true }))]);
  let pagesOpen = $state(false);

  /** Escape closes what is open, innermost first, then clears the selection. */
  function onKey(e: KeyboardEvent) {
    if (e.key !== 'Escape' || e.defaultPrevented) return;
    if (ui.dialog || games.open) return; // the dialogs close themselves
    if (pagesOpen) pagesOpen = false;
    else if ((e.target as HTMLElement)?.closest?.('input, textarea, select')) return;
    else hub.selection = { kind: 'none' };
  }

  let restarting = $state(false);
  async function restartService() {
    restarting = true;
    toast('Restarting the service…');
    await invoke('restart_service').catch((e) => toast(String(e)));
    restarting = false;
  }
</script>

<svelte:window onkeydown={onKey} onclick={() => (pagesOpen = false)} />

<div class="app">
  {#if !hub.online}
    <div class="offline">
      <div class="badge"><Power size={22} /></div>
      <h1>The MasterHUB service isn’t running</h1>
      <p>OpenMasterHUB configures <code>masterhubd</code>, the background service that drives the hardware. Start it, or run <code>masterhubd</code> in a terminal.</p>
      <button class="btn primary" onclick={() => invoke('start_service').catch((e) => toast(String(e)))}>Start service</button>
    </div>
  {:else}
    <Sidebar />
    <main>
      <div class="topbar">
        <div class="crumbs">
          <span class="muted">{profile?.name ?? 'Default'}</span>
          <ChevronRight size={13} class="faint" />
          <div class="anchor">
            <button class="page" aria-haspopup="menu" aria-expanded={pagesOpen} disabled={pageList.length < 2} title={pageList.length < 2 ? undefined : 'Switch page'} onclick={(e) => { e.stopPropagation(); pagesOpen = !pagesOpen; }}>
              {#if inFolder}<Folder size={13} />{:else}<LayoutGrid size={13} />{/if}
              {page?.name ?? 'Main'}
              {#if pageList.length > 1}<ChevronDown size={13} class="faint" />{/if}
            </button>
            {#if pagesOpen}
              <div class="menu" style="left:0;top:30px" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
                {#each pageList as p (p.name)}
                  <button class:current={p.name === page?.name} onclick={() => { pagesOpen = false; showPage(p.name); }}>
                    {#if p.folder}<Folder size={14} />{:else}<LayoutGrid size={14} />{/if}{p.name}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </div>
        <div class="tools">
          {#if focused}
            <span class="chip" title="Focused application (picks the profile)"><AppWindow size={12} />{focused}</span>
          {/if}
          <button class="btn ghost sm icon" title="Restart service" disabled={restarting} onclick={restartService}><RotateCw size={14} /></button>
        </div>
      </div>
      <div class="stage">
        <div class="canvas"><Hub /></div>
      </div>
    </main>
    <Inspector />
  {/if}

  <Dialog />
  <GameDialog />
  {#if hub.toast}<div class="toast">{hub.toast}</div>{/if}
</div>

<style>
  .app {
    display: flex;
    height: 100%;
  }
  main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 52px;
    padding: 0 18px;
    border-bottom: 1px solid var(--border);
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .anchor {
    position: relative;
  }
  .page {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 8px;
    margin-left: -4px;
    font: inherit;
    font-weight: 500;
    color: var(--text);
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .page:hover:not(:disabled) {
    background: var(--bg-hover);
  }
  .page:disabled {
    cursor: default;
  }
  .menu .current {
    color: var(--text);
    background: var(--bg-active);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 9px;
    font-size: 12px;
    color: var(--text-2);
    border: 1px solid var(--border);
    border-radius: 99px;
  }
  .stage {
    flex: 1;
    min-height: 0;
    display: flex;
    background-image: radial-gradient(var(--border) 1px, transparent 1px);
    background-size: 22px 22px;
    background-position: center;
  }
  .canvas {
    flex: 1;
    min-width: 0;
    padding: 36px;
  }
  .offline {
    margin: auto;
    max-width: 420px;
    display: grid;
    justify-items: center;
    gap: 10px;
    text-align: center;
  }
  .offline .badge {
    width: 48px;
    height: 48px;
    display: grid;
    place-items: center;
    border-radius: 12px;
    color: var(--text-2);
    background: var(--bg-raised);
    box-shadow: 0 0 0 1px var(--border);
  }
  .offline h1 {
    font-size: 17px;
    font-weight: 600;
    letter-spacing: -0.01em;
    margin-top: 6px;
  }
  .offline p {
    color: var(--text-2);
    margin-bottom: 8px;
  }
  .toast {
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 60;
    padding: 9px 14px;
    background: var(--bg-raised);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    animation: pop 0.15s var(--ease);
  }
</style>

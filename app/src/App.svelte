<!--
  The app shell: rail + the open area, plus app-wide overlays. Areas live in their own folders
  (components/<area>/) so each can change without touching this file.
-->
<script lang="ts">
  import EditorArea from './components/editor/EditorArea.svelte';
  import FirstRun from './components/firstrun/FirstRun.svelte';
  import GlobalArea from './components/global/GlobalArea.svelte';
  import LibraryArea from './components/library/LibraryArea.svelte';
  import LibraryOverlays from './components/library/LibraryOverlays.svelte';
  import Palette from './components/palette/Palette.svelte';
  import NewProfileFlow from './components/profiles/NewProfileFlow.svelte';
  import SettingsArea from './components/settings/SettingsArea.svelte';
  import SetupsArea from './components/setups/SetupsArea.svelte';
  import DialogHost from './components/shell/DialogHost.svelte';
  import Rail from './components/shell/Rail.svelte';
  import ToastHost from './components/shell/ToastHost.svelte';
  import ServiceState from './components/status/ServiceState.svelte';
  import { connect, handleUndoKeys, hub } from './lib/hub.svelte';
  import { loadUserThemes } from './lib/theme/theme.svelte';
  import { ui } from './lib/ui.svelte';

  connect();
  loadUserThemes();

  function onKey(e: KeyboardEvent) {
    if (handleUndoKeys(e)) return;
    if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      ui.palette = !ui.palette;
      return;
    }
    if (e.key !== 'Escape' || e.defaultPrevented) return;
    if (ui.dialog || ui.palette || ui.newProfile || ui.firstRun) return; // they close themselves
    if ((e.target as HTMLElement)?.closest?.('input, textarea, select')) return;
    if (ui.drawer) ui.drawer = false;
    else hub.selection = { kind: 'none' };
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="app">
  <Rail />
  <main class="area" aria-label={ui.area}>
    {#if ui.area === 'settings'}
      <SettingsArea />
    {:else if !hub.online}
      <ServiceState />
    {:else if ui.area === 'editor'}
      <EditorArea />
    {:else if ui.area === 'setups'}
      <SetupsArea />
    {:else if ui.area === 'library'}
      <LibraryArea />
    {:else if ui.area === 'global'}
      <GlobalArea />
    {/if}
  </main>

  <LibraryOverlays />
  {#if ui.newProfile}<NewProfileFlow />{/if}
  {#if ui.palette}<Palette />{/if}
  {#if ui.firstRun}<FirstRun />{/if}
  <DialogHost />
  <ToastHost />
</div>

<style>
  .app {
    display: flex;
    height: 100%;
    min-width: 1100px;
    min-height: 700px;
  }
  .area {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--color-bg);
  }
</style>

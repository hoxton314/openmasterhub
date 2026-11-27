<!--
  Area layout: [sidebar 236 | topbar over (main | inspector 344)], as in the artboards: the topbar
  spans the canvas and the inspector.
  ≤1100 px wide: the sidebar collapses (52 px, `collapsed` passed to its snippet) and the inspector
  narrows to 320 px (S01b). 1101–1279 px: the inspector becomes a drawer over the canvas, opened
  with `ui.drawer`. Any snippet may be omitted.
-->
<script lang="ts" module>
  /** Window widths (px) at which the layout changes. */
  export const COLLAPSE_BELOW = 1100;
  export const DRAWER_BELOW = 1280;
</script>

<script lang="ts">
  import { X } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import { ui } from '../../lib/ui.svelte';
  import IconButton from '../ui/IconButton.svelte';

  let {
    sidebar,
    topbar,
    main,
    inspector,
    inspectorLabel = 'Inspector',
    chrome = true,
  }: {
    sidebar?: Snippet<[{ collapsed: boolean }]>;
    topbar?: Snippet<[{ drawer: boolean }]>;
    main: Snippet;
    inspector?: Snippet;
    inspectorLabel?: string;
    /** false: the sidebar and inspector draw their own width, background and border (legacy panels). */
    chrome?: boolean;
  } = $props();

  let width = $state(typeof innerWidth === 'number' ? innerWidth : 1440);
  const collapsed = $derived(width <= COLLAPSE_BELOW);
  const drawer = $derived(!collapsed && width < DRAWER_BELOW);
  $effect(() => {
    ui.layout.sidebarCollapsed = collapsed;
    ui.layout.inspectorDrawer = drawer;
    if (!drawer) ui.drawer = false;
  });
</script>

<svelte:window bind:innerWidth={width} />

<div class="layout" class:collapsed class:bare={!chrome}>
  {#if sidebar}
    <aside class="sidebar" aria-label="Sidebar">{@render sidebar({ collapsed })}</aside>
  {/if}
  <div class="right">
    {#if topbar}<header class="topbar">{@render topbar({ drawer })}</header>{/if}
    <div class="body">
      <div class="main">{@render main()}</div>
      {#if inspector && !drawer}
        <aside class="inspector" aria-label={inspectorLabel}>{@render inspector()}</aside>
      {/if}
    </div>
    {#if inspector && drawer && ui.drawer}
      <aside class="inspector drawer" class:below-topbar={!!topbar} aria-label={inspectorLabel}>
        <span class="close"><IconButton icon={X} size="sm" label="Close {inspectorLabel.toLowerCase()}" onclick={() => (ui.drawer = false)} /></span>
        {@render inspector()}
      </aside>
    {/if}
  </div>
</div>

<style>
  .layout {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    height: 100%;
  }
  .sidebar {
    width: var(--size-sidebar);
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    background: var(--color-bg);
    border-right: var(--panel-border-width) solid var(--color-border);
  }
  .collapsed .sidebar {
    width: var(--size-sidebar-collapsed);
  }
  .right {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .topbar {
    height: var(--size-topbar);
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px 0 16px;
    background: var(--color-surface);
    border-bottom: var(--panel-border-width) solid var(--color-border);
  }
  .main {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .inspector {
    width: var(--size-inspector);
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
    background: var(--color-surface);
    border-left: var(--panel-border-width) solid var(--color-border);
  }
  .collapsed .inspector {
    width: var(--size-inspector-narrow);
  }
  .bare .sidebar,
  .bare .inspector {
    flex-direction: row;
    width: auto;
    background: none;
    border: 0;
    overflow: visible;
  }
  .drawer,
  .bare .drawer {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 30;
    box-shadow: var(--shadow-popover);
    animation: slide-in-right var(--motion-panel);
  }
  .below-topbar {
    top: var(--size-topbar);
  }
  .close {
    position: absolute;
    top: 14px;
    right: 52px; /* left of the inspector's own ⋯ menu */
    z-index: 1;
  }
</style>

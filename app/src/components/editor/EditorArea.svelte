<!--
  The editor: setup-aware sidebar, topbar, hub canvas with banners and status line, inspector.
  Profile settings (S20) and pages and folders (S21) are sub-routes that replace the canvas
  and inspector and keep the sidebar.
-->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { isEditingLive } from '../../lib/hub.svelte';
  import { ui } from '../../lib/ui.svelte';
  import Inspector from '../inspector/Inspector.svelte';
  import PagesFolders from '../profiles/PagesFolders.svelte';
  import ProfileSettings from '../profiles/ProfileSettings.svelte';
  import { hardwareIssue, vendorApp, type VendorApp } from '../status/hardware';
  import HardwareState from '../status/HardwareState.svelte';
  import Layout from '../shell/Layout.svelte';
  import HubCanvas from './HubCanvas.svelte';
  import NotLiveBanner, { showing } from './NotLiveBanner.svelte';
  import Sidebar from './Sidebar.svelte';
  import StatusLine from './StatusLine.svelte';
  import Topbar from './Topbar.svelte';

  const view = $derived(ui.route?.view);
  const sub = $derived(view === 'profile-settings' || view === 'pages');
  const banner = $derived(!isEditingLive() || showing.left > 0);

  // Hardware trouble (no modules, replug, vendor app) replaces the canvas and inspector.
  let vendor = $state<VendorApp | null>(null);
  let timer: ReturnType<typeof setInterval> | undefined;
  onMount(() => {
    const check = async () => (vendor = await vendorApp());
    check();
    timer = setInterval(check, 3000);
  });
  onDestroy(() => clearInterval(timer));
  const trouble = $derived(hardwareIssue(vendor));
</script>

{#if trouble && !sub}
  <Layout>
    {#snippet sidebar({ collapsed })}<Sidebar {collapsed} />{/snippet}
    {#snippet main()}<HardwareState />{/snippet}
  </Layout>
{:else if sub}
  <Layout>
    {#snippet sidebar({ collapsed })}<Sidebar {collapsed} />{/snippet}
    {#snippet main()}
      {#if view === 'profile-settings'}<ProfileSettings />{:else}<PagesFolders />{/if}
    {/snippet}
  </Layout>
{:else}
  <Layout>
    {#snippet sidebar({ collapsed })}<Sidebar {collapsed} />{/snippet}
    {#snippet topbar({ drawer })}<Topbar {drawer} />{/snippet}
    {#snippet main()}
      <div class="stage">
        {#if banner}<div class="banner"><NotLiveBanner /></div>{/if}
        <HubCanvas />
        <StatusLine />
      </div>
    {/snippet}
    {#snippet inspector()}<Inspector />{/snippet}
  </Layout>
{/if}

<style>
  .stage {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background-color: var(--color-canvas);
    background-image: radial-gradient(var(--color-canvas-dot) 1px, transparent 1px);
    background-size: 20px 20px;
    background-position: center;
  }
  .banner {
    padding: 12px 16px 0;
  }
</style>

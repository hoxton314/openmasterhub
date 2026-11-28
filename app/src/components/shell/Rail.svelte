<!-- The 64 px rail: logo tile and the five areas (Settings pinned to the bottom). -->
<script lang="ts">
  import { Globe, LayoutGrid, Layers, Library, Settings } from 'lucide-svelte';
  import { theme } from '../../lib/theme/theme.svelte';
  import { ui, type Area } from '../../lib/ui.svelte';
  import type { Icon } from '../ui/icon';
  import RailItem from '../ui/RailItem.svelte';

  const top: { area: Area; label: string; icon: Icon }[] = [
    { area: 'editor', label: 'Editor', icon: LayoutGrid },
    { area: 'setups', label: 'Setups', icon: Layers },
    { area: 'library', label: 'Library', icon: Library },
    { area: 'global', label: 'Global', icon: Globe },
  ];
</script>

<nav class="rail" aria-label="Areas">
  <div class="logo" role="img" aria-label="OpenMasterHUB">
    {#each Array(9) as _}<span></span>{/each}
  </div>
  {#each top as a (a.area)}
    <RailItem icon={a.icon} label={a.label} active={ui.area === a.area} onclick={() => ui.go(a.area)} />
  {/each}
  <div class="spacer"></div>
  <RailItem icon={Settings} label="Settings" active={ui.area === 'settings'} badge={theme.problems.length > 0} onclick={() => ui.go('settings')} />
</nav>

<style>
  .rail {
    width: var(--size-rail);
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 14px 0 12px;
    background: var(--color-rail);
    border-right: var(--panel-border-width) solid var(--color-border);
  }
  .logo {
    display: grid;
    grid-template-columns: repeat(3, 5px);
    gap: 2px;
    place-content: center;
    width: 32px;
    height: 32px;
    margin-bottom: 10px;
    background: var(--color-accent-fill);
    border-radius: var(--radius-md);
  }
  .logo span {
    width: 5px;
    height: 5px;
    border-radius: 1px;
    background: var(--color-on-accent);
  }
  .spacer {
    flex: 1;
  }
</style>

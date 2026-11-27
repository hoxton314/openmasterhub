<!-- Settings area (S28): a list of sections on the left, the chosen one on the right. The section is the route (`#settings/appearance`). -->
<script lang="ts">
  import { Info, Keyboard, Palette, Power, FileText, Bell } from 'lucide-svelte';
  import { ui } from '../../lib/ui.svelte';
  import Layout from '../shell/Layout.svelte';
  import type { Icon } from '../ui/icon';
  import About from './About.svelte';
  import Appearance from './Appearance.svelte';
  import ConfigFile from './ConfigFile.svelte';
  import Feedback from './Feedback.svelte';
  import Service from './Service.svelte';
  import Shortcuts from './Shortcuts.svelte';

  const SECTIONS: { id: string; label: string; icon: Icon }[] = [
    { id: 'appearance', label: 'Appearance', icon: Palette },
    { id: 'service', label: 'Service', icon: Power },
    { id: 'config', label: 'Config file', icon: FileText },
    { id: 'shortcuts', label: 'Keyboard shortcuts', icon: Keyboard },
    { id: 'feedback', label: 'Feedback', icon: Bell },
    { id: 'about', label: 'About', icon: Info },
  ];
  const current = $derived(SECTIONS.find((s) => s.id === ui.route?.view)?.id ?? 'appearance');
</script>

<Layout>
  {#snippet sidebar({ collapsed })}
    <nav class="nav" aria-label="Settings sections">
      {#if !collapsed}<h1 class="t-heading">Settings</h1>{/if}
      {#each SECTIONS as s (s.id)}
        <button type="button" class:active={current === s.id} aria-current={current === s.id ? 'page' : undefined} title={s.label} onclick={() => ui.open({ view: s.id })}>
          <s.icon size={16} />
          {#if !collapsed}<span>{s.label}</span>{/if}
        </button>
      {/each}
    </nav>
  {/snippet}
  {#snippet main()}
    <div class="content">
      {#if current === 'appearance'}<Appearance />
      {:else if current === 'service'}<Service />
      {:else if current === 'config'}<ConfigFile />
      {:else if current === 'shortcuts'}<Shortcuts />
      {:else if current === 'feedback'}<Feedback />
      {:else}<About />{/if}
    </div>
  {/snippet}
</Layout>

<style>
  .nav {
    display: grid;
    align-content: start;
    gap: 2px;
    padding: var(--space-3);
    width: 100%;
  }
  h1 {
    margin: 0;
    padding: var(--space-2) var(--space-3) var(--space-3);
  }
  button {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    height: var(--size-row-sidebar);
    padding: 0 var(--space-3);
    border: 0;
    border-radius: var(--radius-sidebar-row);
    background: none;
    color: var(--idle-item-fg);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  button:hover {
    background: var(--color-selection);
  }
  button.active {
    background: var(--active-item-bg);
    color: var(--active-item-fg);
  }
  .content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-8) var(--space-8);
  }
</style>

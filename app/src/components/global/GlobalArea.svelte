<!-- Global area (S26): everything above every setup and profile. `#global/audio-groups` opens the groups page (S27). -->
<script lang="ts">
  import { ChevronRight, Volume2 } from 'lucide-svelte';
  import { hub } from '../../lib/hub.svelte';
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import AudioGroups from './AudioGroups.svelte';
  import Backlight from './Backlight.svelte';
  import LedsStatus from './LedsStatus.svelte';
  import PinnedKeys from './PinnedKeys.svelte';

  const groups = $derived(hub.config.audio_group ?? []);
  const summary = (g: { apps?: string[]; steam?: boolean }) => {
    const n = g.apps?.length ?? 0;
    return [n ? `${n} app${n === 1 ? '' : 's'}` : '', g.steam ? 'Steam' : ''].filter(Boolean).join(' + ') || 'empty';
  };
</script>

{#if ui.route?.view === 'audio-groups'}
  <AudioGroups />
{:else}
  <div class="scroll"><div class="page">
    <header>
      <h1 class="t-title">Global</h1>
      <p class="t-small muted">Above every setup and profile</p>
    </header>
    <PinnedKeys />
    <LedsStatus />
    <Backlight />
    <section>
      <div class="head">
        <SectionLabel as="h2">Audio groups</SectionLabel>
        <Button size="sm" iconRight={ChevronRight} onclick={() => ui.open({ view: 'audio-groups' })}>Manage</Button>
      </div>
      {#each groups as g (g.name)}
        <button type="button" class="group" onclick={() => ui.open({ view: 'audio-groups' })}>
          <Volume2 size={16} />
          <b>{g.name}</b>
          <span class="t-small muted">{summary(g)}</span>
        </button>
      {:else}
        <p class="t-small faint">No groups yet. A group lets one knob control several apps.</p>
      {/each}
    </section>
  </div></div>
{/if}

<style>
  .scroll {
    flex: 1;
    overflow-y: auto;
  }
  .page {
    padding: var(--space-7) var(--space-8);
    display: grid;
    align-content: start;
    gap: var(--space-7);
    max-width: 760px;
  }
  header {
    display: grid;
    gap: 2px;
  }
  h1,
  p {
    margin: 0;
  }
  section {
    display: grid;
    gap: var(--space-2);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .group {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    font: inherit;
    text-align: left;
    color: var(--color-text-1);
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-card);
    cursor: pointer;
  }
  .group:hover {
    border-color: var(--color-border-2);
  }
</style>

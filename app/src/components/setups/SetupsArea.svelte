<!-- Setups (S14–S18): the overview of every setup, and one setup in detail. -->
<script lang="ts">
  import { Plus, RefreshCw } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import { games, refreshGames } from '../../lib/games.svelte';
  import { hub } from '../../lib/hub.svelte';
  import { effectiveSetups, setupByName } from '../../lib/model';
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import Layout from '../shell/Layout.svelte';
  import AreaHeader from './AreaHeader.svelte';
  import NewSetup from './NewSetup.svelte';
  import SetupCard from './SetupCard.svelte';
  import SetupDetail from './SetupDetail.svelte';
  import SetupTriggers from './SetupTriggers.svelte';

  const all = $derived(effectiveSetups(hub.config));
  const activeName = $derived(hub.state.setup ?? all[0]?.name);
  const detail = $derived(ui.route?.view === 'setup-detail' ? (setupByName(hub.config, String(ui.route.setup ?? '')) ?? all[0]) : undefined);

  let creating = $state(false);
  let triggers = $state(false);
  onMount(() => {
    if (!games.list.length) refreshGames();
  });

  const open = (name: string) => ui.open({ view: 'setup-detail', setup: name });
  function created(name: string) {
    creating = false;
    open(name);
  }
</script>

{#if detail}
  <SetupDetail setup={detail} onnew={() => (creating = true)} onselect={open} />
{:else}
  <Layout>
    {#snippet main()}
      <AreaHeader title="Setups" subtitle="A setup is a set of profiles plus how they switch. One is active at a time.">
        {#snippet actions()}<Button variant="primary" icon={Plus} onclick={() => (creating = true)}>New setup</Button>{/snippet}
      </AreaHeader>
      <div class="scroll">
        <div class="cards">
          {#each all as s (s.name)}
            <SetupCard setup={s} active={s.name === activeName} selected={s.name === activeName} onopen={() => open(s.name)} />
          {/each}
          <section class="new" aria-label="New setup">
            <h3 class="t-heading">New setup</h3>
            <p class="t-small muted">Start from a template or from scratch.</p>
            <NewSetup onpick={created} />
          </section>
        </div>

        <aside class="note">
          <RefreshCw size={18} />
          <div class="text">
            <b>Switching between setups</b>
            <p class="t-small muted">Shortcuts, the tray and the palette switch at once. From the hub it takes a long-press on the dial, then a press to confirm, so it never happens by accident.</p>
          </div>
          <Button onclick={() => (triggers = true)}>Edit setup triggers</Button>
        </aside>
      </div>
    {/snippet}
  </Layout>
{/if}

{#if creating}
  <Dialog title="New setup" subtitle="Start from a template or from scratch." width={460} onclose={() => (creating = false)}>
    <NewSetup onpick={created} />
  </Dialog>
{/if}
{#if triggers}<SetupTriggers onclose={() => (triggers = false)} />{/if}

<style>
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    align-content: start;
    gap: 20px;
    padding: 28px 32px 32px;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
    gap: 16px;
    align-items: stretch;
  }
  .new {
    display: grid;
    align-content: start;
    gap: 8px;
    padding: 18px;
    border: 1px dashed var(--color-border-2);
    border-radius: var(--radius-card);
  }
  .new :global(.list) {
    margin-top: 6px;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 18px;
    background: var(--color-surface);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-hairline);
  }
  .note :global(svg) {
    flex: none;
    color: var(--color-text-2);
  }
  .text {
    flex: 1;
  }
</style>

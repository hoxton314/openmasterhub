<!-- A game mode's setup plan (S23, replaces GameDialog): folder, files, artwork, controls, and
  the approval. Nothing happens in the game folder until "Approve and turn on". -->
<script lang="ts">
  import { Check, FileDown, FolderOpen, ImageIcon, PackagePlus } from 'lucide-svelte';
  import { invoke, pickFolder } from '../../lib/backend';
  import { games, refreshGames } from '../../lib/games.svelte';
  import { editProfile, hub, toast } from '../../lib/hub.svelte';
  import { gameControls, gameError, gameState, integ, refreshBridge } from '../../lib/integrations.svelte';
  import { ui } from '../../lib/ui.svelte';
  import TemplatePreview from '../profiles/TemplatePreview.svelte';
  import type { GamePlan, GameStatus, GameStep } from '../../lib/types';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import Kbd from '../ui/Kbd.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import IntegrationStates from './IntegrationStates.svelte';

  let { game }: { game: GameStatus } = $props();

  /** The folder the user chose; empty = the configured or detected one. */
  let chosen = $state('');
  let plan = $state<GamePlan | null>(null);
  let busy = $state(false);

  const st = $derived(gameState(game));
  const needsFolder = $derived(game.game !== 'crimson_desert');
  const files = $derived(plan?.steps.filter((s) => s.kind === 'cet_bridge' || s.kind === 'deadlock_cfg' || s.kind === 'wow_addon') ?? []);
  const artwork = $derived(plan?.steps.filter((s) => s.kind === 'deadlock_icons') ?? []);
  const ctl = $derived(gameControls(game.game));
  const appClass = $derived(game.steam ? `steam_app_${game.steam}` : null);
  const profile = $derived(hub.config.profile?.find((p) => appClass && p.apps?.includes(appClass)));

  // A different game starts over.
  $effect(() => {
    game.game;
    chosen = '';
    plan = null;
    load();
  });

  async function load() {
    const id = game.game;
    plan = null;
    const p = await invoke<GamePlan>('games_plan', { game: id, path: chosen || null }).catch(
      (e): GamePlan => ({ game: id, title: game.title, path: chosen || null, detected: false, steps: [], notes: [], error: String(e) }),
    );
    if (game.game === id) plan = p;
  }

  async function browse() {
    const dir = await pickFolder(`Where is ${game.title} installed?`);
    if (!dir) return;
    chosen = dir;
    await load();
  }

  async function approve() {
    if (!plan || plan.error) return;
    busy = true;
    delete integ.errors[game.game];
    try {
      // The folder exactly as planned; a detected one is left for setup to find again.
      await invoke('games_setup', { game: game.game, path: plan.detected ? null : plan.path });
      toast({ text: `${game.title} game mode is on`, kind: 'success' });
      await refreshGames();
      await refreshBridge();
      ui.go('library', null);
    } catch (e) {
      integ.errors[game.game] = String(e).replace(/^Error: /, '');
      await load();
    }
    busy = false;
  }

  async function turnOff() {
    busy = true;
    try {
      await invoke('games_disable', { game: game.game });
      delete integ.errors[game.game];
      toast(`${game.title} game mode is off`);
      await refreshGames();
      await refreshBridge();
    } catch (e) {
      toast({ text: String(e), kind: 'danger' });
    }
    busy = false;
  }

  async function retry() {
    await refreshBridge();
  }

  function step(s: GameStep): { icon: typeof Check; text: string; paths: string[] } {
    switch (s.kind) {
      case 'cet_bridge':
        return { icon: PackagePlus, text: 'Install the bridge mod (Cyber Engine Tweaks) into', paths: [s.path] };
      case 'deadlock_cfg':
        return { icon: PackagePlus, text: 'Add pulpit.cfg and an “exec pulpit” line to autoexec.cfg in', paths: [s.path] };
      case 'deadlock_icons':
        return { icon: FileDown, text: 'Extract the game’s icons from your copy and keep them on this computer only', paths: [s.from, s.to] };
      case 'wow_addon':
        return { icon: PackagePlus, text: 'Install the PULPIT add-on into', paths: [s.path] };
      case 'enable':
        return { icon: Check, text: 'Turn the game mode on', paths: [] };
    }
  }

  const chipVariant = $derived(({ off: 'off', needs_setup: 'warning', on: 'success', running: 'success', bridge_down: 'warning', error: 'danger', checking: 'neutral' } as const)[st.state]);
</script>

<div class="page">
  <header>
    <div class="head">
      <h1 class="t-title">{game.title} mode</h1>
      <Chip variant={chipVariant}>{st.label}</Chip>
      {#if artwork.length}<span class="art"><Chip variant="outline" icon={ImageIcon}>artwork from your install</Chip></span>{/if}
    </div>
    <p class="t-body muted">{game.about}</p>
  </header>

  <div class="cols">
  <section class="plan" aria-label="Setup plan">
    <div class="intro">
      <SectionLabel as="h2">Setup plan</SectionLabel>
      <p class="t-small muted">{game.enabled ? 'The mode is on. Setting it up again redoes these steps, for example after a game update.' : 'Nothing changes in your game folder until you approve.'}</p>
    </div>

    {#if needsFolder}
      <div class="block">
        <h3 class="t-heading"><span class="n">1</span>Game folder</h3>
        <div class="line">
          <code class="path t-mono grow">{plan?.path ?? chosen ?? ''}{#if !plan?.path && plan}<span class="faint">Not found: choose the folder</span>{/if}</code>
          {#if plan?.detected && plan.path}<Chip variant="outline" icon={null}>Found via Steam</Chip>{/if}
          <Button size="sm" icon={FolderOpen} onclick={browse} disabled={busy}>Browse…</Button>
        </div>
      </div>
    {/if}

    {#if plan && !plan.error}
      {#if files.length}
        <div class="block">
          <h3 class="t-heading"><span class="n">{needsFolder ? 2 : 1}</span>Files it will put into the game</h3>
          {#each files as s}
            {@const l = step(s)}
            <div class="line"><l.icon size={14} /><span class="grow t-body">{l.text} {#each l.paths as p}<code class="t-mono">{p}</code>{/each}</span></div>
          {/each}
        </div>
      {/if}
      {#if artwork.length}
        <div class="block">
          <h3 class="t-heading"><span class="n">{(needsFolder ? 2 : 1) + (files.length ? 1 : 0)}</span>Artwork it will extract from your install</h3>
          {#each artwork as s}
            {@const l = step(s)}
            <div class="line"><l.icon size={14} /><span class="grow t-body">{l.text}: <code class="t-mono">{l.paths[0]}</code> into <code class="t-mono">{l.paths[1]}</code></span></div>
          {/each}
          <p class="t-small muted">Stored in your config folder. Nothing from the game ships with OpenMasterHUB.</p>
        </div>
      {/if}
      <div class="block">
        <h3 class="t-heading"><span class="n">{[needsFolder, files.length, artwork.length].filter(Boolean).length + 1}</span>Controls it takes over</h3>
        <div class="line wrap">
          {#each ctl.takes as c}<Chip variant="integration">{c}</Chip>{/each}
          {#each ctl.stays as c}<Chip variant="outline" icon={null}>{c} stay yours</Chip>{/each}
        </div>
        {#if appClass}
          <p class="t-small muted">Only while <Kbd>{appClass}</Kbd> is focused. Your own profiles come back when you leave the game.</p>
        {/if}
      </div>
      {#each plan.notes as n}<p class="t-small note">{n}</p>{/each}
    {/if}

    <div class="actions">
      {#if game.enabled}
        <Button id="approve" variant="primary" disabled={busy || !plan || !!plan.error} onclick={approve}>{busy ? 'Setting up…' : 'Set up again'}</Button>
        <Button variant="danger" disabled={busy} onclick={turnOff}>Turn off</Button>
      {:else}
        <Button id="approve" variant="primary" disabled={busy || !plan || !!plan.error} onclick={approve}>{busy ? 'Setting up…' : 'Approve and turn on'}</Button>
        <Button variant="ghost" disabled={busy} onclick={() => ui.go('library', null)}>Not now</Button>
      {/if}
    </div>
  </section>

  <aside class="side" aria-label="States and profiles">
    <section class="col" aria-label="States">
      <SectionLabel as="h2">States</SectionLabel>
      <IntegrationStates
        state={st.state}
        {plan}
        error={gameError(game)}
        onbrowse={browse}
        onretry={retry}
        onsetup={() => document.getElementById('approve')?.focus()}
        onturnoff={turnOff}
      />
    </section>
    {#if profile}
      <section class="col" aria-label="Profiles it provides">
        <SectionLabel as="h2">Profiles it provides</SectionLabel>
        <div class="provided">
          <div class="thumb" aria-hidden="true"><TemplatePreview {profile} unit={9} /></div>
          <div class="grow">
            <div class="t-heading name">{profile.name}</div>
            <div class="t-small muted">Keypad and dial drawn live while the game is focused</div>
          </div>
          <Button size="sm" onclick={() => (editProfile(profile.name), ui.go('editor'))}>Open in editor</Button>
        </div>
      </section>
    {/if}
  </aside>
  </div>
</div>

<style>
  .page {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 24px;
    box-sizing: border-box;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .art {
    margin-left: auto;
  }
  .cols {
    display: flex;
    align-items: flex-start;
    gap: 28px;
    flex-wrap: wrap;
  }
  .cols > .plan {
    flex: 1 1 460px;
    min-width: 0;
  }
  .side {
    flex: 0 1 400px;
    min-width: 280px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .provided {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: var(--color-surface);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-hairline);
  }
  .thumb {
    flex: none;
    max-width: 120px;
    overflow: hidden;
    border-radius: var(--radius-control);
  }
  .name {
    color: var(--color-text-1);
  }
  h1,
  h3,
  p {
    margin: 0;
  }
  .plan {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px;
    background: var(--color-surface);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-hairline);
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 12px;
    border-top: var(--panel-border-width) solid var(--color-border);
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--color-text-1);
  }
  .n {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: var(--radius-pill);
    background: var(--chip-accent-bg);
    color: var(--chip-accent-fg);
    font-size: 11px;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--color-text-2);
  }
  .line.wrap {
    flex-wrap: wrap;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  code {
    display: inline-block;
    max-width: 100%;
    overflow-wrap: anywhere;
    padding: 2px 6px;
    margin: 0 2px;
    color: var(--color-text-1);
    background: var(--color-sunken);
    border-radius: var(--radius-sm);
  }
  .path {
    padding: 6px 10px;
    border-radius: var(--radius-control);
  }
  .note {
    color: var(--color-warning);
  }
  .actions {
    display: flex;
    gap: 8px;
    padding-top: 4px;
  }
</style>

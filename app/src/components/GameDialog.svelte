<script lang="ts">
  import { Check, FileDown, FolderOpen, Gamepad2, PackagePlus, TriangleAlert } from 'lucide-svelte';
  import { invoke, pickFolder } from '../lib/backend';
  import { games, refreshGames } from '../lib/games.svelte';
  import { toast } from '../lib/hub.svelte';
  import type { GamePlan, GameStep } from '../lib/types';

  const g = $derived(games.open);
  /** The folder the user chose; empty = the configured or detected one. */
  let chosen = $state('');
  let plan = $state<GamePlan | null>(null);
  let busy = $state(false);

  // A new game opens with its own folder.
  $effect(() => {
    if (g) {
      chosen = '';
      plan = null;
      load();
    }
  });

  async function load() {
    if (!g) return;
    const game = g.game;
    const p = await invoke<GamePlan>('games_plan', { game, path: chosen || null }).catch((e) => ({
      game,
      title: g!.title,
      path: chosen || null,
      detected: false,
      steps: [],
      notes: [],
      error: String(e),
    }));
    if (games.open?.game === game) plan = p;
  }

  async function browse() {
    const dir = await pickFolder(`Where is ${g?.title} installed?`);
    if (!dir) return;
    chosen = dir;
    await load();
  }

  function close() {
    if (!busy) games.open = null;
  }

  async function approve() {
    if (!g || !plan || plan.error) return;
    busy = true;
    try {
      // The folder exactly as planned; a detected one is left for setup to find again.
      await invoke('games_setup', { game: g.game, path: plan.detected ? null : plan.path });
      toast(`${g.title} game mode is on`);
      await refreshGames();
      busy = false;
      games.open = null;
    } catch (e) {
      busy = false;
      toast(String(e));
      await load();
    }
  }

  async function turnOff() {
    if (!g) return;
    busy = true;
    try {
      await invoke('games_disable', { game: g.game });
      toast(`${g.title} game mode is off`);
      await refreshGames();
      games.open = null;
    } catch (e) {
      toast(String(e));
    }
    busy = false;
  }

  function label(s: GameStep): { icon: typeof Check; text: string; paths: string[] } {
    switch (s.kind) {
      case 'cet_bridge':
        return { icon: PackagePlus, text: 'Install the bridge mod (Cyber Engine Tweaks) into', paths: [s.path] };
      case 'deadlock_cfg':
        return { icon: PackagePlus, text: 'Add pulpit.cfg and an “exec pulpit” line to autoexec.cfg in', paths: [s.path] };
      case 'deadlock_icons':
        return { icon: FileDown, text: 'Extract the game’s icons from your copy, kept on this computer only', paths: [s.from, s.to] };
      case 'wow_addon':
        return { icon: PackagePlus, text: 'Install the PULPIT add-on into', paths: [s.path] };
      case 'enable':
        return { icon: Check, text: 'Turn the game mode on (it takes over the keys while the game has focus)', paths: [] };
    }
  }

  const needsFolder = $derived(g?.game !== 'crimson_desert');
</script>

{#if g}
  <div class="scrim" role="presentation" onclick={close}>
    <div class="dialog" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.key === 'Escape' && close()}>
      <div class="head">
        <div class="badge"><Gamepad2 size={18} /></div>
        <div>
          <h2>{g.title}</h2>
          <span class="state" class:on={g.enabled}>{g.enabled ? 'Game mode on' : 'Game mode off'}</span>
        </div>
      </div>
      <p>{g.about}</p>

      {#if needsFolder}
        <div class="folder">
          <span class="lbl">Game folder</span>
          <div class="row">
            <input class="input mono" readonly value={plan?.path ?? chosen} placeholder="Not found: choose the folder" />
            <button class="btn" onclick={browse} disabled={busy}><FolderOpen size={14} />Browse…</button>
          </div>
          {#if plan?.detected && plan.path}<span class="hint">Found through Steam</span>{/if}
        </div>
      {/if}

      {#if !plan}
        <p class="hint">Checking…</p>
      {:else if plan.error}
        <div class="note warn"><TriangleAlert size={14} /><span>{plan.error}</span></div>
      {:else}
        <div class="steps">
          <span class="lbl">Approve to do this:</span>
          <ol>
            {#each plan.steps as s}
              {@const l = label(s)}
              <li>
                <l.icon size={14} />
                <span>
                  {l.text}
                  {#each l.paths as p, i}
                    {#if i > 0}<span class="arrow">into</span>{/if}
                    <code>{p}</code>
                  {/each}
                </span>
              </li>
            {/each}
          </ol>
        </div>
        {#each plan.notes as n}<div class="note"><TriangleAlert size={14} /><span>{n}</span></div>{/each}
        <p class="hint">Nothing from the game is shared or uploaded: OpenMasterHUB ships no game files and reads the look from your install.</p>
      {/if}

      <div class="actions">
        {#if g.enabled}<button class="btn ghost danger-text" onclick={turnOff} disabled={busy}>Turn off</button>{/if}
        <span class="spacer"></span>
        <button class="btn ghost" onclick={close} disabled={busy}>Cancel</button>
        <button class="btn primary" onclick={approve} disabled={busy || !plan || !!plan.error}>
          {busy ? 'Setting up…' : g.enabled ? 'Set up again' : 'Approve and turn on'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: start center;
    padding-top: 12vh;
    background: rgb(0 0 0 / 0.5);
    animation: fade-in 0.12s ease-out;
  }
  .dialog {
    width: 520px;
    max-width: calc(100vw - 32px);
    padding: 18px;
    display: grid;
    gap: 12px;
    background: var(--bg-raised);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-pop);
    animation: pop 0.14s var(--ease);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .badge {
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    border-radius: var(--radius);
    color: var(--accent-text);
    background: var(--accent-soft);
  }
  h2 {
    font-size: 15px;
    font-weight: 600;
  }
  .state {
    font-size: 12px;
    color: var(--text-3);
  }
  .state.on {
    color: var(--success);
  }
  p {
    color: var(--text-2);
  }
  .lbl {
    font-size: 12px;
    color: var(--text-2);
  }
  .folder {
    display: grid;
    gap: 6px;
  }
  .row {
    display: flex;
    gap: 6px;
  }
  .row .input {
    flex: 1;
    min-width: 0;
    height: 32px;
    font-size: 12px;
  }
  .hint {
    font-size: 12px;
    color: var(--text-3);
  }
  .steps {
    display: grid;
    gap: 6px;
  }
  ol {
    list-style: none;
    display: grid;
    gap: 8px;
    padding: 10px 12px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  li {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    line-height: 1.45;
  }
  li :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
    color: var(--accent-text);
  }
  code {
    display: block;
    margin-top: 2px;
    font-size: 11.5px;
    color: var(--text-2);
    word-break: break-all;
  }
  .arrow {
    display: block;
    margin-top: 2px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .note {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .note :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
    color: var(--warning);
  }
  .note.warn {
    color: var(--text);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
  }
  .spacer {
    flex: 1;
  }
  .danger-text {
    color: var(--danger);
  }
  .actions .btn:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>

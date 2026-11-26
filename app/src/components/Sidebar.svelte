<script lang="ts">
  import { ChevronDown, Folder, Gamepad2, LayoutGrid, Ellipsis, Plus, UserRound } from 'lucide-svelte';
  import { games, refreshGames } from '../lib/games.svelte';
  import { currentPage, currentProfile, edit, hub, profileMut, save, setProfile, showPage, toast } from '../lib/hub.svelte';
  import { invoke } from '../lib/backend';
  import { renameProfile as renameProfileRefs, retarget } from '../lib/model/rename';
  import { ui } from '../lib/ui.svelte';
  import type { GameStatus, Profile } from '../lib/types';

  const profiles = $derived(hub.config.profile ?? []);

  /** A Steam game's window (`steam_app_123`), not Steam itself. */
  const STEAM_GAME = /^steam_app_\d/i;
  const isGame = (p: Profile, i: number) => i > 0 && (p.game ?? (p.apps ?? []).some((a) => STEAM_GAME.test(a)));
  /** The ready-made game mode for the same Steam game as profile `p`. */
  const modeOf = (p: Profile) =>
    games.list.find((g) => g.steam != null && (p.apps ?? []).some((a) => a.toLowerCase() === `steam_app_${g.steam}`));
  const listed = $derived(profiles.map((p, i) => ({ p, i, game: isGame(p, i) })));
  const plain = $derived(listed.filter((x) => !x.game));
  const gameProfiles = $derived(listed.filter((x) => x.game));
  /** Game modes no profile stands for: listed on their own. */
  const looseModes = $derived(games.list.filter((g) => !gameProfiles.some((x) => modeOf(x.p)?.game === g.game)));

  /** `com.mitchellh.ghostty` → `ghostty`; exe names and globs stay as they are. */
  const pretty = (a: string) => (/\.exe$/i.test(a) || a.includes('*') ? a : a.split('.').pop() || a);
  function appsLine(p: Profile, i: number) {
    if (i === 0) return 'Default';
    const apps = p.apps ?? [];
    if (!apps.length) return 'Manual only';
    return apps.length > 1 ? `${pretty(apps[0])} +${apps.length - 1}` : pretty(apps[0]);
  }
  function gameLine(p: Profile, i: number, mode: GameStatus | undefined) {
    if (mode) return mode.enabled ? 'Game mode on' : 'Game mode available';
    const named = (p.apps ?? []).filter((a) => !STEAM_GAME.test(a));
    if (named.length < (p.apps ?? []).length) return 'Steam game';
    return appsLine(p, i);
  }

  /** Collapsed sections, remembered on this computer. */
  const FOLD_KEY = 'omh.sidebar.folded';
  let folded = $state<string[]>((() => {
    try {
      return JSON.parse(localStorage.getItem(FOLD_KEY) ?? '[]');
    } catch {
      return [];
    }
  })());
  function fold(id: string) {
    folded = folded.includes(id) ? folded.filter((x) => x !== id) : [...folded, id];
    try {
      localStorage.setItem(FOLD_KEY, JSON.stringify(folded));
    } catch {
      /* private mode */
    }
  }
  const active = $derived(currentProfile());
  const pages = $derived(active?.page ?? []);
  const folders = $derived(active?.folder ?? []);
  const page = $derived(currentPage()?.name);
  const moduleCount = $derived(hub.modules.filter((m) => m.kind !== 'base').length);

  $effect(() => {
    refreshGames();
  });

  let menu = $state(''); // e.g. "profile:Default", "page:Main", "add-page", "add-profile"
  const toggle = (id: string) => (e: MouseEvent) => {
    e.stopPropagation();
    menu = menu === id ? '' : id;
  };

  function uniqueName(base: string) {
    const taken = new Set([...pages, ...folders].map((x) => x.name));
    for (let i = 1; ; i++) if (!taken.has(`${base} ${i}`)) return `${base} ${i}`;
  }

  async function addPage(folder: boolean) {
    menu = '';
    const name = uniqueName(folder ? 'Folder' : 'Page');
    edit((c) => {
      const p = profileMut(c);
      (folder ? (p.folder ??= []) : (p.page ??= [])).push({ name });
    });
    await save();
    await showPage(name);
  }

  function renamePage(old: string) {
    menu = '';
    ui.ask({
      title: 'Rename',
      value: old,
      ok: 'Rename',
      done: async (name) => {
        if (!name || name === old) return;
        if ([...pages, ...folders].some((x) => x.name === name)) return toast('A page or folder with that name already exists');
        edit((c) => {
          const pr = profileMut(c);
          for (const pg of [...(pr.page ?? []), ...(pr.folder ?? [])]) if (pg.name === old) pg.name = name;
          retarget(c, (a) => ('page' in a && a.page === old) || ('folder' in a && a.folder === old), (a) => {
            if ('page' in a) a.page = name;
            if ('folder' in a) a.folder = name;
          });
        });
        await save();
        if (page === old) await showPage(name);
      },
    });
  }

  function movePage(name: string, delta: number) {
    menu = '';
    edit((c) => {
      const p = profileMut(c);
      for (const list of [p.page ?? [], p.folder ?? []]) {
        const i = list.findIndex((x) => x.name === name);
        if (i < 0) continue;
        const j = Math.max(0, Math.min(list.length - 1, i + delta));
        [list[i], list[j]] = [list[j], list[i]];
      }
    });
  }

  function deletePage(name: string) {
    menu = '';
    ui.confirm({
      title: `Delete “${name}”?`,
      body: 'Its keys are deleted too. This can’t be undone.',
      ok: 'Delete',
      done: async () => {
        edit((c) => {
          const p = profileMut(c);
          p.page = (p.page ?? []).filter((x) => x.name !== name);
          p.folder = (p.folder ?? []).filter((x) => x.name !== name);
        });
        await save();
        const first = currentProfile()?.page?.[0]?.name;
        if (first && page === name) await showPage(first);
      },
    });
  }

  function newProfile(game = false) {
    menu = '';
    ui.ask({
      title: game ? 'New game profile' : 'New profile',
      placeholder: game ? 'e.g. Elden Ring' : 'e.g. Streaming',
      ok: 'Create',
      done: async (name) => {
        if (!name || profiles.some((p) => p.name === name)) return toast('Choose a new, unique name');
        edit((c) => {
          c.profile ??= [];
          if (c.profile.length === 0) c.profile.push({ name: 'Default', page: [{ name: 'Main' }] });
          c.profile.push({ name, page: [{ name: 'Main' }], ...(game ? { game: true } : {}) });
        });
        await save();
        await setProfile(name);
        if (game) toast('Set the game’s window under “Activate for apps…” (Steam games: steam_app_<id>)');
      },
    });
  }

  function renameProfile(old: string) {
    menu = '';
    ui.ask({
      title: 'Rename profile',
      value: old,
      ok: 'Rename',
      done: async (name) => {
        if (!name || name === old) return;
        if (profiles.some((p) => p.name === name)) return toast('A profile with that name already exists');
        edit((c) => renameProfileRefs(c, old, name), { label: 'rename' });
        await save();
        if (active?.name === old || hub.state.profile === old) await setProfile(name);
      },
    });
  }

  async function profileApps(name: string) {
    menu = '';
    const p = profiles.find((x) => x.name === name);
    if (!p) return;
    if (profiles[0]?.name === name) return toast('The default profile applies when no other profile matches');
    const suggestions = await invoke<string[]>('open_window_classes').catch(() => []);
    ui.ask({
      title: `Apps for “${name}”`,
      body: 'Activates while a window with one of these classes is focused. Separate with commas; * is a wildcard.',
      value: (p.apps ?? []).join(', '),
      placeholder: 'obs, steam_app_*',
      suggestions,
      ok: 'Save',
      done: (v) =>
        edit((c) => {
          const target = (c.profile ?? []).find((x) => x.name === name);
          if (target) target.apps = v.split(',').map((s) => s.trim()).filter(Boolean);
        }),
    });
  }

  function setGame(name: string, game: boolean) {
    menu = '';
    edit((c) => {
      const p = (c.profile ?? []).find((x) => x.name === name);
      if (!p) return;
      const guess = (p.apps ?? []).some((a) => STEAM_GAME.test(a));
      // Only store it when it differs from the guess.
      p.game = game === guess ? undefined : game;
    });
  }

  function deleteProfile(name: string) {
    menu = '';
    if (profiles[0]?.name === name) return toast('The default profile can’t be deleted');
    ui.confirm({
      title: `Delete profile “${name}”?`,
      body: 'All its pages, folders and bindings are deleted.',
      ok: 'Delete',
      done: async () => {
        edit((c) => (c.profile = (c.profile ?? []).filter((x) => x.name !== name)));
        await save();
        if (active?.name === name) await setProfile(profiles[0].name);
      },
    });
  }
</script>

<svelte:window onclick={() => (menu = '')} onkeydown={(e) => e.key === 'Escape' && (menu = '')} />

<nav class="sidebar">
  <div class="brand">
    <div class="logo" aria-hidden="true">
      <svg viewBox="0 0 24 24"><rect x="2.5" y="4" width="19" height="16" rx="4" fill="none" stroke="currentColor" stroke-width="1.6" /><rect x="6" y="8" width="3" height="3" rx="0.8" fill="currentColor" /><rect x="10.5" y="8" width="3" height="3" rx="0.8" fill="currentColor" opacity="0.55" /><rect x="15" y="8" width="3" height="3" rx="0.8" fill="currentColor" opacity="0.55" /><circle cx="16.5" cy="15" r="1.8" fill="currentColor" /></svg>
    </div>
    <div class="name">
      <strong>OpenMasterHUB</strong>
      <span class="status" class:on={hub.online}><i></i>{hub.online ? `${moduleCount} module${moduleCount === 1 ? '' : 's'}` : 'Service offline'}</span>
    </div>
  </div>

  <div class="scroller">
    {#snippet head(id: string, label: string, count: number, add?: () => void, addTitle?: string)}
      <div class="section-head">
        <button class="fold" aria-expanded={!folded.includes(id)} onclick={() => fold(id)}>
          <ChevronDown size={13} class="chev" />
          <span>{label}</span>
          {#if folded.includes(id)}<span class="n">{count}</span>{/if}
        </button>
        {#if add}<button class="btn ghost sm icon" title={addTitle} onclick={add}><Plus size={14} /></button>{/if}
      </div>
    {/snippet}

    {#snippet profileRow(p: Profile, i: number, game: boolean)}
      {@const mode = game ? modeOf(p) : undefined}
      <div class="item-wrap">
        <button class="item" class:active={p.name === active?.name} onclick={() => setProfile(p.name)} title={i === 0 ? 'Applies when no other profile matches the focused window' : p.apps?.length ? `Follows: ${p.apps.join(', ')}` : 'Only when you switch to it'}>
          {#if game}<Gamepad2 size={14} class={mode?.enabled ? 'on' : ''} />{:else}<UserRound size={14} />{/if}
          <span class="text">
            <span class="title">{p.name}</span>
            <span class="sub">{game ? gameLine(p, i, mode) : appsLine(p, i)}</span>
          </span>
          {#if hub.state.profile === p.name}<span class="live" title="Active on the hub"></span>{/if}
        </button>
        <button class="btn ghost sm icon more" title="More" aria-haspopup="menu" onclick={toggle(`profile:${p.name}`)}><Ellipsis size={14} /></button>
        {#if menu === `profile:${p.name}`}
          <div class="menu" style="right:6px;top:34px" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
            <button onclick={() => renameProfile(p.name)}>Rename…</button>
            {#if i > 0}<button onclick={() => profileApps(p.name)}>Activate for apps…</button>{/if}
            {#if mode}<button onclick={() => { menu = ''; games.open = mode; }}><Gamepad2 size={14} />Game mode…</button>{/if}
            {#if i > 0}<button onclick={() => setGame(p.name, !game)}>{game ? 'Move to Profiles' : 'Move to Games'}</button>{/if}
            {#if i > 0}<hr /><button class="danger" onclick={() => deleteProfile(p.name)}>Delete</button>{/if}
          </div>
        {/if}
      </div>
    {/snippet}

    <div class="section">
      {@render head('profiles', 'Profiles', plain.length, () => newProfile(), 'New profile')}
      {#if !folded.includes('profiles')}
        {#each plain as { p, i } (p.name)}{@render profileRow(p, i, false)}{/each}
      {/if}
    </div>

    {#if gameProfiles.length || games.list.length}
      <div class="section">
        {@render head('games', 'Games', gameProfiles.length + looseModes.length, () => newProfile(true), 'New game profile')}
        {#if !folded.includes('games')}
          {#each gameProfiles as { p, i } (p.name)}{@render profileRow(p, i, true)}{/each}
          {#if looseModes.length}
            <div class="divider-label">Ready-made game modes</div>
            {#each looseModes as g (g.game)}
              <button class="item" onclick={() => (games.open = g)} title={g.about}>
                <Gamepad2 size={14} class={g.enabled ? 'on' : ''} />
                <span class="text">
                  <span class="title">{g.title}</span>
                  <span class="sub">{g.enabled ? (g.assets ? 'Game mode on' : 'Game mode on · stand-in look') : 'Off · click to set up'}</span>
                </span>
              </button>
            {/each}
          {/if}
        {/if}
      </div>
    {/if}

    <div class="section">
      <div class="section-head">
        <button class="fold" aria-expanded={!folded.includes('pages')} onclick={() => fold('pages')}>
          <ChevronDown size={13} class="chev" />
          <span>Keypad pages</span>
          {#if folded.includes('pages')}<span class="n">{pages.length + folders.length}</span>{/if}
        </button>
        <div class="anchor">
          <button class="btn ghost sm icon" title="Add page or folder" aria-haspopup="menu" onclick={toggle('add-page')}><Plus size={14} /></button>
          {#if menu === 'add-page'}
            <div class="menu" style="right:0;top:28px" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
              <button onclick={() => addPage(false)}><LayoutGrid size={14} />New page</button>
              <button onclick={() => addPage(true)}><Folder size={14} />New folder</button>
            </div>
          {/if}
        </div>
      </div>
      {#if !folded.includes('pages')}
        {#each [...pages.map((p) => ({ p, folder: false })), ...folders.map((p) => ({ p, folder: true }))] as { p, folder }, i (p.name)}
          {#if folder && i === pages.length}<div class="divider-label">Folders</div>{/if}
          <div class="item-wrap">
            <button class="item" class:active={p.name === page} onclick={() => showPage(p.name)}>
              {#if folder}<Folder size={14} />{:else}<LayoutGrid size={14} />{/if}
              <span class="text"><span class="title">{p.name}</span></span>
              <span class="count" title="Keys set">{p.key?.length ?? 0}</span>
            </button>
            <button class="btn ghost sm icon more" title="More" aria-haspopup="menu" onclick={toggle(`page:${p.name}`)}><Ellipsis size={14} /></button>
            {#if menu === `page:${p.name}`}
              <div class="menu" style="right:6px;top:30px" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
                <button onclick={() => renamePage(p.name)}>Rename…</button>
                <button onclick={() => movePage(p.name, -1)}>Move up</button>
                <button onclick={() => movePage(p.name, 1)}>Move down</button>
                <hr />
                <button class="danger" onclick={() => deletePage(p.name)}>Delete</button>
              </div>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  </div>
</nav>

<style>
  .sidebar {
    width: 236px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    overflow: hidden;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 14px 10px;
  }
  .logo {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    color: var(--accent-text);
    background: var(--accent-soft);
  }
  .logo svg {
    width: 20px;
    height: 20px;
  }
  .name {
    display: grid;
    line-height: 1.25;
  }
  .name strong {
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-2);
  }
  .status i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--danger);
  }
  .status.on i {
    background: var(--success);
    box-shadow: 0 0 0 3px rgb(76 183 130 / 0.18);
  }
  .scroller {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-bottom: 12px;
    scrollbar-gutter: stable;
  }
  .section {
    padding: 8px 4px 4px 8px;
    display: grid;
    gap: 1px;
    align-content: start;
  }
  .section-head {
    position: sticky;
    top: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 30px;
    padding: 0 2px 0 2px;
    background: var(--bg-sidebar);
  }
  .fold {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 6px;
    font: inherit;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-2);
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .fold:hover {
    color: var(--text);
    background: var(--bg-hover);
  }
  .fold :global(.chev) {
    color: var(--text-3);
    transition: transform 0.15s var(--ease);
  }
  .fold[aria-expanded='false'] :global(.chev) {
    transform: rotate(-90deg);
  }
  .n {
    font-size: 11px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .item :global(svg.on) {
    color: var(--success);
    opacity: 1;
  }
  .divider-label {
    padding: 10px 8px 4px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .anchor,
  .item-wrap {
    position: relative;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    min-height: 30px;
    padding: 5px 34px 5px 8px;
    font: inherit;
    color: var(--text-2);
    text-align: left;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }
  .item :global(svg) {
    flex-shrink: 0;
    opacity: 0.8;
  }
  .item:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .item.active {
    background: var(--bg-active);
    color: var(--text);
  }
  .text {
    display: grid;
    min-width: 0;
    flex: 1;
    line-height: 1.3;
  }
  .title {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    font-size: 11.5px;
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    font-size: 11.5px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .live {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
  .more {
    position: absolute;
    right: 4px;
    top: 50%;
    transform: translateY(-50%);
    opacity: 0;
  }
  .item-wrap:hover .more,
  .more:focus-visible {
    opacity: 1;
  }
  .item-wrap:hover .count {
    visibility: hidden;
  }
</style>

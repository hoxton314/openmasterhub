<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import Hub from './components/Hub.svelte';
  import Inspector from './components/Inspector.svelte';
  import { connect, currentPage, currentProfile, edit, hub, profileMut, save, setProfile, showPage, toast } from './lib/hub.svelte';
  import type { Action, Config, Page } from './lib/types';

  connect();

  const profiles = $derived((hub.config.profile ?? []).map((p) => p.name));
  const pages = $derived(currentProfile()?.page ?? []);
  const folders = $derived(currentProfile()?.folder ?? []);
  const page = $derived(currentPage()?.name);
  const moduleCount = $derived(hub.modules.filter((m) => m.kind !== 'base').length);

  // ───────────────────────────── dialogs ─────────────────────────────
  let dialog = $state<null | { title: string; body?: string; value: string; suggestions?: string[]; ok: string; danger?: boolean; input: boolean; done: (v: string) => void }>(null);
  const ask = (title: string, value: string, done: (v: string) => void) => (dialog = { title, value, ok: 'OK', input: true, done });
  const confirm = (title: string, body: string, done: () => void) => (dialog = { title, body, value: '', ok: 'Delete', danger: true, input: false, done: () => done() });

  let menu = $state<'' | 'profile' | 'add' | 'page'>('');
  const closeMenu = () => (menu = '');

  // ───────────────────────────── pages & folders ─────────────────────────────
  function uniqueName(base: string) {
    const p = currentProfile();
    const taken = new Set([...(p?.page ?? []), ...(p?.folder ?? [])].map((x) => x.name));
    for (let i = 1; ; i++) if (!taken.has(`${base} ${i}`)) return `${base} ${i}`;
  }

  async function addPage(folder: boolean) {
    closeMenu();
    const name = uniqueName(folder ? 'Folder' : 'Page');
    edit((c) => {
      const p = profileMut(c);
      (folder ? (p.folder ??= []) : (p.page ??= [])).push({ name });
    });
    await save();
    await showPage(name);
  }

  /** Rename references in actions when a page/folder/profile is renamed. */
  function retarget(c: Config, pred: (a: Action) => boolean, set: (a: Action) => void) {
    for (const p of c.profile ?? []) {
      const actions: (Action | undefined)[] = [];
      for (const pg of [...(p.page ?? []), ...(p.folder ?? [])]) for (const k of pg.key ?? []) actions.push(k.press, k.release);
      for (const k of p.knob ?? []) actions.push(k.cw, k.ccw, k.press, k.release);
      if (p.dial) actions.push(p.dial.cw, p.dial.ccw, p.dial.press, p.dial.release);
      for (const a of actions) if (a && pred(a)) set(a);
    }
  }

  function renamePage() {
    closeMenu();
    const old = page;
    if (!old) return;
    ask('Rename', old, async (name) => {
      if (!name || name === old) return;
      const p = currentProfile();
      if ([...(p?.page ?? []), ...(p?.folder ?? [])].some((x) => x.name === name)) return toast('A page or folder with that name already exists');
      edit((c) => {
        const pr = profileMut(c);
        for (const pg of [...(pr.page ?? []), ...(pr.folder ?? [])]) if (pg.name === old) pg.name = name;
        retarget(c, (a) => ('page' in a && a.page === old) || ('folder' in a && a.folder === old), (a) => {
          if ('page' in a) a.page = name;
          if ('folder' in a) a.folder = name;
        });
      });
      await save();
      await showPage(name);
    });
  }

  async function movePage(delta: number) {
    closeMenu();
    edit((c) => {
      const p = profileMut(c);
      for (const list of [p.page ?? [], p.folder ?? []]) {
        const i = list.findIndex((x) => x.name === page);
        if (i < 0) continue;
        const j = Math.max(0, Math.min(list.length - 1, i + delta));
        [list[i], list[j]] = [list[j], list[i]];
      }
    });
  }

  function deletePage() {
    closeMenu();
    const name = page;
    if (!name) return;
    confirm(`Delete “${name}”?`, 'Its keys are deleted too.', async () => {
      edit((c) => {
        const p = profileMut(c);
        p.page = (p.page ?? []).filter((x) => x.name !== name);
        p.folder = (p.folder ?? []).filter((x) => x.name !== name);
      });
      await save();
      const first = currentProfile()?.page?.[0]?.name;
      if (first) await showPage(first);
    });
  }

  // ───────────────────────────── profiles ─────────────────────────────
  function newProfile() {
    closeMenu();
    ask('New profile', '', async (name) => {
      if (!name || profiles.includes(name)) return toast('Choose a new, unique name');
      edit((c) => {
        c.profile ??= [];
        if (c.profile.length === 0) c.profile.push({ name: 'Default', page: [{ name: 'Main' }] });
        c.profile.push({ name, page: [{ name: 'Main' }] });
      });
      await save();
      await setProfile(name);
    });
  }

  function renameProfile() {
    closeMenu();
    const old = currentProfile()?.name;
    if (!old) return;
    ask('Rename profile', old, async (name) => {
      if (!name || name === old) return;
      if (profiles.includes(name)) return toast('A profile with that name already exists');
      edit((c) => {
        for (const p of c.profile ?? []) if (p.name === old) p.name = name;
        retarget(c, (a) => 'profile' in a && a.profile === old, (a) => 'profile' in a && (a.profile = name));
      });
      await save();
      await setProfile(name);
    });
  }

  async function profileApps() {
    closeMenu();
    const p = currentProfile();
    if (!p) return;
    if (profiles[0] === p.name) return toast('The default profile is used when no other profile matches; give apps to another profile');
    const suggestions = await invoke<string[]>('open_window_classes').catch(() => []);
    dialog = {
      title: `Apps for “${p.name}”`,
      body: 'The profile activates when a window with one of these classes is focused. Separate with commas; * is a wildcard.',
      value: (p.apps ?? []).join(', '),
      suggestions,
      ok: 'Save',
      input: true,
      done: (v) => edit((c) => (profileMut(c).apps = v.split(',').map((s) => s.trim()).filter(Boolean))),
    };
  }

  function deleteProfile() {
    closeMenu();
    const p = currentProfile();
    if (!p) return;
    if (profiles[0] === p.name) return toast('The default (first) profile can’t be deleted');
    confirm(`Delete profile “${p.name}”?`, 'All its pages, folders and bindings are deleted.', async () => {
      edit((c) => (c.profile = (c.profile ?? []).filter((x) => x.name !== p.name)));
      await save();
      await setProfile(profiles[0]);
    });
  }

  const addSuggestion = (s: string) => {
    if (!dialog) return;
    const items = dialog.value.split(',').map((x) => x.trim()).filter(Boolean);
    if (!items.includes(s)) items.push(s);
    dialog.value = items.join(', ');
  };

  const isFolder = (p: Page) => folders.includes(p);
</script>

<svelte:window onclick={closeMenu} />

<div class="app">
  <header>
    <div class="left">
      <select value={currentProfile()?.name ?? ''} onchange={(e) => setProfile(e.currentTarget.value)} title="Active profile">
        {#each profiles.length ? profiles : ['Default'] as p}<option value={p}>{p}</option>{/each}
      </select>
      <div class="menu-anchor">
        <button class="flat icon" title="Profile settings" onclick={(e) => { e.stopPropagation(); menu = menu === 'profile' ? '' : 'profile'; }}>⚙</button>
        {#if menu === 'profile'}
          <div class="popover" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
            <button onclick={newProfile}>New profile</button>
            <button onclick={renameProfile}>Rename profile…</button>
            <button onclick={profileApps}>Activate for apps…</button>
            <button class="danger" onclick={deleteProfile}>Delete profile</button>
          </div>
        {/if}
      </div>
    </div>
    <div class="title">
      <strong>OpenMasterHUB</strong>
      <small>
        {#if hub.online}{moduleCount} module{moduleCount === 1 ? '' : 's'}{#if hub.state.active_app} · focused: {hub.state.active_app}{/if}{:else}service offline{/if}
      </small>
    </div>
    <div class="right"></div>
  </header>

  {#if !hub.online}
    <div class="offline">
      <div class="glyph">⏻</div>
      <h1>The MasterHUB service isn’t running</h1>
      <p>The app edits the configuration of <b>masterhubd</b>, which drives the hardware.<br />Start it, or run <code>masterhubd</code> in a terminal.</p>
      <button class="suggested pill" onclick={() => invoke('start_service').catch((e) => toast(String(e)))}>Start service</button>
    </div>
  {:else}
    <main>
      <div class="stage">
        <div class="canvas"><Hub /></div>
        <nav class="pagebar">
          <span class="caption">PAGES</span>
          <div class="linked">
            {#each [...pages, ...folders] as p (p.name)}
              <button class:active={p.name === page} class:folder-start={isFolder(p) && folders[0] === p} onclick={() => showPage(p.name)}>
                {#if isFolder(p)}<span class="folder-icon">🗀</span>{/if}{p.name}
              </button>
            {/each}
          </div>
          <div class="menu-anchor">
            <button class="flat icon" title="Add page or folder" onclick={(e) => { e.stopPropagation(); menu = menu === 'add' ? '' : 'add'; }}>＋</button>
            {#if menu === 'add'}
              <div class="popover up" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
                <button onclick={() => addPage(false)}>New page</button>
                <button onclick={() => addPage(true)}>New folder</button>
              </div>
            {/if}
          </div>
          <div class="spacer"></div>
          {#if page}
            <div class="menu-anchor">
              <button class="flat icon" title="“{page}”" onclick={(e) => { e.stopPropagation(); menu = menu === 'page' ? '' : 'page'; }}>⋮</button>
              {#if menu === 'page'}
                <div class="popover up right" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
                  <button onclick={renamePage}>Rename…</button>
                  <button onclick={() => movePage(-1)}>Move left</button>
                  <button onclick={() => movePage(1)}>Move right</button>
                  <button class="danger" onclick={deletePage}>Delete</button>
                </div>
              {/if}
            </div>
          {/if}
        </nav>
      </div>
      <aside><Inspector /></aside>
    </main>
  {/if}

  {#if dialog}
    {@const d = dialog}
    <div class="scrim" role="presentation" onclick={() => (dialog = null)}>
      <div class="dialog" role="dialog" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.key === 'Escape' && (dialog = null)}>
        <h2>{d.title}</h2>
        {#if d.body}<p>{d.body}</p>{/if}
        {#if d.input}
          <!-- svelte-ignore a11y_autofocus -->
          <input autofocus bind:value={d.value} onkeydown={(e) => { if (e.key === 'Enter') { d.done(d.value.trim()); dialog = null; } }} />
        {/if}
        {#if d.suggestions?.length}
          <div class="caption">OPEN WINDOWS</div>
          <div class="chips">{#each d.suggestions as s}<button class="chip" onclick={() => addSuggestion(s)}>{s}</button>{/each}</div>
        {/if}
        <div class="actions">
          <button onclick={() => (dialog = null)}>Cancel</button>
          <button class={d.danger ? 'destructive' : 'suggested'} onclick={() => { d.done(d.value.trim()); dialog = null; }}>{d.ok}</button>
        </div>
      </div>
    </div>
  {/if}

  {#if hub.toast}<div class="toast">{hub.toast}</div>{/if}
</div>

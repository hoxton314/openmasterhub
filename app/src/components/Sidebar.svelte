<script lang="ts">
  import { Folder, LayoutGrid, Ellipsis, Plus, UserRound } from 'lucide-svelte';
  import { currentPage, currentProfile, edit, hub, profileMut, save, setProfile, showPage, toast } from '../lib/hub.svelte';
  import { invoke } from '../lib/backend';
  import { ui } from '../lib/ui.svelte';
  import type { Action, Config } from '../lib/types';

  const profiles = $derived(hub.config.profile ?? []);
  const active = $derived(currentProfile());
  const pages = $derived(active?.page ?? []);
  const folders = $derived(active?.folder ?? []);
  const page = $derived(currentPage()?.name);
  const moduleCount = $derived(hub.modules.filter((m) => m.kind !== 'base').length);

  let menu = $state(''); // e.g. "profile:Default", "page:Main", "add-page", "add-profile"
  const toggle = (id: string) => (e: MouseEvent) => {
    e.stopPropagation();
    menu = menu === id ? '' : id;
  };

  function uniqueName(base: string) {
    const taken = new Set([...pages, ...folders].map((x) => x.name));
    for (let i = 1; ; i++) if (!taken.has(`${base} ${i}`)) return `${base} ${i}`;
  }

  /** Rename references in actions when a page/folder/profile is renamed. */
  function retarget(c: Config, pred: (a: Action) => boolean, set: (a: Action) => void) {
    for (const p of c.profile ?? []) {
      const actions: (Action | undefined)[] = [];
      for (const pg of [...(p.page ?? []), ...(p.folder ?? [])]) for (const k of pg.key ?? []) actions.push(k.press, k.release);
      for (const k of p.knob ?? []) actions.push(k.cw, k.ccw, k.press, k.release);
      if (p.dial) actions.push(p.dial.cw, p.dial.ccw, p.dial.press, p.dial.release);
      for (const r of p.roller ?? []) actions.push(r.away, r.toward);
      for (const a of actions) if (a && pred(a)) set(a);
    }
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

  function newProfile() {
    menu = '';
    ui.ask({
      title: 'New profile',
      placeholder: 'e.g. Streaming',
      ok: 'Create',
      done: async (name) => {
        if (!name || profiles.some((p) => p.name === name)) return toast('Choose a new, unique name');
        edit((c) => {
          c.profile ??= [];
          if (c.profile.length === 0) c.profile.push({ name: 'Default', page: [{ name: 'Main' }] });
          c.profile.push({ name, page: [{ name: 'Main' }] });
        });
        await save();
        await setProfile(name);
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
        edit((c) => {
          for (const p of c.profile ?? []) if (p.name === old) p.name = name;
          retarget(c, (a) => 'profile' in a && a.profile === old, (a) => 'profile' in a && (a.profile = name));
        });
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

<svelte:window onclick={() => (menu = '')} />

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

  <div class="section">
    <div class="section-head">
      <span>Profiles</span>
      <button class="btn ghost sm icon" title="New profile" onclick={newProfile}><Plus size={14} /></button>
    </div>
    {#each profiles as p, i (p.name)}
      <div class="item-wrap">
        <button class="item" class:active={p.name === active?.name} onclick={() => setProfile(p.name)}>
          <UserRound size={14} />
          <span class="text">
            <span class="title">{p.name}</span>
            <span class="sub">{i === 0 ? 'Default' : p.apps?.length ? p.apps.join(', ') : 'Manual only'}</span>
          </span>
          {#if hub.state.profile === p.name}<span class="live" title="Active on the hub"></span>{/if}
        </button>
        <button class="btn ghost sm icon more" title="More" onclick={toggle(`profile:${p.name}`)}><Ellipsis size={14} /></button>
        {#if menu === `profile:${p.name}`}
          <div class="menu" style="right:6px;top:34px" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
            <button onclick={() => renameProfile(p.name)}>Rename…</button>
            {#if i > 0}<button onclick={() => profileApps(p.name)}>Activate for apps…</button>{/if}
            {#if i > 0}<hr /><button class="danger" onclick={() => deleteProfile(p.name)}>Delete</button>{/if}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="section grow">
    <div class="section-head">
      <span>Keypad pages</span>
      <div class="anchor">
        <button class="btn ghost sm icon" title="Add page or folder" onclick={toggle('add-page')}><Plus size={14} /></button>
        {#if menu === 'add-page'}
          <div class="menu" style="right:0;top:28px" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
            <button onclick={() => addPage(false)}><LayoutGrid size={14} />New page</button>
            <button onclick={() => addPage(true)}><Folder size={14} />New folder</button>
          </div>
        {/if}
      </div>
    </div>
    {#each [...pages.map((p) => ({ p, folder: false })), ...folders.map((p) => ({ p, folder: true }))] as { p, folder }, i (p.name)}
      {#if folder && i === pages.length}<div class="divider-label">Folders</div>{/if}
      <div class="item-wrap">
        <button class="item" class:active={p.name === page} onclick={() => showPage(p.name)}>
          {#if folder}<Folder size={14} />{:else}<LayoutGrid size={14} />{/if}
          <span class="text"><span class="title">{p.name}</span></span>
          <span class="count">{p.key?.length ?? 0}</span>
        </button>
        <button class="btn ghost sm icon more" title="More" onclick={toggle(`page:${p.name}`)}><Ellipsis size={14} /></button>
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
  .section {
    padding: 8px 8px 4px;
    display: grid;
    gap: 1px;
    align-content: start;
  }
  .section.grow {
    flex: 1;
    overflow-y: auto;
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 28px;
    padding: 0 2px 0 8px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-2);
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

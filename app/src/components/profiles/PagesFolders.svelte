<!--
  Pages and folders (S21): the page cycle (drag to reorder) and the folders a key opens. Opened as
  the `pages` route of the editor, for the profile being edited.
-->
<script lang="ts">
  import { ArrowLeft, ChevronLeft, ChevronRight, Folder, GripVertical, MoreHorizontal, Pencil, Plus, Trash2 } from 'lucide-svelte';
  import { currentProfile, edit, editor, showPage, toastUndo } from '../../lib/hub.svelte';
  import { renamePage } from '../../lib/model';
  import type { Action, KeyConfig, Page, Profile } from '../../lib/types';
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import Card from '../ui/Card.svelte';
  import Chip from '../ui/Chip.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Menu, { type MenuEntry } from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import { uniqueName } from '../../lib/templates';
  import TemplatePreview from './TemplatePreview.svelte';

  const profile = $derived(currentProfile());
  const pages = $derived(profile?.page ?? []);
  const folders = $derived(profile?.folder ?? []);

  type Where = { page: string; key: number };
  const press = (k: KeyConfig): Action[] => [k.press, k.release].filter((a): a is Action => !!a);
  /** Keys that open `folder` (`{ folder }`) or page (`{ page }`), as "key 14 on Views". */
  function openers(p: Profile, kind: 'folder' | 'page', name: string): Where[] {
    const out: Where[] = [];
    for (const pg of [...(p.page ?? []), ...(p.folder ?? [])]) for (const k of pg.key ?? []) if (press(k).some((a) => kind in a && (a as Record<string, unknown>)[kind] === name)) out.push({ page: pg.name, key: k.index + 1 });
    return out;
  }
  const backKey = (f: Page) => (f.key ?? []).find((k) => press(k).some((a) => 'back' in a))?.index;
  const keysIn = (pg: Page) => (pg.key ?? []).filter((k) => k.label || k.image || k.widget || k.press).length;

  // ── mutations ──
  const names = $derived([...pages, ...folders].map((p) => p.name));

  function mutate(f: (p: Profile) => void, label: string) {
    const name = profile?.name;
    edit(
      (c) => {
        const p = (c.profile ?? []).find((x) => x.name === name);
        if (p) f(p);
      },
      { label },
    );
  }

  function move(from: number, to: number) {
    if (from === to || to < 0 || to >= pages.length) return;
    mutate((p) => {
      const [m] = p.page!.splice(from, 1);
      p.page!.splice(to, 0, m);
    }, 'Reorder pages');
  }

  function add(kind: 'page' | 'folder') {
    ui.ask({
      title: kind === 'page' ? 'Add page' : 'Add folder',
      body: kind === 'page' ? 'A page joins the cycle after the last one.' : 'A folder is opened by a key and left with Back. It is not part of the cycle.',
      value: uniqueName(names, kind === 'page' ? `Page ${pages.length + 1}` : 'Folder'),
      ok: kind === 'page' ? 'Add page' : 'Add folder',
      done: (v) => {
        const n = v.trim();
        if (!n || names.includes(n)) return ui.toast({ text: n ? `“${n}” is already a page or folder.` : 'Give it a name.', kind: 'warning' });
        mutate((p) => {
          if (kind === 'page') (p.page ??= []).push({ name: n, key: [] });
          else (p.folder ??= []).push({ name: n, key: [{ index: 0, label: 'Back', background: '#3a3a46', press: { back: true } }] });
        }, `Add ${kind} ${n}`);
        if (kind === 'page') showPage(n);
      },
    });
  }

  function rename(pg: Page) {
    ui.ask({
      title: `Rename ${pg.name}`,
      value: pg.name,
      ok: 'Rename',
      done: (v) => {
        const n = v.trim();
        if (!n || n === pg.name) return;
        if (names.includes(n)) return ui.toast({ text: `“${n}” is already a page or folder.`, kind: 'warning' });
        const old = pg.name;
        const prof = profile?.name;
        edit((c) => renamePage(c, prof!, old, n), { label: `Rename ${old}` });
        if (editor.page === old) editor.page = n;
      },
    });
  }

  function remove(pg: Page, kind: 'page' | 'folder') {
    if (kind === 'page' && pages.length < 2) return ui.toast({ text: 'A profile keeps at least one page.', kind: 'warning' });
    const by = profile ? openers(profile, kind, pg.name) : [];
    ui.confirm({
      title: `Delete ${kind} ${pg.name}?`,
      body: `${keysIn(pg)} set key${keysIn(pg) === 1 ? '' : 's'} on it will go.${by.length ? ` ${by.length} key${by.length > 1 ? 's open' : ' opens'} it and will do nothing.` : ''} Ctrl+Z restores it.`,
      ok: `Delete ${kind}`,
      done: () => {
        mutate((p) => {
          if (kind === 'page') p.page = (p.page ?? []).filter((x) => x.name !== pg.name);
          else p.folder = (p.folder ?? []).filter((x) => x.name !== pg.name);
        }, `Delete ${pg.name}`);
        if (editor.page === pg.name) editor.page = '';
        toastUndo(`Deleted ${kind} “${pg.name}”.`);
      },
    });
  }

  function menu(pg: Page, kind: 'page' | 'folder', i: number): MenuEntry[] {
    return [
      { label: 'Open in editor', icon: Pencil, onselect: () => openIn(pg.name) },
      { label: 'Rename', icon: Pencil, onselect: () => rename(pg) },
      ...(kind === 'page' ? [{ label: 'Move earlier', icon: ChevronLeft, disabled: i === 0, onselect: () => move(i, i - 1) }, { label: 'Move later', icon: ChevronRight, disabled: i === pages.length - 1, onselect: () => move(i, i + 1) }] : []),
      { separator: true },
      { label: 'Delete', icon: Trash2, danger: true, onselect: () => remove(pg, kind) },
    ];
  }

  function openIn(name: string) {
    showPage(name);
    ui.open(null);
  }

  // ── drag to reorder ──
  let dragging = $state<number | null>(null);
  let over = $state<number | null>(null);
  function drop(to: number) {
    if (dragging !== null) move(dragging, to);
    dragging = over = null;
  }

  const back = () => ui.open(null);
  const count = $derived(`${pages.length} page${pages.length === 1 ? '' : 's'} · ${folders.length} folder${folders.length === 1 ? '' : 's'}`);
</script>

<div class="page">
  {#if !profile}
    <EmptyState title="No profile" body="Pick a profile in the sidebar first." actionLabel="Back to editor" onaction={back} />
  {:else}
    <header>
      <div class="titles">
        <h1 class="t-title">{profile.name} · pages and folders</h1>
        <span class="muted t-small">{count}</span>
      </div>
      <Button icon={ArrowLeft} onclick={back}>Back to editor</Button>
    </header>

    <section>
      <SectionLabel as="h2">Page cycle</SectionLabel>
      <p class="muted t-small">Next and previous go left to right. Drag to reorder.</p>
      <ol class="cards">
        {#each pages as pg, i (pg.name)}
          <li
            class:dragging={dragging === i}
            class:over={over === i && dragging !== i}
            draggable="true"
            ondragstart={(e) => ((dragging = i), e.dataTransfer?.setData('text/plain', pg.name))}
            ondragover={(e) => (e.preventDefault(), (over = i))}
            ondragleave={() => over === i && (over = null)}
            ondrop={(e) => (e.preventDefault(), drop(i))}
            ondragend={() => (dragging = over = null)}
          >
            <Card padding={10} label={pg.name}>
              <div class="well"><TemplatePreview keys={pg.key ?? []} unit={20} /></div>
              <div class="meta">
                <GripVertical size={14} class="grip" />
                <div class="grow">
                  <b>{pg.name}</b>
                  <span class="muted t-small">Page {i + 1}{openers(profile, 'page', pg.name).length ? ` · opened by key ${openers(profile, 'page', pg.name)[0].key}` : ''}</span>
                </div>
                {@render more(pg, 'page', i)}
              </div>
            </Card>
          </li>
        {/each}
        <li><button type="button" class="add" onclick={() => add('page')}><Plus size={18} />Add page</button></li>
      </ol>
    </section>

    <section>
      <SectionLabel as="h2">Folders</SectionLabel>
      <p class="muted t-small">Opened by a key, left with Back. Not part of the cycle.</p>
      <ol class="cards">
        {#each folders as f, i (f.name)}
          {@const by = openers(profile, 'folder', f.name)}
          {@const bk = backKey(f)}
          <li>
            <Card padding={10} label={f.name}>
              <div class="well"><TemplatePreview keys={f.key ?? []} unit={20} /></div>
              <div class="meta">
                <Folder size={14} />
                <div class="grow">
                  <b>{f.name}</b>
                  <span class="muted t-small">
                    {#if by.length}Opened by key {by[0].key} on {by[0].page}{bk !== undefined ? ` · Back on key ${bk + 1}` : ''}{:else}No key opens it yet{/if}
                  </span>
                </div>
                {#if !by.length}<Chip variant="warning">Unreachable</Chip>{/if}
                {@render more(f, 'folder', i)}
              </div>
            </Card>
          </li>
        {/each}
        <li><button type="button" class="add" onclick={() => add('folder')}><Plus size={18} />Add folder</button></li>
      </ol>
    </section>
  {/if}
</div>

{#snippet more(pg: Page, kind: 'page' | 'folder', i: number)}
  <Popover role="menu" label="{pg.name} actions" placement="bottom-end">
    {#snippet trigger({ toggle, attrs })}<IconButton icon={MoreHorizontal} size="sm" label="Actions for {pg.name}" {...attrs} onclick={toggle} />{/snippet}
    {#snippet children({ close })}<Menu items={menu(pg, kind, i)} onclose={close} />{/snippet}
  </Popover>
{/snippet}

<style>
  .page {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    align-content: start;
    gap: 22px;
    width: 100%;
    max-width: 960px;
    margin: 0 auto;
    padding: 24px 24px 48px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .titles {
    flex: 1;
    display: grid;
    gap: 2px;
  }
  section {
    display: grid;
    gap: 8px;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    min-width: 0;
  }
  li.dragging {
    opacity: 0.4;
  }
  li.over {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
    border-radius: var(--radius-card);
  }
  .well {
    display: grid;
    place-items: center;
    padding: 10px;
    background: var(--hub-base);
    border-radius: var(--radius-well);
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--color-text-3);
  }
  .grow {
    flex: 1;
    min-width: 0;
    display: grid;
    color: var(--color-text-1);
  }

  .add {
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 6px;
    min-height: 140px;
    font: inherit;
    color: var(--color-text-2);
    background: none;
    border: 1px dashed var(--color-border-2);
    border-radius: var(--radius-card);
    cursor: pointer;
  }
  .add:hover {
    color: var(--color-text-1);
    border-color: var(--color-accent);
  }
</style>

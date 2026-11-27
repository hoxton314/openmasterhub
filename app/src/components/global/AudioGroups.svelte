<!-- Audio groups page (S27): control several apps as one. List on the left, the chosen group on the right. -->
<script lang="ts">
  import { ArrowLeft, Plus, Trash2 } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import { invoke } from '../../lib/backend';
  import { edit, hub, toastUndo } from '../../lib/hub.svelte';
  import type { AudioGroup, AudioInfo } from '../../lib/types';
  import { ui } from '../../lib/ui.svelte';
  import Avatar from '../ui/Avatar.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import TextField from '../ui/TextField.svelte';
  import Toggle from '../ui/Toggle.svelte';
  import { usedBy } from './usage';

  const groups = $derived(hub.config.audio_group ?? []);
  let selected = $state<string | undefined>();
  const group = $derived(groups.find((g) => g.name === selected) ?? groups[0]);
  let playing = $state<string[]>([]);

  onMount(async () => {
    playing = (await invoke<AudioInfo>('audio_info').catch(() => undefined))?.apps ?? [];
  });

  /** Apps to offer: playing now plus the ones already in the group. */
  const apps = $derived(
    [...playing, ...(group?.apps ?? [])].filter((a, i, all) => all.findIndex((b) => b.toLowerCase() === a.toLowerCase()) === i),
  );
  const inGroup = (app: string) => group?.apps?.some((a) => a.toLowerCase() === app.toLowerCase()) ?? false;
  const uses = $derived(group ? usedBy(hub.config, group.name) : []);

  function mut(f: (g: AudioGroup) => void, label: string) {
    const name = group?.name;
    edit((c) => {
      const g = c.audio_group?.find((x) => x.name === name);
      if (g) f(g);
    }, { label });
  }

  function add() {
    ui.ask({
      title: 'New audio group',
      body: 'Name it after what it holds, for example Games or Voice.',
      placeholder: 'Games',
      ok: 'Create',
      done: (name) => {
        name = name.trim();
        if (!name || groups.some((g) => g.name === name)) return;
        edit((c) => (c.audio_group = [...(c.audio_group ?? []), { name, apps: [] }]), { label: 'new audio group' });
        selected = name;
      },
    });
  }

  function rename(to: string) {
    to = to.trim();
    const from = group?.name;
    if (!from || !to || to === from || groups.some((g) => g.name === to)) return;
    edit((c) => {
      const g = c.audio_group?.find((x) => x.name === from);
      if (g) g.name = to;
      // Actions that target the group follow its new name.
      const fix = (a: unknown) => {
        const o = a as { volume?: { group?: string }; mute?: { group?: string } } | undefined;
        for (const t of [o?.volume, o?.mute]) if (t && typeof t === 'object' && t.group === from) t.group = to;
      };
      for (const p of c.profile ?? []) {
        for (const pg of [...(p.page ?? []), ...(p.folder ?? [])]) for (const k of pg.key ?? []) (fix(k.press), fix(k.release));
        for (const k of p.knob ?? []) [k.cw, k.ccw, k.press, k.release].forEach(fix);
        if (p.dial) [p.dial.cw, p.dial.ccw, p.dial.press, p.dial.release, p.dial.long_press].forEach(fix);
        for (const r of p.roller ?? []) (fix(r.away), fix(r.toward));
        for (const f of p.fader ?? []) fix(f.change);
      }
      for (const k of c.pinned ?? []) (fix(k.press), fix(k.release));
    }, { label: 'rename audio group' });
    selected = to;
  }

  function remove() {
    const name = group?.name;
    if (!name) return;
    edit((c) => (c.audio_group = (c.audio_group ?? []).filter((g) => g.name !== name)), { label: 'delete audio group' });
    selected = undefined;
    toastUndo(`Deleted “${name}”`);
  }

  function toggleApp(app: string, on: boolean) {
    mut((g) => (g.apps = on ? [...(g.apps ?? []), app] : (g.apps ?? []).filter((a) => a.toLowerCase() !== app.toLowerCase())), 'change audio group');
  }

  const summary = (g: AudioGroup) => `${g.apps?.length ?? 0}${g.steam ? ' + Steam' : ''}`;
</script>

<div class="page">
  <header>
    <IconButton icon={ArrowLeft} label="Back to Global" onclick={() => ui.open(null)} />
    <div class="titles">
      <h1 class="t-title">Audio groups</h1>
      <p class="t-small muted">Control several apps as one</p>
    </div>
    <Button variant="primary" icon={Plus} onclick={add}>New group</Button>
  </header>

  {#if !groups.length}
    <EmptyState title="No audio groups yet" body="A group lets one knob or key control the volume of several apps, for example every game." actionLabel="New group" onaction={add} />
  {:else}
    <div class="split">
      <ul class="list" aria-label="Audio groups">
        {#each groups as g (g.name)}
          <li>
            <button type="button" class:active={g.name === group?.name} onclick={() => (selected = g.name)}>
              <span class="name">{g.name}</span>
              <span class="t-small meta">{summary(g)}</span>
            </button>
          </li>
        {/each}
      </ul>

      {#if group}
        <section class="detail">
          <TextField label="Name" value={group.name} onchange={(e) => rename((e.currentTarget as HTMLInputElement).value)} />
          <div class="block">
            <SectionLabel as="h2">Apps playing now and recent</SectionLabel>
            {#each apps as app (app)}
              <div class="app">
                <Avatar name={app} size={22} />
                <span class="grow">{app}</span>
                {#if playing.includes(app)}<Chip variant="success" icon={null}>playing</Chip>{/if}
                <Toggle title="{app} in {group.name}" checked={inGroup(app)} onchange={(on) => toggleApp(app, on)} />
              </div>
            {/each}
            {#if !apps.length}<p class="t-small faint">Nothing is playing audio right now.</p>{/if}
          </div>
          <div class="app steam">
            <span class="grow">
              <span class="t-body">Everything Steam launches</span><br />
              <span class="t-small muted">Any process started by Steam joins the group</span>
            </span>
            <Toggle title="Everything Steam launches" checked={group.steam ?? false} onchange={(on) => mut((g) => (g.steam = on || undefined), 'change audio group')} />
          </div>
          <div class="block">
            <SectionLabel as="h2">Used by</SectionLabel>
            {#each uses as u (u.where + u.control)}
              <div class="use"><b>{u.control}</b><span class="muted">{u.where}</span></div>
            {:else}
              <p class="t-small faint">Nothing uses this group yet. Pick it as the target of a volume or mute action.</p>
            {/each}
          </div>
          <div><Button variant="danger" icon={Trash2} onclick={remove}>Delete group</Button></div>
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .page {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-7) var(--space-8);
    display: grid;
    align-content: start;
    gap: var(--space-5);
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .titles {
    flex: 1;
  }
  h1,
  p {
    margin: 0;
  }
  .split {
    display: grid;
    grid-template-columns: 220px minmax(0, 520px);
    gap: var(--space-6);
    align-items: start;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  .list button {
    width: 100%;
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
    height: var(--size-row-sidebar);
    padding: 0 var(--space-3);
    border: 0;
    border-radius: var(--radius-sidebar-row);
    background: none;
    color: var(--idle-item-fg);
    font: inherit;
    cursor: pointer;
  }
  .list button:hover {
    background: var(--color-selection);
  }
  .list button.active {
    background: var(--active-item-bg);
    color: var(--active-item-fg);
  }
  .meta {
    opacity: 0.7;
  }
  .detail {
    display: grid;
    gap: var(--space-5);
  }
  .block {
    display: grid;
    gap: var(--space-2);
  }
  .app {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-row);
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .use {
    display: flex;
    justify-content: space-between;
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--color-border);
  }
</style>

<!-- "When does Claude apply?": pick an open or recent window, or type a pattern, with a live check of what it matches and wins. -->
<script lang="ts">
  import { AppWindow, Info, Trash2, TriangleAlert } from 'lucide-svelte';
  import { onMount, untrack } from 'svelte';
  import { hub } from '../../lib/hub.svelte';
  import { conflictText, ruleConflicts, sampleOf } from '../../lib/model/conflicts';
  import { globMatch, matchApp, type EffectiveSetup } from '../../lib/model';
  import type { Config } from '../../lib/types';
  import Avatar from '../ui/Avatar.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import TextField from '../ui/TextField.svelte';
  import { setPattern } from './ops';
  import { knownClasses, recentOnly, refreshWindows, windows } from './windows.svelte';

  let { setup, profile, index, onclose }: { setup: EffectiveSetup; profile: string; /** Pattern being edited; omitted = add. */ index?: number; onclose: () => void } = $props();

  const editing = $derived(index !== undefined);
  let pattern = $state(untrack(() => (hub.config.profile ?? []).find((p) => p.name === profile)?.apps?.[index ?? -1] ?? ''));
  let tab = $state<'open' | 'recent' | 'type'>(untrack(() => (index !== undefined ? 'type' : 'open')));
  onMount(refreshWindows);

  const typed = $derived(pattern.trim());
  const openMatches = $derived(typed ? windows.open.filter((w) => globMatch(typed, w.class)) : []);
  const recent = $derived(recentOnly());

  /** The window the preview talks about: the first open match, else a class the pattern matches. */
  const sample = $derived(openMatches[0]?.class ?? (typed ? sampleOf(typed) : ''));

  /** The setup as it would be with this pattern. */
  const preview = $derived.by(() => {
    if (!typed) return undefined;
    const c = structuredClone($state.snapshot(hub.config)) as Config;
    const p = (c.profile ?? []).find((x) => x.name === profile);
    if (!p) return undefined;
    const apps = [...(p.apps ?? [])];
    if (editing) apps[index!] = typed;
    else if (!apps.includes(typed)) apps.push(typed);
    p.apps = apps;
    const s = { ...setup, profiles: setup.profiles.includes(profile) ? setup.profiles : [...setup.profiles, profile] };
    c.setup = effectiveSetupsWith(c, s);
    const win = sample ? matchApp(c, s, sample) : undefined;
    const dead = ruleConflicts(c, s.name, knownClasses()).find((x) => x.loser.profile === profile && x.loser.pattern === typed);
    return { win, dead };
  });

  function effectiveSetupsWith(c: Config, s: EffectiveSetup) {
    const rest = (c.setup ?? []).filter((x) => x.name !== s.name);
    const { implicit: _i, ...plain } = s;
    return [...rest, plain];
  }

  function save() {
    if (!typed) return;
    setPattern(profile, index, typed);
    onclose();
  }
  function remove() {
    setPattern(profile, index, undefined);
    onclose();
  }
</script>

<Dialog title="When does {profile} apply?" subtitle="Pick a window that is open now, or type a pattern." width={780} {onclose}>
  <div class="grid">
    <div class="left">
      <Segmented
        label="Where to pick from"
        full
        value={tab}
        onchange={(v) => (tab = v)}
        options={[
          { value: 'open', label: 'Open now' },
          { value: 'recent', label: 'Recent' },
          { value: 'type', label: 'Type a pattern' },
        ]}
      />
      <div class="list" role="listbox" aria-label="Windows">
        {#if tab === 'type'}
          <p class="t-small muted">Type the pattern on the right. Examples: <code>blender</code>, <code>steam_app_*</code>, <code>Godot*.exe</code>.</p>
        {:else}
          {@const items = tab === 'open' ? windows.open : recent.map((c) => ({ class: c, title: undefined as string | undefined }))}
          {#each items as w (w.class)}
            <button type="button" role="option" aria-selected={typed.toLowerCase() === w.class.toLowerCase()} class="win" class:on={typed.toLowerCase() === w.class.toLowerCase()} onclick={() => (pattern = w.class)}>
              <Avatar name={w.class.split(/[._-]/).filter((x) => x && x !== 'com' && x !== 'org').at(-1) ?? w.class} size={24} />
              <span class="what">
                <b>{w.title ? w.title : w.class}</b>
              </span>
              <code>{w.class}</code>
            </button>
          {:else}
            <p class="t-small muted">{tab === 'open' ? (windows.loaded ? 'No windows found.' : 'Looking for windows…') : 'Windows you pick or that were open show up here.'}</p>
          {/each}
        {/if}
      </div>
      <p class="foot t-chip faint"><Info size={13} /><span>Linux: window class from Hyprland or KDE. Windows: the .exe name.</span></p>
    </div>

    <div class="right">
      <TextField bind:value={pattern} label="Pattern" mono spellcheck={false} placeholder="e.g. blender or Godot*.exe" help="* matches anything. Not case sensitive." onkeydown={(e) => e.key === 'Enter' && save()} />
      {#if typed}
        {#if openMatches.length}
          <Chip variant="success">Matches {openMatches.length} open {openMatches.length === 1 ? 'window' : 'windows'}</Chip>
        {:else}
          <Chip variant="outline" icon={AppWindow}>Matches no open window</Chip>
        {/if}
        {#if preview?.dead}
          {@const t = conflictText(preview.dead)}
          <p class="note warn"><TriangleAlert size={14} /><span>{t.head}{#if t.witness}&nbsp;<code>{t.witness}</code>{/if}{t.tail ? (t.witness ? '' : ' ') + t.tail : ''}</span></p>
        {:else if preview?.win?.kind === 'rule' && sample}
          {@const w = preview.win}
          {#if w.profile?.name === profile}
            <p class="note ok"><Info size={14} /><span>{profile} will win for <code>{sample}</code>{#if w.shadowed.length}; it is checked before {w.shadowed.map((h) => h.profile.name).join(', ')}{/if}.</span></p>
          {:else}
            <p class="note warn"><TriangleAlert size={14} /><span>{w.profile?.name}’s rule <code>{w.hit?.pattern}</code> is checked first and matches <code>{sample}</code>, so {profile} will not win there.</span></p>
          {/if}
        {/if}
      {/if}
    </div>
  </div>
  {#snippet footer()}
    {#if editing}<span class="remove"><Button variant="ghost" icon={Trash2} onclick={remove}>Remove rule</Button></span>{/if}
    <Button variant="secondary" onclick={onclose}>Cancel</Button>
    <Button variant="primary" disabled={!typed} onclick={save}>{editing ? 'Save rule' : 'Add rule'}</Button>
  {/snippet}
</Dialog>

<style>
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
    gap: 20px;
    min-height: 280px;
  }
  .left,
  .right {
    display: grid;
    align-content: start;
    gap: 10px;
    min-width: 0;
  }
  .left {
    grid-template-rows: auto 1fr auto;
  }
  .list {
    display: grid;
    align-content: start;
    gap: 2px;
    min-height: 160px;
  }
  .win {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
    padding: 0 10px;
    text-align: left;
    color: var(--color-text-1);
    background: none;
    border: 2px solid transparent;
    border-radius: var(--radius-panel);
    cursor: pointer;
  }
  .win:hover {
    background: var(--color-selection);
  }
  .on {
    background: var(--color-selection);
    border-color: var(--color-accent);
  }
  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .win code {
    flex: none;
    max-width: 45%;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--color-text-2);
  }
  code {
    font-family: var(--type-mono-family);
    font-size: var(--type-mono-size);
  }
  .foot,
  .note {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  .foot :global(svg),
  .note :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .note {
    padding: 10px 12px;
    font-size: var(--type-small-size);
    color: var(--color-text-1);
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-panel);
  }
  .warn {
    border-color: var(--color-warning);
  }
  .warn :global(svg) {
    color: var(--color-warning);
  }
  .ok :global(svg) {
    color: var(--color-accent);
  }
  .remove {
    margin-right: auto;
  }
</style>

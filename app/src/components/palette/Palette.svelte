<!-- Command palette (S07), mounted by App while `ui.palette` is true (Ctrl K). Fuzzy search over
  profiles, controls, actions and settings; `>` limits it to commands. Keyboard only: ↑↓ Enter Esc. -->
<script lang="ts">
  import { CornerDownLeft, Globe, Keyboard, LayoutGrid, Layers, Palette as PaletteIcon, Play, Plus, Power, Redo2, Search, Settings, Undo2, Users } from 'lucide-svelte';
  import { tick } from 'svelte';
  import { invoke } from '../../lib/backend';
  import { editor, editProfile, hasCapability, hub, redo, setProfile, setSetup, toast, undo, undoState } from '../../lib/hub.svelte';
  import { effectiveSetups } from '../../lib/model/setups';
  import { theme } from '../../lib/theme/theme.svelte';
  import { ui } from '../../lib/ui.svelte';
  import { saveAppearance } from '../settings/actions';
  import Avatar from '../ui/Avatar.svelte';
  import type { Icon } from '../ui/icon';
  import Kbd from '../ui/Kbd.svelte';
  import { rank } from './search';

  type Group = 'Profiles' | 'Controls' | 'Actions' | 'Settings';
  interface Entry {
    id: string;
    group: Group;
    label: string;
    hint?: string;
    icon?: Icon;
    avatar?: { name: string; color?: string };
    /** Only found after `>`. */
    command?: boolean;
    run: () => void;
  }

  let query = $state('');
  let active = $state(0);
  let input: HTMLInputElement | undefined = $state();
  let list: HTMLElement | undefined = $state();

  const close = () => (ui.palette = false);
  const go = (f: () => void) => () => {
    close();
    f();
  };

  /** Everything the palette can open or do, built fresh from the current config. */
  const entries = $derived.by<Entry[]>(() => {
    const c = hub.config;
    const out: Entry[] = [];

    for (const p of c.profile ?? []) {
      out.push({
        id: `profile:${p.name}`,
        group: 'Profiles',
        label: p.name,
        hint: [p.name === hub.state.profile ? 'live' : '', p.apps?.length ? p.apps.slice(0, 2).join(', ') : ''].filter(Boolean).join(' · ') || undefined,
        avatar: { name: p.name, color: p.color },
        run: go(() => {
          ui.go('editor');
          editProfile(p.name);
        }),
      });
    }

    const prof = (c.profile ?? []).find((p) => p.name === editor.profile);
    if (prof) {
      for (const pg of [...(prof.page ?? []), ...(prof.folder ?? [])]) {
        for (const k of pg.key ?? []) {
          out.push({
            id: `key:${pg.name}:${k.index}`,
            group: 'Controls',
            label: `Key ${k.index + 1}${k.label ? ` · ${k.label}` : ''}`,
            hint: `${prof.name} · ${pg.name}`,
            icon: LayoutGrid,
            run: go(() => {
              ui.go('editor');
              editProfile(prof.name, pg.name);
              hub.selection = { kind: 'key', index: k.index };
            }),
          });
        }
      }
      if (prof.dial) out.push({ id: 'dial', group: 'Controls', label: `Dial${prof.dial.label ? ` · ${prof.dial.label}` : ''}`, hint: prof.name, icon: LayoutGrid, run: go(() => ((ui.go('editor'), (hub.selection = { kind: 'dial' })))) });
      for (const k of prof.knob ?? []) out.push({ id: `knob:${k.index}`, group: 'Controls', label: `Knob ${k.index + 1}`, hint: prof.name, icon: LayoutGrid, run: go(() => ((ui.go('editor'), (hub.selection = { kind: 'knob', index: k.index })))) });
    }

    const setups = effectiveSetups(c).filter((s) => !s.implicit);
    if (hasCapability('setups'))
      for (const s of setups) out.push({ id: `setup:${s.name}`, group: 'Actions', label: `Switch setup to ${s.name}`, hint: s.shortcut, icon: Layers, run: go(() => void setSetup(s.name)) });
    for (const p of c.profile ?? [])
      out.push({ id: `show:${p.name}`, group: 'Actions', label: `Show ${p.name} on the hub`, icon: Play, command: true, run: go(() => void setProfile(p.name)) });
    out.push(
      { id: 'new-profile', group: 'Actions', label: 'New profile', icon: Plus, run: go(() => ui.openNewProfile({})) },
      { id: 'undo', group: 'Actions', label: undoState.canUndo ? `Undo ${undoState.undoLabel}` : 'Undo', hint: 'Ctrl Z', icon: Undo2, command: true, run: go(() => void (undo() ?? toast('Nothing to undo'))) },
      { id: 'redo', group: 'Actions', label: 'Redo', hint: 'Ctrl Shift Z', icon: Redo2, command: true, run: go(() => void (redo() ?? toast('Nothing to redo'))) },
      { id: 'restart', group: 'Actions', label: 'Restart service', icon: Power, command: true, run: go(() => void invoke('restart_service').then(() => toast('Restarting the service')).catch((e) => toast({ text: `${e}`, kind: 'danger' }))) },
      { id: 'area:setups', group: 'Actions', label: 'Open Setups', icon: Layers, command: true, run: go(() => ui.go('setups')) },
      { id: 'area:global', group: 'Actions', label: 'Open Global', icon: Globe, command: true, run: go(() => ui.go('global')) },
    );

    for (const t of theme.list)
      out.push({ id: `theme:${t.id}`, group: 'Settings', label: `Theme: ${t.name}`, hint: theme.active === t.id ? 'in use' : undefined, icon: PaletteIcon, run: go(() => saveAppearance({ theme: t.id }, 'theme change')) });
    const pages: [string, string, Icon][] = [
      ['appearance', 'Appearance', PaletteIcon],
      ['service', 'Service', Power],
      ['config', 'Config file', Settings],
      ['shortcuts', 'Keyboard shortcuts', Keyboard],
      ['feedback', 'Feedback', Users],
      ['about', 'About', Settings],
    ];
    for (const [view, label, icon] of pages) out.push({ id: `settings:${view}`, group: 'Settings', label, hint: 'Settings', icon, run: go(() => ui.go('settings', { view })) });
    return out;
  });

  const commandMode = $derived(query.startsWith('>'));
  const text = $derived(commandMode ? query.slice(1) : query);
  const results = $derived.by(() => {
    const pool = commandMode ? entries.filter((e) => e.command) : entries.filter((e) => e.group !== 'Actions' || !e.command || text.trim());
    const found = rank(text, pool, (e) => `${e.label} ${e.hint ?? ''}`);
    return text.trim() || commandMode ? found.slice(0, 40) : found.filter((e) => !e.command).slice(0, 40);
  });
  /** Results with their group header where the group changes (ranked results mix groups when searching). */
  const rows = $derived(results.map((e, i) => ({ e, header: !text.trim() && e.group !== results[i - 1]?.group ? e.group : undefined, i })));

  $effect(() => {
    void query;
    active = 0;
  });
  $effect(() => {
    input?.focus();
  });

  async function move(delta: number) {
    if (!results.length) return;
    active = (active + delta + results.length) % results.length;
    await tick();
    list?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape') (e.preventDefault(), e.stopPropagation(), close());
    else if (e.key === 'ArrowDown') (e.preventDefault(), move(1));
    else if (e.key === 'ArrowUp') (e.preventDefault(), move(-1));
    else if (e.key === 'Enter') (e.preventDefault(), results[active]?.run());
    else if (e.key === 'Tab') e.preventDefault(); // stay in the search box
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div class="scrim" onmousedown={(e) => e.target === e.currentTarget && close()}>
  <div class="palette" role="dialog" aria-modal="true" aria-label="Search" tabindex="-1" onkeydown={onkey}>
    <div class="search">
      <Search size={16} />
      <input
        bind:this={input}
        bind:value={query}
        role="combobox"
        aria-expanded="true"
        aria-controls="palette-results"
        aria-activedescendant={results[active] ? `pal-${results[active].id}` : undefined}
        aria-label="Search profiles, controls, actions and settings"
        placeholder={commandMode ? 'Run a command…' : 'Search profiles, controls, actions, settings'}
        autocomplete="off"
        spellcheck="false" />
      <Kbd>Ctrl</Kbd><Kbd>K</Kbd>
    </div>
    <div class="results" id="palette-results" role="listbox" bind:this={list}>
      {#each rows as { e, header, i } (e.id)}
        {#if header}<div class="t-label group">{header}</div>{/if}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div id="pal-{e.id}" class="item" class:active={i === active} role="option" tabindex="-1" aria-selected={i === active} onmousemove={() => (active = i)} onclick={() => e.run()}>
          {#if e.avatar}<Avatar {...e.avatar} size={20} />{:else if e.icon}<span class="icon"><e.icon size={16} /></span>{/if}
          <span class="label">{e.label}</span>
          {#if text.trim()}<span class="t-small group-tag">{e.group}</span>{/if}
          {#if e.hint}<span class="t-small hint">{e.hint}</span>{/if}
          {#if i === active}<CornerDownLeft size={13} />{/if}
        </div>
      {:else}
        <div class="empty t-small muted">Nothing matches “{text}”. Type &gt; for commands.</div>
      {/each}
    </div>
    <div class="foot t-small">
      <span><Kbd>↑</Kbd><Kbd>↓</Kbd> move</span>
      <span><Kbd>Enter</Kbd> open</span>
      <span><Kbd>Esc</Kbd> close</span>
      <span class="grow">Type &gt; for commands</span>
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    background: var(--color-overlay);
    animation: fade-in var(--motion-fast);
  }
  .palette {
    width: min(620px, calc(100vw - 32px));
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--color-dialog);
    border-radius: var(--radius-dialog);
    box-shadow: var(--shadow-dialog);
    animation: pop var(--motion-panel);
  }
  .search {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-4);
    height: 52px;
    color: var(--color-text-3);
    border-bottom: 1px solid var(--color-border);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: 0;
    background: none;
    color: var(--color-text-1);
    font: inherit;
    font-size: var(--type-heading-size);
  }
  input::placeholder {
    color: var(--color-text-3);
  }
  .results {
    flex: 1;
    min-height: 120px;
    overflow-y: auto;
    padding: var(--space-2);
  }
  .group {
    padding: var(--space-3) var(--space-3) var(--space-1);
  }
  .item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    height: var(--size-row-sidebar);
    padding: 0 var(--space-3);
    border-radius: var(--radius-sidebar-row);
    color: var(--color-text-1);
    cursor: pointer;
  }
  .item.active {
    background: var(--row-selected-bg);
    color: var(--row-selected-fg);
  }
  .icon {
    display: grid;
    color: var(--color-text-3);
  }
  .item.active .icon,
  .item.active .hint,
  .item.active .group-tag {
    color: inherit;
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint,
  .group-tag {
    color: var(--color-text-3);
    white-space: nowrap;
  }
  .empty {
    padding: var(--space-6) var(--space-4);
    text-align: center;
  }
  .foot {
    display: flex;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-4);
    color: var(--color-text-3);
    border-top: 1px solid var(--color-border);
  }
  .foot span {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  .grow {
    margin-left: auto;
  }
</style>

<!-- The setup's profiles in the order they are checked: drag to reorder, one row per rule, the
     fallback pinned last under its own label. Conflicts sit under the row that cannot win. -->
<script lang="ts">
  import { ArrowDown, Ellipsis, Plus } from 'lucide-svelte';
  import { hub } from '../../lib/hub.svelte';
  import { matchApp, profileByName, setupsOf, type EffectiveSetup } from '../../lib/model';
  import { ruleConflicts, type ConflictFix } from '../../lib/model/conflicts';
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Menu, { type MenuEntry } from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';
  import RuleRow from '../ui/RuleRow.svelte';
  import ConflictNotice from './ConflictNotice.svelte';
  import AddProfileButton from './AddProfileButton.svelte';
  import { applyConflictFix, moveProfile, openInEditor, removeProfileFromSetup, setFallback } from './ops';
  import { knownClasses } from './windows.svelte';

  let {
    setup,
    testClass = '',
    title = 'Profiles, in the order they are checked',
    onrule,
  }: { setup: EffectiveSetup; testClass?: string; title?: string; /** Open the rule editor for a profile (a pattern index edits it, none adds). */ onrule: (profile: string, index?: number) => void } = $props();

  const conflicts = $derived(ruleConflicts(hub.config, setup.name, knownClasses()));
  const winner = $derived(testClass.trim() ? matchApp(hub.config, setup, testClass.trim()) : undefined);
  const fallback = $derived(profileByName(hub.config, setup.fallback));

  const linkedText = (name: string) => {
    const others = setupsOf(hub.config, name).filter((s) => s.name !== setup.name);
    return others.length ? `Linked · also in ${others.map((s) => s.name).join(', ')}` : undefined;
  };

  function rowMenu(name: string, i: number): MenuEntry[] {
    const apps = profileByName(hub.config, name)?.apps ?? [];
    return [
      ...apps.slice(0, 5).map((p, n): MenuEntry => ({ label: `Edit ${p}`, onselect: () => onrule(name, n) })),
      { label: 'Add app…', icon: Plus, onselect: () => onrule(name) },
      { separator: true },
      { label: 'Move up', disabled: i === 0, onselect: () => moveProfile(setup.name, i, i - 1) },
      { label: 'Move down', disabled: i === setup.profiles.length - 1, onselect: () => moveProfile(setup.name, i, i + 1) },
      { label: 'Make this the fallback', onselect: () => setFallback(setup.name, name) },
      { separator: true },
      { label: 'Open in editor', onselect: () => openInEditor(setup.name, name) },
      { label: 'Remove from setup', danger: true, onselect: () => removeProfileFromSetup(setup.name, name) },
    ];
  }

  function fallbackMenu(): MenuEntry[] {
    const others = (hub.config.profile ?? []).filter((p) => p.name !== setup.fallback && !setup.profiles.includes(p.name));
    return [
      { heading: 'Use as fallback' },
      ...others.map((p): MenuEntry => ({ label: p.name, onselect: () => setFallback(setup.name, p.name) })),
      { separator: true },
      ...(fallback ? [{ label: 'Open in editor', onselect: () => openInEditor(setup.name, fallback.name) } as MenuEntry] : []),
    ];
  }

  // Drag to reorder (the menu has Move up / Move down for the keyboard).
  let from = $state<number | undefined>();
  let slot = $state<number | undefined>(); // insertion position 0..n

  function over(e: DragEvent, i: number) {
    if (from === undefined) return;
    e.preventDefault();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    slot = e.clientY < r.top + r.height / 2 ? i : i + 1;
  }
  function drop(e: DragEvent) {
    e.preventDefault();
    if (from !== undefined && slot !== undefined) moveProfile(setup.name, from, slot > from ? slot - 1 : slot);
    from = slot = undefined;
  }
</script>

<section class="list" aria-label="App rules">
  <header>
    <div>
      <h2 class="t-heading">{title}</h2>
      <p class="t-small muted">First match wins. Drag to reorder.</p>
    </div>
    <AddProfileButton {setup} onrule={(p) => onrule(p)} />
  </header>

  {#if !setup.profiles.length}
    <div class="empty"><EmptyState title="No profiles in {setup.name} yet" body="Add a profile and say which app it belongs to." actionLabel="Add a new profile" onaction={() => ui.openNewProfile({ setup: setup.name, position: 0 })} /></div>
  {/if}

  <div class="rows" role="list" ondragover={(e) => from !== undefined && e.preventDefault()} ondrop={drop}>
    {#each setup.profiles as name, i (name)}
      {@const p = profileByName(hub.config, name)}
      {@const wins = winner?.kind === 'rule' && winner.profile?.name === name ? winner : undefined}
      {#if p}
        <div
          class="block"
          class:dragging={from === i}
          class:before={from !== undefined && slot === i && slot !== from && slot !== from + 1}
          class:after={from !== undefined && i === setup.profiles.length - 1 && slot === i + 1 && slot !== from + 1}
          role="listitem"
          ondragover={(e) => over(e, i)}
        >
          <div class="rule" class:wins={!!wins} draggable="true" role="group" aria-label="Rule {i + 1}: {name}" ondragstart={(e) => { from = i; e.dataTransfer?.setData('text/plain', name); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }} ondragend={() => (from = slot = undefined)}>
            <RuleRow index={i + 1} {name} color={p.color} globs={p.apps ?? []} linked={linkedText(name)}>
              {#snippet actions()}<IconButton icon={Plus} size="sm" label="Add an app to {name}" onclick={() => onrule(name)} />{/snippet}
              {#snippet menu()}
                {#if wins}<Chip variant="success">Wins for {testClass.trim()}</Chip>{/if}
                <Popover placement="bottom-end" role="menu" label="{name} menu">
                  {#snippet trigger({ toggle, attrs })}<IconButton icon={Ellipsis} size="sm" label="{name} menu" {...attrs} onclick={toggle} />{/snippet}
                  {#snippet children({ close })}<Menu items={rowMenu(name, i)} onclose={close} label="{name} menu" />{/snippet}
                </Popover>
              {/snippet}
            </RuleRow>
          </div>
          {#each conflicts.filter((x) => x.loser.profile === name) as x (x.loser.pattern)}
            <ConflictNotice conflict={x} onfix={(f: ConflictFix) => applyConflictFix(setup.name, f)} />
          {/each}
        </div>
      {/if}
    {/each}
  </div>

  {#if setup.mode !== 'manual'}
    <div class="fallback-label t-small muted"><ArrowDown size={13} /> Fallback · always last</div>
    {#if fallback}
      <div class="rule" class:wins={winner?.kind === 'fallback'}>
        <RuleRow name={fallback.name} color={fallback.color} fallback globs={[]} linked={linkedText(fallback.name)}>
          {#snippet menu()}
            {#if winner?.kind === 'fallback'}<Chip variant="success">Wins for {testClass.trim()}</Chip>{/if}
            <Popover placement="bottom-end" role="menu" label="Fallback menu">
              {#snippet trigger({ toggle, attrs })}<IconButton icon={Ellipsis} size="sm" label="Fallback menu" {...attrs} onclick={toggle} />{/snippet}
              {#snippet children({ close })}<Menu items={fallbackMenu()} onclose={close} label="Fallback menu" />{/snippet}
            </Popover>
          {/snippet}
        </RuleRow>
      </div>
    {:else}
      <Popover placement="bottom-start" role="menu" label="Choose a fallback">
        {#snippet trigger({ toggle, attrs })}<Button variant="secondary" icon={Plus} {...attrs} onclick={toggle}>Choose a fallback profile</Button>{/snippet}
        {#snippet children({ close })}<Menu items={fallbackMenu()} onclose={close} label="Choose a fallback" />{/snippet}
      </Popover>
    {/if}
  {/if}
</section>

<style>
  .list {
    display: grid;
    gap: 8px;
    align-content: start;
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 4px;
  }
  .rows {
    display: grid;
    gap: 8px;
  }
  .block {
    position: relative;
    display: grid;
    gap: 6px;
  }
  .block.dragging {
    opacity: 0.45;
  }
  .block.before::before,
  .block.after::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    height: 3px;
    background: var(--color-accent);
    border-radius: var(--radius-pill);
  }
  .block.before::before {
    top: -6px;
  }
  .block.after::after {
    bottom: -6px;
  }
  .rule {
    border-radius: var(--radius-panel);
  }
  .rule.wins {
    outline: 2px solid var(--color-accent);
  }
  .fallback-label {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
  }
  .empty {
    padding: 8px 0 16px;
  }
</style>

<!-- One setup on the overview: icon, mode, blurb, its profiles, triggers, Active / Make active / Open. -->
<script lang="ts">
  import { ArrowRight, Ellipsis, Inbox, Keyboard, Play, Tally2, CircleDot } from 'lucide-svelte';
  import { hub } from '../../lib/hub.svelte';
  import { games } from '../../lib/games.svelte';
  import { profileByName, type EffectiveSetup } from '../../lib/model';
  import Avatar from '../ui/Avatar.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import { prettyKeys } from '../../lib/keys';
  import Menu, { type MenuEntry } from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';
  import StatusDot from '../ui/StatusDot.svelte';
  import { deleteSetup, duplicateSetup, makeActive, modeInfo, plural, profileCount, renameSetup, setupBlurb } from './ops';

  let { setup, active, selected = false, onopen }: { setup: EffectiveSetup; active: boolean; selected?: boolean; onopen: () => void } = $props();

  const mode = $derived(modeInfo(setup.mode));
  const Mode = $derived(mode.icon);
  const rows = $derived.by(() => {
    const names = [...setup.profiles, ...(setup.mode !== 'manual' && setup.fallback ? [setup.fallback] : [])];
    return names.map((n, i) => {
      const p = profileByName(hub.config, n);
      const fb = setup.mode !== 'manual' && n === setup.fallback;
      const integration = !!p && games.list.some((g) => g.enabled && g.steam && p.apps?.includes(`steam_app_${g.steam}`));
      const apps = p?.apps ?? [];
      return { name: n, color: p?.color, fb, integration, glob: apps.length ? apps[0] + (apps.length > 1 ? ` +${apps.length - 1}` : '') : '', n: i + 1 };
    });
  });
  const shown = $derived(rows.length > 4 ? [...rows.slice(0, 3), rows.at(-1)!] : rows);
  const more = $derived(rows.length > 4 ? rows.length - 4 : 0);

  const menu = $derived<MenuEntry[]>([
    { label: 'Open', icon: ArrowRight, onselect: onopen },
    { label: 'Make active', icon: Play, disabled: active, onselect: () => makeActive(setup.name) },
    { separator: true },
    { label: 'Rename…', onselect: () => renameSetup(setup.name) },
    { label: 'Duplicate', onselect: () => duplicateSetup(setup.name) },
    { separator: true },
    { label: 'Delete…', danger: true, onselect: () => deleteSetup(setup.name) },
  ]);
</script>

<article class="card" class:selected aria-label={setup.name}>
  <header>
    <span class="tile" style:--accent={setup.color}><Mode size={18} /></span>
    <div class="titles">
      <h3 class="t-heading">{setup.name}</h3>
      <span class="t-small muted">{mode.line.replace('Follow apps, with manual override', 'Hybrid · manual override')} · {plural(profileCount(setup), 'profile')}</span>
    </div>
    <Popover placement="bottom-end" role="menu" label="{setup.name} menu">
      {#snippet trigger({ toggle, attrs })}<IconButton icon={Ellipsis} label="{setup.name} menu" {...attrs} onclick={toggle} />{/snippet}
      {#snippet children({ close })}<Menu items={menu} onclose={close} label="{setup.name} menu" />{/snippet}
    </Popover>
  </header>

  <p class="blurb t-small muted">{setupBlurb(hub.config, setup)}</p>

  <ul class="profiles">
    {#each shown as r (r.name)}
      {#if more && r === shown.at(-1)}<li class="more t-chip faint">+{more} more {more === 1 ? 'profile' : 'profiles'}</li>{/if}
      <li>
        <Avatar name={r.name} color={r.color} size={18} />
        <span class="name">{r.name}</span>
        {#if r.fb}<span class="meta fb">Fallback</span>{:else if setup.mode === 'manual'}<span class="meta">{r.n}</span>{:else if r.integration}<span class="meta">integration</span>{:else if r.glob}<span class="meta mono">{r.glob}</span>{/if}
      </li>
    {/each}
  </ul>

  <div class="chips">
    {#if setup.shortcut}<Chip icon={Keyboard}>{prettyKeys(setup.shortcut).join(' ')}</Chip>{/if}
    {#if setup.mode === 'manual' && setup.next}<Chip icon={Keyboard}>{prettyKeys(setup.next).join(' ')}</Chip>{/if}
    <Chip icon={Inbox}>Tray</Chip>
    {#if setup.show_on_dial}<Chip icon={CircleDot}>Shown on dial</Chip>{/if}
    {#if setup.mode === 'manual' && setup.profiles.some((p) => profileByName(hub.config, p)?.roller?.some((r) => (r.away && 'profile' in r.away)))}<Chip icon={Tally2}>Roller</Chip>{/if}
  </div>

  <footer>
    {#if active}
      <span class="live"><StatusDot size={7} /> Active on the hub</span>
    {:else}
      <Button size="sm" onclick={() => makeActive(setup.name)}>Make active</Button>
    {/if}
    <Button variant="ghost" size="sm" iconRight={ArrowRight} onclick={onopen}>Open</Button>
  </footer>
</article>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px;
    background: var(--color-surface);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-hairline);
  }
  .selected {
    box-shadow: 0 0 0 2px var(--color-accent);
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .tile {
    display: inline-grid;
    place-items: center;
    flex: none;
    width: 40px;
    height: 40px;
    color: var(--accent, var(--color-accent));
    background: var(--color-selection);
    border-radius: var(--radius-panel);
  }
  .titles {
    flex: 1;
    min-width: 0;
    display: grid;
  }
  .profiles {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 10px 12px;
    list-style: none;
    background: var(--color-sunken);
    border-radius: var(--radius-panel);
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 22px;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    flex: none;
    max-width: 50%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--type-micro-size);
    color: var(--color-text-3);
  }
  .mono {
    font-family: var(--type-mono-family);
  }
  .more {
    padding-left: 26px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-height: 24px;
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: auto;
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    color: var(--live-color);
  }
</style>

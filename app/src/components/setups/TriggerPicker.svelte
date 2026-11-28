<!-- "Switch with" (manual) and "Pin with" (hybrid): the ways to change profile, each a card with a toggle.
     Manual setups also get a mini hub: click the dial or a roller to make it a trigger. -->
<script lang="ts">
  import { CircleDot, Inbox, Keyboard, Search, Tally2 } from 'lucide-svelte';
  import { edit, hasCapability, hub } from '../../lib/hub.svelte';
  import type { EffectiveSetup } from '../../lib/model';
  import type { Action, Config, Profile, Side } from '../../lib/types';
  import Chip from '../ui/Chip.svelte';
  import KeyCapture from '../ui/KeyCapture.svelte';
  import KeyCombo from '../ui/KeyCombo.svelte';
  import Toggle from '../ui/Toggle.svelte';
  import MiniHub, { type HubControl } from './MiniHub.svelte';
  import { updateSetup } from './ops';

  let { setup, kind }: { setup: EffectiveSetup; kind: 'manual' | 'hybrid' } = $props();

  let focus = $state<'shortcut' | HubControl | undefined>();
  const members = $derived((hub.config.profile ?? []).filter((p) => setup.profiles.includes(p.name)));

  const isAction = (a: Action | undefined, v: Action) => !!a && JSON.stringify(a) === JSON.stringify(v);
  const NEXT: Action = { profile: 'next' };
  const PREV: Action = { profile: 'prev' };
  const PICK: Action = { profile: 'pick' };

  const rollerOn = (side: Side) => members.length > 0 && members.every((p) => isAction(p.roller?.find((r) => r.side === side)?.away, NEXT));
  const dialOn = $derived(members.length > 0 && members.every((p) => isAction(p.dial?.long_press, PICK)));
  const canRoller = $derived(hasCapability('profile_cycle'));
  const canDial = $derived(hasCapability('long_press') && hasCapability('profile_cycle'));

  function setRoller(side: Side, on: boolean) {
    edit((c: Config) => {
      for (const p of c.profile ?? []) {
        if (!setup.profiles.includes(p.name)) continue;
        p.roller ??= [];
        let r = p.roller.find((x) => x.side === side);
        if (on) {
          if (!r) p.roller.push((r = { side }));
          r.away = NEXT;
          r.toward = PREV;
        } else if (r && isAction(r.away, NEXT)) {
          delete r.away;
          if (isAction(r.toward, PREV)) delete r.toward;
        }
      }
    }, { label: `${side} roller switches ${setup.name}` });
  }
  function setDial(on: boolean) {
    edit((c: Config) => {
      for (const p of c.profile ?? []) {
        if (!setup.profiles.includes(p.name)) continue;
        if (on) (p.dial ??= {}).long_press = PICK;
        else if (p.dial && isAction(p.dial.long_press, PICK)) delete p.dial.long_press;
      }
    }, { label: `dial long-press switches ${setup.name}` });
  }

  const setShortcut = (key: 'next' | 'prev' | 'pin', combo: string | undefined) =>
    updateSetup(setup.name, (s) => (combo ? (s[key] = combo) : delete s[key]), 'change shortcut');

  const shortcut = $derived(kind === 'hybrid' ? setup.pin : setup.next);
  const shortcutKey = $derived(kind === 'hybrid' ? 'pin' : 'next');

  const active = $derived<HubControl[]>([...(dialOn ? (['dial'] as const) : []), ...((['left', 'right'] as const).filter((s) => rollerOn(s)) as HubControl[])]);

  function pick(c: HubControl) {
    focus = c;
    if (c === 'dial' && canDial && !dialOn) setDial(true);
    if (c !== 'dial' && canRoller && !rollerOn(c)) setRoller(c, true);
  }
  const title = $derived(kind === 'hybrid' ? 'Pin with' : 'Switch with');
  const missing = (cap: string) => `This service version cannot do this yet (needs ${cap})`;
</script>

<section class="picker" aria-label={title}>
  <h2 class="t-heading">{title}</h2>

  {#if kind === 'manual'}
    <div class="hubbox">
      <MiniHub selected={focus === 'shortcut' ? undefined : focus} {active} onpick={pick} />
      <p class="t-small muted hint">
        Click a control on the mini hub to make it a switch trigger.
        {#if focus === 'dial'}<b class="fg">Dial</b> is selected.{:else if focus === 'left' || focus === 'right'}<b class="fg">{focus === 'left' ? 'Left' : 'Right'} roller</b> is selected.{/if}
      </p>
    </div>
  {/if}

  <div class="rows">
    <div class="row" class:focus={focus === 'shortcut'}>
      <button type="button" class="head" aria-expanded={focus === 'shortcut'} onclick={() => (focus = focus === 'shortcut' ? undefined : 'shortcut')}>
        <span class="ic"><Keyboard size={16} /></span>
        <span class="text">
          <b>Keyboard shortcut</b>
          <span class="t-small muted">{kind === 'hybrid' ? 'Pins the profile live now' : 'Next profile · works in any app'}</span>
        </span>
        <KeyCombo combo={shortcut} empty="Not set" />
      </button>
      {#if focus === 'shortcut'}
        <div class="body">
          <KeyCapture label={kind === 'hybrid' ? 'Pin shortcut' : 'Next profile'} value={shortcut} requireModifier fallback onchange={(v) => setShortcut(shortcutKey, v)} />
          {#if kind === 'manual'}
            <KeyCapture label="Previous profile (optional)" value={setup.prev} requireModifier fallback onchange={(v) => setShortcut('prev', v)} />
          {/if}
          {#if shortcut}<button type="button" class="clear t-small" onclick={() => { setShortcut(shortcutKey, undefined); if (kind === 'manual') setShortcut('prev', undefined); }}>Remove shortcut</button>{/if}
          <p class="t-chip faint">Global shortcuts work on X11 and Windows. On Wayland, bind a key to <code>masterhubd ctl</code> in your compositor.</p>
        </div>
      {/if}
    </div>

    {#if kind === 'manual'}
      {#each ['right', 'left'] as const as side (side)}
        <div class="row" class:focus={focus === side}>
          <div class="head static">
            <span class="ic"><Tally2 size={16} /></span>
            <span class="text">
              <b>{side === 'right' ? 'Right' : 'Left'} roller</b>
              <span class="t-small muted">Taken from every profile in {setup.name}. Away: next, toward: previous</span>
            </span>
            <Toggle checked={rollerOn(side)} disabled={!canRoller} title={canRoller ? `${side} roller switches profile` : missing('profile_cycle')} onchange={(v) => { focus = side; setRoller(side, v); }} />
          </div>
        </div>
      {/each}
    {/if}

    <div class="row" class:focus={focus === 'dial'}>
      <div class="head static">
        <span class="ic"><CircleDot size={16} /></span>
        <span class="text">
          <b>Dial long-press</b>
          <span class="t-small muted">{kind === 'hybrid' ? 'Picks and pins' : 'Turn to choose, press to switch'} · pick on the dial</span>
        </span>
        <Toggle checked={dialOn} disabled={!canDial} title={canDial ? 'Dial long-press picks a profile' : missing('long_press')} onchange={(v) => { focus = 'dial'; setDial(v); }} />
      </div>
    </div>

    {#if kind === 'manual'}
      <div class="row">
        <div class="head static">
          <span class="ic"><Inbox size={16} /></span>
          <span class="text"><b>Tray menu</b><span class="t-small muted">Lists the {setup.profiles.length} profiles</span></span>
          <Chip icon={null}>Always on</Chip>
        </div>
      </div>
      <div class="row">
        <div class="head static">
          <span class="ic"><Search size={16} /></span>
          <span class="text"><b>Command palette</b><span class="t-small muted">“Switch to …”</span></span>
          <Chip icon={null}>Ctrl K</Chip>
        </div>
      </div>
    {/if}
  </div>
</section>

<style>
  .picker {
    display: grid;
    gap: 10px;
    align-content: start;
  }
  .hubbox {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 16px;
    padding: 14px;
    background: var(--color-canvas);
    border-radius: var(--radius-panel);
  }
  .hint {
    flex: 1;
    min-width: 140px;
  }
  .fg {
    color: var(--color-text-1);
  }
  .rows {
    display: grid;
    gap: 8px;
  }
  .row {
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-panel);
  }
  .focus {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 1px var(--color-accent);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 12px 14px;
    text-align: left;
    color: var(--color-text-1);
    background: none;
    border: 0;
    border-radius: inherit;
  }
  button.head {
    cursor: pointer;
  }
  button.head:hover {
    background: var(--color-selection);
  }
  .ic {
    display: inline-flex;
    flex: none;
    color: var(--color-text-2);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 1px;
  }
  .body {
    display: grid;
    gap: 12px;
    padding: 4px 14px 14px;
    border-top: 1px solid var(--color-border);
    padding-top: 12px;
  }
  .clear {
    justify-self: start;
    padding: 0;
    color: var(--color-text-2);
    background: none;
    border: 0;
    text-decoration: underline;
    cursor: pointer;
  }
  code {
    font-family: var(--type-mono-family);
  }
</style>

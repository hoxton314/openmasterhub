<script lang="ts">
  import { CircleDot, Disc3, Grid3x3, MousePointerClick, RectangleHorizontal, SlidersHorizontal, SquareDashed, X } from 'lucide-svelte';
  import ActionEditor from './ActionEditor.svelte';
  import ColorField from './ColorField.svelte';
  import FaceEditor from './FaceEditor.svelte';
  import { currentPage, currentProfile, dialMut, displayKey, edit, faderMut, hub, keyMut, knobMut, pageMut, rollerMut } from '../lib/hub.svelte';
  import { TARGET_DIAL, type Action, type Face, type ModuleKind, type Orientation } from '../lib/types';

  const s = $derived(hub.selection);
  const profile = $derived(currentProfile());
  const page = $derived(currentPage());
  const module = $derived(s.kind === 'module' ? hub.modules.find((m) => m.serial === s.serial) : undefined);
  const serialOf = (kind: ModuleKind) => hub.modules.find((m) => m.kind === kind)?.serial ?? '';
  /** The service turns screen images to read upright on the desk; turn them back to show here. */
  const turnOf = (kind: ModuleKind) =>
    hub.config.upright === false ? 0 : ((hub.modules.find((m) => m.kind === kind)?.placement?.rotation ?? 0) + (hub.state.rotation ?? 0)) % 360;

  const KNOB_NAMES = ['Bottom-left knob', 'Bottom-right knob', 'Top knob'];
  const MODULE: Record<ModuleKind, { name: string; icon: typeof Grid3x3 }> = {
    base: { name: 'Base', icon: SquareDashed },
    keypad: { name: 'Display keypad', icon: Grid3x3 },
    fader: { name: 'Faders', icon: SlidersHorizontal },
    knob: { name: 'Knobs', icon: CircleDot },
    roller: { name: 'Rollers', icon: RectangleHorizontal },
    dial: { name: 'Dial', icon: Disc3 },
  };
  const ORIENTATIONS: [Orientation, string][] = [
    ['auto', 'Automatic'],
    ['landscape', 'Landscape'],
    ['portrait', 'Portrait'],
    ['landscape_flipped', 'Landscape, upside down'],
    ['portrait_flipped', 'Portrait, upside down'],
  ];

  const header = $derived.by(() => {
    switch (s.kind) {
      case 'key': return { icon: Grid3x3, title: `Key ${s.index + 1}`, sub: `${page?.name ?? ''} · ${profile?.name ?? ''}` };
      case 'dial': return { icon: Disc3, title: 'Dial', sub: profile?.name ?? '' };
      case 'knob': return { icon: CircleDot, title: KNOB_NAMES[s.index], sub: profile?.name ?? '' };
      case 'fader': return { icon: SlidersHorizontal, title: `Fader ${s.index + 1}`, sub: profile?.name ?? '' };
      case 'roller': return { icon: RectangleHorizontal, title: `${s.side === 'left' ? 'Left' : 'Right'} roller`, sub: profile?.name ?? '' };
      case 'module': return module ? { icon: MODULE[module.kind].icon, title: MODULE[module.kind].name, sub: module.serial } : null;
      default: return null;
    }
  });

  const keyFace = (i: number): Face => page?.key?.find((k) => k.index === i) ?? {};
  const keyAction = (i: number, which: 'press' | 'release') => page?.key?.find((k) => k.index === i)?.[which];

  function ledValue(kind: 'base' | 'fader' | 'knob', i: number): string | undefined {
    const v = hub.config.leds?.[kind];
    return Array.isArray(v) ? v[i] : v;
  }
  function setLed(kind: 'base' | 'fader' | 'knob', i: number, color: string | undefined) {
    edit((c) => {
      c.leds ??= {};
      if (kind === 'base') {
        c.leds.base = color;
        return;
      }
      const n = kind === 'fader' ? 5 : 3;
      const cur = c.leds[kind];
      const list = Array.from({ length: n }, (_, k) => (Array.isArray(cur) ? cur[k] : cur) ?? '#ffffff');
      list[i] = color ?? '#ffffff';
      c.leds[kind] = list;
    });
  }
  const LED_SWATCHES = ['#ffffff', '#ef4444', '#f97316', '#eab308', '#22c55e', '#06b6d4', '#3b82f6', '#a855f7', '#ec4899'];
</script>

<aside class="inspector">
  {#if header}
    {@const I = header.icon}
    <header>
      <span class="hico"><I size={15} /></span>
      <div class="htext">
        <h2>{header.title}</h2>
        <span>{header.sub}</span>
      </div>
      <button class="btn ghost sm icon" title="Close" onclick={() => (hub.selection = { kind: 'none' })}><X size={14} /></button>
    </header>
  {/if}

  <div class="scroll">
    {#key JSON.stringify(s) + hub.state.profile + hub.state.page}
      {#if s.kind === 'none'}
        <div class="overview">
          <div class="empty">
            <MousePointerClick size={22} />
            <h3>Select a control</h3>
            <p>Click a key, knob, fader, roller or the dial to set what it shows and does.</p>
          </div>
          <div class="group">
            <div class="group-title">Connected modules</div>
            {#each hub.modules as m (m.serial)}
              {@const I = MODULE[m.kind].icon}
              <button class="mod" onclick={() => (hub.selection = { kind: 'module', serial: m.serial })}>
                <I size={14} />
                <span class="mname">{MODULE[m.kind].name}</span>
                <span class="mono faint">{m.firmware ?? ''}</span>
              </button>
            {/each}
          </div>
        </div>
      {:else if s.kind === 'key'}
        {@const i = s.index}
        <FaceEditor face={keyFace(i)} live={hub.displays[displayKey(serialOf('keypad'), i)]} liveTurn={turnOf('keypad')} onchange={(f) => edit((c) => f(keyMut(c, i)))} />
        <div class="group">
          <div class="group-title">Triggers</div>
          <ActionEditor title="Press" value={keyAction(i, 'press')} onchange={(a) => edit((c) => (keyMut(c, i).press = a))} />
          <ActionEditor title="Release" value={keyAction(i, 'release')} onchange={(a) => edit((c) => (keyMut(c, i).release = a))} />
        </div>
        <div class="group">
          <button class="btn ghost sm danger-text" onclick={() => edit((c) => { const p = pageMut(c); p.key = (p.key ?? []).filter((k) => k.index !== i); })}>Reset key</button>
        </div>
      {:else if s.kind === 'dial'}
        {@const d = profile?.dial ?? {}}
        <FaceEditor face={d} round live={hub.displays[displayKey(serialOf('dial'), TARGET_DIAL)]} liveTurn={turnOf('dial')} onchange={(f) => edit((c) => f(dialMut(c)))} />
        <div class="group">
          <div class="group-title">Triggers</div>
          {#each [['cw', 'Turn right'], ['ccw', 'Turn left'], ['press', 'Press'], ['release', 'Release']] as const as [k, label]}
            <ActionEditor title={label} value={d[k]} onchange={(a: Action | undefined) => edit((c) => (dialMut(c)[k] = a))} />
          {/each}
        </div>
      {:else if s.kind === 'knob'}
        {@const i = s.index}
        {@const k = profile?.knob?.find((x) => x.index === i) ?? { index: i }}
        <div class="group first">
          <div class="group-title">Triggers</div>
          {#each [['cw', 'Turn right'], ['ccw', 'Turn left'], ['press', 'Press'], ['release', 'Release']] as const as [key, label]}
            <ActionEditor title={label} value={k[key]} onchange={(a: Action | undefined) => edit((c) => (knobMut(c, i)[key] = a))} />
          {/each}
        </div>
        <div class="group">
          <div class="group-title">Ring colour</div>
          <ColorField value={ledValue('knob', i)} swatches={LED_SWATCHES} resettable={false} onchange={(v) => setLed('knob', i, v)} />
        </div>
      {:else if s.kind === 'fader'}
        {@const i = s.index}
        <div class="group first">
          <div class="group-title">Trigger</div>
          <ActionEditor title="Move" value={profile?.fader?.find((x) => x.index === i)?.change} onchange={(a) => edit((c) => (faderMut(c, i).change = a))} />
          <p class="hint">Runs on every movement; <code>{'{value}'}</code> becomes the position, 0 to 100.</p>
        </div>
        <div class="group">
          <div class="group-title">Track colour</div>
          <ColorField value={ledValue('fader', i)} swatches={LED_SWATCHES} resettable={false} onchange={(v) => setLed('fader', i, v)} />
        </div>
      {:else if s.kind === 'roller'}
        {@const side = s.side}
        {@const r = profile?.roller?.find((x) => x.side === side)}
        <div class="group first">
          <div class="group-title">Triggers</div>
          <ActionEditor title="Roll away" value={r?.away} onchange={(a) => edit((c) => (rollerMut(c, side).away = a))} />
          <ActionEditor title="Roll toward you" value={r?.toward} onchange={(a) => edit((c) => (rollerMut(c, side).toward = a))} />
        </div>
      {:else if module}
        <div class="group first">
          <div class="field"><span class="label">Serial</span><span class="mono value">{module.serial}</span></div>
          <div class="field"><span class="label">Firmware</span><span class="mono value">{module.firmware ?? 'unknown'}</span></div>
          <div class="field"><span class="label">Socket</span><span class="mono value">{module.socket ?? 'unknown'}</span></div>
        </div>
        {#if module.kind === 'base'}
          <div class="group">
            <div class="group-title">Placement</div>
            <div class="field">
              <span class="label">Orientation</span>
              <select class="select" value={hub.config.orientation ?? 'auto'} onchange={(e) => { const v = e.currentTarget.value as Orientation; edit((c) => (c.orientation = v === 'auto' ? undefined : v)); }}>
                {#each ORIENTATIONS as [v, label]}<option value={v}>{label}</option>{/each}
              </select>
            </div>
            <label class="field">
              <span class="label">Upright screens</span>
              <span class="switch">
                <input type="checkbox" checked={hub.config.upright !== false} onchange={(e) => { const on = e.currentTarget.checked; edit((c) => (c.upright = on ? undefined : false)); }} />
                <i></i>
              </span>
            </label>
            <p class="hint">Turns key and dial images to read upright from your side.</p>
          </div>
          <div class="group">
            <div class="group-title">Base light</div>
            <ColorField value={ledValue('base', 0)} swatches={LED_SWATCHES} resettable={false} onchange={(v) => setLed('base', 0, v)} />
          </div>
        {/if}
        {#if module.kind === 'base' || module.kind === 'fader' || module.kind === 'knob'}
          <div class="group">
            <div class="field">
              <span class="label">LED brightness</span>
              <input type="range" min="1" max="100" value={hub.config.leds?.brightness ?? 100} onchange={(e) => { const v = +e.currentTarget.value; edit((c) => ((c.leds ??= {}).brightness = v)); }} />
            </div>
          </div>
        {/if}
        {#if module.kind === 'base' || module.kind === 'keypad' || module.kind === 'dial'}
          <div class="group">
            <div class="field">
              <span class="label">Screen brightness</span>
              <input type="range" min="0" max="100" value={hub.config.backlight} onchange={(e) => { const v = +e.currentTarget.value; edit((c) => (c.backlight = v)); }} />
            </div>
          </div>
        {/if}
      {/if}
    {/key}
  </div>
</aside>

<style>
  .inspector {
    width: 340px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-panel);
    border-left: 1px solid var(--border);
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 52px;
    padding: 0 10px 0 16px;
    border-bottom: 1px solid var(--border);
    animation: fade-in 0.15s ease-out;
  }
  .hico {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: 7px;
    color: var(--accent-text);
    background: var(--accent-soft);
  }
  .htext {
    flex: 1;
    min-width: 0;
    display: grid;
    line-height: 1.25;
  }
  .htext h2 {
    font-size: 13.5px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .htext span {
    font-size: 12px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
  }
  .scroll > :global(.group:first-child),
  .group.first {
    border-top: none;
  }
  .hint {
    font-size: 12px;
    color: var(--text-3);
  }
  .value {
    color: var(--text-2);
    user-select: text;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .danger-text {
    justify-self: start;
    color: var(--danger);
  }
  .overview .empty {
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 48px 28px 32px;
    text-align: center;
    color: var(--text-3);
  }
  .overview .empty h3 {
    color: var(--text);
    font-size: 14px;
    font-weight: 600;
    margin-top: 4px;
  }
  .overview .empty p {
    color: var(--text-2);
    max-width: 240px;
  }
  .mod {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 32px;
    padding: 0 8px;
    margin: 0 -8px;
    font: inherit;
    color: var(--text-2);
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
  }
  .mod:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
  .mname {
    flex: 1;
    color: var(--text);
  }
  .switch {
    position: relative;
    justify-self: start;
    width: 32px;
    height: 18px;
  }
  .switch input {
    position: absolute;
    inset: 0;
    opacity: 0;
    margin: 0;
    cursor: pointer;
    z-index: 1;
  }
  .switch i {
    position: absolute;
    inset: 0;
    border-radius: 99px;
    background: var(--border-strong);
    transition: background 0.15s;
  }
  .switch i::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: white;
    transition: transform 0.15s var(--ease);
  }
  .switch input:checked + i {
    background: var(--accent);
  }
  .switch input:checked + i::after {
    transform: translateX(14px);
  }
</style>

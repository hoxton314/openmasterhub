<script lang="ts">
  import ActionEditor from './ActionEditor.svelte';
  import FaceEditor from './FaceEditor.svelte';
  import { currentPage, currentProfile, dialMut, edit, faderMut, hub, keyMut, knobMut, pageMut, rollerMut } from '../lib/hub.svelte';
  import type { Action, Face, Orientation } from '../lib/types';

  const s = $derived(hub.selection);
  const KNOB_NAMES = ['Bottom-left knob', 'Bottom-right knob', 'Top knob'];
  const KINDS: Record<string, string> = {
    base: 'Base',
    keypad: '15-key display pad',
    fader: '5 faders',
    knob: '3 knobs',
    roller: '2 rollers',
    dial: 'Dial with display',
  };
  const ORIENTATIONS: [Orientation, string][] = [
    ['auto', 'Automatic'],
    ['portrait', 'Portrait'],
    ['landscape', 'Landscape'],
    ['portrait_flipped', 'Portrait, upside down'],
    ['landscape_flipped', 'Landscape, upside down'],
  ];

  const keyFace = (i: number): Face => currentPage()?.key?.find((k) => k.index === i) ?? {};
  const keyAction = (i: number, which: 'press' | 'release') => currentPage()?.key?.find((k) => k.index === i)?.[which];
  const profile = $derived(currentProfile());
  const module = $derived(s.kind === 'module' ? hub.modules.find((m) => m.serial === s.serial) : undefined);

  function ledValue(kind: 'base' | 'fader' | 'knob', i: number): string {
    const v = hub.config.leds?.[kind];
    return (Array.isArray(v) ? v[i] : v) ?? '#ffffff';
  }
  function setLed(kind: 'base' | 'fader' | 'knob', i: number, color: string) {
    edit((c) => {
      c.leds ??= {};
      if (kind === 'base') {
        c.leds.base = color;
        return;
      }
      const n = kind === 'fader' ? 5 : 3;
      const cur = c.leds[kind];
      const list = Array.from({ length: n }, (_, k) => (Array.isArray(cur) ? cur[k] : cur) ?? '#ffffff');
      list[i] = color;
      c.leds[kind] = list;
    });
  }
</script>

<div class="inspector">
  {#key JSON.stringify(s) + hub.state.profile + hub.state.page}
    {#if s.kind === 'none'}
      <div class="status">
        <div class="glyph">⌨</div>
        <h2>Select a control</h2>
        <p>Click a key, knob, fader, roller or the dial on the hub to configure it. Click a module's body for its lighting and details.</p>
      </div>
    {:else if s.kind === 'key'}
      {@const i = s.index}
      <section>
        <h3>Key {i + 1} <span class="dim">· {currentPage()?.name ?? ''}</span></h3>
        <FaceEditor face={keyFace(i)} onchange={(f) => edit((c) => f(keyMut(c, i)))} />
      </section>
      <section>
        <h3>Actions</h3>
        <ActionEditor title="On press" value={keyAction(i, 'press')} onchange={(a) => edit((c) => (keyMut(c, i).press = a))} />
        <ActionEditor title="On release" value={keyAction(i, 'release')} onchange={(a) => edit((c) => (keyMut(c, i).release = a))} />
      </section>
      <section>
        <button class="destructive" onclick={() => edit((c) => { const p = pageMut(c); p.key = (p.key ?? []).filter((k) => k.index !== i); })}>Clear key</button>
      </section>
    {:else if s.kind === 'dial'}
      {@const d = profile?.dial ?? {}}
      <section>
        <h3>Dial screen</h3>
        <FaceEditor face={d} onchange={(f) => edit((c) => f(dialMut(c)))} />
      </section>
      <section>
        <h3>Actions</h3>
        {#each [['cw', 'Turn clockwise'], ['ccw', 'Turn counter-clockwise'], ['press', 'Press'], ['release', 'Release']] as const as [k, label]}
          <ActionEditor title={label} value={d[k]} onchange={(a: Action | undefined) => edit((c) => (dialMut(c)[k] = a))} />
        {/each}
      </section>
    {:else if s.kind === 'knob'}
      {@const i = s.index}
      {@const k = profile?.knob?.find((x) => x.index === i) ?? { index: i }}
      <section>
        <h3>{KNOB_NAMES[i]}</h3>
        {#each [['cw', 'Turn clockwise'], ['ccw', 'Turn counter-clockwise'], ['press', 'Press'], ['release', 'Release']] as const as [key, label]}
          <ActionEditor title={label} value={k[key]} onchange={(a: Action | undefined) => edit((c) => (knobMut(c, i)[key] = a))} />
        {/each}
      </section>
    {:else if s.kind === 'fader'}
      {@const i = s.index}
      <section>
        <h3>Fader {i + 1}</h3>
        <p class="hint">Runs on every movement. <code>{'{value}'}</code> is replaced by the position, 0 (left) to 100 (right).</p>
        <ActionEditor title="On change" value={profile?.fader?.find((x) => x.index === i)?.change} onchange={(a) => edit((c) => (faderMut(c, i).change = a))} />
      </section>
    {:else if s.kind === 'roller'}
      {@const side = s.side}
      {@const r = profile?.roller?.find((x) => x.side === side)}
      <section>
        <h3>{side === 'left' ? 'Left' : 'Right'} roller</h3>
        <ActionEditor title="Roll away from you" value={r?.away} onchange={(a) => edit((c) => (rollerMut(c, side).away = a))} />
        <ActionEditor title="Roll toward you" value={r?.toward} onchange={(a) => edit((c) => (rollerMut(c, side).toward = a))} />
      </section>
    {:else if module}
      <section>
        <h3>{KINDS[module.kind]}</h3>
        <div class="row prop"><span>Serial number</span><code>{module.serial}</code></div>
        <div class="row prop"><span>Firmware</span><code>{module.firmware ?? 'unknown'}</code></div>
        <div class="row prop"><span>Socket</span><code>{module.socket ?? 'unknown'}</code></div>
      </section>
      {#if module.kind === 'base'}
        <section>
          <h3>Placement</h3>
          <label class="row">
            <span>Orientation</span>
            <select value={hub.config.orientation ?? 'auto'} onchange={(e) => { const v = e.currentTarget.value as Orientation; edit((c) => (c.orientation = v === 'auto' ? undefined : v)); }}>
              {#each ORIENTATIONS as [v, label]}<option value={v}>{label}</option>{/each}
            </select>
          </label>
          <label class="row">
            <span>Keep screens upright</span>
            <input type="checkbox" class="switch" checked={hub.config.upright !== false} onchange={(e) => { const on = e.currentTarget.checked; edit((c) => (c.upright = on ? undefined : false)); }} />
          </label>
        </section>
      {/if}
      {#if module.kind === 'base' || module.kind === 'fader' || module.kind === 'knob'}
        {@const kind = module.kind}
        {@const names = kind === 'base' ? ['Base LED'] : kind === 'knob' ? KNOB_NAMES : [1, 2, 3, 4, 5].map((n) => `Fader ${n}`)}
        <section>
          <h3>Lighting</h3>
          <p class="hint">Shared by all profiles; applied immediately.</p>
          {#each names as name, i}
            <div class="row"><span>{name}</span><input type="color" value={ledValue(kind, i)} oninput={(e) => setLed(kind, i, e.currentTarget.value)} /></div>
          {/each}
          <label class="row">
            <span>LED brightness</span>
            <input type="range" min="1" max="100" value={hub.config.leds?.brightness ?? 100} onchange={(e) => { const v = +e.currentTarget.value; edit((c) => ((c.leds ??= {}).brightness = v)); }} />
          </label>
        </section>
      {/if}
      {#if module.kind === 'base' || module.kind === 'keypad' || module.kind === 'dial'}
        <section>
          <h3>Screens</h3>
          <label class="row">
            <span>Screen brightness</span>
            <input type="range" min="0" max="100" value={hub.config.backlight} onchange={(e) => { const v = +e.currentTarget.value; edit((c) => (c.backlight = v)); }} />
          </label>
        </section>
      {/if}
    {/if}
  {/key}
</div>

<style>
  .inspector {
    padding: 18px;
  }
  .status {
    text-align: center;
    margin-top: 30%;
    color: var(--fg-dim);
  }
  .status .glyph {
    font-size: 64px;
    opacity: 0.6;
  }
  .status h2 {
    color: var(--fg);
  }
  .prop code {
    color: var(--fg-dim);
    user-select: text;
  }
  .switch {
    width: 18px;
    height: 18px;
  }
</style>

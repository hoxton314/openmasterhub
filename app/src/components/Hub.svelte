<script lang="ts">
  import { hub, displayKey, flashKey, ledColor } from '../lib/hub.svelte';
  import {
    FOOTPRINT, GRID_COLS, GRID_ROWS, KNOB_CENTERS, ROLLER_RECTS, apply, faderX, keyRect, placementMatrix, svg, then,
    translate, viewMatrix, type Matrix,
  } from '../lib/geometry';
  import { TARGET_DIAL, type ModuleState, type Selection } from '../lib/types';

  const PAD = 0.5;

  interface Placed {
    m: ModuleState;
    matrix: Matrix;
    unknown: boolean;
  }

  // Known sockets where they sit (possibly overhanging the Base); unknown ones in a tray.
  const layout = $derived.by(() => {
    const view = viewMatrix(hub.state.rotation ?? 90);
    const mods = [...hub.modules].sort((a, b) => Number(a.kind !== 'base') - Number(b.kind !== 'base'));
    const placed: Placed[] = [];
    const tray: ModuleState[] = [];
    for (const m of mods) {
      const p = m.placement ?? (m.kind === 'base' ? { col: 0, row: 0, cols: GRID_COLS, rows: GRID_ROWS, rotation: 0 } : undefined);
      if (p) placed.push({ m, matrix: then(placementMatrix(p, m.kind), view), unknown: false });
      else tray.push(m);
    }
    let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
    const add = (mx: Matrix, [w, h]: [number, number]) => {
      for (const [x, y] of [[0, 0], [w, 0], [0, h], [w, h]]) {
        const [px, py] = apply(mx, x, y);
        x0 = Math.min(x0, px); y0 = Math.min(y0, py); x1 = Math.max(x1, px); y1 = Math.max(y1, py);
      }
    };
    for (const p of placed) add(p.matrix, FOOTPRINT[p.m.kind]);
    if (!isFinite(x0)) add(view, [GRID_COLS, GRID_ROWS]);
    let tx = x0;
    const ty = y1 + 0.9;
    for (const m of tray) {
      const matrix = translate(tx, ty);
      add(matrix, FOOTPRINT[m.kind]);
      tx += FOOTPRINT[m.kind][0] + 0.3;
      placed.push({ m, matrix, unknown: true });
    }
    const top = tray.length ? 0.35 : 0;
    return { placed, viewBox: `${x0 - PAD} ${y0 - PAD - top} ${x1 - x0 + 2 * PAD} ${y1 - y0 + 2 * PAD + top}` };
  });

  const sel = (s: Selection) => (e: Event) => {
    e.stopPropagation();
    hub.selection = s;
  };
  const isSel = (s: Selection) => JSON.stringify(hub.selection) === JSON.stringify(s);
  const flash = (serial: string, control: string, index: number) => hub.flashes[flashKey(serial, control, index)];
</script>

<svg class="hub" viewBox={layout.viewBox} role="presentation" onclick={() => (hub.selection = { kind: 'none' })}>
  <defs>
    <clipPath id="key-clip" clipPathUnits="objectBoundingBox"><rect width="1" height="1" rx="0.13" /></clipPath>
    <clipPath id="round-clip" clipPathUnits="objectBoundingBox"><circle cx="0.5" cy="0.5" r="0.5" /></clipPath>
  </defs>

  {#each layout.placed as { m, matrix, unknown } (m.serial)}
    {@const [w, h] = FOOTPRINT[m.kind]}
    {@const modSel = isSel({ kind: 'module', serial: m.serial })}
    <g transform={svg(matrix)} class="module" class:unknown>
      {#if m.kind === 'base'}
        {@const led = ledColor('base', 0)}
        {#if led}<rect class="base-led" x="-0.05" y="-0.05" width={w + 0.1} height={h + 0.1} rx="0.27" stroke={led} />{/if}
        <rect class="base" width={w} height={h} rx="0.22" onclick={sel({ kind: 'module', serial: m.serial })} role="presentation" />
        {#each Array(GRID_COLS * GRID_ROWS) as _, i}
          <circle cx={(i % GRID_COLS) + 0.5} cy={Math.floor(i / GRID_COLS) + 0.5} r="0.035" class="socket" />
        {/each}
      {:else}
        <rect class="body" x="0.06" y="0.06" width={w - 0.12} height={h - 0.12} rx="0.18" onclick={sel({ kind: 'module', serial: m.serial })} role="presentation" />
      {/if}

      {#if m.kind === 'keypad'}
        <rect class="strip" x="0.6" y="2.52" width="2.8" height="0.2" rx="0.05" />
        <text class="brand" x="2" y="2.655">COOLER MASTER</text>
        {#each Array(15) as _, i}
          {@const [x, y, s] = keyRect(i)}
          {@const src = hub.displays[displayKey(m.serial, i)]}
          {@const selected = isSel({ kind: 'key', index: i })}
          <g class="control" class:selected onclick={sel({ kind: 'key', index: i })} role="presentation">
            <rect class="slot" x={x} y={y} width={s} height={s} rx="0.08" />
            {#if src}<image href={src} x={x} y={y} width={s} height={s} clip-path="url(#key-clip)" preserveAspectRatio="none" />{/if}
            <rect class="outline" x={x} y={y} width={s} height={s} rx="0.08" />
            {#key flash(m.serial, 'key', i)}
              {#if flash(m.serial, 'key', i)}<rect class="flash" x={x} y={y} width={s} height={s} rx="0.08" />{/if}
            {/key}
          </g>
        {/each}
      {:else if m.kind === 'dial'}
        {@const src = hub.displays[displayKey(m.serial, TARGET_DIAL)]}
        <g class="control" class:selected={isSel({ kind: 'dial' })} onclick={sel({ kind: 'dial' })} role="presentation">
          <circle class="ring" cx="1" cy="1" r="0.78" />
          {#each Array(60) as _, i}
            {@const a = (i / 60) * Math.PI * 2}
            <line class="tick" x1={1 + 0.7 * Math.cos(a)} y1={1 + 0.7 * Math.sin(a)} x2={1 + 0.76 * Math.cos(a)} y2={1 + 0.76 * Math.sin(a)} />
          {/each}
          <circle class="slot" cx="1" cy="1" r="0.62" />
          {#if src}<image href={src} x="0.38" y="0.38" width="1.24" height="1.24" clip-path="url(#round-clip)" />{/if}
          <circle class="outline" cx="1" cy="1" r="0.78" />
          {#key flash(m.serial, 'dial', 0)}
            {#if flash(m.serial, 'dial', 0)}<circle class="flash" cx="1" cy="1" r="0.78" />{/if}
          {/key}
        </g>
      {:else if m.kind === 'knob'}
        {#each KNOB_CENTERS as [cx, cy], i}
          {@const led = ledColor('knob', i)}
          <g class="control" class:selected={isSel({ kind: 'knob', index: i })} onclick={sel({ kind: 'knob', index: i })} role="presentation">
            {#if led}<circle class="led" cx={cx} cy={cy} r="0.355" stroke={led} />{/if}
            <circle class="cap" cx={cx} cy={cy} r="0.29" />
            <line class="pointer" x1={cx} y1={cy - 0.11} x2={cx} y2={cy - 0.24} />
            <circle class="outline" cx={cx} cy={cy} r="0.29" />
            {#key flash(m.serial, 'knob', i)}
              {#if flash(m.serial, 'knob', i)}<circle class="flash" cx={cx} cy={cy} r="0.355" />{/if}
            {/key}
          </g>
        {/each}
      {:else if m.kind === 'roller'}
        {#each [['left', 0], ['right', 1]] as const as [side, idx]}
          {@const [x, y, rw, rh] = ROLLER_RECTS[side]}
          <g class="control" class:selected={isSel({ kind: 'roller', side })} onclick={sel({ kind: 'roller', side })} role="presentation">
            <rect class="cap" x={x} y={y} width={rw} height={rh} rx={rh / 2} />
            {#each Array(15) as _, k}
              {@const lx = x + 0.2 + (k * (rw - 0.4)) / 14}
              <line class="tick" x1={lx} x2={lx} y1={y + 0.14} y2={y + rh - 0.14} />
            {/each}
            <rect class="outline" x={x} y={y} width={rw} height={rh} rx={rh / 2} />
            {#key flash(m.serial, 'roller', idx)}
              {#if flash(m.serial, 'roller', idx)}<rect class="flash" x={x} y={y} width={rw} height={rh} rx={rh / 2} />{/if}
            {/key}
          </g>
        {/each}
      {:else if m.kind === 'fader'}
        {#each Array(5) as _, i}
          {@const x = faderX(i)}
          {@const led = ledColor('fader', i)}
          {@const pos = (hub.faders[`${m.serial}/${i}`] ?? 50) / 100}
          {@const ky = 1.7 - pos * 1.4}
          <g class="control" class:selected={isSel({ kind: 'fader', index: i })} onclick={sel({ kind: 'fader', index: i })} role="presentation">
            <rect x={x - 0.3} y="0.15" width="0.6" height="1.7" rx="0.12" class="hit" />
            <line class="track" x1={x} y1="0.3" x2={x} y2="1.7" stroke={led ?? 'currentColor'} />
            <rect class="cap fader-cap" x={x - 0.17} y={ky - 0.1} width="0.34" height="0.2" rx="0.05" />
            <rect class="outline" x={x - 0.3} y="0.15" width="0.6" height="1.7" rx="0.12" />
            {#key flash(m.serial, 'fader', i)}
              {#if flash(m.serial, 'fader', i)}<rect class="flash" x={x - 0.17} y={ky - 0.1} width="0.34" height="0.2" rx="0.05" />{/if}
            {/key}
          </g>
        {/each}
      {/if}

      {#if unknown}
        <rect class="unknown-outline" x="0.02" y="0.02" width={w - 0.04} height={h - 0.04} rx="0.18" />
        <text x="0.06" y="-0.14" class="unknown-label">Position unknown · socket {m.socket ?? '?'}</text>
      {/if}
      {#if modSel}<rect class="module-sel" x="0.03" y="0.03" width={w - 0.06} height={h - 0.06} rx="0.2" />{/if}
    </g>
  {/each}
</svg>

<style>
  .hub {
    width: 100%;
    height: 100%;
    display: block;
    overflow: visible;
  }
  .hub :global(*) {
    vector-effect: non-scaling-stroke;
  }

  .base {
    fill: var(--hub-base);
    stroke: var(--hub-base-stroke);
    stroke-width: 1;
    cursor: pointer;
  }
  .base-led {
    fill: none;
    stroke-width: 6;
    opacity: 0.22;
    filter: blur(3px);
    pointer-events: none;
  }
  .socket {
    fill: var(--text-3);
    opacity: 0.35;
    pointer-events: none;
  }
  .body {
    fill: var(--hub-module);
    stroke: var(--hub-module-stroke);
    stroke-width: 1;
    cursor: pointer;
    transition: stroke 0.15s;
  }
  .body:hover {
    stroke: var(--text-3);
  }

  .control {
    cursor: pointer;
  }
  .slot {
    fill: var(--hub-empty);
  }
  .cap {
    fill: var(--hub-control);
  }
  .ring {
    fill: var(--hub-control);
  }
  .outline {
    fill: none;
    stroke: var(--hub-control-stroke);
    stroke-width: 1;
    transition: stroke 0.15s, stroke-width 0.15s;
  }
  .hit {
    fill: transparent;
  }
  .tick {
    stroke: var(--hub-control-stroke);
    stroke-width: 1;
  }
  .pointer {
    stroke: var(--text-2);
    stroke-width: 1.5;
    stroke-linecap: round;
  }
  .led {
    fill: none;
    stroke-width: 2;
    opacity: 0.9;
  }
  .track {
    stroke-width: 2;
    stroke-linecap: round;
    opacity: 0.7;
  }
  .fader-cap {
    stroke: var(--hub-control-stroke);
    stroke-width: 1;
    transition: y 0.08s linear;
  }
  .fader-cap + .outline {
    stroke: transparent;
  }

  .control:hover .outline {
    stroke: var(--text-3);
  }
  .control.selected .outline,
  .control.selected .fader-cap + .outline {
    stroke: var(--accent);
    stroke-width: 2;
  }

  .flash {
    fill: none;
    stroke: var(--accent);
    stroke-width: 3;
    animation: flash 0.45s ease-out forwards;
    pointer-events: none;
  }
  @keyframes flash {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }

  .strip {
    fill: var(--hub-empty);
    pointer-events: none;
  }
  .brand {
    font-size: 0.1px;
    letter-spacing: 0.02px;
    font-weight: 600;
    fill: var(--text-3);
    text-anchor: middle;
    pointer-events: none;
  }
  .module-sel {
    fill: var(--accent-soft);
    stroke: var(--accent);
    stroke-width: 1.5;
    pointer-events: none;
  }
  .unknown-outline {
    fill: none;
    stroke: var(--warning);
    stroke-width: 1;
    stroke-dasharray: 4 3;
    pointer-events: none;
  }
  .unknown-label {
    font-size: 0.15px;
    fill: var(--warning);
  }
</style>

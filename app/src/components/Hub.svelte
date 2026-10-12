<script lang="ts">
  import { hub, displayKey, flashKey, ledColor } from '../lib/hub.svelte';
  import {
    FOOTPRINT, GRID_COLS, GRID_ROWS, KNOB_CENTERS, ROLLER_RECTS, apply, faderX, keyRect, placementMatrix, svg, then,
    translate, viewMatrix, type Matrix,
  } from '../lib/geometry';
  import { TARGET_DIAL, type ModuleState, type Selection } from '../lib/types';

  const PAD = 0.35;

  interface Placed {
    m: ModuleState;
    matrix: Matrix;
    unknown: boolean;
  }

  // Lay out: known sockets where they sit (possibly overhanging), unknown ones in a tray.
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
    const ty = y1 + 0.8;
    for (const m of tray) {
      const matrix = translate(tx, ty);
      add(matrix, FOOTPRINT[m.kind]);
      tx += FOOTPRINT[m.kind][0] + 0.3;
      placed.push({ m, matrix, unknown: true });
    }
    return { placed, viewBox: `${x0 - PAD} ${y0 - PAD - (tray.length ? 0.3 : 0)} ${x1 - x0 + 2 * PAD} ${y1 - y0 + 2 * PAD + (tray.length ? 0.3 : 0)}` };
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
    <linearGradient id="body" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#2b2b30" />
      <stop offset="1" stop-color="#17171a" />
    </linearGradient>
    <radialGradient id="knob" cx="0.4" cy="0.35" r="0.7">
      <stop offset="0" stop-color="#5c5c63" />
      <stop offset="1" stop-color="#1f1f22" />
    </radialGradient>
    <linearGradient id="roller" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#19191b" />
      <stop offset="0.5" stop-color="#4a4a50" />
      <stop offset="1" stop-color="#141416" />
    </linearGradient>
    <clipPath id="key-clip" clipPathUnits="objectBoundingBox"><rect width="1" height="1" rx="0.12" /></clipPath>
    <clipPath id="round-clip" clipPathUnits="objectBoundingBox"><circle cx="0.5" cy="0.5" r="0.5" /></clipPath>
    <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="0.08" />
    </filter>
  </defs>

  {#each layout.placed as { m, matrix, unknown } (m.serial)}
    {@const [w, h] = FOOTPRINT[m.kind]}
    <g transform={svg(matrix)} class="module" class:selected={isSel({ kind: 'module', serial: m.serial })}>
      {#if m.kind === 'base'}
        {@const led = ledColor('base', 0)}
        {#if led}
          <rect x="-0.12" y="-0.12" width={w + 0.24} height={h + 0.24} rx="0.32" fill="none" stroke={led} stroke-width="0.1" opacity="0.55" filter="url(#glow)" />
        {/if}
        <rect width={w} height={h} rx="0.2" fill="#0f0f12" onclick={sel({ kind: 'module', serial: m.serial })} role="presentation" />
        {#each Array(GRID_COLS * GRID_ROWS) as _, i}
          <rect x={(i % GRID_COLS) + 0.08} y={Math.floor(i / GRID_COLS) + 0.08} width="0.84" height="0.84" rx="0.1" fill="white" opacity="0.035" pointer-events="none" />
        {/each}
      {:else}
        <rect x="0.05" y="0.05" width={w - 0.1} height={h - 0.1} rx="0.16" fill="url(#body)" stroke="white" stroke-opacity="0.08" stroke-width="0.02" onclick={sel({ kind: 'module', serial: m.serial })} role="presentation" />
      {/if}

      {#if m.kind === 'keypad'}
        {#each Array(15) as _, i}
          {@const [x, y, s] = keyRect(i)}
          {@const src = hub.displays[displayKey(m.serial, i)]}
          <g class="control" onclick={sel({ kind: 'key', index: i })} role="presentation">
            <rect x={x - 0.03} y={y - 0.03} width={s + 0.06} height={s + 0.06} rx="0.09" fill="#08080a" />
            {#if src}
              <image href={src} x={x} y={y} width={s} height={s} clip-path="url(#key-clip)" preserveAspectRatio="none" />
            {:else}
              <rect x={x} y={y} width={s} height={s} rx="0.07" fill="#1e2026" />
            {/if}
            {#key flash(m.serial, 'key', i)}
              {#if flash(m.serial, 'key', i)}<rect class="flash" x={x - 0.02} y={y - 0.02} width={s + 0.04} height={s + 0.04} rx="0.08" />{/if}
            {/key}
            {#if isSel({ kind: 'key', index: i })}<rect class="sel" x={x - 0.05} y={y - 0.05} width={s + 0.1} height={s + 0.1} rx="0.1" />{/if}
          </g>
        {/each}
        <rect x="0.55" y="2.5" width="2.9" height="0.26" rx="0.04" fill="#221f1f" pointer-events="none" />
        <text x="2" y="2.68" class="brand">C O O L E R &nbsp; M A S T E R</text>
      {:else if m.kind === 'dial'}
        {@const src = hub.displays[displayKey(m.serial, TARGET_DIAL)]}
        <g class="control" onclick={sel({ kind: 'dial' })} role="presentation">
          <circle cx="1" cy="1" r="0.8" fill="url(#knob)" />
          {#each Array(48) as _, i}
            {@const a = (i / 48) * Math.PI * 2}
            <line x1={1 + 0.72 * Math.cos(a)} y1={1 + 0.72 * Math.sin(a)} x2={1 + 0.8 * Math.cos(a)} y2={1 + 0.8 * Math.sin(a)} stroke="black" stroke-opacity="0.5" stroke-width="0.012" />
          {/each}
          {#if src}
            <image href={src} x="0.38" y="0.38" width="1.24" height="1.24" clip-path="url(#round-clip)" />
          {:else}
            <circle cx="1" cy="1" r="0.62" fill="#0c0c0e" />
          {/if}
          {#key flash(m.serial, 'dial', 0)}
            {#if flash(m.serial, 'dial', 0)}<circle class="flash" cx="1" cy="1" r="0.82" />{/if}
          {/key}
          {#if isSel({ kind: 'dial' })}<circle class="sel" cx="1" cy="1" r="0.86" />{/if}
        </g>
      {:else if m.kind === 'knob'}
        {#each KNOB_CENTERS as [cx, cy], i}
          {@const led = ledColor('knob', i)}
          <g class="control" onclick={sel({ kind: 'knob', index: i })} role="presentation">
            {#if led}<circle cx={cx} cy={cy} r="0.36" fill="none" stroke={led} stroke-width="0.045" filter="url(#glow)" />{/if}
            {#if led}<circle cx={cx} cy={cy} r="0.36" fill="none" stroke={led} stroke-width="0.03" />{/if}
            <circle cx={cx} cy={cy} r="0.3" fill="url(#knob)" />
            <line x1={cx} y1={cy - 0.12} x2={cx} y2={cy - 0.27} stroke="#e8e8e8" stroke-opacity="0.75" stroke-width="0.03" stroke-linecap="round" />
            {#key flash(m.serial, 'knob', i)}
              {#if flash(m.serial, 'knob', i)}<circle class="flash" cx={cx} cy={cy} r="0.4" />{/if}
            {/key}
            {#if isSel({ kind: 'knob', index: i })}<circle class="sel" cx={cx} cy={cy} r="0.44" />{/if}
          </g>
        {/each}
      {:else if m.kind === 'roller'}
        {#each [['left', 0], ['right', 1]] as const as [side, idx]}
          {@const [x, y, rw, rh] = ROLLER_RECTS[side]}
          <g class="control" onclick={sel({ kind: 'roller', side })} role="presentation">
            <rect x={x} y={y} width={rw} height={rh} rx="0.1" fill="url(#roller)" />
            {#each Array(12) as _, k}
              <line x1={x + 0.1} x2={x + rw - 0.1} y1={y + 0.04 + (k * (rh - 0.08)) / 11} y2={y + 0.04 + (k * (rh - 0.08)) / 11} stroke="black" stroke-opacity="0.35" stroke-width="0.012" />
            {/each}
            {#key flash(m.serial, 'roller', idx)}
              {#if flash(m.serial, 'roller', idx)}<rect class="flash" x={x - 0.03} y={y - 0.03} width={rw + 0.06} height={rh + 0.06} rx="0.12" />{/if}
            {/key}
            {#if isSel({ kind: 'roller', side })}<rect class="sel" x={x - 0.06} y={y - 0.06} width={rw + 0.12} height={rh + 0.12} rx="0.14" />{/if}
          </g>
        {/each}
      {:else if m.kind === 'fader'}
        {#each Array(5) as _, i}
          {@const x = faderX(i)}
          {@const led = ledColor('fader', i)}
          {@const pos = (hub.faders[`${m.serial}/${i}`] ?? 50) / 100}
          {@const ky = 1.75 - pos * 1.5}
          <g class="control" onclick={sel({ kind: 'fader', index: i })} role="presentation">
            <rect x={x - 0.3} y="0.12" width="0.6" height="1.76" fill="transparent" />
            {#if led}<line x1={x} y1="0.25" x2={x} y2="1.75" stroke={led} stroke-width="0.1" opacity="0.6" />{/if}
            <line x1={x} y1="0.25" x2={x} y2="1.75" stroke="black" stroke-width="0.05" />
            <rect class="fader-cap" x={x - 0.2} y={ky - 0.12} width="0.4" height="0.24" rx="0.05" />
            {#key flash(m.serial, 'fader', i)}
              {#if flash(m.serial, 'fader', i)}<rect class="flash" x={x - 0.24} y={ky - 0.16} width="0.48" height="0.32" rx="0.07" />{/if}
            {/key}
            {#if isSel({ kind: 'fader', index: i })}<rect class="sel" x={x - 0.3} y="0.12" width="0.6" height="1.76" rx="0.1" />{/if}
          </g>
        {/each}
      {/if}

      {#if unknown}
        <rect x="0.02" y="0.02" width={w - 0.04} height={h - 0.04} rx="0.16" fill="none" stroke="#ffbf4d" stroke-width="0.025" stroke-dasharray="0.12 0.08" pointer-events="none" />
        <text x="0.05" y="-0.12" class="unknown">position unknown (socket {m.socket ?? '?'})</text>
      {/if}
      {#if isSel({ kind: 'module', serial: m.serial })}
        <rect class="sel" x="0.04" y="0.04" width={w - 0.08} height={h - 0.08} rx="0.16" />
      {/if}
    </g>
  {/each}

  {#if !hub.modules.some((m) => m.kind === 'base')}
    <text x={GRID_ROWS / 2} y={GRID_COLS / 2} class="empty">No MasterHUB connected</text>
  {/if}
</svg>

<style>
  .hub {
    width: 100%;
    height: 100%;
    display: block;
  }
  .control {
    cursor: pointer;
  }
  .control:hover > :first-child {
    filter: brightness(1.25);
  }
  .sel {
    fill: none;
    stroke: var(--accent);
    stroke-width: 0.04;
  }
  .flash {
    fill: none;
    stroke: var(--glow);
    stroke-width: 0.05;
    animation: fade 0.35s ease-out forwards;
  }
  @keyframes fade {
    from { opacity: 1; }
    to { opacity: 0; }
  }
  .fader-cap {
    fill: #45454c;
    transition: y 0.08s linear;
  }
  .brand {
    font-size: 0.13px;
    fill: #d8c0b8;
    opacity: 0.8;
    text-anchor: middle;
    pointer-events: none;
  }
  .unknown {
    font-size: 0.16px;
    fill: #ffbf4d;
  }
  .empty {
    font-size: 0.3px;
    fill: var(--fg-dim);
    text-anchor: middle;
  }
</style>

// Hub drawing geometry, in cells. Each module is drawn in its natural orientation; an SVG
// transform places it on the Base (vertical frame) and rotates the Base to the user's view.

import type { ModuleKind, ModuleState, Placement } from './types';

export const GRID_COLS = 4;
export const GRID_ROWS = 6;

export const FOOTPRINT: Record<ModuleKind, [number, number]> = {
  base: [GRID_COLS, GRID_ROWS],
  keypad: [4, 3],
  fader: [4, 2],
  knob: [2, 2],
  dial: [2, 2],
  roller: [4, 1],
};

/** 2D affine matrix [a, b, c, d, e, f] as in SVG `matrix()`. */
export type Matrix = [number, number, number, number, number, number];

export const IDENTITY: Matrix = [1, 0, 0, 1, 0, 0];

/** m1 applied first, then m2. */
export function then(m1: Matrix, m2: Matrix): Matrix {
  const [a1, b1, c1, d1, e1, f1] = m1;
  const [a2, b2, c2, d2, e2, f2] = m2;
  return [a2 * a1 + c2 * b1, b2 * a1 + d2 * b1, a2 * c1 + c2 * d1, b2 * c1 + d2 * d1, a2 * e1 + c2 * f1 + e2, b2 * e1 + d2 * f1 + f2];
}

export const translate = (x: number, y: number): Matrix => [1, 0, 0, 1, x, y];

/** Clockwise rotation by a multiple of 90° (screen coordinates, y down). */
export function rot(deg: number): Matrix {
  switch (((deg % 360) + 360) % 360) {
    case 90:
      return [0, 1, -1, 0, 0, 0];
    case 180:
      return [-1, 0, 0, -1, 0, 0];
    case 270:
      return [0, -1, 1, 0, 0, 0];
    default:
      return IDENTITY;
  }
}

export const apply = (m: Matrix, x: number, y: number): [number, number] => [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]];

export const svg = (m: Matrix) => `matrix(${m.join(' ')})`;

/** Base vertical frame → view, for the Base's rotation on the desk. */
export function viewMatrix(rotation: number): Matrix {
  switch (rotation) {
    case 90:
      return then(rot(90), translate(GRID_ROWS, 0));
    case 180:
      return then(rot(180), translate(GRID_COLS, GRID_ROWS));
    case 270:
      return then(rot(270), translate(0, GRID_COLS));
    default:
      return IDENTITY;
  }
}

/** Natural module coordinates → Base vertical frame. */
export function placementMatrix(p: Placement, kind: ModuleKind): Matrix {
  const [w, h] = FOOTPRINT[kind];
  let m = rot(p.rotation);
  switch (p.rotation) {
    case 90:
      m = then(m, translate(h, 0));
      break;
    case 180:
      m = then(m, translate(w, h));
      break;
    case 270:
      m = then(m, translate(0, w));
      break;
  }
  return then(m, translate(p.col, p.row));
}

// Control positions in natural module coordinates.

export function keyRect(i: number): [number, number, number, number] {
  const size = 0.6;
  const gap = 0.13;
  const x0 = (4 - (5 * size + 4 * gap)) / 2;
  return [x0 + (i % 5) * (size + gap), 0.24 + Math.floor(i / 5) * (size + gap), size, size];
}

export const KNOB_CENTERS: [number, number][] = [
  [0.56, 1.38],
  [1.44, 1.38],
  [1.0, 0.62],
];

export const ROLLER_RECTS: Record<'left' | 'right', [number, number, number, number]> = {
  left: [0.18, 0.2, 1.72, 0.6],
  right: [2.1, 0.2, 1.72, 0.6],
};

export const faderX = (i: number) => 0.4 + i * 0.8;

// ───────────── pixel layout of the editor canvas (HubCanvas) ─────────────
// The canvas draws the hub as HTML at 1 cell = U px (so token sizes in the theme read 1:1),
// then scales the whole stage to fit. Module-local positions below are in px, natural orientation.

/** Pixels per cell. */
export const U = 92;
/** The Base's visible margin around its sockets. */
export const BASE_MARGIN = 17;

export type Box = [x: number, y: number, w: number, h: number];

export const KEY_PX = 62;
const KEY_GAP = 6;

/** Key `i` (row-major, 5×3) in the keypad module. */
export function keyBox(i: number): Box {
  const x0 = (FOOTPRINT.keypad[0] * U - (5 * KEY_PX + 4 * KEY_GAP)) / 2;
  return [x0 + (i % 5) * (KEY_PX + KEY_GAP), 17 + Math.floor(i / 5) * (KEY_PX + KEY_GAP), KEY_PX, KEY_PX];
}

/** The dial module: ring centre and radii. */
export const DIAL_PX = { cx: U, cy: U, ring: 78, face: 62 };
/** Knob centres and radius in the knob module (a row of three). */
export const KNOB_PX = { r: 27, centers: [[33, 80], [92, 80], [151, 80]] as [number, number][] };
/** Roller rectangles in the roller module. */
export const ROLLER_PX: Record<'left' | 'right', Box> = { left: [20, 18, 152, 48], right: [196, 18, 152, 48] };
/** Fader track box `i` (5 across the fader module). */
export const faderBox = (i: number): Box => [40 + i * 72 - 20, 18, 40, 148];

/** Matrix in cells → CSS `matrix()` in px. */
export const cssMatrix = (m: Matrix) => `matrix(${m[0]}, ${m[1]}, ${m[2]}, ${m[3]}, ${m[4] * U}, ${m[5] * U})`;

/** Axis-aligned bounds, in px, of a natural-px box under a cell matrix (rotations are multiples of 90°). */
export function boundsPx(m: Matrix, [x, y, w, h]: Box): Box {
  const pts = [[x, y], [x + w, y], [x, y + h], [x + w, y + h]].map(([px, py]) => [m[0] * px + m[2] * py + m[4] * U, m[1] * px + m[3] * py + m[5] * U]);
  const xs = pts.map((p) => p[0]);
  const ys = pts.map((p) => p[1]);
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys)];
}

export interface PlacedModule {
  m: ModuleState;
  matrix: Matrix;
  /** In the tray below the hub: the service did not report where it sits. */
  unknown: boolean;
  /** Px bounds of the module in the world. */
  bounds: Box;
}

/**
 * Where every module sits on the canvas, shared by the editor and the read-only drawings so they
 * agree: the Base first, placed modules by socket and rotation, unplaced ones in a tray below.
 * `w`/`h` are the stage size in px including `pad`; `ox`/`oy` move the world into it.
 */
export function layoutModules(modules: ModuleState[], rotation: number, pad: number, tray = true) {
  const view = viewMatrix(rotation);
  const mods = [...modules].sort((a, b) => Number(a.kind !== 'base') - Number(b.kind !== 'base'));
  const placed: PlacedModule[] = [];
  const unplaced: ModuleState[] = [];
  const fp = (k: ModuleKind): Box => [0, 0, FOOTPRINT[k][0] * U, FOOTPRINT[k][1] * U];
  for (const m of mods) {
    const p = m.placement ?? (m.kind === 'base' ? { col: 0, row: 0, cols: GRID_COLS, rows: GRID_ROWS, rotation: 0 } : undefined);
    if (p) {
      const matrix = then(placementMatrix(p, m.kind), view);
      placed.push({ m, matrix, unknown: false, bounds: boundsPx(matrix, fp(m.kind)) });
    } else unplaced.push(m);
  }
  const rest = tray ? unplaced : [];
  let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
  const grow = ([x, y, w, h]: Box, extra = 0) => {
    x0 = Math.min(x0, x - extra);
    y0 = Math.min(y0, y - extra);
    x1 = Math.max(x1, x + w + extra);
    y1 = Math.max(y1, y + h + extra);
  };
  for (const p of placed) grow(p.bounds, p.m.kind === 'base' ? BASE_MARGIN : 0);
  if (!isFinite(x0)) grow(boundsPx(view, [0, 0, GRID_COLS * U, GRID_ROWS * U]), BASE_MARGIN);
  let tx = x0 / U;
  const ty = y1 / U + 0.5;
  for (const m of rest) {
    const matrix = translate(tx, ty);
    const t = { m, matrix, unknown: true, bounds: boundsPx(matrix, fp(m.kind)) };
    grow(t.bounds);
    tx += FOOTPRINT[m.kind][0] + 0.3;
    placed.push(t);
  }
  const label = rest.length ? 20 : 0;
  return { placed, w: x1 - x0 + 2 * pad, h: y1 - y0 + 2 * pad + label, ox: pad - x0, oy: pad - y0 + label };
}

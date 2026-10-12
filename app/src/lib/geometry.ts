// Hub drawing geometry, in cells. Each module is drawn in its natural orientation; an SVG
// transform places it on the Base (vertical frame) and rotates the Base to the user's view.

import type { ModuleKind, Placement } from './types';

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

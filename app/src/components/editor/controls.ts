// Controls of the hub as the canvas sees them: ids, captions, and which control lies where
// (for arrow-key navigation).

import { audioTargetName } from '../../lib/hub.svelte';
import { prettyKeys } from '../../lib/keys';
import type { ControlRef } from '../../lib/model';
import type { Action, Selection } from '../../lib/types';

export const ctlId = (c: ControlRef | Selection): string => {
  switch (c.kind) {
    case 'key':
    case 'knob':
    case 'fader':
      return `${c.kind}:${c.index}`;
    case 'roller':
      return `roller:${c.side}`;
    default:
      return c.kind === 'module' ? `module:${c.serial}` : c.kind;
  }
};

export const sameSel = (a: Selection, b: Selection) => ctlId(a) === ctlId(b);

/** A short name for what an action does, shown under knobs and rollers. */
export function shortAction(a: Action | undefined): string {
  if (!a) return '';
  if ('volume' in a) return audioTargetName(typeof a.volume === 'string' ? {} : a.volume) || 'Volume';
  if ('mute' in a) return audioTargetName(typeof a.mute === 'string' ? {} : a.mute) || 'Mute';
  if ('keys' in a) {
    const combo = typeof a.keys === 'string' ? a.keys : a.keys.combo;
    return prettyKeys(combo).join(' ');
  }
  if ('cmd' in a) return a.cmd.split(/\s+/)[0].split('/').pop() ?? '';
  if ('page' in a) return a.page === 'next' || a.page === 'prev' ? 'Pages' : a.page;
  if ('folder' in a) return a.folder;
  if ('back' in a) return 'Back';
  if ('setup' in a) return 'Setups';
  if ('pin' in a) return 'Pin';
  if ('session' in a || 'sessions' in a) return 'Sessions';
  if ('profile' in a) return a.profile === 'next' || a.profile === 'prev' || a.profile === 'pick' ? 'Profiles' : a.profile;
  return '';
}

export type Dir = 'left' | 'right' | 'up' | 'down';

export interface Spot {
  id: string;
  sel: Selection;
  cx: number;
  cy: number;
}

/** The control nearest to `from` in a direction (screen space), or undefined at the edge. */
export function neighbour(spots: Spot[], from: string, dir: Dir): Spot | undefined {
  const a = spots.find((s) => s.id === from);
  if (!a) return spots[0];
  const vx = dir === 'left' ? -1 : dir === 'right' ? 1 : 0;
  const vy = dir === 'up' ? -1 : dir === 'down' ? 1 : 0;
  let best: Spot | undefined;
  let bestScore = Infinity;
  for (const s of spots) {
    if (s.id === from) continue;
    const dx = s.cx - a.cx;
    const dy = s.cy - a.cy;
    const along = dx * vx + dy * vy;
    if (along <= 1) continue;
    const across = Math.abs(dx * vy) + Math.abs(dy * vx);
    if (across > along * 1.6 + 20) continue;
    const score = along + across * 2;
    if (score < bestScore) [best, bestScore] = [s, score];
  }
  return best;
}

/** "Key 9", "Keys 2, 4", "Knobs", "Right roller" for a group of controls. */
export function describeControls(ctls: ControlRef[]): string {
  const parts: string[] = [];
  const keys = ctls.filter((c) => c.kind === 'key').map((c) => (c as { index: number }).index + 1);
  if (keys.length === 15) parts.push('All keys');
  else if (keys.length) parts.push(`${keys.length > 1 ? 'Keys' : 'Key'} ${keys.join(', ')}`);
  if (ctls.some((c) => c.kind === 'dial')) parts.push('Dial');
  const knobs = ctls.filter((c) => c.kind === 'knob').length;
  if (knobs === 3) parts.push('Knobs');
  else if (knobs) parts.push(`Knob${knobs > 1 ? 's' : ''} ${ctls.filter((c) => c.kind === 'knob').map((c) => (c as { index: number }).index + 1).join(', ')}`);
  const rollers = ctls.filter((c) => c.kind === 'roller').map((c) => (c as { side: string }).side);
  if (rollers.length === 2) parts.push('Rollers');
  else if (rollers.length) parts.push(`${rollers[0] === 'left' ? 'Left' : 'Right'} roller`);
  const faders = ctls.filter((c) => c.kind === 'fader').length;
  if (faders === 5) parts.push('Faders');
  else if (faders) parts.push(`Fader${faders > 1 ? 's' : ''} ${ctls.filter((c) => c.kind === 'fader').map((c) => (c as { index: number }).index + 1).join(', ')}`);
  const out = parts.length > 2 ? `${parts.slice(0, -1).join(', ')} and ${parts.at(-1)!.toLowerCase()}` : parts.join(' and ').replace(/ and ([A-Z])/, (_, c) => ` and ${c.toLowerCase()}`);
  return out;
}

/** Is a group's subject plural ("Keys 2, 4", "Knobs and right roller")? */
export const isPlural = (ctls: ControlRef[]) => ctls.length > 1;

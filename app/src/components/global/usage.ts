// Where an audio group is used, for the "Used by" list (S27).

import type { Action, Config, Profile } from '../../lib/types';

export interface Usage {
  /** Profile name, or "Pinned keys". */
  where: string;
  /** "Knob 3", "Dial", "Key 14 · Views" … */
  control: string;
}

const targets = (a: Action | undefined, group: string) => !!a && (('volume' in a && typeof a.volume !== 'string' && a.volume.group === group) || ('mute' in a && typeof a.mute !== 'string' && a.mute.group === group));

function inProfile(p: Profile, group: string): Usage[] {
  const out: Usage[] = [];
  const add = (control: string, ...actions: (Action | undefined)[]) => {
    if (actions.some((a) => targets(a, group))) out.push({ where: p.name, control });
  };
  for (const pg of [...(p.page ?? []), ...(p.folder ?? [])]) for (const k of pg.key ?? []) add(`Key ${k.index + 1} · ${pg.name}`, k.press, k.release);
  for (const k of p.knob ?? []) add(`Knob ${k.index + 1}`, k.cw, k.ccw, k.press, k.release);
  if (p.dial) add('Dial', p.dial.cw, p.dial.ccw, p.dial.press, p.dial.release, p.dial.long_press);
  for (const r of p.roller ?? []) add(`${r.side === 'left' ? 'Left' : 'Right'} roller`, r.away, r.toward);
  for (const f of p.fader ?? []) add(`Fader ${f.index + 1}`, f.change);
  return out;
}

/** Every control whose volume or mute action targets the group, across all profiles and pinned keys. */
export function usedBy(c: Config, group: string): Usage[] {
  const pins = (c.pinned ?? []).filter((k) => targets(k.press, group) || targets(k.release, group)).map((k) => ({ where: 'Pinned keys', control: `Key ${k.index + 1}` }));
  return [...(c.profile ?? []).flatMap((p) => inProfile(p, group)), ...pins];
}

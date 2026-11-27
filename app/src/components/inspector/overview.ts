// What the profile overview (nothing selected) shows about a profile: how it applies, what it
// inherits from its base, and which modules it sets.

import { ownerOf, allControls, type ControlRef, type Owner } from '../../lib/model/precedence';
import type { Config, ModuleKind } from '../../lib/types';

export interface Overview {
  /** Controls provided by the base profile, grouped for chips ("Knob 1–3", "Right roller"). */
  inherited: string[];
  /** One status per module kind: "set here", "inherited", "3 pages, 1 folder"... */
  modules: Partial<Record<ModuleKind, string>>;
}

const range = (nums: number[]) => {
  const out: string[] = [];
  for (let i = 0; i < nums.length; ) {
    let j = i;
    while (j + 1 < nums.length && nums[j + 1] === nums[j] + 1) j++;
    out.push(j > i + 1 ? `${nums[i]}–${nums[j]}` : nums.slice(i, j + 1).join(', '));
    i = j + 1;
  }
  return out.join(', ');
};

export function overview(c: Config, profile: string, page: string | undefined, setup: string | undefined): Overview {
  const p = (c.profile ?? []).find((x) => x.name === profile);
  const own = new Map<string, Owner>();
  const key = (x: ControlRef) => JSON.stringify(x);
  for (const ctl of allControls()) own.set(key(ctl), ownerOf(c, { profile, page, setup }, ctl));
  const kindOwner = (ctl: ControlRef) => own.get(key(ctl))!;
  const isBase = (ctl: ControlRef) => kindOwner(ctl).kind === 'base';
  const isOwn = (ctl: ControlRef) => kindOwner(ctl).kind === 'profile';

  const inherited: string[] = [];
  const keys = Array.from({ length: 15 }, (_, i) => i).filter((i) => isBase({ kind: 'key', index: i }));
  if (keys.length) inherited.push(`Key${keys.length > 1 ? 's' : ''} ${range(keys.map((i) => i + 1))}`);
  if (isBase({ kind: 'dial' })) inherited.push('Dial');
  const knobs = [0, 1, 2].filter((i) => isBase({ kind: 'knob', index: i }));
  if (knobs.length) inherited.push(`Knob${knobs.length > 1 ? 's' : ''} ${range(knobs.map((i) => i + 1))}`);
  const rollers = (['left', 'right'] as const).filter((side) => isBase({ kind: 'roller', side }));
  if (rollers.length === 2) inherited.push('Rollers');
  else if (rollers.length) inherited.push(`${rollers[0] === 'left' ? 'Left' : 'Right'} roller`);
  const faders = [0, 1, 2, 3, 4].filter((i) => isBase({ kind: 'fader', index: i }));
  if (faders.length) inherited.push(`Fader${faders.length > 1 ? 's' : ''} ${range(faders.map((i) => i + 1))}`);

  const count = (list: ControlRef[]) => {
    const n = list.filter(isOwn).length;
    const m = list.filter(isBase).length;
    if (n === list.length) return 'set here';
    if (m === list.length) return 'inherited';
    if (!n && !m) return 'not set';
    return [n && `${n} set here`, m && `${m} inherited`].filter(Boolean).join(', ');
  };
  const word = (ctl: ControlRef) => (isOwn(ctl) ? 'set' : isBase(ctl) ? 'inherited' : 'not set');
  const l = word({ kind: 'roller', side: 'left' });
  const r = word({ kind: 'roller', side: 'right' });

  const pages = p?.page?.length ?? 0;
  const folders = p?.folder?.length ?? 0;
  const keypad = `${pages} page${pages === 1 ? '' : 's'}${folders ? `, ${folders} folder${folders === 1 ? '' : 's'}` : ''}`;

  return {
    inherited,
    modules: {
      keypad: keypad,
      dial: word({ kind: 'dial' }) === 'set' ? 'set here' : word({ kind: 'dial' }),
      knob: count([0, 1, 2].map((index) => ({ kind: 'knob', index }) as ControlRef)),
      roller: l === r ? (l === 'set' ? 'set here' : l) : `left ${l}, right ${r}`,
      fader: count([0, 1, 2, 3, 4].map((index) => ({ kind: 'fader', index }) as ControlRef)),
    },
  };
}

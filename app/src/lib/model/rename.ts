// Renaming keeps every reference intact: actions, setups, bases and pinned-key exclusions.
// Call these inside `edit()`.

import type { Action, Config, Profile } from '../types';

/** Every action slot in a profile (undefined slots included, so callers can count). */
export function profileActions(p: Profile): (Action | undefined)[] {
  const out: (Action | undefined)[] = [];
  for (const pg of [...(p.page ?? []), ...(p.folder ?? [])]) for (const k of pg.key ?? []) out.push(k.press, k.release);
  for (const k of p.knob ?? []) out.push(k.cw, k.ccw, k.press, k.release);
  if (p.dial) out.push(p.dial.cw, p.dial.ccw, p.dial.press, p.dial.release, p.dial.long_press);
  for (const r of p.roller ?? []) out.push(r.away, r.toward);
  for (const f of p.fader ?? []) out.push(f.change);
  return out;
}

/** Every action in the config: profiles and pinned keys. */
export function allActions(c: Config): Action[] {
  const pinned = (c.pinned ?? []).flatMap((k) => [k.press, k.release]);
  return [...(c.profile ?? []).flatMap(profileActions), ...pinned].filter((a): a is Action => !!a);
}

/** Change every action that matches `pred` with `set`. */
export function retarget(c: Config, pred: (a: Action) => boolean, set: (a: Action) => void) {
  for (const a of allActions(c)) if (pred(a)) set(a);
}

const swap = (list: string[] | undefined, old: string, name: string) => list?.map((x) => (x === old ? name : x));

/** Rename a profile and every reference to it. */
export function renameProfile(c: Config, old: string, name: string) {
  for (const p of c.profile ?? []) {
    if (p.name === old) p.name = name;
    if (p.base === old) p.base = name;
  }
  retarget(c, (a) => 'profile' in a && a.profile === old, (a) => 'profile' in a && (a.profile = name));
  for (const s of c.setup ?? []) {
    s.profiles = swap(s.profiles, old, name);
    if (s.fallback === old) s.fallback = name;
  }
  for (const k of c.pinned ?? []) k.except = swap(k.except, old, name);
}

/** Rename a page or folder of one profile and the actions that open it (in that profile). */
export function renamePage(c: Config, profile: string, old: string, name: string) {
  const p = (c.profile ?? []).find((x) => x.name === profile);
  if (!p) return;
  for (const pg of [...(p.page ?? []), ...(p.folder ?? [])]) if (pg.name === old) pg.name = name;
  for (const a of profileActions(p)) {
    if (a && 'page' in a && a.page === old) a.page = name;
    if (a && 'folder' in a && a.folder === old) a.folder = name;
  }
}

/** Rename a setup and every reference to it. */
export function renameSetup(c: Config, old: string, name: string) {
  for (const s of c.setup ?? []) if (s.name === old) s.name = name;
  retarget(c, (a) => 'setup' in a && a.setup === old, (a) => 'setup' in a && (a.setup = name));
  for (const k of c.pinned ?? []) k.except_setups = swap(k.except_setups, old, name);
}

/** Where a profile is referenced, for delete dialogs ("It is used in Work and Play"). */
export function profileReferences(c: Config, name: string): { setups: string[]; bases: string[]; actions: number } {
  return {
    setups: (c.setup ?? []).filter((s) => s.profiles?.includes(name) || s.fallback === name).map((s) => s.name),
    bases: (c.profile ?? []).filter((p) => p.base === name).map((p) => p.name),
    actions: allActions(c).filter((a) => 'profile' in a && a.profile === name).length,
  };
}

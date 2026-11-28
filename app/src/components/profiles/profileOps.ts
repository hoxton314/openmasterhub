// Config operations of the profile lifecycle. Call them inside `edit()`.

import { materialiseSetups, profileReferences, setupsOf } from '../../lib/model';
import type { Config, Profile, Setup } from '../../lib/types';
import { uniqueName } from '../../lib/templates';

const copy = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;

function setupNamed(c: Config, name: string): Setup | undefined {
  return materialiseSetups(c).find((s) => s.name === name);
}

/** Put `profile` into a setup's list at `position` (0-based, clamped; default last). */
export function addToSetup(c: Config, profile: string, setup: string, position?: number) {
  const s = setupNamed(c, setup);
  if (!s) return;
  s.profiles ??= [];
  if (s.profiles.includes(profile)) return;
  const at = position === undefined ? s.profiles.length : Math.max(0, Math.min(position, s.profiles.length));
  s.profiles.splice(at, 0, profile);
}

/** Add a new profile to the config and to the rule list of every setup in `setups`, at `position`. */
export function insertProfile(c: Config, profile: Profile, setups: string[], position?: number) {
  c.profile ??= [];
  materialiseSetups(c);
  c.profile.push(profile);
  for (const s of setups) addToSetup(c, profile.name, s, position);
}

/** Remove a profile from the config and every setup; profiles based on it lose their base. */
export function deleteProfile(c: Config, name: string) {
  materialiseSetups(c);
  c.profile = (c.profile ?? []).filter((p) => p.name !== name);
  for (const p of c.profile) if (p.base === name) delete p.base;
  for (const s of c.setup ?? []) {
    s.profiles = (s.profiles ?? []).filter((n) => n !== name);
    if (s.fallback === name) delete s.fallback;
    if (s.pin === name) delete s.pin;
  }
  for (const k of c.pinned ?? []) if (k.except) k.except = k.except.filter((n) => n !== name);
}

/** Give `setup` its own duplicate of a linked profile, named `name`. Returns the duplicate. */
export function separateCopy(c: Config, profile: string, setup: string, name: string): Profile | undefined {
  const src = (c.profile ?? []).find((p) => p.name === profile);
  const s = setupNamed(c, setup);
  if (!src || !s) return undefined;
  const dup = copy(src);
  dup.name = uniqueName((c.profile ?? []).map((p) => p.name), name.trim() || `${profile} copy`);
  const at = (c.profile ?? []).indexOf(src);
  c.profile!.splice(at + 1, 0, dup);
  s.profiles = (s.profiles ?? []).map((n) => (n === profile ? dup.name : n));
  if (s.fallback === profile) s.fallback = dup.name;
  if (s.pin === profile) s.pin = dup.name;
  return dup;
}

/** Where a profile is used: setup name plus "rule 3" / "fallback" / "pinned". */
export function usage(c: Config, profile: string): { setup: string; where: string }[] {
  return setupsOf(c, profile).map((s) => {
    const i = s.profiles.indexOf(profile);
    return { setup: s.name, where: i >= 0 ? (s.mode === 'manual' ? `position ${i + 1}` : `rule ${i + 1}`) : 'fallback' };
  });
}

/** "Work", "Work and Play", "Work, Play and Stream". */
export function joinNames(names: string[]): string {
  if (names.length < 2) return names[0] ?? '';
  return `${names.slice(0, -1).join(', ')} and ${names.at(-1)}`;
}

export { profileReferences };

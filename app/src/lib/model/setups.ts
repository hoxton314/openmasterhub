// Setups as the app sees them. A config without `[[setup]]` has one implicit "Default" follow
// setup: every profile but the first in order, the first as fallback; exactly what
// `Config::profile_for_app` does today. Mirrors `Config::effective_setups()` in masterhub-core.

import type { Config, HubState, Profile, Setup, SetupMode } from '../types';
import { firstMatch } from './glob';

/** A setup with defaults filled in. `implicit` = derived from a legacy config, not saved yet. */
export interface EffectiveSetup extends Setup {
  mode: SetupMode;
  profiles: string[];
  implicit: boolean;
}

export const IMPLICIT_SETUP = 'Default';

export function effectiveSetups(c: Config): EffectiveSetup[] {
  if (c.setup?.length) return c.setup.map((s) => ({ ...s, mode: s.mode ?? 'follow', profiles: s.profiles ?? [], implicit: false }));
  const names = (c.profile ?? []).map((p) => p.name);
  return [{ name: IMPLICIT_SETUP, mode: 'follow', profiles: names.slice(1), fallback: names[0], implicit: true }];
}

/** True when the config has no `[[setup]]` yet. */
export const isImplicit = (c: Config) => !c.setup?.length;

/**
 * Write the implicit setup into the config (call inside `edit()` before changing setup
 * properties): "materialise on first edit".
 */
export function materialiseSetups(c: Config): Setup[] {
  if (!c.setup?.length) {
    const [s] = effectiveSetups(c);
    c.setup = [{ name: s.name, mode: 'follow', profiles: [...s.profiles], ...(s.fallback ? { fallback: s.fallback } : {}) }];
  }
  return c.setup;
}

export function setupByName(c: Config, name: string | undefined): EffectiveSetup | undefined {
  return effectiveSetups(c).find((s) => s.name === name);
}

/** The setup the service reports as active, else the first. */
export function activeSetup(c: Config, state?: Pick<HubState, 'setup'>): EffectiveSetup {
  const all = effectiveSetups(c);
  return all.find((s) => s.name === state?.setup) ?? all[0];
}

export const profileByName = (c: Config, name: string | undefined): Profile | undefined => (c.profile ?? []).find((p) => p.name === name);

/** The setup's profiles in priority / cycle order (missing names skipped; not the fallback). */
export function profilesOf(c: Config, s: Setup): Profile[] {
  return (s.profiles ?? []).map((n) => profileByName(c, n)).filter((p): p is Profile => !!p);
}

export function fallbackOf(c: Config, s: Setup): Profile | undefined {
  return s.mode === 'manual' ? undefined : profileByName(c, s.fallback);
}

/** Setups that use a profile (in their list or as fallback). */
export function setupsOf(c: Config, profile: string): EffectiveSetup[] {
  return effectiveSetups(c).filter((s) => s.profiles.includes(profile) || s.fallback === profile);
}

/** A profile used by two or more setups is one linked object. */
export const isLinked = (c: Config, profile: string) => setupsOf(c, profile).length > 1;

/** Profiles no setup uses ("My profiles" in the gallery). */
export function looseProfiles(c: Config): Profile[] {
  const used = new Set(effectiveSetups(c).flatMap((s) => [...s.profiles, ...(s.fallback ? [s.fallback] : [])]));
  return (c.profile ?? []).filter((p) => !used.has(p.name));
}

export interface RuleHit {
  profile: Profile;
  /** Position in the setup's list (0-based). */
  index: number;
  pattern: string;
}

export interface AppMatch {
  /** The winning profile (undefined when nothing matches and there is no fallback). */
  profile?: Profile;
  /** `rule`: `hit` won; `fallback`: no rule matched. */
  kind: 'rule' | 'fallback' | 'none';
  hit?: RuleHit;
  /** Later rules that also match but lose to the winner. */
  shadowed: RuleHit[];
}

/** Which profile a follow/hybrid setup picks for a focused window class (first match wins). */
export function matchApp(c: Config, s: Setup, appClass: string): AppMatch {
  const hits: RuleHit[] = [];
  (s.profiles ?? []).forEach((name, index) => {
    const profile = profileByName(c, name);
    const m = profile && firstMatch(profile.apps, appClass);
    if (profile && m) hits.push({ profile, index, pattern: m.pattern });
  });
  if (hits.length) return { profile: hits[0].profile, kind: 'rule', hit: hits[0], shadowed: hits.slice(1) };
  const fb = fallbackOf(c, s);
  return { profile: fb, kind: fb ? 'fallback' : 'none', shadowed: [] };
}

/** Avatar colour index 1–7 for a profile: its `color` wins (returned as-is), else a name hash. */
export function avatarColor(p: Pick<Profile, 'name' | 'color'>): string {
  if (p.color) return p.color;
  let h = 0;
  for (const ch of p.name) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return `var(--avatar-${(h % 6) + 1})`;
}

/** The base chain of a profile, nearest first, stopping at cycles and missing names. */
export function baseChain(c: Config, profile: string): Profile[] {
  const out: Profile[] = [];
  let cur = profileByName(c, profile)?.base;
  while (cur && !out.some((p) => p.name === cur) && cur !== profile) {
    const p = profileByName(c, cur);
    if (!p) break;
    out.push(p);
    cur = p.base;
  }
  return out;
}

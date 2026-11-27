// App-rule conflicts in a setup: a later rule whose pattern can never win because an earlier
// rule matches every window it matches (first match wins, see `matchApp`). Patterns that merely
// share some windows are not conflicts: one of them has to win those, and the rule tester names
// the loser. Pure functions on the config; the Setups area shows a conflict under the row that
// cannot win, with the fixes from here.

import type { Config } from '../types';
import { globMatch } from './glob';
import { effectiveSetups, profileByName } from './setups';

/** One app-rule pattern of a profile at its position in the setup's list (0-based). */
export interface RuleRef {
  index: number;
  profile: string;
  pattern: string;
}

/**
 * `duplicate`: both patterns match exactly the same windows; `covered`: the earlier pattern
 * matches everything the later one does (and more).
 */
export type ConflictKind = 'duplicate' | 'covered';

export type ConflictFix =
  /** Put the losing profile before the winning one in the setup's list. */
  | { kind: 'move'; profile: string; before: string }
  /** Replace the winner's broad pattern with these exact window classes. */
  | { kind: 'narrow'; profile: string; from: string; to: string[] }
  /** Remove the unused pattern from the losing profile. */
  | { kind: 'drop'; profile: string; pattern: string };

export interface RuleConflict {
  /** The rule that can never win. */
  loser: RuleRef;
  /** The earlier rule that matches first. */
  winner: RuleRef;
  kind: ConflictKind;
  /** A window class the loser matches: a known one when given, else a made-up one. */
  witness: string;
  fixes: ConflictFix[];
}

/**
 * A string both patterns match (`*` wildcards, case-insensitive), or undefined when no window
 * can match both. Shortest-path search over the pairs of pattern positions.
 */
export function globIntersection(a: string, b: string): string | undefined {
  const al = a.toLowerCase();
  const bl = b.toLowerCase();
  const w = b.length + 1;
  const prev = new Map<number, { from: number; ch: string }>();
  const start = 0;
  prev.set(start, { from: -1, ch: '' });
  const queue: number[] = [start];
  for (let head = 0; head < queue.length; head++) {
    const s = queue[head];
    const i = Math.floor(s / w);
    const j = s % w;
    if (i === a.length && j === b.length) {
      let out = '';
      for (let cur = s; cur !== start; ) {
        const p = prev.get(cur)!;
        out = p.ch + out;
        cur = p.from;
      }
      return out;
    }
    const moves: [number, number, string][] = [];
    const starA = al[i] === '*';
    const starB = bl[j] === '*';
    if (starA) moves.push([i + 1, j, '']);
    if (starB) moves.push([i, j + 1, '']);
    if (starA && !starB && j < b.length) moves.push([i, j + 1, b[j]]);
    if (starB && !starA && i < a.length) moves.push([i + 1, j, a[i]]);
    if (!starA && !starB && i < a.length && j < b.length && al[i] === bl[j]) moves.push([i + 1, j + 1, a[i]]);
    for (const [ni, nj, ch] of moves) {
      const n = ni * w + nj;
      if (prev.has(n)) continue;
      prev.set(n, { from: s, ch });
      queue.push(n);
    }
  }
  return undefined;
}

/** Every window `b` matches is also matched by `a`. */
export function globCovers(a: string, b: string): boolean {
  // `*` of b stands for any text: a fresh character no literal of `a` can match, absorbed by a star of `a`.
  return globMatch(a, b.replaceAll('*', '\u0000'));
}

/** A window class `pattern` matches (its wildcards matched by nothing). */
export const sampleOf = (pattern: string) => pattern.replaceAll('*', '');

const hasWildcard = (p: string) => p.includes('*');

/**
 * The conflicts of one setup, in rule order. Manual setups have none (no app rules); profiles
 * missing from the config are skipped. `known` = window classes seen on this PC (open now,
 * recent): used to pick a readable witness and to suggest narrower patterns.
 */
export function ruleConflicts(c: Config, setupName: string, known: string[] = []): RuleConflict[] {
  const setup = effectiveSetups(c).find((s) => s.name === setupName);
  if (!setup || setup.mode === 'manual') return [];
  const rules: RuleRef[] = [];
  setup.profiles.forEach((name, index) => {
    for (const pattern of profileByName(c, name)?.apps ?? []) rules.push({ index, profile: name, pattern });
  });

  const out: RuleConflict[] = [];
  rules.forEach((loser, n) => {
    for (const winner of rules.slice(0, n)) {
      if (winner.profile === loser.profile) continue;
      if (!globCovers(winner.pattern, loser.pattern)) continue;
      const kind: ConflictKind = globCovers(loser.pattern, winner.pattern) ? 'duplicate' : 'covered';
      const witness = known.find((k) => globMatch(loser.pattern, k)) ?? sampleOf(loser.pattern);
      const fixes: ConflictFix[] = [{ kind: 'move', profile: loser.profile, before: winner.profile }];
      const narrower = hasWildcard(winner.pattern)
        ? [...new Set(known)].filter((k) => !hasWildcard(k) && globMatch(winner.pattern, k) && !globMatch(loser.pattern, k))
        : [];
      if (narrower.length) fixes.push({ kind: 'narrow', profile: winner.profile, from: winner.pattern, to: narrower });
      else fixes.push({ kind: 'drop', profile: loser.profile, pattern: loser.pattern });
      out.push({ loser, winner, kind, witness, fixes });
      break; // the earliest rule that matches first is the one to name
    }
  });
  return out;
}

/**
 * "Claude’s claude-* can never win for claude-ghostty: Ghostty’s ghostty* matches first." as
 * three parts, the middle one (a window class, set in mono) empty for an exact pattern.
 */
export function conflictText(x: RuleConflict): { head: string; witness: string; tail: string } {
  const named = hasWildcard(x.loser.pattern) && x.witness !== '';
  return {
    head: `${x.loser.profile}’s ${x.loser.pattern} can never win${named ? ' for' : ':'}`,
    witness: named ? x.witness : '',
    tail: `${named ? ': ' : ''}${x.winner.profile}’s ${x.winner.pattern} matches first.`,
  };
}

/** The profile a fix names as "above": `Move Claude above Ghostty`. */
export function fixLabel(f: ConflictFix): string {
  if (f.kind === 'move') return `Move ${f.profile} above ${f.before}`;
  if (f.kind === 'narrow') return `Narrow ${f.profile}’s rule`;
  return `Remove ${f.pattern} from ${f.profile}`;
}

/**
 * Apply a fix to `c`. Call inside `edit()` (after `materialiseSetups` for a legacy config).
 * Returns false when the fix no longer applies.
 */
export function applyFix(c: Config, setupName: string, f: ConflictFix): boolean {
  if (f.kind === 'move') {
    const list = c.setup?.find((s) => s.name === setupName)?.profiles;
    if (!list) return false;
    const from = list.indexOf(f.profile);
    const to = list.indexOf(f.before);
    if (from < 0 || to < 0 || from < to) return false;
    list.splice(from, 1);
    list.splice(to, 0, f.profile);
    return true;
  }
  const p = profileByName(c, f.profile);
  if (!p?.apps) return false;
  if (f.kind === 'drop') {
    const next = p.apps.filter((a) => a !== f.pattern);
    if (next.length === p.apps.length) return false;
    p.apps = next.length ? next : undefined;
    return true;
  }
  const at = p.apps.indexOf(f.from);
  if (at < 0) return false;
  p.apps.splice(at, 1, ...f.to.filter((t) => !p.apps!.includes(t)));
  return true;
}

import { describe, expect, it } from 'vitest';
import sample from '../mock/snapshot.json';
import type { Config } from '../types';
import { applyFix, conflictText, globCovers, globIntersection, ruleConflicts } from './conflicts';
import { fixture } from './fixtures';
import { globMatch } from './glob';
import { effectiveSetups, matchApp } from './setups';

const clone = (c: Config) => structuredClone(c);

/** `Config::profile_for_app` of masterhub-core, ported: first match after index 0, else 0. */
function profileForApp(c: Config, cls: string): string {
  const ps = c.profile ?? [];
  return (ps.slice(1).find((p) => (p.apps ?? []).some((a) => globMatch(a, cls))) ?? ps[0]).name;
}

describe('globIntersection and globCovers', () => {
  it('finds a window both patterns match', () => {
    expect(globIntersection('claude-*', '*ghostty')).toBe('claude-ghostty');
    expect(globIntersection('blender', 'Blender')).toBe('blender');
    expect(globIntersection('godot*.exe', '*4*')).toMatch(/^godot.*4.*\.exe$/);
  });
  it('finds none for disjoint patterns', () => {
    expect(globIntersection('ghostty*', 'claude-*')).toBeUndefined();
    expect(globIntersection('obs', 'obsidian')).toBeUndefined();
    expect(globIntersection('*.exe', '*.sh')).toBeUndefined();
  });
  it('every witness matches both patterns', () => {
    const pats = ['*', 'a*', '*a', '*a*', 'a*b', 'ab*', 'steam_app_*', '*ghostty', 'x', 'Godot*.exe', 'g*d*t*'];
    for (const a of pats)
      for (const b of pats) {
        const w = globIntersection(a, b);
        if (w !== undefined) expect([a, b, globMatch(a, w), globMatch(b, w)]).toEqual([a, b, true, true]);
      }
  });
  it('covers means a superset', () => {
    expect(globCovers('ghostty*', 'ghostty-claude')).toBe(true);
    expect(globCovers('*', 'anything*')).toBe(true);
    expect(globCovers('a*', '*a')).toBe(false);
    expect(globCovers('*a*', 'a*')).toBe(true);
    expect(globCovers('ghostty-claude', 'ghostty*')).toBe(false);
    expect(globCovers('Obs', 'obs')).toBe(true);
  });
});

describe('ruleConflicts', () => {
  it('detects the S15 conflict: Claude can never win for the Ghostty class', () => {
    const [x, ...rest] = ruleConflicts(fixture, 'Work');
    expect(rest).toHaveLength(0);
    expect(x).toMatchObject({
      kind: 'duplicate',
      witness: 'com.mitchellh.ghostty',
      loser: { profile: 'Claude', pattern: 'com.mitchellh.ghostty', index: 3 },
      winner: { profile: 'Ghostty', pattern: 'com.mitchellh.ghostty', index: 2 },
    });
  });

  it('fix 1: moving Claude above Ghostty puts Claude first', () => {
    const c = clone(fixture);
    const x = ruleConflicts(c, 'Work')[0];
    expect(x.fixes[0]).toEqual({ kind: 'move', profile: 'Claude', before: 'Ghostty' });
    expect(conflictText(x)).toEqual({ head: 'Claude’s com.mitchellh.ghostty can never win:', witness: '', tail: 'Ghostty’s com.mitchellh.ghostty matches first.' });
    expect(applyFix(c, 'Work', x.fixes[0])).toBe(true);
    expect(c.setup![0].profiles).toEqual(['Blender', 'Godot', 'Claude', 'Ghostty']);
    expect(matchApp(c, effectiveSetups(c)[0], 'com.mitchellh.ghostty').profile?.name).toBe('Claude');
    // a duplicate cannot be resolved by order alone: the other profile now loses the class
    expect(ruleConflicts(c, 'Work')[0].loser.profile).toBe('Ghostty');
  });

  it('fix 2: dropping the unused pattern clears the conflict and keeps the rest', () => {
    const c = clone(fixture);
    const x = ruleConflicts(c, 'Work')[0];
    expect(x.fixes[1]).toEqual({ kind: 'drop', profile: 'Claude', pattern: 'com.mitchellh.ghostty' });
    expect(applyFix(c, 'Work', x.fixes[1])).toBe(true);
    expect(c.profile!.find((p) => p.name === 'Claude')!.apps).toEqual(['claude-*']);
    expect(ruleConflicts(c, 'Work')).toEqual([]);
    expect(applyFix(c, 'Work', x.fixes[1])).toBe(false);
  });

  it('a broad earlier rule covers a narrower later one; narrowing uses known windows', () => {
    const c = clone(fixture);
    c.profile!.find((p) => p.name === 'Ghostty')!.apps = ['*ghostty*'];
    c.profile!.find((p) => p.name === 'Claude')!.apps = ['claude-ghostty'];
    const known = ['com.mitchellh.ghostty', 'claude-ghostty', 'firefox'];
    const [x] = ruleConflicts(c, 'Work', known);
    expect(x).toMatchObject({ kind: 'covered', witness: 'claude-ghostty' });
    const narrow = x.fixes.find((f) => f.kind === 'narrow');
    expect(narrow).toEqual({ kind: 'narrow', profile: 'Ghostty', from: '*ghostty*', to: ['com.mitchellh.ghostty'] });
    expect(applyFix(c, 'Work', narrow!)).toBe(true);
    expect(c.profile!.find((p) => p.name === 'Ghostty')!.apps).toEqual(['com.mitchellh.ghostty']);
    expect(ruleConflicts(c, 'Work', known)).toEqual([]);
  });

  it('covered: moving the narrower rule up resolves it for good', () => {
    const c = clone(fixture);
    c.profile!.find((p) => p.name === 'Ghostty')!.apps = ['ghostty*'];
    c.profile!.find((p) => p.name === 'Claude')!.apps = ['ghostty-claude', 'claude-*'];
    const [x, ...rest] = ruleConflicts(c, 'Work');
    expect(rest).toHaveLength(0);
    expect(conflictText(x)).toEqual({ head: 'Claude’s ghostty-claude can never win:', witness: '', tail: 'Ghostty’s ghostty* matches first.' });
    applyFix(c, 'Work', x.fixes[0]);
    expect(matchApp(c, effectiveSetups(c)[0], 'ghostty-claude').profile?.name).toBe('Claude');
    expect(matchApp(c, effectiveSetups(c)[0], 'ghostty').profile?.name).toBe('Ghostty');
    expect(ruleConflicts(c, 'Work')).toEqual([]);
  });

  it('names a window for a wildcard loser', () => {
    const c = clone(fixture);
    c.profile!.find((p) => p.name === 'Ghostty')!.apps = ['*'];
    const x = ruleConflicts(c, 'Work', ['claude-ghostty']).find((y) => y.loser.pattern === 'claude-*')!;
    expect(conflictText(x)).toEqual({ head: 'Claude’s claude-* can never win for', witness: 'claude-ghostty', tail: ': Ghostty’s * matches first.' });
  });

  it('is quiet for partial overlaps (the tester names the loser) and for a broader later rule', () => {
    const c = clone(fixture);
    c.profile!.find((p) => p.name === 'Ghostty')!.apps = ['*ghostty'];
    c.profile!.find((p) => p.name === 'Claude')!.apps = ['claude-*'];
    expect(ruleConflicts(c, 'Work')).toEqual([]);
    expect(matchApp(c, effectiveSetups(c)[0], 'claude-ghostty').shadowed.map((h) => h.profile.name)).toEqual(['Claude']);
    c.profile!.find((p) => p.name === 'Ghostty')!.apps = ['ghostty-claude'];
    c.profile!.find((p) => p.name === 'Claude')!.apps = ['ghostty*'];
    expect(ruleConflicts(c, 'Work')).toEqual([]);
    expect(ruleConflicts(fixture, 'Stream')).toEqual([]); // manual
    expect(ruleConflicts(fixture, 'Play')).toEqual([]);
    expect(ruleConflicts(fixture, 'Nope')).toEqual([]);
  });

  it('works on a legacy config through the implicit setup', () => {
    const c: Config = { backlight: 80, profile: [{ name: 'Default' }, { name: 'A', apps: ['*term*'] }, { name: 'B', apps: ['xterm'] }] };
    const [x] = ruleConflicts(c, 'Default');
    expect(x).toMatchObject({ kind: 'covered', loser: { profile: 'B' }, winner: { profile: 'A' } });
    expect(conflictText(x).head).toBe('B’s xterm can never win:');
  });
});

describe('the tester picks like Config::profile_for_app', () => {
  const classes = ['blender', 'Blender', 'godot', 'Godot_v4.exe', 'discord', 'Discord.exe', 'com.mitchellh.ghostty', 'claude-ghostty', 'claude-code', 'obsidian', 'obs', 'org.gnome.Nautilus', 'firefox', 'steam_app_1091500', ''];
  it('matches on a legacy config', () => {
    const legacy: Config = { backlight: 80, profile: (sample.config.profile as Config['profile'])!.map((p) => ({ name: p.name, apps: p.apps })) };
    const s = effectiveSetups(legacy)[0];
    expect(s.implicit).toBe(true);
    for (const cls of [...classes, 'steam_app_1422450', 'ArcRaiders-Win64', 'com.obsproject.Studio'])
      expect([cls, matchApp(legacy, s, cls).profile?.name]).toEqual([cls, profileForApp(legacy, cls)]);
  });
  it('matches the Work setup of the fixture, whose list order is the rule order', () => {
    const work = effectiveSetups(fixture)[0];
    expect(matchApp(fixture, work, 'blender').profile?.name).toBe('Blender');
    expect(matchApp(fixture, work, 'claude-code').profile?.name).toBe('Claude');
    expect(matchApp(fixture, work, 'firefox')).toMatchObject({ kind: 'fallback', profile: { name: 'Desktop' } });
  });
});

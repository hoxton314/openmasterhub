import { describe, expect, it } from 'vitest';
import type { ClaimInfo, Config } from '../types';
import { fixture } from './fixtures';
import { globMatch } from './glob';
import { ownerOf, whyChain, type ControlRef } from './precedence';
import { profileReferences, renameProfile, renameSetup } from './rename';
import { activeSetup, baseChain, effectiveSetups, isLinked, looseProfiles, materialiseSetups, matchApp, profilesOf } from './setups';

describe('globMatch (same cases as the Rust tests)', () => {
  it('matches like glob_match', () => {
    expect(globMatch('obs', 'OBS')).toBe(true);
    expect(globMatch('steam_app_*', 'steam_app_1234')).toBe(true);
    expect(globMatch('*firefox*', 'org.mozilla.firefox')).toBe(true);
    expect(globMatch('obs', 'obsidian')).toBe(false);
  });
  it('handles middle and trailing wildcards', () => {
    expect(globMatch('Godot*.exe', 'godot_v4.3.exe')).toBe(true);
    expect(globMatch('Godot*.exe', 'godot')).toBe(false);
    expect(globMatch('a*b*c', 'axxbyyc')).toBe(true);
    expect(globMatch('a*b*c', 'axxcyyb')).toBe(false);
    expect(globMatch('*', '')).toBe(true);
  });
});

const legacy: Config = {
  backlight: 80,
  profile: [{ name: 'Default' }, { name: 'Streaming', apps: ['obs', 'com.obsproject.Studio'] }, { name: 'Shell', apps: ['*ghostty'] }],
};

describe('effective setups', () => {
  it('derives one Default follow setup from a legacy config', () => {
    const [s, ...rest] = effectiveSetups(legacy);
    expect(rest).toHaveLength(0);
    expect(s).toMatchObject({ name: 'Default', mode: 'follow', profiles: ['Streaming', 'Shell'], fallback: 'Default', implicit: true });
  });
  it('picks like profile_for_app on a legacy config', () => {
    const s = activeSetup(legacy);
    expect(matchApp(legacy, s, 'obs').profile?.name).toBe('Streaming');
    expect(matchApp(legacy, s, 'kitty')).toMatchObject({ kind: 'fallback', profile: { name: 'Default' } });
    expect(matchApp(legacy, s, 'com.mitchellh.ghostty').hit).toMatchObject({ index: 1, pattern: '*ghostty' });
  });
  it('materialises the implicit setup on first edit only', () => {
    const c = structuredClone(legacy);
    materialiseSetups(c);
    expect(c.setup).toEqual([{ name: 'Default', mode: 'follow', profiles: ['Streaming', 'Shell'], fallback: 'Default' }]);
    c.setup![0].name = 'Work';
    materialiseSetups(c);
    expect(c.setup![0].name).toBe('Work');
  });
  it('reports shadowed rules, links and loose profiles', () => {
    const work = effectiveSetups(fixture)[0];
    const m = matchApp(fixture, work, 'com.mitchellh.ghostty');
    expect(m.profile?.name).toBe('Ghostty');
    expect(m.shadowed.map((h) => h.profile.name)).toEqual(['Claude']);
    expect(profilesOf(fixture, work).map((p) => p.name)).toEqual(['Blender', 'Godot', 'Ghostty', 'Claude']);
    expect(isLinked(fixture, 'Desktop')).toBe(true);
    expect(isLinked(fixture, 'Blender')).toBe(false);
    expect(looseProfiles(fixture).map((p) => p.name)).toEqual([]);
    expect(baseChain(fixture, 'Cyberpunk 2077').map((p) => p.name)).toEqual(['Desktop light', 'Desktop']);
  });
});

const keys = (...idx: number[]): ControlRef[] => idx.map((index) => ({ kind: 'key', index }));

describe('precedence (IA-Precedence scenarios)', () => {
  it('Work · blender focused', () => {
    const ctx = { profile: 'Blender', page: 'Views', setup: 'Work' };
    const reason = { kind: 'rule' as const, pattern: 'blender', index: 0 };
    for (const k of keys(0, 1, 7, 9, 14)) expect(ownerOf(fixture, ctx, k, reason)).toMatchObject({ level: 4, profile: 'Blender' });
    expect(ownerOf(fixture, ctx, { kind: 'key', index: 8 })).toMatchObject({ level: 2, kind: 'pinned' });
    expect(ownerOf(fixture, ctx, { kind: 'dial' }, reason)).toMatchObject({ kind: 'profile', profile: 'Blender' });
    expect(ownerOf(fixture, ctx, { kind: 'knob', index: 1 })).toMatchObject({ level: 6, profile: 'Desktop' });
    expect(ownerOf(fixture, ctx, { kind: 'roller', side: 'right' })).toMatchObject({ level: 6, profile: 'Desktop' });
  });
  it('Play · Cyberpunk 2077 focused', () => {
    const claim: ClaimInfo = { claim: 1, name: 'Cyberpunk 2077 mode', active: true, owns: { keys: [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14], dial: true } };
    const ctx = { profile: 'Cyberpunk 2077', page: 'Combat', setup: 'Play', claims: [claim] };
    expect(ownerOf(fixture, ctx, { kind: 'key', index: 3 })).toMatchObject({ level: 1, kind: 'claim' });
    expect(ownerOf(fixture, ctx, { kind: 'dial' })).toMatchObject({ level: 1 });
    expect(ownerOf(fixture, ctx, { kind: 'key', index: 8 })).toMatchObject({ kind: 'none' }); // excluded from Play
    expect(ownerOf(fixture, ctx, { kind: 'knob', index: 2 })).toMatchObject({ kind: 'profile', profile: 'Cyberpunk 2077' });
    expect(ownerOf(fixture, ctx, { kind: 'roller', side: 'left' })).toMatchObject({ level: 6, profile: 'Desktop light' });
    expect(ownerOf(fixture, ctx, { kind: 'roller', side: 'right' })).toMatchObject({ level: 6, profile: 'Desktop', via: ['Desktop light', 'Desktop'] });
  });
  it('Stream · chosen by hand', () => {
    const ctx = { profile: 'Scenes', setup: 'Stream' };
    expect(ownerOf(fixture, ctx, { kind: 'key', index: 0 }, { kind: 'manual' })).toMatchObject({ level: 3, profile: 'Scenes' });
    const why = whyChain(fixture, { profile: 'Scenes', page: 'Scenes', setup: 'Stream' });
    expect(why.steps[0]).toMatchObject({ level: 3, text: 'Chosen by hand in Stream' });
  });
  it('explains a rule match and inherited controls', () => {
    const why = whyChain(fixture, { profile: 'Blender', page: 'Views', setup: 'Work', active_app: 'blender' });
    expect(why.steps[0].text).toBe('Rule 1 of Work: blender matches blender');
    expect(why.steps.find((s) => s.level === 2)?.controls).toEqual([{ kind: 'key', index: 8 }]);
    expect(why.steps.find((s) => s.level === 6)?.text).toContain('From Desktop (base of Blender)');
  });
});

describe('renames', () => {
  it('retargets profile references everywhere', () => {
    const c = structuredClone(fixture);
    c.pinned![0].except = ['Desktop'];
    c.profile![2].page![0].key![0].press = { profile: 'Desktop' };
    renameProfile(c, 'Desktop', 'Home');
    expect(c.setup![0].fallback).toBe('Home');
    expect(c.setup![1].profiles).toEqual(['Scenes', 'Home']);
    expect(c.profile![1].base).toBe('Home');
    expect(c.pinned![0].except).toEqual(['Home']);
    expect(c.profile![2].page![0].key![0].press).toEqual({ profile: 'Home' });
    expect(profileReferences(c, 'Home').setups).toEqual(['Work', 'Stream']);
  });
  it('retargets setup references', () => {
    const c = structuredClone(fixture);
    renameSetup(c, 'Play', 'Games');
    expect(c.setup![2].name).toBe('Games');
    expect(c.pinned![0].except_setups).toEqual(['Games']);
  });
});

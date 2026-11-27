import { describe, expect, it } from 'vitest';
import type { Config } from '../../lib/types';
import { apply, workProfiles } from './starting';

describe('starting setups', () => {
  it('makes a profile per recognised open window', () => {
    const names = workProfiles(['blender', 'Discord.exe', 'com.mitchellh.ghostty', 'org.gnome.Nautilus']).map((p) => p.name);
    expect(names).toEqual(['Blender', 'Discord', 'Terminal']);
  });

  it('creates Work with a Desktop fallback outside the profile list', () => {
    const c: Config = { backlight: 80 };
    expect(apply(c, 'work', ['blender'])).toBe('Work');
    expect(c.setup).toEqual([{ name: 'Work', mode: 'follow', profiles: ['Blender'], fallback: 'Desktop' }]);
    expect(c.profile?.map((p) => p.name)).toEqual(['Blender', 'Desktop']);
  });

  it('builds Streaming and Play from the bundled setup templates', () => {
    const c: Config = { backlight: 80 };
    expect(apply(c, 'streaming', [])).toBe('Stream');
    expect(apply(c, 'play', [])).toBe('Play');
    expect(c.setup?.map((s) => [s.name, s.mode])).toEqual([['Stream', 'manual'], ['Play', 'follow']]);
    expect(c.profile?.map((p) => p.name)).toEqual(['Streaming', 'Desktop media']);
    expect(c.setup?.[1].fallback).toBe('Desktop media');
  });

  it('never duplicates names that exist', () => {
    const c: Config = { backlight: 80, profile: [{ name: 'Desktop' }], setup: [{ name: 'Play' }] };
    expect(apply(c, 'play', [])).toBe('Play 2');
    expect(new Set(c.profile?.map((p) => p.name)).size).toBe(c.profile?.length);
    expect(c.setup?.map((s) => s.name)).toEqual(['Play', 'Play 2']);
  });

  it('start empty adds one profile and no setup', () => {
    const c: Config = { backlight: 80 };
    expect(apply(c, 'empty', [])).toBeUndefined();
    expect(c.profile?.map((p) => p.name)).toEqual(['Desktop']);
    expect(c.setup).toBeUndefined();
  });
});

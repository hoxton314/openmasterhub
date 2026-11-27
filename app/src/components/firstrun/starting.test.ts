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

  it('reuses profiles and setups that exist', () => {
    const c: Config = { backlight: 80, profile: [{ name: 'Desktop' }], setup: [{ name: 'Play' }] };
    apply(c, 'play', []);
    expect(c.profile).toHaveLength(1);
    expect(c.setup).toHaveLength(1);
  });

  it('start empty adds one profile and no setup', () => {
    const c: Config = { backlight: 80 };
    expect(apply(c, 'empty', [])).toBeUndefined();
    expect(c.profile?.map((p) => p.name)).toEqual(['Desktop']);
    expect(c.setup).toBeUndefined();
  });
});

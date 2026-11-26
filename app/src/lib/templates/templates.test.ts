import { describe, expect, it } from 'vitest';
import type { Config } from '../types';
import { instantiate, listTemplates, moduleWarning, uniqueName } from './index';

const config: Config = { backlight: 80, profile: [{ name: 'Blender' }, { name: 'Desktop' }] };

describe('templates', () => {
  it('lists the bundled templates', () => {
    expect(listTemplates('profile').map((t) => t.id)).toEqual(['blender', 'streaming', 'godot', 'volume-mixer', 'desktop-media', 'claude-sessions', 'desktop-fallback']);
    expect(listTemplates('setup').map((t) => t.name).sort()).toEqual(['Blank', 'Creative', 'Games', 'Streaming']);
    expect(listTemplates('control')).toHaveLength(4);
  });

  it('carries no images (game artwork stays in the user install)', () => {
    for (const t of [...listTemplates('profile'), ...listTemplates('setup'), ...listTemplates('control')]) expect(JSON.stringify(t)).not.toMatch(/"image"/);
  });

  it('instantiates a profile with a unique name, rule and base', () => {
    const t = listTemplates('profile')[0];
    const { profiles } = instantiate(t, config, { apps: ['blender*'], base: 'Desktop' });
    expect(profiles[0]).toMatchObject({ name: 'Blender 2', apps: ['blender*'], base: 'Desktop' });
    expect(t.profile?.name).toBe('Blender'); // the template itself is untouched
  });

  it('instantiates a setup template with its profiles', () => {
    const t = listTemplates('setup').find((x) => x.id === 'setup-creative')!;
    const r = instantiate(t, config);
    expect(r.setup).toMatchObject({ name: 'Creative', mode: 'follow', profiles: ['Blender 2'], fallback: 'Desktop 2' });
    expect(r.profiles.map((p) => p.name)).toEqual(['Blender 2', 'Desktop 2']);
  });

  it('warns about missing modules from the module list', () => {
    const mixer = listTemplates('profile').find((t) => t.id === 'volume-mixer')!;
    expect(moduleWarning(mixer, [{ kind: 'keypad' }, { kind: 'dial' }])).toBe('Uses faders, you have none: it will be skipped');
    expect(moduleWarning(mixer, [{ kind: 'keypad' }, { kind: 'fader' }])).toBeUndefined();
  });

  it('makes names unique', () => {
    expect(uniqueName(['A', 'A 2'], 'A')).toBe('A 3');
  });
});

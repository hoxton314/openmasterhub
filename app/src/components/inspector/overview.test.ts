import { describe, expect, it } from 'vitest';
import type { Config } from '../../lib/types';
import { overview } from './overview';

const config: Config = {
  backlight: 80,
  profile: [
    {
      name: 'Desktop',
      page: [{ name: 'Main' }],
      knob: [{ index: 0, cw: { volume: '+2' } }, { index: 1, cw: { volume: '+2' } }, { index: 2, cw: { volume: '+2' } }],
      roller: [{ side: 'right', away: { page: 'next' } }],
    },
    {
      name: 'Blender',
      base: 'Desktop',
      apps: ['blender'],
      page: [{ name: 'Views', key: [{ index: 0, label: 'Front' }] }, { name: 'Render' }],
      folder: [{ name: 'Shading' }],
      dial: { label: 'Zoom' },
      roller: [{ side: 'left', away: { keys: 'up' } }],
    },
  ],
};

describe('profile overview', () => {
  it('lists what the base profile provides and how each module is set', () => {
    const o = overview(config, 'Blender', 'Views', undefined);
    expect(o.inherited).toEqual(['Knobs 1–3', 'Right roller']);
    expect(o.modules).toEqual({ keypad: '2 pages, 1 folder', dial: 'set here', knob: 'inherited', roller: 'left set, right inherited', fader: 'not set' });
  });

  it('has nothing inherited without a base', () => {
    const o = overview(config, 'Desktop', 'Main', undefined);
    expect(o.inherited).toEqual([]);
    expect(o.modules.knob).toBe('set here');
    expect(o.modules.roller).toBe('left not set, right set');
  });
});

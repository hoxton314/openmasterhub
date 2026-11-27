import { describe, expect, it } from 'vitest';
import type { Config } from '../../lib/types';
import { usedBy } from './usage';

const config: Config = {
  backlight: 80,
  audio_group: [{ name: 'Games' }],
  pinned: [{ index: 8, press: { mute: { state: 'toggle', group: 'Games' } } }],
  profile: [
    { name: 'Play', knob: [{ index: 2, cw: { volume: { level: '+2', group: 'Games' } } }] },
    { name: 'Desk', dial: { cw: { volume: { level: '+2', group: 'Voice' } } }, page: [{ name: 'Main', key: [{ index: 13, press: { mute: { state: 'on', group: 'Games' } } }] }] },
  ],
};

describe('usedBy', () => {
  it('lists every control that targets the group', () => {
    expect(usedBy(config, 'Games')).toEqual([
      { where: 'Play', control: 'Knob 3' },
      { where: 'Desk', control: 'Key 14 · Main' },
      { where: 'Pinned keys', control: 'Key 9' },
    ]);
  });
  it('ignores other groups and plain actions', () => {
    expect(usedBy(config, 'Voice')).toEqual([{ where: 'Desk', control: 'Dial' }]);
    expect(usedBy(config, 'Nothing')).toEqual([]);
  });
});

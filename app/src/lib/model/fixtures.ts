// The IA-Precedence scenarios as a config (used by the tests; mirrors the mock's three setups).

import type { Config } from '../types';

const key = (index: number, label: string) => ({ index, label, press: { keys: 'ctrl+s' } });

export const fixture: Config = {
  backlight: 80,
  pinned: [{ index: 8, label: 'Claude', widget: { kind: 'claude_session' }, except_setups: ['Play'] }],
  setup: [
    { name: 'Work', mode: 'follow', profiles: ['Blender', 'Godot', 'Ghostty', 'Claude'], fallback: 'Desktop' },
    { name: 'Stream', mode: 'manual', profiles: ['Scenes', 'Desktop'], wrap: true },
    { name: 'Play', mode: 'hybrid', profiles: ['Cyberpunk 2077', 'Discord'], fallback: 'Desktop light', release: 'next_app' },
  ],
  profile: [
    {
      name: 'Desktop',
      page: [{ name: 'Main', key: [key(0, 'Files'), key(14, 'Apps')] }],
      dial: { label: 'Volume', cw: { volume: '+2' } },
      knob: [0, 1, 2].map((index) => ({ index, cw: { volume: '+2' } })),
      roller: [{ side: 'left', away: { page: 'next' } }, { side: 'right', away: { keys: 'ctrl+z' } }],
    },
    { name: 'Desktop light', base: 'Desktop', page: [{ name: 'Main' }], roller: [{ side: 'left', away: { page: 'prev' } }] },
    {
      name: 'Blender',
      apps: ['blender'],
      base: 'Desktop',
      page: [{ name: 'Views', key: [...Array.from({ length: 15 }, (_, i) => key(i, `B${i + 1}`))] }, { name: 'Modeling' }],
      dial: { label: 'Zoom view', cw: { keys: 'ctrl+plus' } },
    },
    { name: 'Godot', apps: ['godot', 'Godot*.exe'], base: 'Desktop', page: [{ name: 'Main' }] },
    { name: 'Ghostty', apps: ['com.mitchellh.ghostty'], page: [{ name: 'Main' }] },
    { name: 'Claude', apps: ['claude-*', 'com.mitchellh.ghostty'], page: [{ name: 'Main' }] },
    { name: 'Scenes', apps: ['obs'], page: [{ name: 'Scenes', key: [key(0, 'Live')] }], knob: [{ index: 0, cw: { volume: '+2' } }] },
    { name: 'Cyberpunk 2077', apps: ['steam_app_1091500'], base: 'Desktop light', page: [{ name: 'Combat' }], knob: [0, 1, 2].map((index) => ({ index, cw: { keys: 'f' } })) },
    { name: 'Discord', apps: ['discord', 'Discord.exe'], page: [{ name: 'Main' }] },
  ],
};

// Mock commands for WP3 · setups. Commands here win over the base ones in ../mock.ts.

import type { HubState } from '../../types';
import type { MockHandlers } from '../mock';

const WINDOWS = [
  { class: 'com.mitchellh.ghostty', title: '~/git/openmasterhub' },
  { class: 'claude-ghostty', title: 'claude · omh-ui' },
  { class: 'firefox', title: 'OpenMasterHUB – PR #42' },
  { class: 'blender', title: 'scene.blend' },
  { class: 'obsidian', title: 'Notes' },
  { class: 'steam_app_1091500', title: 'Cyberpunk 2077' },
];

export const handlers: MockHandlers = {
  open_window_titles: () => WINDOWS,
  open_window_classes: () => WINDOWS.map((w) => w.class),
  // Hybrid pin: `pin_profile` pins a profile (the live one when no name), `release_pin` lets the rules resume.
  pin_profile: (args, { snapshot, emit }) => {
    const name = (args?.name as string | undefined) ?? snapshot.state?.profile ?? '';
    snapshot.state = { ...snapshot.state!, profile: name, reason: { kind: 'pin' }, pinned: { profile: name, since: Math.floor(Date.now() / 1000) } } as HubState;
    emit('hub:state', snapshot.state);
  },
  release_pin: (_args, { snapshot, emit }) => {
    const { pinned: _p, ...rest } = snapshot.state!;
    snapshot.state = { ...rest, reason: { kind: 'fallback' } } as HubState;
    emit('hub:state', snapshot.state);
  },
};

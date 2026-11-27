// Mock commands for WP1 · editor area. Commands here win over the base ones in ../mock.ts.
// `?demo=claim` starts with Cyberpunk 2077 live in Play, with its game mode claiming the keypad
// and dial (the S03 situation).

import type { ClaimInfo, Snapshot } from '../../types';
import type { MockHandlers } from '../mock';

let started = false;

function demoClaim(snapshot: Snapshot, emit: (event: string, payload: unknown) => void) {
  started = true;
  if (typeof location === 'undefined' || new URLSearchParams(location.search).get('demo') !== 'claim') return;
  snapshot.state = {
    ...snapshot.state!,
    setup: 'Play',
    profile: 'Cyberpunk 2077',
    page: 'Main',
    active_app: 'Cyberpunk2077.exe',
    reason: { kind: 'rule', pattern: 'steam_app_1091500', index: 0 },
  };
  const claim: ClaimInfo = {
    claim: 1,
    name: 'Cyberpunk 2077 mode',
    active: true,
    owns: { keys: Array.from({ length: 15 }, (_, i) => i), dial: true },
    priority: 50,
    apps: ['steam_app_1091500'],
  };
  setTimeout(() => emit('hub:claims', [claim]), 700);
}

export const handlers: MockHandlers = {
  snapshot: (_args, { snapshot, emit }) => {
    if (!started) demoClaim(snapshot, emit);
    return snapshot;
  },
};

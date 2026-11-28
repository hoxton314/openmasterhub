// Mock commands for WP1 · editor area. Commands here win over the base ones in ../mock.ts.
// `?demo=claim` starts with Cyberpunk 2077 live in Play, with its game mode claiming the keypad
// and dial (the S03 situation).

import type { ClaimInfo } from '../../types';
import type { MockContext, MockHandlers } from '../mock';

const demo = typeof location !== 'undefined' && new URLSearchParams(location.search).get('demo') === 'claim';

/** The state of the demo: the game is live; its claim arrives with the snapshot (see onSnapshot). */
export function init({ snapshot }: MockContext) {
  if (!demo) return;
  snapshot.state = {
    ...snapshot.state!,
    setup: 'Play',
    profile: 'Cyberpunk 2077',
    page: 'Main',
    active_app: 'Cyberpunk2077.exe',
    reason: { kind: 'rule', pattern: 'steam_app_1091500', index: 0 },
  };
}

const claim: ClaimInfo = {
  claim: 7,
  name: 'Cyberpunk 2077 mode',
  active: true,
  owns: { keys: Array.from({ length: 15 }, (_, i) => i), dial: true },
  priority: 50,
  apps: ['steam_app_1091500'],
  pid: 4410,
  program: 'built-in',
};

let announced = false;
/** Upserts the claim (same id as library's), so it is there even with `?claims=none`. */
export function onSnapshot({ emit }: MockContext) {
  if (!demo || announced) return;
  announced = true;
  setTimeout(() => emit('hub:claim', claim), 700);
}

export const handlers: MockHandlers = {};

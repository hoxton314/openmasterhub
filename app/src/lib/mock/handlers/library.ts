// Mock commands for WP5 · library and integrations (game modes). Commands here win over the
// base ones in ../mock.ts.

import type { ClaimInfo, GamePlan, GameStatus } from '../../types';
import type { MockContext, MockHandlers } from '../mock';

// URL switches for design work: `?states=cyberpunk:bridge_down,deadlock:error` forces a game's
// state (bridge_down | error | running); `?hooks=missing` pretends no Claude hooks are installed;
// `?claims=none` starts with no claims at all; `?demo=claim` adds Cyberpunk's claim (see editor.ts).
const q = typeof location === 'undefined' ? new URLSearchParams() : new URLSearchParams(location.search);
const forced = Object.fromEntries((q.get('states') ?? '').split(',').filter(Boolean).map((s) => s.split(':') as [string, string]));
let hooksInstalled = q.get('hooks') !== 'missing';
// Cyberpunk's claim (7) is only there with `?demo=claim` (the editor mock puts the game live then).
const demoClaim = q.get('demo') === 'claim';
let claims: ClaimInfo[] = [
  { claim: 7, name: 'Cyberpunk 2077 mode', active: true, owns: { keys: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14], dial: true }, priority: 50, apps: ['steam_app_1091500'], pid: 4410, program: 'built-in' },
  { claim: 8, name: 'obs-hub-bridge', active: true, owns: { keys: [0, 1, 2, 3], leds: ['base'] }, priority: 20, apps: [], pid: 9021, program: '~/bin/obs-hub' },
  { claim: 9, name: 'my-script.py', active: false, owns: { knobs: [2] }, priority: 10, apps: [], pid: 9733, program: 'python3' },
];
if (!demoClaim) claims = claims.filter((c) => c.claim !== 7);
if (q.get('claims') === 'none') claims = [];

const sessions = [
  { name: 'omh-ui', status: 'ready' },
  { name: 'masterhubd', status: 'working' },
  { name: 'docs', status: 'attention' },
];

export function onSnapshot({ emit }: MockContext) {
  setTimeout(() => emit('hub:claims', claims), 300);
}

export const handlers: MockHandlers = {
  release_claim: (args, { emit }) => {
    claims = claims.filter((c) => c.claim !== args?.claim);
    emit('hub:claims', claims);
  },
  claude_hooks_status: () => (hooksInstalled ? { installed: true, path: '~/.claude/settings.json', hooks: 11, last_event_secs: 12 } : { installed: false, path: '~/.claude/settings.json', hooks: 0, last_event_secs: null }),
  claude_hooks_install: () => void (hooksInstalled = true),
  claude_hooks_remove: () => void (hooksInstalled = false),
  claude_sessions: () => (hooksInstalled ? sessions : []),
  games_bridge: (args) => {
    const f = forced[args?.game as string];
    if (f === 'bridge_down') return 'disconnected';
    if (f === 'error') return 'error';
    return f === 'running' || args?.game === 'cyberpunk' ? 'connected' : null;
  },
  games_list: () => mockGames,
  games_plan: async (args) => {
    await new Promise((r) => setTimeout(r, 700));
    return mockPlan(args?.game as string, args?.path as string | undefined);
  },
  games_setup: async (args) => {
    await new Promise((r) => setTimeout(r, 1200));
    if (forced[args?.game as string] === 'error') throw new Error('Could not write init.lua: permission denied');
    const g = mockGames.find((x) => x.game === args?.game);
    if (g) Object.assign(g, { enabled: true, assets: true, path: (args?.path as string) || g.path });
    return 'done';
  },
  games_disable: (args) => {
    const g = mockGames.find((x) => x.game === args?.game);
    if (g) g.enabled = false;
  },
};

const steam = '/home/you/.local/share/Steam/steamapps/common';
const mockGames: GameStatus[] = [
  { game: 'cyberpunk', steam: 1091500, title: 'Cyberpunk 2077', about: 'The game’s binds, a combat page, and through a Cyber Engine Tweaks mod cheats, stats on the dial, favourites and loadouts.', enabled: false, path: `${steam}/Cyberpunk 2077`, assets: true },
  { game: 'crimson_desert', steam: 3321460, title: 'Crimson Desert', about: 'Menus, quick slots, clips and the community map on the keys. Nothing is installed into the game.', enabled: false, path: null, assets: true },
  { game: 'deadlock', steam: 1422450, title: 'Deadlock', about: 'Your own binds for the shop, scores, chat and replays, console commands on spare keys, and a page with time, machine and music.', enabled: true, path: `${steam}/Deadlock`, assets: true },
  { game: 'wow_forever', steam: null, title: 'WoW Forever', about: 'Your binds and bars with cooldowns; an add-on gives XP, gold, zone and fights to the dial.', enabled: false, path: null, assets: false },
];

function mockPlan(game: string, path?: string): GamePlan {
  const g = mockGames.find((x) => x.game === game)!;
  const dir = path || g.path;
  const plan: GamePlan = { game, title: g.title, path: dir, detected: !path, steps: [], notes: [], error: null };
  const data = '/home/you/.local/share/openmasterhub/games';
  if (!dir && game !== 'crimson_desert') {
    plan.error = `${g.title} was not found through Steam: choose its install folder`;
    return plan;
  }
  if (game === 'cyberpunk') plan.steps.push({ kind: 'cet_bridge', path: `${dir}/bin/x64/plugins/cyber_engine_tweaks/mods/pulpit_bridge` });
  if (game === 'deadlock') {
    plan.steps.push({ kind: 'deadlock_cfg', path: `${dir}/game/citadel/cfg` });
    plan.steps.push({ kind: 'deadlock_icons', from: `${dir}/game/citadel/pak01_dir.vpk`, to: `${data}/deadlock/masks` });
  }
  if (game === 'wow_forever') {
    plan.steps.push({ kind: 'wow_addon', path: `${dir}/Interface/AddOns/PULPIT` });
    plan.notes.push('WoW’s icons and fonts can’t be extracted yet (Battle.net’s CASC storage): the keys use a stand-in look.');
  }
  plan.steps.push({ kind: 'enable' });
  return plan;
}

// `?game=deadlock` opens that game's setup plan, for design work on it.
if (typeof window !== 'undefined') {
  const open = q.get('game');
  if (open)
    setTimeout(async () => {
      const m = await import('../../games.svelte');
      await m.refreshGames();
      m.games.open = m.games.list.find((g) => g.game === open) ?? null;
    }, 800);
}

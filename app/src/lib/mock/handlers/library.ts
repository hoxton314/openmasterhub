// Mock commands for WP5 · library and integrations (game modes). Commands here win over the
// base ones in ../mock.ts.

import type { GamePlan, GameStatus } from '../../types';
import type { MockHandlers } from '../mock';

export const handlers: MockHandlers = {
  games_list: () => mockGames,
  games_plan: (args) => mockPlan(args?.game as string, args?.path as string | undefined),
  games_setup: async (args) => {
    await new Promise((r) => setTimeout(r, 1200));
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

// `?game=deadlock` opens that game's setup dialog, for design work on it.
if (typeof window !== 'undefined') {
  const open = new URLSearchParams(location.search).get('game');
  if (open)
    setTimeout(async () => {
      const m = await import('../../games.svelte');
      await m.refreshGames();
      m.games.open = m.games.list.find((g) => g.game === open) ?? null;
    }, 800);
}

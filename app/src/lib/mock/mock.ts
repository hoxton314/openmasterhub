// Browser-only stand-in for the Tauri backend, serving a snapshot of a real hub.

import sample from './snapshot.json';
import type { GamePlan, GameStatus, Face, Snapshot } from '../types';

type Handler = (e: { payload: unknown }) => void;
const handlers: Record<string, Handler[]> = {};
const emit = (event: string, payload: unknown) => (handlers[event] ?? []).forEach((h) => h({ payload }));
// A git-ignored `snapshot.local.json` (e.g. your own config) replaces the sample when present.
const local = Object.values(import.meta.glob('./snapshot.local.json', { eager: true, import: 'default' }))[0];
const snapshot = structuredClone(local ?? sample) as unknown as Snapshot;

export function listen(event: string, handler: Handler) {
  (handlers[event] ??= []).push(handler);
}

/** A rough browser rendering of a key face, standing in for the Rust renderer. */
function preview(face: Face): string {
  const c = document.createElement('canvas');
  c.width = c.height = 144;
  const g = c.getContext('2d')!;
  g.fillStyle = face.background ?? '#000';
  g.fillRect(0, 0, 144, 144);
  g.fillStyle = face.text_color ?? '#fff';
  g.font = '600 34px Inter Variable, sans-serif';
  g.textAlign = 'center';
  g.textBaseline = 'middle';
  const live = face.widget?.kind === 'claude_session' ? `Session ${(face.widget.slot ?? 0) + 1}` : face.widget ? '42%' : '';
  g.fillText(face.label ?? live, 72, 72, 130);
  return c.toDataURL();
}

export async function invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case 'snapshot':
      return snapshot;
    case 'save_config':
      console.info('[mock] save_config', args?.config);
      return;
    case 'set_profile':
      snapshot.state!.profile = args?.name as string;
      emit('hub:state', { ...snapshot.state, page: '' });
      return;
    case 'show_page':
      emit('hub:state', { ...snapshot.state, page: args?.name });
      return;
    case 'preview_face':
      return preview(args?.face as Face);
    case 'open_window_classes':
      return ['firefox', 'com.mitchellh.ghostty', 'obs', 'steam', 'code-oss'];
    case 'audio_info':
      return { apps: ['Discord', 'firefox', 'spotify'], outputs: ['Speakers', 'Headphones'], inputs: ['USB Microphone', 'Webcam'] };
    case 'games_list':
      return mockGames;
    case 'games_plan':
      return mockPlan(args?.game as string, args?.path as string | undefined);
    case 'games_setup': {
      await new Promise((r) => setTimeout(r, 1200));
      const g = mockGames.find((x) => x.game === args?.game);
      if (g) Object.assign(g, { enabled: true, assets: true, path: (args?.path as string) || g.path });
      return 'done';
    }
    case 'games_disable': {
      const g = mockGames.find((x) => x.game === args?.game);
      if (g) g.enabled = false;
      return;
    }
    default:
      return;
  }
}

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

// Simulated input, so the live-feedback design can be judged: press `i` in the browser.
if (typeof window !== 'undefined') {
  window.addEventListener('keydown', (e) => {
    if (e.key !== 'i' || (e.target as HTMLElement).tagName === 'INPUT') return;
    const keypad = snapshot.modules.find((m) => m.kind === 'keypad');
    if (keypad) emit('hub:input', { serial: keypad.serial, control: 'key', index: Math.floor(Math.random() * 15), event: 'down' });
  });
}

// `?game=deadlock` opens that game's setup dialog, for design work on it.
if (typeof window !== 'undefined') {
  const open = new URLSearchParams(location.search).get('game');
  if (open)
    setTimeout(async () => {
      const m = await import('../games.svelte');
      await m.refreshGames();
      m.games.open = m.games.list.find((g) => g.game === open) ?? null;
    }, 800);
}

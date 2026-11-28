// Library integrations: game modes, Claude Code and the live surface, with their status.
// Games come from `games.svelte.ts`; claims from `hub.claims`; the bridge, hook and session
// state from the service (commands that older services lack fail quietly and read as unknown).

import { invoke } from './backend';
import { games, refreshGames } from './games.svelte';
import { hub } from './hub.svelte';
import type { ClaimInfo, Controls, GameStatus, ModuleKind } from './types';

/** Device (LED and key) colours the service uses when `[status]` leaves them out (docs/CLAUDE_CODE.md). Data, not UI colours: they stay out of the themes. */
export const STATUS_DEFAULTS = { attention: '#ff4000', ready: '#00ff40', working: '#0060ff' } as const;

export type IntegrationState = 'checking' | 'off' | 'needs_setup' | 'on' | 'running' | 'bridge_down' | 'error';
export type IntegrationId = string;

export interface IntegrationEntry {
  id: IntegrationId;
  type: 'game' | 'claude' | 'live';
  title: string;
  subtitle: string;
  state: IntegrationState;
  stateLabel: string;
  needs: ModuleKind[];
  category: string;
  /** The integration is switched on. */
  installed: boolean;
  game?: GameStatus;
}

export interface HookStatus {
  installed: boolean;
  path: string;
  hooks: number;
  /** Seconds since the last hook event reached the service; null = none yet. */
  last_event_secs: number | null;
}

export interface ClaudeSession {
  name: string;
  status: 'attention' | 'ready' | 'working';
}

export const integ = $state({
  /** `games_bridge` per game: connected | disconnected | error, or null/missing = unknown. */
  bridge: {} as Record<string, string | null>,
  /** Failures of the last setup, per game. */
  errors: {} as Record<string, string>,
  hooks: null as HookStatus | null,
  hooksBusy: false,
  sessions: [] as ClaudeSession[],
});

const SUBTITLE: Record<string, string> = {
  cyberpunk: 'Game mode · mod bridge',
  deadlock: 'Game mode · reads your binds',
  wow_forever: 'Game mode · add-on bridge',
  crimson_desert: 'Game mode · keys only',
};

const BRIDGE_GAMES = new Set(['cyberpunk', 'wow_forever']);

/** Does a claim belong to this game's mode? (the mode names its claim after the game). */
function claimOf(g: GameStatus): ClaimInfo | undefined {
  const t = g.title.toLowerCase();
  return hub.claims.find((c) => c.name.toLowerCase().includes(t));
}

export function gameState(g: GameStatus): { state: IntegrationState; label: string } {
  const bridge = integ.bridge[g.game];
  if (integ.errors[g.game] || bridge === 'error') return { state: 'error', label: 'Error' };
  if (!g.enabled) return g.assets ? { state: 'off', label: 'Off' } : { state: 'needs_setup', label: 'Needs setup' };
  if (BRIDGE_GAMES.has(g.game) && bridge === 'disconnected') return { state: 'bridge_down', label: 'On · bridge disconnected' };
  if (claimOf(g)?.active) return { state: 'running', label: 'On · running' };
  if (bridge === 'connected') return { state: 'on', label: 'On · bridge connected' };
  return { state: 'on', label: 'On' };
}

export function gameClaim(g: GameStatus) {
  return claimOf(g);
}

export function gameError(g: GameStatus): string | undefined {
  return integ.errors[g.game] ?? (integ.bridge[g.game] === 'error' ? 'The last setup did not finish: set it up again.' : undefined);
}

/** Which controls a game mode takes over while its game is focused (docs/GAME_MODES.md). */
export function gameControls(game: string): { takes: string[]; stays: string[] } {
  return game === 'crimson_desert' ? { takes: ['Keypad'], stays: ['Dial', 'Knobs', 'Rollers'] } : { takes: ['Keypad', 'Dial', 'Rollers'], stays: ['Knobs'] };
}

/** All integrations as library entries. */
export function integrationEntries(): IntegrationEntry[] {
  const out: IntegrationEntry[] = games.list.map((g) => {
    const s = gameState(g);
    return {
      id: g.game,
      type: 'game' as const,
      title: g.title,
      subtitle: SUBTITLE[g.game] ?? 'Game mode',
      state: s.state,
      stateLabel: s.label,
      needs: g.game === 'crimson_desert' ? (['keypad'] as ModuleKind[]) : (['keypad', 'dial'] as ModuleKind[]),
      category: 'Games',
      installed: g.enabled,
      game: g,
    };
  });
  const h = integ.hooks;
  const n = integ.sessions.length;
  out.push({
    id: 'claude-code',
    type: 'claude',
    title: 'Claude Code',
    subtitle: 'Session keys and LED',
    state: h === null ? 'checking' : !h.installed ? 'needs_setup' : 'on',
    stateLabel: h === null ? 'Checking' : !h.installed ? 'Hooks missing' : n ? `On · ${n} session${n === 1 ? '' : 's'}` : 'On',
    needs: ['keypad', 'base'],
    category: 'Development',
    installed: !!h?.installed,
  });
  const c = hub.claims.length;
  out.push({
    id: 'live-surface',
    type: 'live',
    title: 'Live surface',
    subtitle: 'Other programs on the hub',
    state: c ? 'running' : 'off',
    stateLabel: c ? `${c} client${c === 1 ? '' : 's'}` : 'No clients',
    needs: [],
    category: 'Development',
    installed: c > 0,
  });
  return out;
}

export async function refreshBridge() {
  for (const g of games.list) {
    if (!g.enabled) {
      delete integ.bridge[g.game];
      continue;
    }
    integ.bridge[g.game] = await invoke<string | null>('games_bridge', { game: g.game }).catch(() => null);
  }
}

export async function refreshIntegrations() {
  await refreshGames();
  await Promise.all([refreshBridge(), refreshHooks(), refreshSessions()]);
}

export async function refreshHooks() {
  integ.hooks = await invoke<HookStatus>('claude_hooks_status').catch(() => ({ installed: false, path: '~/.claude/settings.json', hooks: 0, last_event_secs: null }));
}

export async function refreshSessions() {
  integ.sessions = await invoke<ClaudeSession[]>('claude_sessions').catch(() => []);
}

/** Install (or reinstall) or remove the Claude Code hooks in `~/.claude/settings.json`. */
export async function setHooks(install: boolean): Promise<string | undefined> {
  integ.hooksBusy = true;
  try {
    await invoke(install ? 'claude_hooks_install' : 'claude_hooks_remove');
    await refreshHooks();
    await refreshSessions();
    return undefined;
  } catch (e) {
    return String(e);
  } finally {
    integ.hooksBusy = false;
  }
}

export async function releaseClaim(claim: number): Promise<string | undefined> {
  try {
    await invoke('release_claim', { claim });
    return undefined;
  } catch (e) {
    return String(e);
  }
}

/** A claim's controls as short labels: "Keypad", "Keys 1–4", "Knob 3", "LED base". */
export function controlLabels(o: Controls): string[] {
  const out: string[] = [];
  const keys = o.keys ?? [];
  if (keys.length >= 15) out.push('Keypad');
  else if (keys.length) out.push(`${keys.length === 1 ? 'Key' : 'Keys'} ${ranges(keys.map((k) => k + 1))}`);
  if (o.dial) out.push('Dial');
  if (o.knobs?.length) out.push(`${o.knobs.length === 1 ? 'Knob' : 'Knobs'} ${ranges(o.knobs.map((k) => k + 1))}`);
  if (o.faders?.length) out.push(`${o.faders.length === 1 ? 'Fader' : 'Faders'} ${ranges(o.faders.map((k) => k + 1))}`);
  if (o.rollers?.length) out.push(o.rollers.length === 2 ? 'Rollers' : o.rollers[0] === 0 ? 'Left roller' : 'Right roller');
  for (const l of o.leds ?? []) out.push(`LED ${l}`);
  return out;
}

function ranges(nums: number[]): string {
  const s = [...nums].sort((a, b) => a - b);
  const parts: string[] = [];
  for (let i = 0; i < s.length; ) {
    let j = i;
    while (j + 1 < s.length && s[j + 1] === s[j] + 1) j++;
    parts.push(j > i ? `${s[i]}–${s[j]}` : `${s[i]}`);
    i = j + 1;
  }
  return parts.join(', ');
}

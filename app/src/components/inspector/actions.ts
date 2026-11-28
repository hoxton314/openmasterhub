// Action kinds for the editor: icons, availability by capability, and the forms of audio actions.

import { ArrowDownToLine, Bot, CornerUpLeft, Folder, Keyboard, LayoutGrid, Layers, Terminal, UserRound, Volume2, VolumeX } from 'lucide-svelte';
import { hasCapability } from '../../lib/hub.svelte';
import type { Action, AudioTarget, Mute } from '../../lib/types';
import type { Icon } from '../ui/icon';

export type Kind = 'keys' | 'cmd' | 'volume' | 'mute' | 'page' | 'folder' | 'back' | 'profile' | 'setup' | 'session';

export const KINDS: { k: Kind; label: string; icon: Icon }[] = [
  { k: 'keys', label: 'Keys', icon: Keyboard },
  { k: 'cmd', label: 'Command', icon: Terminal },
  { k: 'volume', label: 'Volume', icon: Volume2 },
  { k: 'mute', label: 'Mute', icon: VolumeX },
  { k: 'page', label: 'Page', icon: LayoutGrid },
  { k: 'folder', label: 'Folder', icon: Folder },
  { k: 'back', label: 'Back', icon: CornerUpLeft },
  { k: 'profile', label: 'Profile', icon: UserRound },
  { k: 'setup', label: 'Setup', icon: Layers },
  { k: 'session', label: 'Claude', icon: Bot },
];

export const kindIcon = (k: Kind): Icon => KINDS.find((x) => x.k === k)?.icon ?? ArrowDownToLine;
export const kindLabel = (k: Kind) => KINDS.find((x) => x.k === k)?.label ?? '';

/** Setup and pin actions need a service that knows them. */
export function kindAvailable(k: Kind): boolean {
  return k !== 'setup' || hasCapability('setup_action') || hasCapability('pin');
}

export function kindOf(a: Action | undefined): Kind | undefined {
  if (!a) return undefined;
  if ('cmd' in a) return 'cmd';
  if ('keys' in a) return 'keys';
  if ('volume' in a) return 'volume';
  if ('mute' in a) return 'mute';
  if ('page' in a) return 'page';
  if ('folder' in a) return 'folder';
  if ('back' in a) return 'back';
  if ('setup' in a || 'pin' in a) return 'setup';
  if ('session' in a || 'sessions' in a) return 'session';
  return 'profile';
}

/** Same forms as `VolumeChange::parse` in masterhub-core. */
export const VOLUME = /^\s*([+-]\s*\d+|\d{1,2}|100|\{value\})\s*$/;

export type LevelMode = 'up' | 'down' | 'set' | 'follow';

export function levelMode(level: string): LevelMode {
  const l = level.replace(/\s/g, '');
  if (l === '{value}') return 'follow';
  if (l.startsWith('+')) return 'up';
  if (l.startsWith('-')) return 'down';
  return 'set';
}
export const levelNumber = (level: string): number => Math.abs(parseInt(level.replace(/[\s+]/g, ''), 10)) || 0;
export function buildLevel(mode: LevelMode, n: number): string {
  const v = Math.max(0, Math.min(100, Math.round(n)));
  return mode === 'follow' ? '{value}' : mode === 'up' ? `+${v}` : mode === 'down' ? `-${v}` : String(v);
}

export interface AudioParts {
  level?: string;
  state?: Mute;
  target: AudioTarget;
}

export function audioParts(a: Action | undefined): AudioParts {
  const pick = (t: AudioTarget): AudioTarget => ({
    ...(t.output !== undefined ? { output: t.output } : {}),
    ...(t.input !== undefined ? { input: t.input } : {}),
    ...(t.app !== undefined ? { app: t.app } : {}),
    ...(t.group !== undefined ? { group: t.group } : {}),
  });
  if (a && 'volume' in a) return typeof a.volume === 'string' ? { level: a.volume, target: {} } : { level: a.volume.level, target: pick(a.volume) };
  if (a && 'mute' in a) return typeof a.mute === 'string' ? { state: a.mute, target: {} } : { state: a.mute.state, target: pick(a.mute) };
  return { target: {} };
}

const hasTarget = (t: AudioTarget) => Object.keys(t).length > 0;
export const buildVolume = (level: string, t: AudioTarget): Action => ({ volume: hasTarget(t) ? { level, ...t } : level });
export const buildMute = (state: Mute, t: AudioTarget): Action => ({ mute: hasTarget(t) ? { state, ...t } : state });

export type TargetMode = 'default' | 'device' | 'app' | 'group';

export function targetMode(t: AudioTarget): TargetMode {
  if (t.app !== undefined) return 'app';
  if (t.group !== undefined) return 'group';
  if (t.input !== undefined) return 'device';
  if (t.output !== undefined && t.output !== 'default') return 'device';
  return 'default';
}

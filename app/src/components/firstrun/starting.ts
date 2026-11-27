// The starting setups offered on the last first-run step. They are small on purpose: profiles
// with one empty page each, ready to be filled in the editor. Bundled templates with real
// controls live in lib/templates (Library, New setup).

import type { Config, Profile, Setup } from '../../lib/types';

export type StartingId = 'work' | 'streaming' | 'play' | 'empty';

import { Briefcase, Gamepad2, Radio, Square } from 'lucide-svelte';
import type { Icon } from '../ui/icon';

export interface Starting {
  icon: Icon;
  id: StartingId;
  title: string;
  about: string;
}

export const STARTINGS: Starting[] = [
  { icon: Briefcase, id: 'work', title: 'Work', about: 'Profiles for the apps you have open, and a Desktop fallback' },
  { icon: Radio, id: 'streaming', title: 'Streaming', about: 'Four manual scenes: Starting soon, Live, BRB, Ending' },
  { icon: Gamepad2, id: 'play', title: 'Play', about: 'Your games, plus game modes you approve later from Library' },
  { icon: Square, id: 'empty', title: 'Start empty', about: 'One blank profile' },
];

/** Window classes → profile name, for the apps the Work setup recognises. */
const KNOWN: [RegExp, string][] = [
  [/blender/i, 'Blender'],
  [/discord/i, 'Discord'],
  [/ghostty|kitty|alacritty|konsole|gnome-terminal|wezterm|windowsterminal/i, 'Terminal'],
  [/obsidian/i, 'Obsidian'],
  [/godot/i, 'Godot'],
  [/obs/i, 'OBS'],
  [/firefox|chrome|chromium|brave|zen|vivaldi/i, 'Browser'],
  [/code|zed|jetbrains|idea|nvim/i, 'Editor'],
];

const page = () => [{ name: 'Main', key: [] }];
const blank = (name: string, extra: Partial<Profile> = {}): Profile => ({ name, page: page(), ...extra });

/** Profiles for the open windows: one per recognised app, the first window class of each. */
export function workProfiles(openClasses: string[]): Profile[] {
  const out: Profile[] = [];
  for (const [re, name] of KNOWN) {
    const hit = openClasses.filter((c) => re.test(c));
    if (hit.length && !out.some((p) => p.name === name)) out.push({ name, apps: hit.slice(0, 2), about: `Keys for ${name}`, page: page() });
  }
  return out;
}

/** What to add to the config for a starting choice. Existing profile names are reused, not duplicated. */
export function build(id: StartingId, openClasses: string[]): { profiles: Profile[]; setup?: Setup } {
  const desktop = blank('Desktop', { about: 'When nothing else matches' });
  switch (id) {
    case 'work': {
      const profiles = workProfiles(openClasses);
      return { profiles: [...profiles, desktop], setup: { name: 'Work', mode: 'follow', profiles: profiles.map((p) => p.name), fallback: 'Desktop' } };
    }
    case 'streaming': {
      const names = ['Starting soon', 'Live', 'BRB', 'Ending'];
      return { profiles: names.map((n) => blank(n)), setup: { name: 'Stream', mode: 'manual', profiles: names, wrap: true } };
    }
    case 'play':
      return { profiles: [desktop], setup: { name: 'Play', mode: 'follow', profiles: [], fallback: 'Desktop' } };
    default:
      return { profiles: [blank('Desktop')] };
  }
}

/** Add a starting choice to the config; returns the name of the setup to switch to, if any. */
export function apply(c: Config, id: StartingId, openClasses: string[]): string | undefined {
  const { profiles, setup } = build(id, openClasses);
  const have = new Set((c.profile ?? []).map((p) => p.name));
  c.profile = [...(c.profile ?? []), ...profiles.filter((p) => !have.has(p.name))];
  if (!setup) return undefined;
  if (c.setup?.some((s) => s.name === setup.name)) return setup.name;
  c.setup = [...(c.setup ?? []), setup];
  return setup.name;
}

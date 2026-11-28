// The starting setups offered on the last first-run step, built from the bundled templates in
// lib/templates (the same `instantiate` the Library and New setup use).

import { getTemplate, instantiate, type Template } from '../../lib/templates';
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
  { icon: Radio, id: 'streaming', title: 'Streaming', about: 'A manual setup with OBS scene keys, cycled by hand' },
  { icon: Gamepad2, id: 'play', title: 'Play', about: 'Follows the running game, with a quiet desktop otherwise. Game modes you approve later from Library' },
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

const blank = (name: string, extra: Partial<Profile> = {}): Profile => ({ name, page: [{ name: 'Main', key: [] }], ...extra });

const template = (id: string): Template => {
  const t = getTemplate(id);
  if (!t) throw new Error(`missing bundled template ${id}`);
  return t;
};

/** Profiles for the open windows: one per recognised app, the first window class of each. */
export function workProfiles(openClasses: string[]): Profile[] {
  const out: Profile[] = [];
  for (const [re, name] of KNOWN) {
    const hit = openClasses.filter((c) => re.test(c));
    if (hit.length && !out.some((p) => p.name === name)) out.push({ name, apps: hit.slice(0, 2), about: `Keys for ${name}`, page: [{ name: 'Main', key: [] }] });
  }
  return out;
}

/**
 * What to add to the config for a starting choice, from the bundled templates (Streaming, Games
 * and the Desktop fallback are `instantiate`d like in the Library); Work is built from the open windows.
 */
export function build(id: StartingId, openClasses: string[], config: Config = { backlight: 0 }): { profiles: Profile[]; setup?: Setup } {
  switch (id) {
    case 'work': {
      const apps = workProfiles(openClasses);
      const desktop = instantiate(template('desktop-fallback'), config, { name: 'Desktop' }).profiles[0];
      return { profiles: [...apps, desktop], setup: { name: 'Work', mode: 'follow', profiles: apps.map((p) => p.name), fallback: desktop.name } };
    }
    case 'streaming': {
      const made = instantiate(template('setup-streaming'), config, { name: 'Stream' });
      return { profiles: made.profiles, setup: made.setup };
    }
    case 'play': {
      const made = instantiate(template('setup-games'), config, { name: 'Play' });
      return { profiles: made.profiles, setup: made.setup };
    }
    default:
      return { profiles: [blank('Desktop')] };
  }
}

/** Add a starting choice to the config; returns the name of the setup to switch to, if any. */
export function apply(c: Config, id: StartingId, openClasses: string[]): string | undefined {
  const { profiles, setup } = build(id, openClasses, c);
  const have = new Set((c.profile ?? []).map((p) => p.name));
  c.profile = [...(c.profile ?? []), ...profiles.filter((p) => !have.has(p.name))];
  if (!setup) return undefined;
  if (c.setup?.some((s) => s.name === setup.name)) return setup.name;
  c.setup = [...(c.setup ?? []), setup];
  return setup.name;
}

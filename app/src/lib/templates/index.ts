// Bundled templates: profiles, setups and single-control fragments. Plain JSON in `data/`
// (no game assets: game modes read artwork from the user's install). Library and first run read
// this through `listTemplates` and `instantiate`.

import type { Config, DialConfig, FaderConfig, KnobConfig, ModuleKind, ModuleState, Profile, RollerConfig, Setup, SetupMode } from '../types';

export type TemplateKind = 'profile' | 'setup' | 'control';

/** Controls a control template sets; merged into a profile by the caller. */
export interface ControlsFragment {
  dial?: DialConfig;
  knob?: KnobConfig[];
  fader?: FaderConfig[];
  roller?: RollerConfig[];
}

/** A setup template: `profiles` and `fallback` are ids of profile templates. */
export interface SetupTemplate {
  name: string;
  mode: SetupMode;
  color?: string;
  profiles: string[];
  fallback?: string;
  wrap?: boolean;
  show_on_dial?: boolean;
}

export interface Template {
  id: string;
  kind: TemplateKind;
  name: string;
  about: string;
  category: string;
  /** Modules the template uses; the gallery warns about the ones the hub lacks. */
  needs: ModuleKind[];
  profile?: Profile;
  setup?: SetupTemplate;
  controls?: ControlsFragment;
}

export interface InstantiateOptions {
  /** Name of the new profile or setup (made unique against `config`). */
  name?: string;
  /** App rule for a profile template (default: the template's own). */
  apps?: string[];
  /** Base profile (default: none). */
  base?: string;
}

/** What `instantiate` produced: profiles to append, a setup to add, controls to merge. */
export interface Instantiated {
  profiles: Profile[];
  setup?: Setup;
  controls?: ControlsFragment;
}

/** Profile templates first in the order the gallery shows them. */
const PROFILE_ORDER = ['blender', 'streaming', 'godot', 'volume-mixer', 'desktop-media', 'claude-sessions', 'desktop-fallback'];
function order(t: Template): number {
  const i = PROFILE_ORDER.indexOf(t.id);
  return i < 0 ? 100 : i;
}

const files = import.meta.glob<Template>('./data/*.json', { eager: true, import: 'default' });
const all: Template[] = Object.values(files).sort((a, b) => order(a) - order(b) || a.name.localeCompare(b.name));

export function listTemplates(kind: TemplateKind): Template[] {
  return all.filter((t) => t.kind === kind);
}

export function getTemplate(id: string): Template | undefined {
  return all.find((t) => t.id === id);
}

const MODULE_NAMES: Record<ModuleKind, string> = { base: 'base', keypad: 'keypad', fader: 'faders', knob: 'knobs', roller: 'rollers', dial: 'dial' };
export const moduleName = (k: ModuleKind) => MODULE_NAMES[k];

/** Module names for chips: "Keypad", "Dial", "Knobs". */
export function needsLabels(t: Template): string[] {
  return t.needs.map((k) => MODULE_NAMES[k].replace(/^./, (c) => c.toUpperCase()));
}

/** Modules the template needs that none of `modules` provides. */
export function missingModules(t: Template, modules: Pick<ModuleState, 'kind'>[]): ModuleKind[] {
  return t.needs.filter((k) => !modules.some((m) => m.kind === k));
}

/** "Uses faders, you have none: they will be skipped"; undefined when nothing is missing. */
export function moduleWarning(t: Template, modules: Pick<ModuleState, 'kind'>[]): string | undefined {
  const missing = missingModules(t, modules);
  if (!missing.length) return undefined;
  const names = missing.map(moduleName);
  const list = names.length > 1 ? `${names.slice(0, -1).join(', ')} and ${names.at(-1)}` : names[0];
  return `Uses ${list}, you have none: ${missing.length > 1 ? 'they' : 'it'} will be skipped`;
}

/** `name`, or `name 2`, `name 3` … when taken. */
export function uniqueName(taken: Iterable<string>, name: string): string {
  const set = new Set(taken);
  if (!set.has(name)) return name;
  let n = 2;
  while (set.has(`${name} ${n}`)) n++;
  return `${name} ${n}`;
}

function copy<T>(v: T): T {
  return JSON.parse(JSON.stringify(v)) as T;
}

/**
 * A template as new config pieces; nothing is written to `config`. Names are unique against its
 * profiles and setups (a setup template's profiles are named after their templates).
 */
export function instantiate(template: Template, config: Config, opts: InstantiateOptions = {}): Instantiated {
  const taken = new Set((config.profile ?? []).map((p) => p.name));
  if (template.kind === 'control') return { profiles: [], controls: copy(template.controls ?? {}) };

  if (template.kind === 'profile' && template.profile) {
    const p = copy(template.profile);
    p.name = uniqueName(taken, opts.name?.trim() || p.name);
    if (opts.apps) p.apps = opts.apps.length ? opts.apps : undefined;
    if (opts.base) p.base = opts.base;
    p.about ??= template.about;
    if (!p.apps) delete p.apps;
    return { profiles: [p] };
  }

  const st = template.setup;
  if (!st) return { profiles: [] };
  const made = new Map<string, Profile>();
  const build = (id: string) => {
    const t = getTemplate(id);
    if (!t?.profile) return undefined;
    const p = instantiate(t, { backlight: config.backlight, profile: [...(config.profile ?? []), ...made.values()] }).profiles[0];
    made.set(id, p);
    return p;
  };
  const profiles = st.profiles.map(build).filter((p): p is Profile => !!p);
  const fallback = st.fallback ? build(st.fallback) : undefined;
  const setupNames = (config.setup ?? []).map((s) => s.name);
  const setup: Setup = {
    name: uniqueName(setupNames, opts.name?.trim() || st.name),
    mode: st.mode,
    profiles: profiles.map((p) => p.name),
    ...(fallback && st.mode !== 'manual' ? { fallback: fallback.name } : {}),
    ...(st.color ? { color: st.color } : {}),
    ...(st.wrap ? { wrap: true } : {}),
    ...(st.show_on_dial ? { show_on_dial: true } : {}),
  };
  return { profiles: [...profiles, ...(fallback ? [fallback] : [])], setup };
}

/** Merge a control template's controls into a profile: a dial or a knob/fader/roller replaces the one at the same index or side. */
export function applyControls(profile: Profile, fragment: ControlsFragment): void {
  const f = copy(fragment);
  if (f.dial) profile.dial = f.dial;
  if (f.knob) profile.knob = [...(profile.knob ?? []).filter((k) => !f.knob!.some((n) => n.index === k.index)), ...f.knob].sort((a, b) => a.index - b.index);
  if (f.fader) profile.fader = [...(profile.fader ?? []).filter((k) => !f.fader!.some((n) => n.index === k.index)), ...f.fader].sort((a, b) => a.index - b.index);
  if (f.roller) profile.roller = [...(profile.roller ?? []).filter((k) => !f.roller!.some((n) => n.side === k.side)), ...f.roller];
}

// The theme store: which themes exist (bundled + the user's), which one is chosen, and applying it
// to `<html>`. Applying is instant (CSS variables), so switching never reloads anything.
//
// Settings live in `[app]` of config.toml (written by Settings › Appearance) with localStorage as
// the offline fallback; the last applied variables are cached so `main.ts` can paint the right
// colours before the app mounts.

import { invoke } from '../backend';
import type { AppSettings } from '../types';
import { flatten, inlineStyle, resolveTheme, ThemeError, validateFile, withDensity, type Appearance, type Density, type ResolvedTheme, type ThemeFile, type ThemeOutput } from './engine';

export type ReduceMotion = 'system' | 'on' | 'off';

export interface ThemeSettings {
  /** A theme id, or "system" to follow the OS between `themeDark` and `themeLight`. */
  theme: string;
  themeDark: string;
  themeLight: string;
  /** Unset: the theme's own density. */
  density?: Density;
  reduceMotion: ReduceMotion;
}

export interface ThemeInfo {
  id: string;
  name: string;
  author?: string;
  version?: string;
  description?: string;
  appearance: Appearance;
  followsSystem: boolean;
  source: 'bundled' | 'user';
  /** The user's file name, for user themes. */
  file?: string;
}

/** A user theme file that could not be used, with a readable reason. */
export interface ThemeProblem {
  file: string;
  message: string;
}

/** `themes_list` (Tauri): the files in `<config dir>/themes/`. `json` is the file's text. */
export interface UserThemeFile {
  file: string;
  json: string;
}

const SETTINGS_KEY = 'omh.theme.settings';
const VARS_KEY = 'omh.theme.vars';
export const DEFAULT_SETTINGS: ThemeSettings = { theme: 'system', themeDark: 'dark', themeLight: 'light', reduceMotion: 'system' };

const bundledFiles: ThemeFile[] = Object.values(
  import.meta.glob<ThemeFile>(['../../themes/*.json', '!../../themes/*.schema.json'], { eager: true, import: 'default' }),
).map((json) => validateFile(json, 'bundled theme').file);

const bundled = new Map(bundledFiles.map((f) => [f.id, f]));
let user = new Map<string, { file: ThemeFile; name: string }>();
/** User themes have been listed (until then an unknown chosen id keeps the cached look). */
let userLoaded = false;

function store<T>(key: string, value?: T): T | undefined {
  try {
    if (value === undefined) return JSON.parse(localStorage.getItem(key) ?? 'null') ?? undefined;
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* private mode or blocked storage: the defaults apply */
  }
  return undefined;
}

const media = typeof matchMedia === 'function' ? matchMedia('(prefers-color-scheme: dark)') : undefined;

export const theme = $state({
  settings: { ...DEFAULT_SETTINGS, ...store<Partial<ThemeSettings>>(SETTINGS_KEY) } as ThemeSettings,
  /** Every usable theme, user themes shadowing bundled ones with the same id. */
  list: [] as ThemeInfo[],
  problems: [] as ThemeProblem[],
  /** Id of the theme on screen. */
  active: 'dark',
  appearance: 'dark' as Appearance,
  systemDark: media?.matches ?? true,
});

/** A user theme may inherit the bundled theme it shadows (same id). */
function lookup(id: string, from: string): ThemeFile | undefined {
  if (id !== from && user.has(id)) return user.get(id)!.file;
  return bundled.get(id);
}

function fileOf(id: string): ThemeFile | undefined {
  return user.get(id)?.file ?? bundled.get(id);
}

const resolved = new Map<string, ResolvedTheme>();
function resolve(id: string): ResolvedTheme {
  let r = resolved.get(id);
  if (!r) {
    const f = fileOf(id);
    if (!f) throw new ThemeError(`theme “${id}” does not exist`);
    r = resolveTheme(f, lookup);
    resolved.set(id, r);
  }
  return r;
}

function refreshList() {
  const infos: ThemeInfo[] = [];
  for (const [id, f] of bundled) if (!user.has(id)) infos.push(info(f, 'bundled'));
  for (const [, u] of user) infos.push(info(u.file, 'user', u.name));
  const order = ['dark', 'light', 'signal'];
  const rank = (t: ThemeInfo) => (order.includes(t.id) ? order.indexOf(t.id) : 10);
  theme.list = infos.sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name));
}

function info(f: ThemeFile, source: 'bundled' | 'user', file?: string): ThemeInfo {
  return { id: f.id, name: f.name, author: f.author, version: f.version, description: f.description, appearance: f.appearance, followsSystem: f.followsSystem ?? true, source, file };
}

/** The theme id the settings pick right now. */
export function chosenTheme(s: ThemeSettings = theme.settings): string {
  if (s.theme !== 'system') return s.theme;
  return theme.systemDark ? s.themeDark : s.themeLight;
}

let appliedVars: string[] = [];

function writeToDocument(out: ThemeOutput, reduceMotion: ReduceMotion) {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  for (const k of appliedVars) if (!(k in out.vars)) root.style.removeProperty(k);
  for (const [k, v] of Object.entries(out.vars)) root.style.setProperty(k, v);
  for (const [k, v] of Object.entries(out.attrs)) root.setAttribute(k, v);
  root.setAttribute('data-reduce-motion', reduceMotion);
  appliedVars = Object.keys(out.vars);
}

/** Resolve the chosen theme and write it to `<html>`; falls back to Dark if it is broken. */
export function applyTheme() {
  const id = chosenTheme();
  if (!userLoaded && !fileOf(id)) return; // a user theme: wait for loadUserThemes()
  let t: ResolvedTheme;
  try {
    t = resolve(id);
  } catch (e) {
    report(fileName(id), e);
    t = resolve('dark');
  }
  t = withDensity(t, theme.settings.density);
  const out = flatten(t);
  writeToDocument(out, theme.settings.reduceMotion);
  theme.active = t.id;
  theme.appearance = t.appearance;
  store(VARS_KEY, { ...out, reduceMotion: theme.settings.reduceMotion });
}

/** Paint the last applied theme before anything else runs (no flash of the wrong colours). */
export function applyCached() {
  const c = store<ThemeOutput & { reduceMotion: ReduceMotion }>(VARS_KEY);
  if (c?.vars && c.attrs) writeToDocument(c, c.reduceMotion ?? 'system');
}

/** Change appearance settings (and remember them on this computer). */
export function setThemeSettings(patch: Partial<ThemeSettings>) {
  theme.settings = { ...theme.settings, ...patch };
  store(SETTINGS_KEY, theme.settings);
  applyTheme();
}

/** Take the settings from `[app]` of a loaded config (they win over localStorage). */
export function adoptAppSettings(app: AppSettings | undefined) {
  if (!app) return;
  const patch: Partial<ThemeSettings> = {};
  if (app.theme) patch.theme = app.theme;
  if (app.theme_dark) patch.themeDark = app.theme_dark;
  if (app.theme_light) patch.themeLight = app.theme_light;
  if (app.density) patch.density = app.density;
  if (app.reduce_motion) patch.reduceMotion = app.reduce_motion;
  const next = { ...theme.settings, ...patch };
  if (JSON.stringify(next) !== JSON.stringify(theme.settings)) setThemeSettings(patch);
}

/** The `[app]` fields for the current settings, for saving to config.toml. */
export function appSettingsPatch(s: ThemeSettings = theme.settings): Pick<AppSettings, 'theme' | 'theme_dark' | 'theme_light' | 'density' | 'reduce_motion'> {
  return { theme: s.theme, theme_dark: s.themeDark, theme_light: s.themeLight, density: s.density, reduce_motion: s.reduceMotion };
}

const fileName = (id: string) => user.get(id)?.name ?? `${id}.json`;

function report(file: string, e: unknown) {
  const message = e instanceof Error ? e.message : String(e);
  if (!theme.problems.some((p) => p.file === file && p.message === message)) theme.problems = [...theme.problems, { file, message }];
}

/**
 * Check a theme file's text without installing it; returns the error message, or nothing when
 * the theme is valid (resolved against the current bundled and user themes).
 */
export function checkTheme(text: string, file = 'theme.json'): string | undefined {
  try {
    const { file: f } = validateFile(JSON.parse(text), file);
    resolveTheme(f, (id, from) => (id === f.id && from !== f.id ? f : lookup(id, from)));
    return undefined;
  } catch (e) {
    return e instanceof SyntaxError ? `${file}: not valid JSON (${e.message})` : e instanceof Error ? e.message : String(e);
  }
}

/** Replace the user themes (from `themes_list`); invalid files are reported, never applied. */
export function setUserThemes(files: UserThemeFile[]) {
  const next = new Map<string, { file: ThemeFile; name: string }>();
  const problems: ThemeProblem[] = [];
  for (const { file, json } of files) {
    try {
      const { file: f } = validateFile(JSON.parse(json), file);
      if (next.has(f.id)) throw new ThemeError(`theme “${f.id}” is also defined in ${next.get(f.id)!.name}`);
      next.set(f.id, { file: f, name: file });
    } catch (e) {
      problems.push({ file, message: e instanceof SyntaxError ? `not valid JSON (${e.message})` : e instanceof Error ? e.message : String(e) });
    }
  }
  user = next;
  userLoaded = true;
  resolved.clear();
  // Resolve each user theme now so broken inheritance or values are reported up front. Dropping
  // a theme can break one that inherits it (it resolved while its parent was still listed), so
  // repeat until a pass drops nothing.
  for (let dropped = true; dropped; ) {
    dropped = false;
    for (const [id, u] of [...user]) {
      try {
        resolve(id);
      } catch (e) {
        problems.push({ file: u.name, message: e instanceof Error ? e.message : String(e) });
        user.delete(id);
        resolved.clear();
        dropped = true;
      }
    }
  }
  theme.problems = problems;
  refreshList();
  applyTheme();
}

/** Load the user's themes from `<config dir>/themes/` (a no-op without the backend command). */
export async function loadUserThemes() {
  const files = await invoke<UserThemeFile[]>('themes_list').catch(() => undefined);
  setUserThemes(Array.isArray(files) ? files : []);
}

/** Inline `style` that scopes a theme to one element (theme cards, the component gallery). */
export function themeStyle(id: string, density?: Density): string {
  try {
    return inlineStyle(flatten(withDensity(resolve(id), density)));
  } catch {
    return '';
  }
}

/** Set up bundled themes, apply the chosen one and follow OS light/dark changes. */
export function initTheme() {
  refreshList();
  applyTheme();
  media?.addEventListener('change', (e) => {
    theme.systemDark = e.matches;
    if (theme.settings.theme === 'system') applyTheme();
  });
}

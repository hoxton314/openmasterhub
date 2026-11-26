// Theme engine: validates theme JSON, resolves inheritance and `{group.key}` references, applies
// the density overlay and flattens the tokens into CSS custom properties. Pure functions, no DOM,
// so it is unit-tested (engine.test.ts) and shared by the store and the gallery.
//
// Theme files are untrusted (users drop them into their config dir): every value is checked against
// a whitelist grammar for its kind, so a theme can't inject CSS or load remote resources.

export type Appearance = 'dark' | 'light';
export type Density = 'compact' | 'comfortable';

/** A theme file as written (tokens may be partial when it `inherits`). */
export interface ThemeFile {
  $schema?: string;
  schemaVersion: 1;
  id: string;
  name: string;
  author?: string;
  version?: string;
  description?: string;
  appearance: Appearance;
  /** Eligible for "follow system" (default true). Signal is opt-in. */
  followsSystem?: boolean;
  /** Which density the token values are written for (default compact). */
  density?: Density;
  inherits?: string;
  tokens: Tokens;
}

export type TypeRole = {
  family: 'ui' | 'heading' | 'mono';
  size: number;
  lineHeight: number;
  weight: number;
  case: 'none' | 'uppercase';
  tracking: number;
  color?: string;
};

/** Token groups; every value is a string or number after resolution. */
export interface Tokens {
  color?: Record<string, string>;
  font?: Record<string, string>;
  type?: Record<string, Partial<TypeRole>>;
  radius?: Record<string, number | string>;
  shadow?: Record<string, string>;
  focus?: { width?: number | string; offset?: number | string; color?: string };
  space?: Record<string, number | string>;
  size?: Record<string, number | string>;
  motion?: Record<string, string>;
  style?: Record<string, string | number>;
  avatar?: Record<string, string>;
  hub?: Record<string, string | number>;
}

export interface ResolvedTheme {
  id: string;
  name: string;
  author?: string;
  version?: string;
  description?: string;
  appearance: Appearance;
  followsSystem: boolean;
  density: Density;
  /** Fully merged and resolved tokens. */
  tokens: Required<Tokens>;
  warnings: string[];
}

/** What gets written to `<html>`: custom properties and data attributes. */
export interface ThemeOutput {
  vars: Record<string, string>;
  attrs: Record<string, string>;
}

export class ThemeError extends Error {}

// ───────────────────────────── token spec ─────────────────────────────

type Kind = 'color' | 'px' | 'shadow' | 'motion' | 'font' | 'colorOrShadowOrPx';

const COLORS = [
  'bg', 'surface', 'raised', 'sunken', 'border', 'border2', 'text1', 'text2', 'text3', 'accent', 'accentFill', 'onAccent',
  'focus', 'success', 'warning', 'danger', 'onDanger', 'selection', 'canvas', 'canvasDot', 'overlay', 'claim', 'markerBg',
  'markerFg', 'pressed', 'chip', 'rail', 'dialog', 'onAvatar',
] as const;
/** Optional colours derived from their base at 12% alpha unless a theme sets them. */
const SOFT = ['accent', 'success', 'warning', 'danger'] as const;
const TYPE_ROLES = ['display', 'title', 'heading', 'section', 'body', 'small', 'label', 'mono', 'tree', 'chip', 'micro'] as const;
const RADII = ['xs', 'sm', 'md', 'lg', 'xl', 'pill', 'control', 'segment', 'kbd', 'row', 'sidebarRow', 'chip', 'avatar', 'rail', 'card', 'panel', 'toast', 'popover', 'dialog', 'well'] as const;
const SHADOWS = ['flat', 'hairline', 'popover', 'dialog', 'segment', 'kbd'] as const;
const SPACES = ['1', '2', '3', '4', '5', '6', '7', '8'] as const;
const SIZES = [
  'rail', 'sidebar', 'sidebarCollapsed', 'inspector', 'inspectorNarrow', 'topbar', 'panelPad', 'control', 'controlSm',
  'segment', 'chip', 'rowSidebar', 'rowTree', 'avatar', 'toggleWidth', 'toggleHeight', 'kbd', 'iconButton', 'dialogWidth',
  'dialogBorder',
] as const;
const MOTIONS = ['fast', 'panel', 'toastIn', 'toastOut', 'keyFlash', 'appliedPulse'] as const;
const AVATARS = ['1', '2', '3', '4', '5', '6', '7'] as const;
const HUB_COLORS = [
  'base', 'baseStroke', 'chassis', 'chassisStroke', 'key', 'empty', 'socket', 'keyText', 'keyEdge', 'dialFace', 'dialCaption',
  'dialRing1', 'dialRing2', 'knobLight', 'knobDark', 'knobMark', 'rollerLight', 'rollerDark', 'caption', 'brand', 'logo',
] as const;
const HUB_SHADOWS = ['keyFrame', 'previewFrame', 'chassisShadow', 'baseShadow', 'dialBezel', 'rollerShadow'] as const;
const HUB_RADII = ['keyRadius', 'chassisRadius', 'baseRadius', 'rollerRadius'] as const;

/** Allowed values of the style switches; numbers are listed as 'number'. */
export const STYLE_SWITCHES: Record<string, readonly string[] | 'number'> = {
  headingCase: ['none', 'uppercase'],
  selectedRow: ['tint', 'invert'],
  panelBorderWidth: 'number',
  buttonWeight: 'number',
  secondaryButton: ['raised', 'surface', 'outline'],
  dangerButton: ['outline', 'fill'],
  segmentSelected: ['raised', 'fill'],
  toggleOff: ['fill', 'outline'],
  kbd: ['raised', 'inverse'],
  statusChip: ['tint', 'outline'],
  activeItem: ['tint', 'fill'],
  fieldBorder: ['hairline', 'strong'],
};

/** Every token key per group (the complete root theme must define all of them). */
export const TOKEN_KEYS = {
  color: COLORS,
  font: ['ui', 'heading', 'mono'],
  type: TYPE_ROLES,
  radius: RADII,
  shadow: SHADOWS,
  focus: ['width', 'offset', 'color'],
  space: SPACES,
  size: SIZES,
  motion: MOTIONS,
  style: Object.keys(STYLE_SWITCHES),
  avatar: AVATARS,
  hub: [...HUB_COLORS, ...HUB_SHADOWS, ...HUB_RADII],
} as const;
export type Group = keyof typeof TOKEN_KEYS;
const GROUPS = Object.keys(TOKEN_KEYS) as Group[];
const OPTIONAL: Partial<Record<Group, readonly string[]>> = { color: SOFT.map((c) => `${c}Soft`) };

function kindOf(group: Group, key: string): Kind | 'style' | 'type' {
  switch (group) {
    case 'color':
    case 'avatar':
      return 'color';
    case 'font':
      return 'font';
    case 'type':
      return 'type';
    case 'shadow':
      return 'shadow';
    case 'motion':
      return 'motion';
    case 'style':
      return 'style';
    case 'focus':
      return key === 'color' ? 'color' : 'px';
    case 'hub':
      return (HUB_SHADOWS as readonly string[]).includes(key) ? 'shadow' : (HUB_RADII as readonly string[]).includes(key) ? 'px' : 'color';
    default:
      return 'px';
  }
}

// ───────────────────────────── value grammar ─────────────────────────────

const FORBIDDEN: [RegExp, string][] = [
  [/url\s*\(/i, '“url(” is not allowed'],
  [/var\s*\(/i, '“var(” is not allowed'],
  [/expression/i, '“expression” is not allowed'],
  [/image(-set)?\s*\(/i, 'images are not allowed'],
  [/@/, '“@” is not allowed'],
  [/;/, '“;” is not allowed'],
  [/\\/, 'backslashes are not allowed'],
  [/[<>]/, '“<” and “>” are not allowed'],
  [/[{}]/, 'braces are only allowed in {group.key} references'],
];

const NUM = String.raw`[+-]?(?:\d+\.?\d*|\.\d+)`;
const HEX = /^#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const FUNC_COLOR = new RegExp(String.raw`^(rgba?|hsla?)\(\s*${NUM}(?:deg|%)?(?:\s*[, ]\s*${NUM}%?){2}(?:\s*[,/]\s*${NUM}%?)?\s*\)$`, 'i');
const LENGTH = new RegExp(String.raw`^${NUM}(?:px)?$`);
const DURATION = new RegExp(String.raw`^${NUM}m?s$`);
const EASING = new RegExp(String.raw`^(?:linear|ease|ease-in|ease-out|ease-in-out|cubic-bezier\(\s*${NUM}(?:\s*,\s*${NUM}){3}\s*\))$`);
const FONT_NAME = /^(?:'[A-Za-z0-9 _-]+'|"[A-Za-z0-9 _-]+"|[A-Za-z][A-Za-z0-9 _-]*)$/;

export function isColor(v: string): boolean {
  const s = v.trim();
  return HEX.test(s) || FUNC_COLOR.test(s) || s === 'transparent';
}

/** Split on separators outside parentheses. */
function splitTop(v: string, sep: RegExp): string[] {
  const out: string[] = [];
  let depth = 0;
  let cur = '';
  for (const ch of v) {
    if (ch === '(') depth++;
    if (ch === ')') depth--;
    if (depth === 0 && sep.test(ch)) {
      out.push(cur);
      cur = '';
    } else cur += ch;
  }
  out.push(cur);
  return out.map((s) => s.trim()).filter((s) => s !== '');
}

function isShadow(v: string): boolean {
  if (v.trim() === 'none') return true;
  return splitTop(v, /,/).every((layer) => {
    const parts = splitTop(layer, /\s/);
    let colors = 0;
    let lengths = 0;
    for (const p of parts) {
      if (p === 'inset') continue;
      if (LENGTH.test(p)) lengths++;
      else if (isColor(p)) colors++;
      else return false;
    }
    return lengths >= 2 && lengths <= 4 && colors <= 1;
  });
}

function isMotion(v: string): boolean {
  const parts = splitTop(v, /\s/);
  return parts.length >= 1 && parts.length <= 3 && DURATION.test(parts[0]) && parts.slice(1).every((p) => DURATION.test(p) || EASING.test(p));
}

function isFont(v: string): boolean {
  const names = v.split(',').map((s) => s.trim());
  return names.length > 0 && names.every((n) => FONT_NAME.test(n));
}

function checkForbidden(v: string, where: string) {
  const withoutRefs = v.replace(/\{[a-zA-Z0-9.]+\}/g, '');
  for (const [re, msg] of FORBIDDEN) if (re.test(withoutRefs)) throw new ThemeError(`${where}: ${msg}`);
}

/** Validate one resolved value of a kind; throws a readable error. */
function checkValue(kind: Kind, v: unknown, where: string): string | number {
  if (typeof v === 'number') {
    if (!Number.isFinite(v)) throw new ThemeError(`${where}: not a finite number`);
    if (kind === 'px' || kind === 'colorOrShadowOrPx') return v;
    throw new ThemeError(`${where}: expected ${kind === 'color' ? 'a colour' : `a ${kind}`}, got the number ${v}`);
  }
  if (typeof v !== 'string') throw new ThemeError(`${where}: expected a string or number`);
  checkForbidden(v, where);
  const ok =
    kind === 'color' ? isColor(v)
    : kind === 'shadow' ? isShadow(v)
    : kind === 'motion' ? isMotion(v)
    : kind === 'font' ? isFont(v)
    : kind === 'px' ? LENGTH.test(v.trim())
    : isColor(v) || isShadow(v) || LENGTH.test(v.trim());
  if (!ok) {
    const want = {
      color: 'a colour (#rgb, #rrggbb, #rrggbbaa, rgb(), rgba(), hsl(), hsla())',
      shadow: 'a box-shadow (lengths, optional inset, one colour per layer) or none',
      motion: 'a duration and easing, e.g. “180ms ease-out”',
      font: 'a comma-separated list of font family names',
      px: 'a number of pixels',
      colorOrShadowOrPx: 'a colour, a box-shadow or a number',
    }[kind];
    throw new ThemeError(`${where}: “${v}” is not ${want}`);
  }
  return kind === 'px' ? parseFloat(v) : v.trim();
}

// ───────────────────────────── file validation ─────────────────────────────

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const ID = /^[a-z0-9-]{2,32}$/;
const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const META = ['$schema', 'schemaVersion', 'id', 'name', 'author', 'version', 'description', 'appearance', 'followsSystem', 'density', 'inherits', 'tokens'];

/** Check the shape of a theme file (not yet the resolved values). Returns warnings. */
export function validateFile(json: unknown, label = 'theme'): { file: ThemeFile; warnings: string[] } {
  const warnings: string[] = [];
  if (!isObj(json)) throw new ThemeError(`${label}: not a JSON object`);
  const f = structuredClone(json);
  if (f.schemaVersion !== 1) throw new ThemeError(`${label}: schemaVersion must be 1`);
  if (typeof f.id !== 'string' || !ID.test(f.id)) throw new ThemeError(`${label}: id must be 2–32 characters of a–z, 0–9 and “-”`);
  const where = `theme “${f.id}”`;
  if (typeof f.name !== 'string' || !f.name.trim() || f.name.length > 60) throw new ThemeError(`${where}: name must be a non-empty string (60 characters at most)`);
  if (f.appearance !== 'dark' && f.appearance !== 'light') throw new ThemeError(`${where}: appearance must be “dark” or “light”`);
  for (const k of ['author', 'description'] as const) {
    if (f[k] !== undefined && (typeof f[k] !== 'string' || (f[k] as string).length > 300)) throw new ThemeError(`${where}: ${k} must be a string`);
  }
  if (f.version !== undefined && (typeof f.version !== 'string' || !SEMVER.test(f.version))) throw new ThemeError(`${where}: version must look like 1.0.0`);
  if (f.followsSystem !== undefined && typeof f.followsSystem !== 'boolean') throw new ThemeError(`${where}: followsSystem must be true or false`);
  if (f.density !== undefined && f.density !== 'compact' && f.density !== 'comfortable') throw new ThemeError(`${where}: density must be “compact” or “comfortable”`);
  if (f.inherits !== undefined && (typeof f.inherits !== 'string' || !ID.test(f.inherits))) throw new ThemeError(`${where}: inherits must be a theme id`);
  for (const k of Object.keys(f)) if (!META.includes(k)) warnings.push(`${where}: unknown field “${k}” ignored`);
  if (!isObj(f.tokens)) throw new ThemeError(`${where}: tokens must be an object`);

  for (const [group, values] of Object.entries(f.tokens)) {
    if (!(GROUPS as string[]).includes(group)) throw new ThemeError(`${where}: unknown token group “${group}”`);
    if (!isObj(values)) throw new ThemeError(`${where}: tokens.${group} must be an object`);
    const g = group as Group;
    const known = [...TOKEN_KEYS[g], ...(OPTIONAL[g] ?? [])] as string[];
    for (const [key, v] of Object.entries(values)) {
      const at = `${where}: ${group}.${key}`;
      if (!known.includes(key)) {
        warnings.push(`${at} is not a known token and is ignored`);
        delete values[key];
        continue;
      }
      if (g === 'type') {
        if (!isObj(v)) throw new ThemeError(`${at} must be an object`);
        checkTypeRole(v, at);
      } else if (typeof v !== 'string' && typeof v !== 'number') {
        throw new ThemeError(`${at} must be a string or a number`);
      } else if (typeof v === 'string') {
        checkForbidden(v, at);
      }
    }
  }
  return { file: f as unknown as ThemeFile, warnings };
}

const ROLE_KEYS = ['family', 'size', 'lineHeight', 'weight', 'case', 'tracking', 'color'];

function checkTypeRole(r: Record<string, unknown>, at: string) {
  for (const [k, v] of Object.entries(r)) {
    if (!ROLE_KEYS.includes(k)) throw new ThemeError(`${at}.${k} is not a type property (${ROLE_KEYS.join(', ')})`);
    if (k === 'family' && !['ui', 'heading', 'mono'].includes(v as string)) throw new ThemeError(`${at}.family must be ui, heading or mono`);
    if (k === 'case' && v !== 'none' && v !== 'uppercase') throw new ThemeError(`${at}.case must be none or uppercase`);
    if (k === 'color' && !(COLORS as readonly string[]).includes(v as string)) throw new ThemeError(`${at}.color must name a colour token, e.g. “text3”`);
    if (['size', 'lineHeight', 'weight', 'tracking'].includes(k) && (typeof v !== 'number' || !Number.isFinite(v))) throw new ThemeError(`${at}.${k} must be a number`);
    if (k === 'size' && ((v as number) < 6 || (v as number) > 96)) throw new ThemeError(`${at}.size must be between 6 and 96`);
    if (k === 'weight' && ((v as number) < 100 || (v as number) > 1000)) throw new ThemeError(`${at}.weight must be between 100 and 1000`);
  }
}

// ───────────────────────────── resolution ─────────────────────────────

function deepMerge<T>(base: T, over: unknown): T {
  if (!isObj(base) || !isObj(over)) return (over === undefined ? base : over) as T;
  const out: Record<string, unknown> = { ...base };
  for (const [k, v] of Object.entries(over)) out[k] = k in out ? deepMerge(out[k], v) : structuredClone(v);
  return out as T;
}

/** Looks up a theme file by id; `from` is the id asking (a theme may inherit a bundled one of its own id). */
export type ThemeLookup = (id: string, from: string) => ThemeFile | undefined;

/** Merge a theme with its ancestors (child over parent), max depth 4, no cycles. */
export function mergeChain(file: ThemeFile, lookup: ThemeLookup): { tokens: Tokens; chain: string[] } {
  const chain: ThemeFile[] = [file];
  let cur = file;
  while (cur.inherits) {
    const parent = lookup(cur.inherits, cur.id);
    if (!parent) throw new ThemeError(`theme “${cur.id}” inherits “${cur.inherits}”, which does not exist`);
    if (chain.includes(parent)) throw new ThemeError(`theme “${file.id}”: inheritance cycle (${[...chain.map((c) => c.id), parent.id].join(' → ')})`);
    chain.push(parent);
    if (chain.length > 5) throw new ThemeError(`theme “${file.id}”: inheritance is deeper than 4 levels`);
    cur = parent;
  }
  let tokens: Tokens = {};
  for (const t of [...chain].reverse()) tokens = deepMerge(tokens, t.tokens);
  return { tokens, chain: chain.map((c) => c.id) };
}

const REF = /\{([a-zA-Z0-9]+(?:\.[a-zA-Z0-9]+)+)\}/g;

function getPath(tokens: Tokens, path: string): unknown {
  return path.split('.').reduce<unknown>((o, k) => (isObj(o) ? o[k] : undefined), tokens);
}

/** Resolve `{group.key}` references in every string value (recursively, cycle-safe). */
function resolveRefs(tokens: Tokens, themeId: string): Tokens {
  const out = structuredClone(tokens) as Record<string, Record<string, unknown>>;
  const resolving = new Set<string>();

  const resolve = (path: string): unknown => {
    const v = getPath(out as Tokens, path);
    if (typeof v !== 'string' || !v.includes('{')) return v;
    if (resolving.has(path)) throw new ThemeError(`theme “${themeId}”: reference cycle at ${path}`);
    resolving.add(path);
    let result: unknown;
    const whole = v.match(/^\{([a-zA-Z0-9.]+)\}$/);
    if (whole) {
      result = resolve(whole[1]);
      if (result === undefined) throw new ThemeError(`theme “${themeId}”: ${path} refers to {${whole[1]}}, which does not exist`);
    } else {
      result = v.replace(REF, (_, ref: string) => {
        const r = resolve(ref);
        if (r === undefined || isObj(r)) throw new ThemeError(`theme “${themeId}”: ${path} refers to {${ref}}, which does not exist`);
        return String(r);
      });
    }
    resolving.delete(path);
    const [g, ...rest] = path.split('.');
    let target = out[g] as Record<string, unknown>;
    for (const k of rest.slice(0, -1)) target = target[k] as Record<string, unknown>;
    target[rest[rest.length - 1]] = result;
    return result;
  };

  for (const [g, values] of Object.entries(out)) {
    for (const [k, v] of Object.entries(values ?? {})) {
      if (isObj(v)) for (const sub of Object.keys(v)) resolve(`${g}.${k}.${sub}`);
      else resolve(`${g}.${k}`);
    }
  }
  return out as Tokens;
}

// ───────────────────────────── colours ─────────────────────────────

/** Parse a colour into RGBA (0–255, alpha 0–1); undefined for `transparent`-like values. */
export function parseColor(v: string): [number, number, number, number] | undefined {
  const s = v.trim();
  if (HEX.test(s)) {
    let h = s.slice(1);
    if (h.length <= 4) h = [...h].map((c) => c + c).join('');
    const n = (i: number) => parseInt(h.slice(i, i + 2), 16);
    return [n(0), n(2), n(4), h.length === 8 ? n(6) / 255 : 1];
  }
  const m = s.match(/^(rgba?|hsla?)\((.*)\)$/i);
  if (!m) return undefined;
  const parts = m[2].split(/[\s,/]+/).filter(Boolean);
  const num = (p: string, scale: number) => (p.endsWith('%') ? (parseFloat(p) / 100) * scale : parseFloat(p));
  const alpha = parts[3] !== undefined ? num(parts[3], 1) : 1;
  if (m[1].toLowerCase().startsWith('rgb')) return [num(parts[0], 255), num(parts[1], 255), num(parts[2], 255), alpha];
  const h = (((parseFloat(parts[0]) % 360) + 360) % 360) / 360;
  const sat = num(parts[1], 1);
  const l = num(parts[2], 1);
  const q = l < 0.5 ? l * (1 + sat) : l + sat - l * sat;
  const p = 2 * l - q;
  const hue = (t: number) => {
    t = (t + 1) % 1;
    return t < 1 / 6 ? p + (q - p) * 6 * t : t < 1 / 2 ? q : t < 2 / 3 ? p + (q - p) * (2 / 3 - t) * 6 : p;
  };
  return [hue(h + 1 / 3) * 255, hue(h) * 255, hue(h - 1 / 3) * 255, alpha];
}

/** The colour at `alpha` × its own alpha, as rgba(). */
export function withAlpha(v: string, alpha: number): string {
  const c = parseColor(v);
  if (!c) return 'transparent';
  const r = (x: number) => Math.round(Math.max(0, Math.min(255, x)));
  return `rgba(${r(c[0])},${r(c[1])},${r(c[2])},${+(c[3] * alpha).toFixed(3)})`;
}

// ───────────────────────────── resolve + flatten ─────────────────────────────

/** Density overlay: comfortable = body/small +1 px, controls +4 px, rows +2 px. */
const DENSITY_DELTA: { group: 'type' | 'size'; key: string; delta: number }[] = [
  { group: 'type', key: 'body', delta: 1 },
  { group: 'type', key: 'small', delta: 1 },
  { group: 'size', key: 'control', delta: 4 },
  { group: 'size', key: 'controlSm', delta: 4 },
  { group: 'size', key: 'rowSidebar', delta: 2 },
  { group: 'size', key: 'rowTree', delta: 2 },
];

/** Validate, merge and resolve a theme into complete tokens. Throws ThemeError on any problem. */
export function resolveTheme(file: ThemeFile, lookup: ThemeLookup): ResolvedTheme {
  const warnings: string[] = [];
  const { tokens: merged } = mergeChain(file, lookup);
  const tokens = resolveRefs(merged, file.id) as Required<Tokens>;
  const where = `theme “${file.id}”`;

  for (const g of GROUPS) {
    const values = (tokens[g] ?? {}) as Record<string, unknown>;
    tokens[g] = values as never;
    const missing = (TOKEN_KEYS[g] as readonly string[]).filter((k) => values[k] === undefined);
    if (missing.length) throw new ThemeError(`${where}: missing ${missing.map((k) => `${g}.${k}`).join(', ')}${file.inherits ? '' : ' (add "inherits": "dark" to start from the Dark theme)'}`);
    for (const [k, v] of Object.entries(values)) {
      const at = `${where}: ${g}.${k}`;
      const kind = kindOf(g, k);
      if (kind === 'type') {
        const role = v as Partial<TypeRole>;
        const needed = ['family', 'size', 'lineHeight', 'weight', 'case', 'tracking'].filter((x) => role[x as keyof TypeRole] === undefined);
        if (needed.length) throw new ThemeError(`${at} is missing ${needed.join(', ')}`);
        checkTypeRole(role as Record<string, unknown>, at);
      } else if (kind === 'style') {
        const allowed = STYLE_SWITCHES[k];
        if (allowed === 'number' ? typeof v !== 'number' || v < 0 || v > 1000 : !allowed.includes(v as string)) {
          throw new ThemeError(`${at} must be ${allowed === 'number' ? 'a number' : allowed.map((a) => `“${a}”`).join(' or ')}`);
        }
      } else {
        values[k] = checkValue(kind, v, at);
      }
    }
  }
  for (const c of SOFT) tokens.color[`${c}Soft`] ??= withAlpha(tokens.color[c], 0.12);
  return {
    id: file.id,
    name: file.name,
    author: file.author,
    version: file.version,
    description: file.description,
    appearance: file.appearance,
    followsSystem: file.followsSystem ?? true,
    density: file.density ?? 'compact',
    tokens,
    warnings,
  };
}

/** Apply a density on top of a theme written for another density. */
export function withDensity(t: ResolvedTheme, density: Density | undefined): ResolvedTheme {
  if (!density || density === t.density) return t;
  const sign = density === 'comfortable' ? 1 : -1;
  const tokens = structuredClone(t.tokens);
  for (const d of DENSITY_DELTA) {
    if (d.group === 'type') tokens.type[d.key].size = (tokens.type[d.key].size ?? 13) + sign * d.delta;
    else tokens.size[d.key] = Number(tokens.size[d.key]) + sign * d.delta;
  }
  return { ...t, tokens, density };
}

/** `accentFill` → `accent-fill`, `text1` → `text-1`. */
export const kebab = (s: string) =>
  s
    .replace(/([a-z])([A-Z])/g, '$1-$2')
    .replace(/([a-zA-Z])(\d)/g, '$1-$2')
    .toLowerCase();

const px = (v: number | string) => `${v}px`;

/** Flatten resolved tokens into CSS variables and `<html>` data attributes. */
export function flatten(t: ResolvedTheme): ThemeOutput {
  const k = t.tokens;
  const vars: Record<string, string> = {};
  const set = (name: string, v: string | number) => (vars[`--${name}`] = String(v));

  for (const [n, v] of Object.entries(k.color)) set(`color-${kebab(n)}`, v);
  for (const [n, v] of Object.entries(k.font)) set(`font-${kebab(n)}`, v);
  for (const [n, r] of Object.entries(k.type) as [string, TypeRole][]) {
    const p = `type-${kebab(n)}`;
    set(`${p}-size`, px(r.size));
    set(`${p}-lh`, r.lineHeight);
    set(`${p}-weight`, r.weight);
    set(`${p}-family`, k.font[r.family]);
    set(`${p}-case`, r.case);
    set(`${p}-tracking`, `${r.tracking}em`);
    if (r.color) set(`${p}-color`, k.color[r.color]);
  }
  for (const [n, v] of Object.entries(k.radius)) set(`radius-${kebab(n)}`, px(v));
  for (const [n, v] of Object.entries(k.shadow)) set(`shadow-${kebab(n)}`, v);
  set('focus-width', px(k.focus.width!));
  set('focus-offset', px(k.focus.offset!));
  set('focus-color', k.focus.color!);
  for (const [n, v] of Object.entries(k.space)) set(`space-${n}`, px(v));
  for (const [n, v] of Object.entries(k.size)) set(`size-${kebab(n)}`, px(v));
  for (const [n, v] of Object.entries(k.motion)) set(`motion-${kebab(n)}`, v);
  for (const [n, v] of Object.entries(k.avatar)) set(`avatar-${n}`, v);
  for (const [n, v] of Object.entries(k.hub)) set(`hub-${kebab(n)}`, typeof v === 'number' ? px(v) : v);

  // Style switches become component variables, so they also work inside a scoped preview.
  const s = k.style as Record<string, string | number>;
  const c = k.color;
  set('panel-border-width', px(s.panelBorderWidth));
  set('button-weight', s.buttonWeight);
  set('heading-case', s.headingCase);
  const strong = s.fieldBorder === 'strong';
  set('field-border', strong ? `2px solid ${c.border2}` : `1px solid ${c.border}`);
  set('panel-border', `${px(s.panelBorderWidth)} solid ${c.border}`);
  const inv = s.selectedRow === 'invert';
  set('row-selected-bg', inv ? c.text1 : c.selection);
  set('row-selected-fg', inv ? c.bg : c.text1);
  set('row-selected-meta', inv ? c.bg : c.text3);
  set('row-selected-icon', inv ? c.bg : c.accent);
  set('row-selected-live', inv ? c.bg : c.success);
  const sec = s.secondaryButton;
  set('button-secondary-bg', sec === 'outline' ? 'transparent' : sec === 'surface' ? c.surface : c.raised);
  set('button-secondary-border', sec === 'outline' ? `2px solid ${c.border2}` : `1px solid ${c.border2}`);
  const dangerFill = s.dangerButton === 'fill';
  set('button-danger-bg', dangerFill ? c.danger : 'transparent');
  set('button-danger-fg', dangerFill ? c.onDanger : c.danger);
  set('button-danger-border', dangerFill ? '1px solid transparent' : `1px solid ${c.danger}`);
  const segFill = s.segmentSelected === 'fill';
  set('segment-selected-bg', segFill ? c.accentFill : c.raised);
  set('segment-selected-fg', segFill ? c.onAccent : c.text1);
  set('segment-selected-shadow', segFill ? 'none' : k.shadow.segment);
  set('segment-selected-weight', segFill ? 700 : 600);
  set('segment-idle-fg', segFill ? c.text1 : c.text2);
  set('segment-idle-weight', segFill ? 700 : 500);
  const toggleOutline = s.toggleOff === 'outline';
  set('toggle-off-bg', toggleOutline ? 'transparent' : c.border2);
  set('toggle-off-knob', toggleOutline ? c.border2 : c.onAccent);
  set('toggle-on-knob', c.onAccent);
  const kbdInv = s.kbd === 'inverse';
  set('kbd-bg', kbdInv ? c.text1 : c.raised);
  set('kbd-fg', kbdInv ? c.raised : c.text1);
  set('kbd-border', kbdInv ? c.text1 : c.border2);
  const outline = s.statusChip === 'outline';
  for (const st of ['success', 'warning', 'danger'] as const) {
    set(`chip-${st}-bg`, outline ? 'transparent' : c[`${st}Soft`]);
    set(`chip-${st}-fg`, outline ? c.text1 : c[st]);
    set(`chip-${st}-border`, outline ? `2px solid ${c[st]}` : '1px solid transparent');
  }
  set('chip-accent-bg', outline ? c.accentFill : c.accentSoft);
  set('chip-accent-fg', outline ? c.onAccent : c.accent);
  set('chip-neutral-bg', c.chip);
  set('chip-outline-border', outline ? `2px solid ${c.border2}` : `1px solid ${c.border2}`);
  const fill = s.activeItem === 'fill';
  set('active-item-bg', fill ? c.accentFill : c.selection);
  set('active-item-fg', fill ? c.onAccent : c.accent);
  set('idle-item-fg', fill ? c.text1 : c.text2);
  set('live-color', c.success);
  set('live-halo', c.successSoft);

  const attrs: Record<string, string> = { 'data-theme': t.id, 'data-appearance': t.appearance, 'data-density': t.density };
  for (const [n, v] of Object.entries(s)) if (typeof v === 'string') attrs[`data-${kebab(n)}`] = v;
  vars['color-scheme'] = t.appearance;
  return { vars, attrs };
}

/** `--a: b; --c: d` for scoping a theme to one element (previews, the gallery). */
export function inlineStyle(out: ThemeOutput): string {
  return Object.entries(out.vars)
    .map(([k, v]) => `${k}: ${v}`)
    .join('; ');
}

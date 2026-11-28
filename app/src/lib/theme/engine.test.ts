import { describe, expect, it } from 'vitest';
import dark from '../../themes/dark.json';
import light from '../../themes/light.json';
import signal from '../../themes/signal.json';
import schema from '../../themes/theme.schema.json';
import { flatten, kebab, resolveTheme, ThemeError, TOKEN_KEYS, validateFile, withAlpha, withDensity, type ThemeFile } from './engine';

const files = Object.fromEntries([dark, light, signal].map((j) => [j.id, validateFile(j).file])) as Record<string, ThemeFile>;
const lookup = (id: string) => files[id];
const resolve = (id: string) => resolveTheme(files[id], lookup);
const vars = (id: string) => flatten(resolve(id)).vars;

const userTheme = (tokens: object, extra: object = {}) => ({ schemaVersion: 1, id: 'mine', name: 'Mine', appearance: 'dark', inherits: 'dark', tokens, ...extra });
const resolveUser = (json: unknown) => resolveTheme(validateFile(json).file, lookup);

describe('bundled themes', () => {
  it('resolve with the values of the token tables', () => {
    expect(vars('dark')).toMatchObject({ '--color-bg': '#0E0F12', '--color-text-1': '#ECEDF0', '--color-accent-fill': '#5E63EB', '--radius-control': '7px', '--size-control': '32px', '--type-body-size': '13px' });
    expect(vars('light')).toMatchObject({ '--color-bg': '#F3F4F6', '--color-success-soft': '#E3F4EC', '--radius-dialog': '14px', '--hub-key': '#24272D' });
    expect(vars('signal')).toMatchObject({
      '--color-bg': '#1F2A8A',
      '--radius-dialog': '0px',
      '--size-control': '36px',
      '--type-heading-case': 'uppercase',
      '--type-heading-family': "Archivo, 'Inter Variable', Inter, sans-serif",
      '--row-selected-bg': '#FFFFFF',
      '--row-selected-fg': '#1F2A8A',
      '--focus-width': '3px',
    });
  });
  it('keep the hub identical in every theme', () => {
    const hub = (id: string) => Object.fromEntries(Object.entries(vars(id)).filter(([k]) => k.startsWith('--hub-')));
    expect(hub('light')).toEqual(hub('dark'));
    expect(hub('signal')).toEqual(hub('dark'));
  });
  it('resolve references and derive soft colours', () => {
    const v = vars('dark');
    expect(v['--shadow-popover']).toBe('0 12px 32px rgba(0,0,0,0.50), 0 0 0 1px #2A2D34');
    expect(v['--shadow-dialog']).toBe(v['--shadow-popover']);
    expect(v['--color-success-soft']).toBe('rgba(61,214,140,0.12)');
    expect(v['--hub-dial-bezel']).toBe('0 0 0 14px #1E2024, 0 0 0 16px #3A3D44');
  });
  it('set style switches as data attributes', () => {
    expect(flatten(resolve('signal')).attrs).toMatchObject({ 'data-selected-row': 'invert', 'data-heading-case': 'uppercase', 'data-appearance': 'dark' });
  });
  it('apply the density overlay relative to the theme', () => {
    expect(flatten(withDensity(resolve('dark'), 'comfortable')).vars['--size-control']).toBe('36px');
    expect(flatten(withDensity(resolve('signal'), 'compact')).vars['--type-body-size']).toBe('13px');
    expect(withDensity(resolve('dark'), 'compact').tokens.size.control).toBe(32);
  });
});

describe('user themes', () => {
  it('inherit dark and override two colours', () => {
    const t = resolveUser(userTheme({ color: { accent: '#FF00AA', accentFill: 'hsl(320, 100%, 40%)' } }));
    const v = flatten(t).vars;
    expect(v['--color-accent']).toBe('#FF00AA');
    expect(v['--color-bg']).toBe('#0E0F12');
    expect(v['--color-accent-soft']).toBe('rgba(255,0,170,0.12)');
  });
  it('reject injection attempts with a readable error', () => {
    expect(() => resolveUser(userTheme({ color: { bg: "url('https://evil.example/x.png')" } }))).toThrow(/url\(/);
    expect(() => resolveUser(userTheme({ color: { bg: 'red; background: blue' } }))).toThrow(ThemeError);
    expect(() => resolveUser(userTheme({ shadow: { popover: '0 0 0 1px var(--x)' } }))).toThrow(/var\(/);
    expect(() => resolveUser(userTheme({ font: { ui: 'Inter, "x\\"' } }))).toThrow(ThemeError);
    expect(() => resolveUser(userTheme({ color: { bg: 'u\\72l(x)' } }))).toThrow(/backslash/);
    expect(() => resolveUser(userTheme({ motion: { fast: '120ms steps(2)' } }))).toThrow(/duration/);
  });
  it('report missing tokens, bad refs and cycles', () => {
    expect(() => resolveUser(userTheme({ color: { bg: '#000' } }, { inherits: undefined }))).toThrow(/missing color\.surface/);
    expect(() => resolveUser(userTheme({ color: { bg: '{color.nope}' } }))).toThrow(/does not exist/);
    expect(() => resolveUser(userTheme({ color: { bg: '{color.surface}', surface: '{color.bg}' } }))).toThrow(/cycle/);
    expect(() => resolveUser(userTheme({}, { inherits: 'nope' }))).toThrow(/does not exist/);
    expect(() => validateFile(userTheme({}, { id: 'Bad Id' }))).toThrow(/id must be/);
  });
  it('warn about unknown tokens without failing', () => {
    const { warnings } = validateFile(userTheme({ color: { acent: '#fff' } }));
    expect(warnings[0]).toMatch(/color\.acent is not a known token/);
  });
});

describe('helpers', () => {
  it('kebab-cases token names', () => {
    expect(kebab('accentFill')).toBe('accent-fill');
    expect(kebab('text1')).toBe('text-1');
    expect(kebab('dialRing2')).toBe('dial-ring-2');
  });
  it('applies alpha to every colour syntax', () => {
    expect(withAlpha('#fff', 0.5)).toBe('rgba(255,255,255,0.5)');
    expect(withAlpha('rgba(255,255,255,0.30)', 0.5)).toBe('rgba(255,255,255,0.15)');
    expect(withAlpha('hsl(0, 100%, 50%)', 1)).toBe('rgba(255,0,0,1)');
  });
});

describe('theme.schema.json', () => {
  const groups = schema.properties.tokens.properties as Record<string, { properties: Record<string, unknown> }>;
  it('lists every token the engine knows', () => {
    for (const [g, keys] of Object.entries(TOKEN_KEYS)) for (const k of keys) expect(groups[g]?.properties, `${g}.${k}`).toHaveProperty(k);
  });
  it('accepts the bundled themes', () => {
    const colour = new RegExp(schema.$defs.color.pattern);
    for (const t of [dark, light, signal]) {
      for (const k of Object.keys(t)) expect(schema.properties, k).toHaveProperty(k);
      for (const [g, values] of Object.entries(t.tokens)) {
        for (const [k, v] of Object.entries(values as object)) {
          expect(groups[g].properties, `${t.id}: ${g}.${k}`).toHaveProperty(k);
          if (g === 'color' || g === 'avatar') expect(v, `${t.id}: ${g}.${k}`).toMatch(colour);
        }
      }
    }
  });
});

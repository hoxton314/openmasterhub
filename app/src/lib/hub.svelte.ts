// App state: the config being edited, live hub state from the service, and the selection.
// Edits go through `edit()`, which saves to the service shortly after the last change.

import { invoke, listen } from './backend';
import type { Action, Config, Display, HubState, InputMsg, KeyConfig, ModuleState, Page, Profile, Selection, Snapshot, Side } from './types';

export const hub = $state({
  online: false,
  loaded: false,
  config: { backlight: 80 } as Config,
  configPath: '',
  modules: [] as ModuleState[],
  state: { profile: '', page: '', in_folder: false, horizontal: true, rotation: 90 } as HubState,
  displays: {} as Record<string, string>,
  selection: { kind: 'none' } as Selection,
  /** `serial/control/index` → time of the last input, for the glow. */
  flashes: {} as Record<string, number>,
  faders: {} as Record<string, number>,
  toast: '' as string,
});

export const displayKey = (serial: string, target: number) => `${serial}/${target}`;
export const flashKey = (serial: string, control: string, index: number) => `${serial}/${control}/${index}`;

export async function reload() {
  const s = await invoke<Snapshot>('snapshot');
  hub.online = s.online;
  if (!s.online) return;
  if (s.config) hub.config = s.config;
  hub.configPath = s.config_path ?? '';
  hub.modules = s.modules;
  if (s.state) hub.state = s.state;
  hub.displays = Object.fromEntries(s.displays.map((d) => [displayKey(d.serial, d.target), d.src]));
  hub.loaded = true;
}

export async function connect() {
  await listen<boolean>('hub:online', (e) => {
    if (e.payload) setTimeout(reload, 600);
    else hub.online = false;
  });
  await listen<InputMsg>('hub:input', (e) => {
    const i = e.payload;
    hub.flashes[flashKey(i.serial, i.control, i.index)] = performance.now();
    if (i.value !== undefined && i.value !== null) hub.faders[`${i.serial}/${i.index}`] = i.value;
  });
  await listen<HubState>('hub:state', (e) => {
    hub.state = e.payload;
  });
  await listen<ModuleState[]>('hub:modules', (e) => {
    hub.modules = e.payload;
  });
  await listen<Display>('hub:display', (e) => {
    hub.displays[displayKey(e.payload.serial, e.payload.target)] = e.payload.src;
  });
  await listen('hub:config-changed', () => {
    if (!saveTimer) reload();
  });
  await reload();
}

export function toast(text: string) {
  hub.toast = text;
  setTimeout(() => {
    if (hub.toast === text) hub.toast = '';
  }, 5000);
}

// ───────────────────────────── editing ─────────────────────────────

let saveTimer: ReturnType<typeof setTimeout> | undefined;

/** Apply an edit to the config and save soon (debounced). */
export function edit(f: (c: Config) => void) {
  f(hub.config);
  tidy(hub.config);
  clearTimeout(saveTimer);
  saveTimer = setTimeout(save, 350);
}

export async function save() {
  clearTimeout(saveTimer);
  saveTimer = undefined;
  try {
    await invoke('save_config', { config: $state.snapshot(hub.config) });
  } catch (e) {
    toast(`Not saved: ${e}`);
  }
}

const isEmpty = (o: object) => Object.values(o).every((v) => v === undefined || v === null || v === '');

/** Drop empty entries so the saved TOML stays tidy. */
function tidy(c: Config) {
  for (const p of c.profile ?? []) {
    for (const page of [...(p.page ?? []), ...(p.folder ?? [])]) {
      page.key = (page.key ?? []).filter((k) => !isEmpty({ ...k, index: undefined }));
    }
    p.knob = (p.knob ?? []).filter((k) => !isEmpty({ ...k, index: undefined }));
    p.fader = (p.fader ?? []).filter((f) => f.change);
    p.roller = (p.roller ?? []).filter((r) => r.away || r.toward);
    if (p.dial && isEmpty(p.dial)) delete p.dial;
  }
}

// ───────────────────────────── lookups ─────────────────────────────

export function currentProfile(): Profile | undefined {
  const ps = hub.config.profile ?? [];
  return ps.find((p) => p.name === hub.state.profile) ?? ps[0];
}

/** The profile being edited, created if the config has none. */
export function profileMut(c: Config): Profile {
  c.profile ??= [];
  if (c.profile.length === 0) c.profile.push({ name: 'Default', page: [{ name: 'Main' }] });
  return c.profile.find((p) => p.name === hub.state.profile) ?? c.profile[0];
}

export function currentPage(): Page | undefined {
  const p = currentProfile();
  if (!p) return undefined;
  return [...(p.page ?? []), ...(p.folder ?? [])].find((x) => x.name === hub.state.page) ?? p.page?.[0];
}

export function pageMut(c: Config): Page {
  const p = profileMut(c);
  p.page ??= [];
  if (p.page.length === 0) p.page.push({ name: 'Main' });
  return [...p.page, ...(p.folder ?? [])].find((x) => x.name === hub.state.page) ?? p.page[0];
}

export function keyMut(c: Config, index: number): KeyConfig {
  const page = pageMut(c);
  page.key ??= [];
  let k = page.key.find((x) => x.index === index);
  if (!k) {
    k = { index };
    page.key.push(k);
    page.key.sort((a, b) => a.index - b.index);
    k = page.key.find((x) => x.index === index)!;
  }
  return k;
}

export function knobMut(c: Config, index: number) {
  const p = profileMut(c);
  p.knob ??= [];
  let k = p.knob.find((x) => x.index === index);
  if (!k) p.knob.push((k = { index }));
  return p.knob.find((x) => x.index === index)!;
}

export function faderMut(c: Config, index: number) {
  const p = profileMut(c);
  p.fader ??= [];
  if (!p.fader.some((x) => x.index === index)) p.fader.push({ index });
  return p.fader.find((x) => x.index === index)!;
}

export function rollerMut(c: Config, side: Side) {
  const p = profileMut(c);
  p.roller ??= [];
  if (!p.roller.some((x) => x.side === side)) p.roller.push({ side });
  return p.roller.find((x) => x.side === side)!;
}

export function dialMut(c: Config) {
  const p = profileMut(c);
  return (p.dial ??= {});
}

export async function setProfile(name: string) {
  hub.state.profile = name;
  hub.state.page = '';
  try {
    await invoke('set_profile', { name });
  } catch (e) {
    toast(String(e));
  }
}

export async function showPage(name: string) {
  hub.state.page = name;
  try {
    await invoke('show_page', { name });
  } catch (e) {
    toast(String(e));
  }
}

export function describe(a: Action | undefined): string {
  if (!a) return 'Nothing';
  if ('cmd' in a) return `Run: ${a.cmd}`;
  if ('keys' in a) return `Keys: ${a.keys}`;
  if ('page' in a) return a.page === 'next' ? 'Next page' : a.page === 'prev' ? 'Previous page' : `Go to page “${a.page}”`;
  if ('folder' in a) return `Open folder “${a.folder}”`;
  if ('back' in a) return 'Leave folder';
  return `Switch to profile “${a.profile}”`;
}

/** Effective LED colour of a module's LED (profile override over global). */
export function ledColor(kind: 'base' | 'fader' | 'knob', index: number): string | undefined {
  const pick = (v: string | string[] | undefined) => (Array.isArray(v) ? v[index] : v);
  const o = currentProfile()?.leds;
  const g = hub.config.leds;
  return pick(o?.[kind]) ?? pick(g?.[kind]);
}

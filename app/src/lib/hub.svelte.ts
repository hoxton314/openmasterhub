// App state: the config being edited, live hub state from the service, what the editor shows
// (separate from what is live) and the selection. Edits go through `edit()`, which records undo
// history and saves to the service shortly after the last change.

import { invoke, listen } from './backend';
import { activeSetup } from './model/setups';
import { adoptAppSettings } from './theme/theme.svelte';
import type { Action, AudioTarget, Capability, ClaimInfo, Config, Display, HubState, InputMsg, KeyConfig, ModuleState, Page, Profile, Selection, Snapshot, Side } from './types';
import { ui, type ToastSpec } from './ui.svelte';

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
  /** What the running service implements beyond the 0.8 baseline (see `hasCapability`). */
  capabilities: [] as string[],
  /** Claims on the hub (integrations, live-surface clients), from `hub:claim` events. */
  claims: [] as ClaimInfo[],
});

/**
 * What the editor shows. With `followLive` it tracks the live profile and page (`hub.state`);
 * picking another profile stops following, and the editor no longer changes what is live.
 */
export const editor = $state({
  setup: '',
  profile: '',
  page: '',
  followLive: true,
});

/** Undo/redo availability, for buttons. */
export const undoState = $state({ canUndo: false, canRedo: false, undoLabel: '', redoLabel: '' });

export function hasCapability(name: Capability | string): boolean {
  return hub.capabilities.includes(name);
}

/** The editor shows the live profile. */
export function isEditingLive(): boolean {
  return editor.profile === hub.state.profile;
}

function syncEditor() {
  if (!editor.followLive) return;
  editor.profile = hub.state.profile;
  editor.page = hub.state.page;
  editor.setup = hub.state.setup ?? activeSetup(hub.config, hub.state).name;
}

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
  hub.capabilities = s.capabilities ?? [];
  hub.displays = Object.fromEntries(s.displays.map((d) => [displayKey(d.serial, d.target), d.src]));
  hub.loaded = true;
  adoptAppSettings(hub.config.app);
  if (!hub.config.profile?.some((p) => p.name === editor.profile)) editor.followLive = true;
  syncEditor();
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
    syncEditor();
  });
  await listen<ClaimInfo>('hub:claim', (e) => {
    hub.claims = [...hub.claims.filter((c) => c.claim !== e.payload.claim), e.payload];
  });
  await listen<ClaimInfo[]>('hub:claims', (e) => {
    hub.claims = e.payload;
  });
  await listen<ModuleState[]>('hub:modules', (e) => {
    hub.modules = e.payload;
  });
  await listen<Display>('hub:display', (e) => {
    hub.displays[displayKey(e.payload.serial, e.payload.target)] = e.payload.src;
  });
  await listen('hub:config-changed', () => {
    if (saveTimer) return;
    // Someone else changed the file: our undo history no longer applies to it.
    undoStack.length = redoStack.length = 0;
    updateHistory();
    reload();
  });
  await reload();
}

/** Show a toast (see `ui.toast` for actions). */
export function toast(spec: string | ToastSpec) {
  return ui.toast(spec);
}

// ───────────────────────────── editing ─────────────────────────────

let saveTimer: ReturnType<typeof setTimeout> | undefined;

export interface EditOptions {
  /** Shown in undo toasts and tooltips ("Undo rename"). */
  label?: string;
  /** Edits with the same key within a second merge into one undo step (typing, sliders). */
  coalesce?: string;
}

interface HistoryEntry {
  config: Config;
  editor: { setup: string; profile: string; page: string };
  label: string;
  coalesce?: string;
  at: number;
}

const undoStack: HistoryEntry[] = [];
const redoStack: HistoryEntry[] = [];
const HISTORY_MAX = 100;

function updateHistory() {
  undoState.canUndo = undoStack.length > 0;
  undoState.canRedo = redoStack.length > 0;
  undoState.undoLabel = undoStack.at(-1)?.label ?? '';
  undoState.redoLabel = redoStack.at(-1)?.label ?? '';
}

const where = () => ({ setup: editor.setup, profile: editor.profile, page: editor.page });

/** Apply an edit to the config, record it for undo and save soon (debounced). */
export function edit(f: (c: Config) => void, opts: EditOptions = {}) {
  const before = $state.snapshot(hub.config) as Config;
  f(hub.config);
  tidy(hub.config);
  if (JSON.stringify(before) === JSON.stringify($state.snapshot(hub.config))) return;
  const now = Date.now();
  const top = undoStack.at(-1);
  if (opts.coalesce && top?.coalesce === opts.coalesce && now - top.at < 1000) top.at = now;
  else {
    undoStack.push({ config: before, editor: where(), label: opts.label ?? 'change', coalesce: opts.coalesce, at: now });
    if (undoStack.length > HISTORY_MAX) undoStack.shift();
  }
  redoStack.length = 0;
  updateHistory();
  scheduleSave();
}

function scheduleSave() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(save, 350);
}

function restore(from: HistoryEntry[], to: HistoryEntry[]): string | undefined {
  const entry = from.pop();
  if (!entry) return undefined;
  to.push({ config: $state.snapshot(hub.config) as Config, editor: where(), label: entry.label, at: Date.now() });
  hub.config = entry.config;
  if (entry.editor.profile !== editor.profile && hub.config.profile?.some((p) => p.name === entry.editor.profile)) {
    editor.followLive = entry.editor.profile === hub.state.profile;
    Object.assign(editor, entry.editor);
  }
  updateHistory();
  scheduleSave();
  return entry.label;
}

/** Undo the last edit; returns its label. */
export function undo() {
  return restore(undoStack, redoStack);
}

/** Redo the last undone edit; returns its label. */
export function redo() {
  return restore(redoStack, undoStack);
}

/** A toast with an Undo button for the edit just made. */
export function toastUndo(text: string) {
  return ui.toast({ text, action: { label: 'Undo', run: () => undo() } });
}

/** Ctrl Z / Ctrl Shift Z / Ctrl Y outside text fields (text fields keep their own undo). */
export function handleUndoKeys(e: KeyboardEvent): boolean {
  if (!(e.ctrlKey || e.metaKey) || e.altKey || e.defaultPrevented) return false;
  const t = e.target as HTMLElement | null;
  if (t?.closest?.('input, textarea, select, [contenteditable="true"]')) return false;
  const k = e.key.toLowerCase();
  const isRedo = (k === 'z' && e.shiftKey) || (k === 'y' && !e.shiftKey);
  if (k !== 'z' && !isRedo) return false;
  e.preventDefault();
  if (isRedo) redo();
  else undo();
  return true;
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

/** The profile shown in the editor (the live one while following). */
export function currentProfile(): Profile | undefined {
  const ps = hub.config.profile ?? [];
  return ps.find((p) => p.name === editor.profile) ?? ps[0];
}

/** The profile being edited, created if the config has none. */
export function profileMut(c: Config): Profile {
  c.profile ??= [];
  if (c.profile.length === 0) c.profile.push({ name: 'Default', page: [{ name: 'Main' }] });
  return c.profile.find((p) => p.name === editor.profile) ?? c.profile[0];
}

/** The page shown in the editor. */
export function currentPage(): Page | undefined {
  const p = currentProfile();
  if (!p) return undefined;
  return [...(p.page ?? []), ...(p.folder ?? [])].find((x) => x.name === editor.page) ?? p.page?.[0];
}

export function pageMut(c: Config): Page {
  const p = profileMut(c);
  p.page ??= [];
  if (p.page.length === 0) p.page.push({ name: 'Main' });
  return [...p.page, ...(p.folder ?? [])].find((x) => x.name === editor.page) ?? p.page[0];
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

/** Make a profile live on the hub (the editor follows it). */
export async function setProfile(name: string) {
  hub.state.profile = name;
  hub.state.page = '';
  editor.followLive = true;
  syncEditor();
  try {
    await invoke('set_profile', { name });
  } catch (e) {
    toast(String(e));
  }
}

/**
 * Edit a profile without making it live. Picking the live profile resumes following it;
 * any other stops following (the editor shows the not-live banner).
 */
export function editProfile(name: string, page = '') {
  editor.followLive = name === hub.state.profile;
  editor.profile = name;
  editor.page = page || (editor.followLive ? hub.state.page : '');
}

/** Turn "Follow live" on or off. */
export function setFollowLive(on: boolean) {
  editor.followLive = on;
  syncEditor();
}

/** Show a page in the editor; on the live profile it is also shown on the hub. */
export async function showPage(name: string) {
  editor.page = name;
  if (!isEditingLive()) return;
  hub.state.page = name;
  try {
    await invoke('show_page', { name });
  } catch (e) {
    toast(String(e));
  }
}

/** Make a setup active (services with the `setups` capability; otherwise only the editor). */
export async function setSetup(name: string) {
  editor.setup = name;
  if (!hasCapability('setups')) return;
  try {
    await invoke('set_setup', { name });
  } catch (e) {
    toast(String(e));
  }
}

export function describe(a: Action | undefined): string {
  if (!a) return 'Nothing';
  if ('cmd' in a) return `Run: ${a.cmd}`;
  if ('keys' in a) return typeof a.keys === 'string' ? `Keys: ${a.keys}` : `Keys: ${a.keys.combo}${a.keys.hold ? ` (hold ${a.keys.hold} ms)` : ''}`;
  if ('setup' in a) return a.setup === 'next' ? 'Next setup' : a.setup === 'prev' ? 'Previous setup' : a.setup === 'pick' ? 'Pick a setup on the dial' : `Switch to setup “${a.setup}”`;
  if ('pin' in a) return a.pin === 'release' ? 'Release the pin' : 'Pin or release the live profile';
  if ('page' in a) return a.page === 'next' ? 'Next page' : a.page === 'prev' ? 'Previous page' : `Go to page “${a.page}”`;
  if ('folder' in a) return `Open folder “${a.folder}”`;
  if ('back' in a) return 'Leave folder';
  if ('volume' in a) {
    const v = typeof a.volume === 'string' ? { level: a.volume } : a.volume;
    const what = audioTargetName(v);
    const name = what ? `${what} volume` : 'Volume';
    if (v.level === '{value}') return `${name}: follow the fader`;
    return /^[+-]/.test(v.level) ? `${name} ${v.level}%` : `${name}: set to ${v.level}%`;
  }
  if ('mute' in a) {
    const m = typeof a.mute === 'string' ? { state: a.mute } : a.mute;
    const what = audioTargetName(m);
    const verb = m.state === 'toggle' ? 'Mute' : m.state === 'on' ? 'Mute' : 'Unmute';
    return `${verb}${what ? ` ${what}` : ''}${m.state === 'toggle' ? ': toggle' : ''}`;
  }
  if ('session' in a) return `Open Claude session ${a.session + 1}`;
  if ('sessions' in a) return a.sessions === 'prev' ? 'Previous Claude sessions' : 'Next Claude sessions';
  if (a.profile === 'next') return 'Next profile';
  if (a.profile === 'prev') return 'Previous profile';
  if (a.profile === 'pick') return 'Pick a profile on the dial';
  return `Switch to profile “${a.profile}”`;
}

/** What a volume or mute action controls, for summaries; empty for the default output. */
export function audioTargetName(t: AudioTarget): string {
  const device = (d: string | undefined, fallback: string) => (!d || d === 'default' ? fallback : d);
  if (t.input !== undefined) return device(t.input, 'Microphone');
  if (t.app) return t.app;
  if (t.group) return `“${t.group}”`;
  return device(t.output, '');
}

/** Effective LED colour of a module's LED (profile override over global). */
export function ledColor(kind: 'base' | 'fader' | 'knob', index: number): string | undefined {
  const pick = (v: string | string[] | undefined) => (Array.isArray(v) ? v[index] : v);
  const o = currentProfile()?.leds;
  const g = hub.config.leds;
  return pick(o?.[kind]) ?? pick(g?.[kind]);
}

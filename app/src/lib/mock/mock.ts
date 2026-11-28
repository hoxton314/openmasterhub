// Browser-only stand-in for the Tauri backend, serving a snapshot of a real hub (dev builds only).
//
// Commands are looked up in the per-area handler files first (handlers/*.ts, one per work
// package, so areas never edit the same mock file), then in the base commands below.
// An area module exports `handlers` and optionally `init(ctx)` (once, before the app starts:
// apply URL switches to the shared snapshot) and `onSnapshot(ctx)` (each time the app asks for
// the snapshot, with listeners registered: schedule events). Nobody overrides `snapshot` itself,
// so every URL switch composes: `?service=…`, `?modules=none`, `?replug`, `?vendor`, `?firstrun`,
// `?demo=claim`, `?states=…`, `?claims=none`, `?hooks=missing`, `?game=…`, `?caps=none`.

import sample from './snapshot.json';
import type { Face, Snapshot } from '../types';
import type { UserThemeFile } from '../theme/theme.svelte';
import * as editor from './handlers/editor';
import * as global from './handlers/global';
import * as inspector from './handlers/inspector';
import * as library from './handlers/library';
import * as profiles from './handlers/profiles';
import * as setups from './handlers/setups';
import * as system from './handlers/system';

type Handler = (e: { payload: unknown }) => void;

/** What a handler gets besides the command's arguments. */
export interface MockContext {
  snapshot: Snapshot;
  emit: (event: string, payload: unknown) => void;
}
export type MockCommand = (args: Record<string, unknown> | undefined, ctx: MockContext) => unknown;
export type MockHandlers = Record<string, MockCommand>;
/** What a handler file exports. */
export interface MockModule {
  handlers: MockHandlers;
  init?: (ctx: MockContext) => void;
  onSnapshot?: (ctx: MockContext) => void;
}

const listeners: Record<string, Handler[]> = {};
const emit = (event: string, payload: unknown) => (listeners[event] ?? []).forEach((h) => h({ payload }));
// A git-ignored `snapshot.local.json` (e.g. your own config) replaces the sample when present.
const local = Object.values(import.meta.glob('./snapshot.local.json', { eager: true, import: 'default' }))[0];
const snapshot = structuredClone(local ?? sample) as unknown as Snapshot;

/** Every capability, so design work sees every control; `?caps=none` hides them. */
const ALL_CAPABILITIES = ['setups', 'base', 'hold', 'long_press', 'setup_action', 'profile_cycle', 'pin', 'except_setups', 'blink_on_setup', 'knob_volume', 'face_fit', 'widget_refresh', 'identify', 'needs_replug', 'system_info', 'preview_profile', 'release_claim', 'claim_details'];
const params = typeof location === 'undefined' ? new URLSearchParams() : new URLSearchParams(location.search);
snapshot.capabilities = params.get('caps') === 'none' ? [] : (snapshot.capabilities ?? ALL_CAPABILITIES);

const ctx: MockContext = { snapshot, emit };
const modules: MockModule[] = [editor, inspector, setups, profiles, library, global, system];
const areas: MockHandlers[] = modules.map((m) => m.handlers);
for (const m of modules) m.init?.(ctx);

export function listen(event: string, handler: Handler) {
  (listeners[event] ??= []).push(handler);
}

/** A rough browser rendering of a key face, standing in for the Rust renderer. */
function preview(face: Face): string {
  const c = document.createElement('canvas');
  c.width = c.height = 144;
  const g = c.getContext('2d')!;
  g.fillStyle = face.background ?? '#000';
  g.fillRect(0, 0, 144, 144);
  g.fillStyle = face.text_color ?? '#fff';
  g.font = '600 34px Inter Variable, sans-serif';
  g.textAlign = 'center';
  g.textBaseline = 'middle';
  const live = face.widget?.kind === 'claude_session' ? `Session ${(face.widget.slot ?? 0) + 1}` : face.widget ? '42%' : '';
  g.fillText(face.label ?? live, 72, 72, 130);
  return c.toDataURL();
}

/** Example user themes from `<config dir>/themes/`: one valid, one rejected (it tries `url(`). */
const userThemes: UserThemeFile[] = [
  {
    file: 'orchid.json',
    json: JSON.stringify({
      $schema: './theme.schema.json',
      schemaVersion: 1,
      id: 'orchid',
      name: 'Orchid',
      author: 'Example',
      description: 'Dark, with a pink accent: a two-colour user theme.',
      appearance: 'dark',
      inherits: 'dark',
      tokens: { color: { accent: '#F07FD0', accentFill: '#C2459F' } },
    }),
  },
  {
    file: 'broken.json',
    json: JSON.stringify({ schemaVersion: 1, id: 'broken', name: 'Broken', appearance: 'dark', inherits: 'dark', tokens: { color: { bg: "url('https://example.com/bg.png')" } } }),
  },
];

async function base(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case 'snapshot':
      for (const m of modules) m.onSnapshot?.(ctx);
      return snapshot;
    case 'save_config':
      snapshot.config = args?.config as Snapshot['config'];
      console.info('[mock] save_config', args?.config);
      return;
    case 'set_profile':
      snapshot.state = { ...snapshot.state!, profile: args?.name as string, page: '', reason: { kind: 'action' } };
      emit('hub:state', snapshot.state);
      return;
    case 'show_page':
      snapshot.state = { ...snapshot.state!, page: args?.name as string };
      emit('hub:state', snapshot.state);
      return;
    case 'set_setup':
      snapshot.state = { ...snapshot.state!, setup: args?.name as string };
      emit('hub:state', snapshot.state);
      return;
    case 'preview_face':
      return preview(args?.face as Face);
    case 'open_window_classes':
      return ['firefox', 'com.mitchellh.ghostty', 'obs', 'steam', 'code-oss', 'blender'];
    case 'audio_info':
      return { apps: ['Discord', 'firefox', 'spotify'], outputs: ['Speakers', 'Headphones'], inputs: ['USB Microphone', 'Webcam'] };
    case 'themes_list':
      return userThemes;
    case 'themes_dir':
      return '/home/you/.config/openmasterhub/themes';
    case 'start_service':
    case 'restart_service':
      return;
    default:
      console.info('[mock] unhandled command', cmd, args);
      return;
  }
}

export async function invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  for (const h of areas) if (cmd in h) return h[cmd](args, ctx);
  return base(cmd, args);
}

// Simulated input, so the live-feedback design can be judged: press `i` in the browser.
if (typeof window !== 'undefined') {
  window.addEventListener('keydown', (e) => {
    if (e.key !== 'i' || (e.target as HTMLElement).tagName === 'INPUT' || e.ctrlKey || e.metaKey) return;
    const keypad = snapshot.modules.find((m) => m.kind === 'keypad');
    if (keypad) emit('hub:input', { serial: keypad.serial, control: 'key', index: Math.floor(Math.random() * 15), event: 'down' });
  });
}

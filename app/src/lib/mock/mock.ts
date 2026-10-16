// Browser-only stand-in for the Tauri backend, serving a snapshot of a real hub.

import data from './snapshot.json';
import type { Face, Snapshot } from '../types';

type Handler = (e: { payload: unknown }) => void;
const handlers: Record<string, Handler[]> = {};
const emit = (event: string, payload: unknown) => (handlers[event] ?? []).forEach((h) => h({ payload }));
const snapshot = structuredClone(data) as unknown as Snapshot;

export function listen(event: string, handler: Handler) {
  (handlers[event] ??= []).push(handler);
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

export async function invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case 'snapshot':
      return snapshot;
    case 'save_config':
      console.info('[mock] save_config', args?.config);
      return;
    case 'set_profile':
      snapshot.state!.profile = args?.name as string;
      emit('hub:state', { ...snapshot.state, page: '' });
      return;
    case 'show_page':
      emit('hub:state', { ...snapshot.state, page: args?.name });
      return;
    case 'preview_face':
      return preview(args?.face as Face);
    case 'open_window_classes':
      return ['firefox', 'com.mitchellh.ghostty', 'obs', 'steam', 'code-oss'];
    default:
      return;
  }
}

// Simulated input, so the live-feedback design can be judged: press `i` in the browser.
if (typeof window !== 'undefined') {
  window.addEventListener('keydown', (e) => {
    if (e.key !== 'i' || (e.target as HTMLElement).tagName === 'INPUT') return;
    const keypad = snapshot.modules.find((m) => m.kind === 'keypad');
    if (keypad) emit('hub:input', { serial: keypad.serial, control: 'key', index: Math.floor(Math.random() * 15), event: 'down' });
  });
}

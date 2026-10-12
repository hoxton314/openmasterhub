// Talks to the Tauri backend, or, when the UI runs in a plain browser (design work), to a
// mock that serves a snapshot captured from a real hub.

import type { Face, Snapshot } from './types';

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

type Handler<T> = (e: { payload: T }) => void;

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }
  if (!import.meta.env.DEV) throw new Error('not running inside the OpenMasterHUB app');
  return (await import('./mock/mock')).invoke(cmd, args) as Promise<T>;
}

export async function listen<T>(event: string, handler: Handler<T>): Promise<void> {
  if (inTauri) {
    const { listen } = await import('@tauri-apps/api/event');
    await listen<T>(event, handler);
    return;
  }
  // The mock (with its sample hub) only exists in dev builds, for browser previews.
  if (import.meta.env.DEV) (await import('./mock/mock')).listen(event, handler as Handler<unknown>);
}

export async function pickImage(): Promise<string | undefined> {
  if (!inTauri) return '/mock-image.png';
  const { open } = await import('@tauri-apps/plugin-dialog');
  const path = await open({ multiple: false, filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif'] }] });
  return typeof path === 'string' ? path : undefined;
}

export type { Face, Snapshot };

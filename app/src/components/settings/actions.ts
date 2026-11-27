// Small helpers for the settings pages: talk to the backend and report problems as toasts.

import { invoke } from '../../lib/backend';
import { edit, toast } from '../../lib/hub.svelte';
import { appSettingsPatch, setThemeSettings, type ThemeSettings } from '../../lib/theme/theme.svelte';
import type { AppSettings } from '../../lib/types';

/** Run a backend command; failures become a toast instead of an unhandled rejection. */
export async function run<T>(cmd: string, args?: Record<string, unknown>, failure?: string): Promise<T | undefined> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    toast({ text: `${failure ?? cmd}: ${e}`, kind: 'danger' });
    return undefined;
  }
}

/** Open a file or folder with the system's default program. */
export const openPath = (path: string) => run('open_path', { path }, 'Could not open it');

/** Change appearance settings: applies at once, remembers them on this computer and saves them to `[app]`. */
export function saveAppearance(patch: Partial<ThemeSettings>, label = 'appearance change') {
  setThemeSettings(patch);
  edit(
    (c) => {
      c.app = { ...c.app, ...appSettingsPatch() };
    },
    { label, coalesce: 'appearance' },
  );
}

/** Change other `[app]` fields. */
export function saveApp(patch: Partial<AppSettings>, label = 'setting change') {
  edit(
    (c) => {
      c.app = { ...c.app, ...patch };
    },
    { label },
  );
}

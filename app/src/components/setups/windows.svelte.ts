// Windows for the rule editor and tester: open now (the service's `open_window_titles`, else the
// plain class list) and recent (classes seen before, kept in localStorage).

import { invoke } from '../../lib/backend';

export interface OpenWindow {
  class: string;
  /** Window title when the service gives one. */
  title?: string;
}

const KEY = 'omh.recentWindows';

function readRecent(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(v) ? v.filter((x) => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

export const windows = $state({ open: [] as OpenWindow[], recent: readRecent(), loaded: false });

export async function refreshWindows() {
  let list: OpenWindow[] = [];
  try {
    list = await invoke<OpenWindow[]>('open_window_titles');
  } catch {
    // an older app or service: classes only
  }
  if (!Array.isArray(list) || !list.length) {
    const classes = await invoke<string[]>('open_window_classes').catch(() => [] as string[]);
    list = (Array.isArray(classes) ? classes : []).map((c) => ({ class: c }));
  }
  windows.open = list.filter((w) => w?.class);
  const seen = new Set(windows.open.map((w) => w.class.toLowerCase()));
  windows.recent = [...windows.open.map((w) => w.class), ...windows.recent.filter((r) => !seen.has(r.toLowerCase()))].slice(0, 40);
  try {
    localStorage.setItem(KEY, JSON.stringify(windows.recent));
  } catch {
    // storage unavailable: recent lasts for this session
  }
  windows.loaded = true;
}

/** Recent classes that are not open now. */
export const recentOnly = () => windows.recent.filter((r) => !windows.open.some((w) => w.class.toLowerCase() === r.toLowerCase()));

/** Every class known on this PC (for conflict suggestions). */
export const knownClasses = () => [...new Set([...windows.open.map((w) => w.class), ...windows.recent])];

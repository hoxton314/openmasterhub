// Shared UI state: which area and sub-view is open, overlays (palette, first run, new profile),
// the inspector drawer, the modal dialog and the toast queue.

export type Area = 'editor' | 'setups' | 'library' | 'global' | 'settings';
export const AREAS: Area[] = ['editor', 'setups', 'library', 'global', 'settings'];

/**
 * A sub-view inside an area, e.g. `{ view: 'profile-settings', profile: 'Blender' }`,
 * `{ view: 'pages' }`, `{ view: 'setup-detail', setup: 'Work' }`, `{ view: 'integration', id: 'cyberpunk' }`,
 * `{ view: 'audio-groups' }`, `{ view: 'appearance' }`. `null` is the area's main view.
 */
export interface Route {
  view: string;
  [param: string]: string | number | boolean | undefined;
}

export interface DialogSpec {
  title: string;
  body?: string;
  /** Show a text field with this initial value. */
  value?: string;
  placeholder?: string;
  suggestions?: string[];
  ok: string;
  danger?: boolean;
  input: boolean;
  done: (value: string) => void;
}

export type ToastKind = 'info' | 'success' | 'warning' | 'danger';

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface ToastSpec {
  text: string;
  kind?: ToastKind;
  /** One ghost button, e.g. Undo or Details. Running it dismisses the toast. */
  action?: ToastAction;
  /** Milliseconds; 0 keeps it until dismissed. Default 5000 (8000 with an action). */
  timeout?: number;
}

export interface Toast extends Required<Pick<ToastSpec, 'text' | 'kind'>> {
  id: number;
  action?: ToastAction;
  /** Set while it plays its exit animation. */
  leaving?: boolean;
}

/** Where the new-profile flow inserts the profile. */
export interface NewProfileRequest {
  setup?: string;
  /** Position in the setup's list (default: last before the fallback). */
  position?: number;
}

let nextToast = 1;
const timers = new Map<number, ReturnType<typeof setTimeout>>();

export const ui = $state({
  area: 'editor' as Area,
  route: null as Route | null,
  /** The last sub-view per area, restored when coming back. */
  routes: {} as Partial<Record<Area, Route | null>>,
  /** Inspector drawer open (windows narrower than 1280 px). */
  drawer: false,
  /** Written by shell/Layout from the window width: ≤1100 px collapses the sidebar, <1280 px makes the inspector a drawer. */
  layout: { sidebarCollapsed: false, inspectorDrawer: false },
  /** Ctrl K command palette. */
  palette: false,
  /** First-run flow (S29). */
  firstRun: false,
  /** New-profile gallery (S19), with where to insert. */
  newProfile: null as NewProfileRequest | null,
  dialog: null as DialogSpec | null,
  toasts: [] as Toast[],

  /** Open an area (and optionally a sub-view in it). */
  go(area: Area, route?: Route | null) {
    ui.routes[ui.area] = ui.route;
    ui.area = area;
    ui.route = route === undefined ? (ui.routes[area] ?? null) : route;
    writeHash();
  },

  /** Open a sub-view of the current area; `null` returns to its main view. */
  open(route: Route | null) {
    ui.route = route;
    writeHash();
  },

  openNewProfile(req: NewProfileRequest = {}) {
    ui.newProfile = req;
  },

  ask(spec: Omit<DialogSpec, 'input'>) {
    ui.dialog = { ...spec, value: spec.value ?? '', input: true };
  },

  confirm(spec: Omit<DialogSpec, 'input' | 'done'> & { done: () => void }) {
    ui.dialog = { ...spec, danger: spec.danger ?? true, input: false, done: () => spec.done() };
  },

  /** Show a toast; returns its id. */
  toast(spec: string | ToastSpec): number {
    const s: ToastSpec = typeof spec === 'string' ? { text: spec } : spec;
    const id = nextToast++;
    ui.toasts = [...ui.toasts.filter((t) => t.text !== s.text).slice(-3), { id, text: s.text, kind: s.kind ?? 'info', action: s.action }];
    const timeout = s.timeout ?? (s.action ? 8000 : 5000);
    if (timeout > 0) timers.set(id, setTimeout(() => ui.dismiss(id), timeout));
    return id;
  },

  /** Remove a toast (after a short exit animation). */
  dismiss(id: number) {
    clearTimeout(timers.get(id));
    timers.delete(id);
    const t = ui.toasts.find((x) => x.id === id);
    if (!t || t.leaving) return;
    t.leaving = true;
    setTimeout(() => (ui.toasts = ui.toasts.filter((x) => x.id !== id)), 150);
  },
});

// `#setups` / `#setups/setup-detail` in the URL, so a reload or a design link keeps the place.
function writeHash() {
  if (typeof location === 'undefined') return;
  const hash = `#${ui.area}${ui.route ? `/${ui.route.view}` : ''}`;
  if (location.hash !== hash) history.replaceState(null, '', hash);
}

function readHash() {
  if (typeof location === 'undefined') return;
  const [area, view] = location.hash.slice(1).split('/');
  if ((AREAS as string[]).includes(area)) {
    ui.area = area as Area;
    ui.route = view ? { view } : null;
  }
}
readHash();

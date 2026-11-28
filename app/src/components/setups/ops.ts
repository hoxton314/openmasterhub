// Config operations of the Setups area. Each one runs inside `edit()` (undoable, saved soon) and
// materialises the implicit `Default` setup of a legacy config first, so `[[setup]]` is only
// written once something about a setup changes.

import { edit, editProfile, editor, hub, toastUndo } from '../../lib/hub.svelte';
import { applyFix, type ConflictFix } from '../../lib/model/conflicts';
import { effectiveSetups, materialiseSetups, profileByName, renameSetup as renameSetupRefs, setupByName, type EffectiveSetup } from '../../lib/model';
import { instantiate, type Template } from '../../lib/templates';
import { ui } from '../../lib/ui.svelte';
import type { Config, Setup, SetupMode } from '../../lib/types';
import { Hand, AppWindow, Pin } from 'lucide-svelte';

export const MODES: { value: SetupMode; label: string; line: string; icon: typeof Hand }[] = [
  { value: 'follow', label: 'Follow the focused app', line: 'Follow the focused app', icon: AppWindow },
  { value: 'manual', label: 'Manual', line: 'Manual', icon: Hand },
  { value: 'hybrid', label: 'Follow apps, with manual override', line: 'Follow apps, with manual override', icon: Pin },
];
export const modeInfo = (m: SetupMode) => MODES.find((x) => x.value === m)!;

/** "7 profiles": the setup's list plus its fallback. */
export const profileCount = (s: EffectiveSetup) => s.profiles.length + (s.mode !== 'manual' && s.fallback ? 1 : 0);
export const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

/** The one sentence under a card title. */
export function setupBlurb(c: Config, s: EffectiveSetup): string {
  if (s.mode === 'manual') return 'Profiles in order. Focus does nothing; you switch.';
  const lead = s.mode === 'hybrid' ? 'Follows the app in front until you pin a profile.' : 'Changes with the app in front.';
  const bases = new Set(s.profiles.map((n) => profileByName(c, n)?.base).filter(Boolean));
  if (s.fallback && bases.has(s.fallback)) return `${lead} Controls a profile leaves out come from ${s.fallback}.`;
  return lead;
}

/** The setup `name` in `c`, materialising a legacy config first. */
export function setupMut(c: Config, name: string): Setup | undefined {
  materialiseSetups(c);
  return c.setup!.find((s) => s.name === name);
}

export function updateSetup(name: string, f: (s: Setup) => void, label: string, coalesce?: string) {
  edit((c) => {
    const s = setupMut(c, name);
    if (s) f(s);
  }, { label, coalesce });
}

export function setMode(name: string, mode: SetupMode) {
  updateSetup(name, (s) => {
    s.mode = mode;
    if (mode === 'hybrid') {
      s.release ??= 'same_trigger';
    }
  }, 'change mode');
}

/** Add a setup from a template; returns its name. */
export function createFromTemplate(t: Template): string {
  let name = '';
  edit((c) => {
    materialiseSetups(c);
    const made = instantiate(t, c);
    if (!made.setup) return;
    c.profile ??= [];
    c.profile.push(...made.profiles);
    c.setup!.push(made.setup);
    name = made.setup.name;
  }, { label: 'add setup' });
  return name;
}

export function renameSetup(old: string) {
  ui.ask({
    title: `Rename ${old}`,
    placeholder: 'Setup name',
    value: old,
    ok: 'Rename',
    done: (name) => {
      const n = name.trim();
      if (!n || n === old) return;
      if (effectiveSetups(hub.config).some((s) => s.name === n)) return void ui.toast({ text: `There is already a setup called ${n}`, kind: 'warning' });
      edit((c) => {
        materialiseSetups(c);
        renameSetupRefs(c, old, n);
      }, { label: 'rename setup' });
      if (editor.setup === old) editor.setup = n;
      if (ui.route?.setup === old) ui.open({ ...ui.route, setup: n });
    },
  });
}

export function duplicateSetup(name: string): string | undefined {
  let made: string | undefined;
  edit((c) => {
    materialiseSetups(c);
    const src = c.setup!.find((s) => s.name === name);
    if (!src) return;
    const taken = new Set(c.setup!.map((s) => s.name));
    let n = `${name} copy`;
    for (let i = 2; taken.has(n); i++) n = `${name} copy ${i}`;
    // Profiles stay linked: the copy lists the same profile names. Shortcuts are not copied (they would clash).
    const { shortcut: _s, next: _n, prev: _p, pin: _pin, ...rest } = structuredClone(src);
    c.setup!.push({ ...rest, name: n, profiles: [...(src.profiles ?? [])] });
    made = n;
  }, { label: 'duplicate setup' });
  return made;
}

export function deleteSetup(name: string) {
  const s = setupByName(hub.config, name);
  if (!s) return;
  if (effectiveSetups(hub.config).length < 2) return void ui.toast({ text: 'Keep at least one setup', kind: 'warning' });
  ui.confirm({
    title: `Delete ${name}?`,
    body: `The setup goes away. Its profiles (${plural(profileCount(s), 'profile')}) stay in the config and can be added to another setup.`,
    ok: 'Delete setup',
    done: () => {
      edit((c) => {
        materialiseSetups(c);
        c.setup = c.setup!.filter((x) => x.name !== name);
        for (const k of c.pinned ?? []) k.except_setups = k.except_setups?.filter((x) => x !== name);
      }, { label: 'delete setup' });
      if (ui.route?.setup === name) ui.open(null);
      toastUndo(`Deleted ${name}`);
    },
  });
}

/** Move a profile inside a setup's list (`to` = new index; clamped). */
export function moveProfile(setup: string, from: number, to: number) {
  updateSetup(setup, (s) => {
    const list = s.profiles ?? [];
    const target = Math.max(0, Math.min(list.length - 1, to));
    if (from === target || from < 0 || from >= list.length) return;
    const [p] = list.splice(from, 1);
    list.splice(target, 0, p);
  }, 'reorder profiles');
}

export function addProfileToSetup(setup: string, profile: string, at?: number) {
  updateSetup(setup, (s) => {
    s.profiles ??= [];
    if (s.profiles.includes(profile) || s.fallback === profile) return;
    s.profiles.splice(at ?? s.profiles.length, 0, profile);
  }, 'add profile to setup');
}

export function removeProfileFromSetup(setup: string, profile: string) {
  updateSetup(setup, (s) => (s.profiles = (s.profiles ?? []).filter((p) => p !== profile)), 'remove profile from setup');
  toastUndo(`Removed ${profile} from ${setup}`);
}

export function setFallback(setup: string, profile: string | undefined) {
  updateSetup(setup, (s) => {
    s.profiles = (s.profiles ?? []).filter((p) => p !== profile);
    if (profile) s.fallback = profile;
    else delete s.fallback;
  }, 'change fallback');
}

/** Set, replace (`index`) or remove (`pattern` undefined) one app-rule pattern of a profile. */
export function setPattern(profile: string, index: number | undefined, pattern: string | undefined) {
  edit((c) => {
    const p = profileByName(c, profile);
    if (!p) return;
    const apps = [...(p.apps ?? [])];
    if (pattern === undefined) {
      if (index !== undefined) apps.splice(index, 1);
    } else if (index === undefined) {
      if (!apps.includes(pattern)) apps.push(pattern);
    } else apps[index] = pattern;
    p.apps = apps.length ? apps : undefined;
  }, { label: pattern === undefined ? 'remove app rule' : 'edit app rule' });
}

export function applyConflictFix(setup: string, fix: ConflictFix) {
  edit((c) => {
    materialiseSetups(c);
    applyFix(c, setup, fix);
  }, { label: 'fix rule conflict' });
  toastUndo('Rule conflict fixed');
}

/** Make a setup the active one (mock and real service: `set_setup`). */
export { setSetup as makeActive } from '../../lib/hub.svelte';

/** Open a setup's profile in the Editor. */
export function openInEditor(setup: string, profile?: string) {
  const s = setupByName(hub.config, setup);
  if (!s) return;
  const live = hub.state.profile;
  const pick = profile ?? (s.profiles.includes(live) || s.fallback === live ? live : (s.fallback ?? s.profiles[0]));
  editor.setup = setup;
  if (pick) editProfile(pick);
  ui.go('editor');
}


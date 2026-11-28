// Ownership and edits of the selected control: which layer provides it (precedence), and the
// edits the inspector states offer (override an inherited control, edit or hide a pinned key,
// reset, copy). Edits go through `edit()` so they can be undone.

import { currentPage, currentProfile, edit, editProfile, editor, hub, isEditingLive, keyMut, profileMut } from '../../lib/hub.svelte';
import { controlIn, ownerOf, pinnedApplies, type ControlRef, type Owner } from '../../lib/model/precedence';
import { profileByName } from '../../lib/model/setups';
import type { Config, PinnedKey, Profile, Selection } from '../../lib/types';

/** The selected control as a model reference (modules have none). */
export function refOf(s: Selection): ControlRef | undefined {
  switch (s.kind) {
    case 'key': return { kind: 'key', index: s.index };
    case 'dial': return { kind: 'dial' };
    case 'knob': return { kind: 'knob', index: s.index };
    case 'fader': return { kind: 'fader', index: s.index };
    case 'roller': return { kind: 'roller', side: s.side };
    default: return undefined;
  }
}

/** Who provides a control in the edited profile's view. Claims only count on the live profile. */
export function ownerFor(ctl: ControlRef): Owner {
  const live = isEditingLive();
  return ownerOf(
    hub.config,
    { profile: currentProfile()?.name ?? '', page: currentPage()?.name, setup: editor.setup || undefined, claims: live ? hub.claims : [] },
    ctl,
    live ? hub.state.reason : undefined,
  );
}

/** The config a base profile provides for a control (what an inherited control shows). */
export function baseConfig(owner: Extract<Owner, { kind: 'base' }>, ctl: ControlRef): object | undefined {
  const self = currentProfile();
  const base = profileByName(hub.config, owner.profile);
  return self && base ? controlIn(base, ctl, currentPage()?.name, self) : undefined;
}

const clone = <T>(x: T): T => JSON.parse(JSON.stringify(x));

/** Name of the page in `base` that is shown for the edited page (same position; folders by name). */
function basePageName(self: Profile, base: Profile, page: string | undefined): string {
  if ((self.folder ?? []).some((f) => f.name === page)) return page ?? '';
  const n = Math.max(0, (self.page ?? []).findIndex((x) => x.name === page));
  return base.page?.[n]?.name ?? '';
}

/** Edit an inherited control in the profile that provides it. */
export function editInBase(owner: Extract<Owner, { kind: 'base' }>) {
  const self = currentProfile();
  const base = profileByName(hub.config, owner.profile);
  if (self && base) editProfile(base.name, basePageName(self, base, currentPage()?.name));
}

/** The pinned key a key control shows, for editing it in place. */
export function pinnedMut(c: Config, index: number): PinnedKey | undefined {
  const profile = currentProfile()?.name ?? '';
  return (c.pinned ?? []).find((p) => p.index === index && pinnedApplies(p, profile, editor.setup || undefined));
}

/** Stop showing a pinned key in the edited profile (adds it to `except`). */
export function hidePinned(index: number) {
  const profile = currentProfile()?.name;
  if (!profile) return;
  edit((c) => {
    const p = pinnedMut(c, index);
    if (p) p.except = [...(p.except ?? []), profile];
  }, { label: 'hide pinned key' });
}

/** The control's own config in the edited profile (or undefined). */
export function ownConfig(c: Config, ctl: ControlRef): (object & { index?: number; side?: string }) | undefined {
  const p = (c.profile ?? []).find((x) => x.name === currentProfile()?.name);
  if (!p) return undefined;
  switch (ctl.kind) {
    case 'key': return [...(p.page ?? []), ...(p.folder ?? [])].find((x) => x.name === currentPage()?.name)?.key?.find((k) => k.index === ctl.index);
    case 'dial': return p.dial;
    case 'knob': return p.knob?.find((k) => k.index === ctl.index);
    case 'fader': return p.fader?.find((f) => f.index === ctl.index);
    case 'roller': return p.roller?.find((r) => r.side === ctl.side);
  }
}

/** Give the edited profile its own copy of an inherited control (face and actions). */
export function overrideHere(owner: Extract<Owner, { kind: 'base' }>, ctl: ControlRef) {
  const src = baseConfig(owner, ctl);
  if (!src) return;
  const data = clone(src) as Record<string, unknown>;
  edit((c) => {
    profileMut(c);
    const target = ensure(c, ctl);
    Object.assign(target, data, ctl.kind === 'key' || ctl.kind === 'knob' || ctl.kind === 'fader' ? { index: ctl.index } : ctl.kind === 'roller' ? { side: ctl.side } : {});
  }, { label: 'override inherited control' });
}

/** The control's entry in the edited profile, created empty when missing. */
function ensure(c: Config, ctl: ControlRef): Record<string, unknown> {
  const p = profileMut(c);
  switch (ctl.kind) {
    case 'key': return keyMut(c, ctl.index) as unknown as Record<string, unknown>;
    case 'dial': return (p.dial ??= {}) as Record<string, unknown>;
    case 'knob': {
      p.knob ??= [];
      if (!p.knob.some((k) => k.index === ctl.index)) p.knob.push({ index: ctl.index });
      return p.knob.find((k) => k.index === ctl.index)! as unknown as Record<string, unknown>;
    }
    case 'fader': {
      p.fader ??= [];
      if (!p.fader.some((f) => f.index === ctl.index)) p.fader.push({ index: ctl.index });
      return p.fader.find((f) => f.index === ctl.index)! as unknown as Record<string, unknown>;
    }
    case 'roller': {
      p.roller ??= [];
      if (!p.roller.some((r) => r.side === ctl.side)) p.roller.push({ side: ctl.side });
      return p.roller.find((r) => r.side === ctl.side)! as unknown as Record<string, unknown>;
    }
  }
}

/** Remove everything the edited profile sets for a control. */
export function resetControl(ctl: ControlRef) {
  edit((c) => {
    const p = profileMut(c);
    switch (ctl.kind) {
      case 'key': {
        const page = [...(p.page ?? []), ...(p.folder ?? [])].find((x) => x.name === currentPage()?.name);
        if (page) page.key = (page.key ?? []).filter((k) => k.index !== ctl.index);
        break;
      }
      case 'dial': delete p.dial; break;
      case 'knob': p.knob = (p.knob ?? []).filter((k) => k.index !== ctl.index); break;
      case 'fader': p.fader = (p.fader ?? []).filter((f) => f.index !== ctl.index); break;
      case 'roller': p.roller = (p.roller ?? []).filter((r) => r.side !== ctl.side); break;
    }
  }, { label: 'reset control' });
}

// ───────────────────────────── copy to… ─────────────────────────────

/** Where a control can be copied to. */
export interface CopyTarget {
  label: string;
  hint?: string;
  run: () => void;
}

/** Copy `src` (this profile's own config of `ctl`) onto another control of the same kind. */
function copyTo(ctl: ControlRef, to: ControlRef, where: { profile?: string; page?: string } = {}) {
  const src = ownConfig(hub.config, ctl);
  if (!src) return;
  const data = clone(src) as Record<string, unknown>;
  delete data.index;
  delete data.side;
  edit((c) => {
    const p = where.profile ? (c.profile ?? []).find((x) => x.name === where.profile) : profileMut(c);
    if (!p) return;
    let target: Record<string, unknown> | undefined;
    if (to.kind === 'key') {
      const page = [...(p.page ?? []), ...(p.folder ?? [])].find((x) => x.name === (where.page ?? currentPage()?.name));
      if (!page) return;
      page.key ??= [];
      let k = page.key.find((x) => x.index === to.index);
      if (!k) page.key.push((k = { index: to.index }));
      target = k as unknown as Record<string, unknown>;
    } else if (where.profile) {
      target = ensureIn(p, to);
    } else target = ensure(c, to);
    if (!target) return;
    for (const k of Object.keys(target)) if (k !== 'index' && k !== 'side') delete target[k];
    Object.assign(target, data);
  }, { label: 'copy control' });
}

function ensureIn(p: Profile, ctl: ControlRef): Record<string, unknown> | undefined {
  switch (ctl.kind) {
    case 'dial': return (p.dial ??= {}) as Record<string, unknown>;
    case 'knob': { p.knob ??= []; if (!p.knob.some((k) => k.index === ctl.index)) p.knob.push({ index: ctl.index }); return p.knob.find((k) => k.index === ctl.index) as unknown as Record<string, unknown>; }
    case 'fader': { p.fader ??= []; if (!p.fader.some((k) => k.index === ctl.index)) p.fader.push({ index: ctl.index }); return p.fader.find((k) => k.index === ctl.index) as unknown as Record<string, unknown>; }
    case 'roller': { p.roller ??= []; if (!p.roller.some((k) => k.side === ctl.side)) p.roller.push({ side: ctl.side }); return p.roller.find((k) => k.side === ctl.side) as unknown as Record<string, unknown>; }
    default: return undefined;
  }
}

/** The controls the selected one can be copied to, in sections for the Copy to… menu. */
export interface CopyPlan {
  /** Other keys on this page (indices), for the key grid. */
  keys: { index: number; run: () => void }[];
  /** Other controls of the same kind (knobs, faders, the other roller). */
  siblings: CopyTarget[];
  pages: CopyTarget[];
  profiles: CopyTarget[];
}

export function copyPlan(ctl: ControlRef, done: (what: string) => void): CopyPlan {
  const profile = currentProfile();
  const page = currentPage();
  const plan: CopyPlan = { keys: [], siblings: [], pages: [], profiles: [] };
  const mk = (to: ControlRef, text: string, where: { profile?: string; page?: string } = {}) => () => {
    copyTo(ctl, to, where);
    done(text);
  };
  const name = (x: ControlRef) => (x.kind === 'key' ? `key ${x.index + 1}` : x.kind === 'knob' ? `knob ${x.index + 1}` : x.kind === 'fader' ? `fader ${x.index + 1}` : x.kind === 'roller' ? `${x.side} roller` : 'the dial');
  if (ctl.kind === 'key') {
    for (let i = 0; i < 15; i++) if (i !== ctl.index) plan.keys.push({ index: i, run: mk({ kind: 'key', index: i }, `Copied ${name(ctl)} to key ${i + 1}`) });
    for (const p of profile?.page ?? []) if (p.name !== page?.name) plan.pages.push({ label: p.name, hint: `key ${ctl.index + 1}`, run: mk(ctl, `Copied ${name(ctl)} to page ${p.name}`, { page: p.name }) });
    for (const f of profile?.folder ?? []) if (f.name !== page?.name) plan.pages.push({ label: f.name, hint: 'folder', run: mk(ctl, `Copied ${name(ctl)} to folder ${f.name}`, { page: f.name }) });
  } else if (ctl.kind === 'knob') {
    for (const i of [0, 1, 2]) if (i !== ctl.index) plan.siblings.push({ label: `Knob ${i + 1}`, run: mk({ kind: 'knob', index: i }, `Copied ${name(ctl)} to knob ${i + 1}`) });
  } else if (ctl.kind === 'fader') {
    for (const i of [0, 1, 2, 3, 4]) if (i !== ctl.index) plan.siblings.push({ label: `Fader ${i + 1}`, run: mk({ kind: 'fader', index: i }, `Copied ${name(ctl)} to fader ${i + 1}`) });
  } else if (ctl.kind === 'roller') {
    const other = ctl.side === 'left' ? 'right' : 'left';
    plan.siblings.push({ label: `${other[0].toUpperCase()}${other.slice(1)} roller`, run: mk({ kind: 'roller', side: other }, `Copied ${name(ctl)} to the ${other} roller`) });
  }
  for (const p of hub.config.profile ?? []) {
    if (p.name === profile?.name) continue;
    if (ctl.kind === 'key') {
      const n = Math.max(0, (profile?.page ?? []).findIndex((x) => x.name === page?.name));
      const target = (profile?.folder ?? []).some((f) => f.name === page?.name) ? page?.name : p.page?.[n]?.name;
      if (target) plan.profiles.push({ label: p.name, hint: `page ${target}`, run: mk(ctl, `Copied ${name(ctl)} to ${p.name}`, { profile: p.name, page: target }) });
    } else plan.profiles.push({ label: p.name, run: mk(ctl, `Copied ${name(ctl)} to ${p.name}`, { profile: p.name }) });
  }
  return plan;
}

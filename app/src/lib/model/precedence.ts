// Which profile owns a control, per IA-Precedence (top wins):
//   1 integration claim (while its app is focused, by priority)
//   2 pinned key (Global), unless the profile or its setup is excluded
//   3 manual choice / hybrid pin ┐
//   4 app rule (first match)     ├ pick the profile; it owns the control if it sets it
//   5 fallback profile           ┘
//   6 base profile (inherited), then nothing.
// Used by the canvas markers, the inspector's inherited/pinned/owned states and "Why?".

import type { ClaimInfo, Config, DialConfig, HubState, KeyConfig, Page, PinnedKey, Profile, Reason, Side } from '../types';
import { activeSetup, baseChain, matchApp, profileByName, setupByName, type EffectiveSetup } from './setups';

export type ControlRef =
  | { kind: 'key'; index: number }
  | { kind: 'dial' }
  | { kind: 'knob'; index: number }
  | { kind: 'roller'; side: Side }
  | { kind: 'fader'; index: number };

/** Whose view is being asked about. */
export interface OwnerContext {
  /** The profile shown (edited or live). */
  profile: string;
  /** Its page (keys); defaults to the first page. */
  page?: string;
  /** The setup it is seen in; defaults to the active one. */
  setup?: string;
  /** Claims on the hub; only active ones count. */
  claims?: ClaimInfo[];
}

export type Owner =
  | { level: 1; kind: 'claim'; claim: ClaimInfo }
  | { level: 2; kind: 'pinned'; pinned: PinnedKey }
  | { level: 3 | 4 | 5; kind: 'profile'; profile: string }
  | { level: 6; kind: 'base'; profile: string; /** Profiles walked, nearest first. */ via: string[] }
  | { level: 0; kind: 'none' };

const filled = (o: object | undefined) => !!o && Object.entries(o).some(([k, v]) => k !== 'index' && k !== 'side' && v !== undefined && v !== null && v !== '');

function claimOwns(c: ClaimInfo, ctl: ControlRef): boolean {
  if (!c.active) return false;
  const o = c.owns ?? {};
  switch (ctl.kind) {
    case 'key':
      return !!o.keys?.includes(ctl.index);
    case 'dial':
      return !!o.dial;
    case 'knob':
      return !!o.knobs?.includes(ctl.index);
    case 'fader':
      return !!o.faders?.includes(ctl.index);
    case 'roller':
      return !!o.rollers?.includes(ctl.side === 'left' ? 0 : 1);
  }
}

/** Is a pinned key shown in this profile within this setup? */
export function pinnedApplies(p: PinnedKey, profile: string, setup: string | undefined): boolean {
  return !(p.except ?? []).includes(profile) && !(setup && (p.except_setups ?? []).includes(setup));
}

/** The pinned key at `index` shown in `profile` (in `setup`), if any. */
export function pinnedAt(c: Config, index: number, profile: string, setup?: string): PinnedKey | undefined {
  return (c.pinned ?? []).find((p) => p.index === index && pinnedApplies(p, profile, setup));
}

/** Page `n` of a base: same position among pages, folders by name. */
function pageIn(p: Profile, ref: Profile, page: string | undefined): Page | undefined {
  const folder = (ref.folder ?? []).find((f) => f.name === page);
  if (folder) return (p.folder ?? []).find((f) => f.name === folder.name);
  const n = Math.max(0, (ref.page ?? []).findIndex((x) => x.name === page));
  return p.page?.[n];
}

/** The control's config in one profile, when that profile sets it. */
export function controlIn(p: Profile, ctl: ControlRef, page?: string, pageOf: Profile = p): KeyConfig | DialConfig | object | undefined {
  let found: object | undefined;
  switch (ctl.kind) {
    case 'key':
      found = pageIn(p, pageOf, page)?.key?.find((k) => k.index === ctl.index);
      break;
    case 'dial':
      found = p.dial;
      break;
    case 'knob':
      found = p.knob?.find((k) => k.index === ctl.index);
      break;
    case 'fader':
      found = p.fader?.find((f) => f.index === ctl.index);
      break;
    case 'roller':
      found = p.roller?.find((r) => r.side === ctl.side);
      break;
  }
  return filled(found) ? found : undefined;
}

/** How the profile came to be shown in its setup (levels 3–5), from the service's reason if given. */
export function profileLevel(reason: Reason | undefined): 3 | 4 | 5 {
  if (!reason) return 4;
  if (reason.kind === 'fallback') return 5;
  if (reason.kind === 'rule') return 4;
  return 3;
}

/** The owner of one control in a profile's view. */
export function ownerOf(c: Config, ctx: OwnerContext, ctl: ControlRef, reason?: Reason): Owner {
  const claim = (ctx.claims ?? []).filter((x) => claimOwns(x, ctl)).sort((a, b) => (b.priority ?? 0) - (a.priority ?? 0))[0];
  if (claim) return { level: 1, kind: 'claim', claim };
  const setup = ctx.setup ?? activeSetup(c).name;
  if (ctl.kind === 'key') {
    const pinned = pinnedAt(c, ctl.index, ctx.profile, setup);
    if (pinned) return { level: 2, kind: 'pinned', pinned };
  }
  const self = profileByName(c, ctx.profile);
  if (!self) return { level: 0, kind: 'none' };
  if (controlIn(self, ctl, ctx.page)) return { level: profileLevel(reason), kind: 'profile', profile: self.name };
  const via: string[] = [];
  for (const b of baseChain(c, self.name)) {
    via.push(b.name);
    if (controlIn(b, ctl, ctx.page, self)) return { level: 6, kind: 'base', profile: b.name, via };
  }
  return { level: 0, kind: 'none' };
}

/** One line of the "Why this profile?" chain. */
export interface WhyStep {
  level: 1 | 2 | 3 | 4 | 5 | 6;
  text: string;
  /** Controls this step covers (empty: the whole profile). */
  controls: ControlRef[];
}

export interface Why {
  profile: string;
  setup: EffectiveSetup;
  steps: WhyStep[];
}

const label = (ctl: ControlRef) =>
  ctl.kind === 'key' ? `key ${ctl.index + 1}` : ctl.kind === 'dial' ? 'the dial' : ctl.kind === 'roller' ? `the ${ctl.side} roller` : `${ctl.kind} ${ctl.index + 1}`;

/** Every control of the hub, for walking ownership. */
export function allControls(): ControlRef[] {
  return [
    ...Array.from({ length: 15 }, (_, index) => ({ kind: 'key' as const, index })),
    { kind: 'dial' },
    ...[0, 1, 2].map((index) => ({ kind: 'knob' as const, index })),
    { kind: 'roller', side: 'left' },
    { kind: 'roller', side: 'right' },
    ...[0, 1, 2, 3, 4].map((index) => ({ kind: 'fader' as const, index })),
  ];
}

/**
 * Explain why `state.profile` is live: the setup step (rule n / fallback / manual / pin), then
 * pinned keys, claims and controls that come from base profiles.
 */
export function whyChain(c: Config, state: Pick<HubState, 'profile' | 'page' | 'setup' | 'reason' | 'active_app'>, claims: ClaimInfo[] = []): Why {
  const setup = setupByName(c, state.setup) ?? activeSetup(c, state);
  const steps: WhyStep[] = [];
  let reason = state.reason;
  if (!reason && setup.mode !== 'manual' && state.active_app) {
    const m = matchApp(c, setup, state.active_app);
    if (m.profile?.name === state.profile) reason = m.kind === 'rule' ? { kind: 'rule', pattern: m.hit!.pattern, index: m.hit!.index } : { kind: 'fallback' };
  }
  const app = state.active_app?.split('.').pop();
  if (reason?.kind === 'rule') steps.push({ level: 4, text: `Rule ${reason.index + 1} of ${setup.name}: ${reason.pattern} matches ${app ?? 'the focused app'}`, controls: [] });
  else if (reason?.kind === 'fallback') steps.push({ level: 5, text: `Fallback of ${setup.name}: no rule matches ${app ?? 'the focused app'}`, controls: [] });
  else if (reason?.kind === 'pin') steps.push({ level: 3, text: `Pinned in ${setup.name}`, controls: [] });
  else if (reason?.kind === 'manual' || setup.mode === 'manual') steps.push({ level: 3, text: `Chosen by hand in ${setup.name}`, controls: [] });
  else if (reason?.kind === 'action') steps.push({ level: 3, text: 'Switched by an action', controls: [] });

  const ctx: OwnerContext = { profile: state.profile, page: state.page, setup: setup.name, claims };
  const groups = new Map<string, WhyStep>();
  for (const ctl of allControls()) {
    const o = ownerOf(c, ctx, ctl, reason);
    let key: string;
    let step: WhyStep;
    if (o.kind === 'claim') [key, step] = [`claim:${o.claim.claim}`, { level: 1, text: `${o.claim.name} claims`, controls: [] }];
    else if (o.kind === 'pinned') [key, step] = ['pinned', { level: 2, text: 'Pinned keys from Global', controls: [] }];
    else if (o.kind === 'base') [key, step] = [`base:${o.profile}`, { level: 6, text: `From ${o.profile} (base of ${state.profile})`, controls: [] }];
    else continue;
    const g = groups.get(key) ?? (groups.set(key, step), step);
    g.controls.push(ctl);
  }
  for (const g of groups.values()) g.text += `: ${g.controls.map(label).join(', ')}`;
  steps.push(...[...groups.values()].sort((a, b) => a.level - b.level));
  return { profile: state.profile, setup, steps };
}

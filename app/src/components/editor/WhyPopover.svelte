<!--
  "Why Blender?": why the live profile is live, from the service's reason and the model's
  precedence (rule n of the setup, controls that come from a base profile, pinned keys, claims).
-->
<script lang="ts">
  import { CircleHelp, CornerLeftUp, Filter, Layers, Pin, Plug, Hand, Zap } from 'lucide-svelte';
  import { hub } from '../../lib/hub.svelte';
  import { allControls, ownerOf, setupByName, activeSetup, matchApp, type ControlRef } from '../../lib/model';
  import type { Reason } from '../../lib/types';
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import Popover from '../ui/Popover.svelte';
  import type { Icon } from '../ui/icon';
  import { describeControls, isPlural } from './controls';

  let open = $state(false);

  const hs = $derived(hub.state);
  const setup = $derived(setupByName(hub.config, hs.setup) ?? activeSetup(hub.config, hs));
  const app = $derived(hs.active_app?.split('.').pop());

  /** The service's reason, else worked out from the focused app. */
  const reason = $derived.by((): Reason | undefined => {
    if (hs.reason) return hs.reason;
    if (setup.mode !== 'manual' && hs.active_app) {
      const m = matchApp(hub.config, setup, hs.active_app);
      if (m.profile?.name === hs.profile) return m.kind === 'rule' ? { kind: 'rule', pattern: m.hit!.pattern, index: m.hit!.index } : { kind: 'fallback' };
    }
    return undefined;
  });

  interface Row {
    icon: Icon;
    title: string;
    sub: string;
    mono?: string;
  }
  const rows = $derived.by((): Row[] => {
    const out: Row[] = [];
    if (reason?.kind === 'rule') out.push({ icon: Layers, title: `Rule ${reason.index + 1} of ${setup.name}`, sub: 'Follow the focused app · first match wins' });
    else if (reason?.kind === 'fallback') out.push({ icon: Layers, title: `Fallback of ${setup.name}`, sub: `No rule in ${setup.name} matches ${app ?? 'the focused app'}` });
    else if (reason?.kind === 'pin') out.push({ icon: Pin, title: `Pinned in ${setup.name}`, sub: 'Stays until the pin is released' });
    else if (reason?.kind === 'manual' || setup.mode === 'manual') out.push({ icon: Hand, title: `Chosen by hand in ${setup.name}`, sub: 'Switched with a key, the dial or a shortcut' });
    else if (reason?.kind === 'action') out.push({ icon: Zap, title: 'Switched by an action', sub: 'A key, the dial or a shortcut picked it' });

    const ctx = { profile: hs.profile, page: hs.page, setup: setup.name, claims: hub.claims };
    const groups = new Map<string, { icon: Icon; ctls: ControlRef[]; sub: (n: number) => string; title?: string; level: number }>();
    for (const ctl of allControls()) {
      const o = ownerOf(hub.config, ctx, ctl, reason);
      let key: string;
      let g;
      if (o.kind === 'claim') [key, g] = [`claim:${o.claim.claim}`, { icon: Plug, ctls: [], level: 1, title: o.claim.name, sub: () => 'claims these while its app is focused' }];
      else if (o.kind === 'pinned') [key, g] = ['pinned', { icon: Pin, ctls: [], level: 2, sub: (n: number) => `${n > 1 ? 'are' : 'is'} pinned everywhere from Global` }];
      else if (o.kind === 'base') [key, g] = [`base:${o.profile}`, { icon: CornerLeftUp, ctls: [], level: 6, sub: (n: number) => `${n > 1 ? 'come' : 'comes'} from ${o.profile} (base profile)` }];
      else continue;
      (groups.get(key) ?? (groups.set(key, g), g)).ctls.push(ctl);
    }
    for (const g of [...groups.values()].sort((a, b) => a.level - b.level)) {
      out.push({ icon: g.icon, title: g.title ?? describeControls(g.ctls), sub: g.title ? `${describeControls(g.ctls)} ${g.sub(g.ctls.length)}` : g.sub(isPlural(g.ctls) ? 2 : 1) });
    }
    return out;
  });

  const because = $derived(
    reason?.kind === 'rule'
      ? { pre: 'because', mono: reason.pattern, post: 'is focused' }
      : reason?.kind === 'fallback'
        ? { pre: `because no rule matches ${app ?? 'the focused app'}`, mono: '', post: '' }
        : reason?.kind === 'pin'
          ? { pre: 'because it is pinned', mono: '', post: '' }
          : reason?.kind === 'manual'
            ? { pre: 'because you chose it', mono: '', post: '' }
            : { pre: 'because an action switched to it', mono: '', post: '' },
  );

  const toSetup = (tester: boolean) => {
    open = false;
    ui.go('setups', { view: 'setup-detail', setup: setup.name, ...(tester ? { tester: true } : {}) });
  };
</script>

<Popover bind:open label="Why {hs.profile}?" width={320}>
  {#snippet trigger({ toggle, attrs })}
    <Button size="sm" icon={CircleHelp} {...attrs} onclick={toggle}>Why {hs.profile}?</Button>
  {/snippet}
  {#snippet children()}
    <div class="why">
      <div class="row">
        <span class="ic"><Layers size={15} /></span>
        <div>
          <div class="title">{hs.profile} is active</div>
          <div class="sub">{because.pre}{#if because.mono} <code>{because.mono}</code> {because.post}{/if}</div>
        </div>
      </div>
      {#each rows as r}
        <div class="row">
          <span class="ic"><r.icon size={15} /></span>
          <div>
            <div class="title">{r.title}</div>
            <div class="sub">{r.sub}</div>
          </div>
        </div>
      {/each}
      <div class="foot">
        <Button size="sm" icon={Filter} onclick={() => toSetup(true)}>Open rule tester</Button>
        <Button size="sm" variant="ghost" onclick={() => toSetup(false)}>Edit rules</Button>
      </div>
    </div>
  {/snippet}
</Popover>

<style>
  .why {
    display: grid;
    gap: 14px;
    padding: 12px;
  }
  .row {
    display: flex;
    gap: 10px;
  }
  .ic {
    flex: none;
    margin-top: 1px;
    color: var(--color-text-3);
  }
  .title {
    font-weight: 600;
  }
  .sub {
    font-size: var(--type-small-size);
    color: var(--color-text-2);
  }
  code {
    font-family: var(--type-mono-family);
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-top: 10px;
    border-top: var(--panel-border-width) solid var(--color-border);
  }
</style>

<!-- Nothing selected: the profile itself: when it applies, what it is based on, which modules it
     sets and its LEDs. -->
<script lang="ts">
  import { AppWindow, CircleDot, Disc3, FlaskConical, Grid3x3, Pencil, RectangleHorizontal, SlidersHorizontal } from 'lucide-svelte';
  import { invoke } from '../../lib/backend';
  import { currentPage, currentProfile, edit, editor, hub } from '../../lib/hub.svelte';
  import { globMatch } from '../../lib/model/glob';
  import { activeSetup, profileByName, setupByName } from '../../lib/model/setups';
  import type { ModuleKind } from '../../lib/types';
  import { ui } from '../../lib/ui.svelte';
  import Avatar from '../ui/Avatar.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import InspectorSection from '../ui/InspectorSection.svelte';
  import Hint from './Hint.svelte';
  import { overview } from './overview';

  const profile = $derived(currentProfile());
  const setup = $derived(setupByName(hub.config, editor.setup) ?? activeSetup(hub.config, hub.state));
  const base = $derived(profileByName(hub.config, profile?.base));
  const info = $derived(profile ? overview(hub.config, profile.name, currentPage()?.name, setup.name) : undefined);

  const index = $derived(profile ? setup.profiles.indexOf(profile.name) : -1);
  const isFallback = $derived(!!profile && setup.fallback === profile.name);
  const patterns = $derived(profile?.apps ?? []);

  let windows = $state<string[]>([]);
  $effect(() => {
    invoke<string[]>('open_window_classes').then((w) => (windows = w)).catch(() => (windows = []));
  });
  const matches = (pattern: string) => windows.filter((w) => globMatch(pattern, w)).length;

  const MODULES: { kind: ModuleKind; label: string; icon: typeof Grid3x3 }[] = [
    { kind: 'keypad', label: 'Keypad', icon: Grid3x3 },
    { kind: 'dial', label: 'Dial', icon: Disc3 },
    { kind: 'knob', label: 'Knobs', icon: CircleDot },
    { kind: 'roller', label: 'Rollers', icon: RectangleHorizontal },
    { kind: 'fader', label: 'Faders', icon: SlidersHorizontal },
  ];
  const connected = $derived(new Set(hub.modules.map((m) => m.kind)));
  const modules = $derived(MODULES.filter((m) => connected.size === 0 || connected.has(m.kind)));

  const toRules = (tester = false) => ui.go('setups', { view: 'setup-detail', setup: setup.name, ...(tester ? { tester: true } : {}) });
  const toSettings = () => profile && ui.go('editor', { view: 'profile-settings', profile: profile.name });
  function overrideLeds(on: boolean) {
    const name = profile?.name;
    edit((c) => {
      const p = c.profile?.find((x) => x.name === name);
      if (p) p.leds = on ? JSON.parse(JSON.stringify(c.leds ?? {})) : undefined;
    }, { label: on ? 'override LEDs' : 'use global LEDs' });
  }
</script>

{#if !profile}
  <EmptyState title="No profile" body="Create a profile to start setting up the hub." />
{:else}
  <InspectorSection title="When it applies">
    {#if setup.mode === 'manual'}
      <p class="t-body line">Chosen by hand in {setup.name}, not by an app rule.</p>
    {:else if isFallback}
      <p class="t-body line">The fallback of {setup.name}: it applies when no rule matches.</p>
    {:else if patterns.length}
      <div class="rules">
        {#each patterns as pattern (pattern)}
          {@const n = matches(pattern)}
          <div class="rule">
            <AppWindow size={14} />
            <code class="t-mono pattern">{pattern}</code>
            <span class="t-small muted kind">window class</span>
            {#if n}<Chip variant="success">matches {n} open window{n === 1 ? '' : 's'}</Chip>{/if}
          </div>
        {/each}
      </div>
      <p class="t-small line">
        {#if index >= 0}Rule {index + 1} of {setup.profiles.length} in {setup.name}.{:else}Not in {setup.name}.{/if}
        {#if setup.fallback && setup.fallback !== profile.name}{setup.fallback} applies when nothing matches.{/if}
      </p>
    {:else}
      <p class="t-body line">No app rule yet, so it only applies when you pick it.</p>
    {/if}
    <div class="buttons">
      <Button size="sm" icon={Pencil} onclick={() => toRules()}>Edit rules</Button>
      <Button size="sm" variant="ghost" icon={FlaskConical} onclick={() => toRules(true)}>Test a window</Button>
    </div>
  </InspectorSection>

  {#if base && info}
    <InspectorSection title="Based on">
      <div class="based">
        <Avatar name={base.name} color={base.color} size={22} />
        <strong class="t-body">{base.name}</strong>
        <span class="t-body muted">{info.inherited.length ? `gives ${info.inherited.join(', ').toLowerCase()}` : 'sets everything this profile does not'}</span>
      </div>
      {#if info.inherited.length}
        <div class="chips">{#each info.inherited as c (c)}<Chip variant="linked">{c}</Chip>{/each}</div>
      {/if}
    </InspectorSection>
  {/if}

  {#if info}
    <InspectorSection title="Modules used">
      <ul class="modules">
        {#each modules as m (m.kind)}
          <li><m.icon size={14} /><span class="t-body name">{m.label}</span><span class="t-small muted">{info.modules[m.kind]}</span></li>
        {/each}
      </ul>
    </InspectorSection>
  {/if}

  <InspectorSection title="LEDs" divider={false}>
    {#if profile.leds}
      <div class="leds"><Chip variant="accent" icon={null}>Custom for {profile.name}</Chip><Button size="sm" variant="ghost" onclick={() => overrideLeds(false)}>Use global</Button></div>
      <Hint>Set the colours from each knob, fader or the base module.</Hint>
    {:else}
      <div class="leds"><Chip variant="outline" icon={null}>Uses global colours</Chip><Button size="sm" variant="ghost" onclick={() => overrideLeds(true)}>Override</Button></div>
    {/if}
    {#if profile}<div class="settings"><Button size="sm" variant="ghost" onclick={toSettings}>All profile settings…</Button></div>{/if}
  </InspectorSection>
{/if}

<style>
  .line {
    margin: 0;
    color: var(--color-text-2);
  }
  .rules {
    display: grid;
    gap: 6px;
  }
  .rule {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    padding: 8px 10px;
    color: var(--color-text-2);
    background: var(--color-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-panel);
  }
  .pattern {
    color: var(--color-text-1);
  }
  .kind {
    flex: 1;
  }
  .buttons,
  .leds {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .leds {
    justify-content: space-between;
  }
  .based {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .modules {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .modules li {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 32px;
    color: var(--color-text-2);
  }
  .modules .name {
    flex: 1;
    color: var(--color-text-1);
  }
  .settings {
    display: flex;
    justify-content: flex-start;
  }
</style>

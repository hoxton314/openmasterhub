<!-- First run (S29a–e, Flow-1): welcome, system checks, modules found, a starting setup, done.
  Mounted by App while `ui.firstRun` is true; `trigger.svelte.ts` sets it for a config without `app.first_run_done`. -->
<script lang="ts">
  import { ArrowRight, Check, FileText, Layers, MousePointerClick, ScanSearch } from 'lucide-svelte';
  import { onMount, untrack } from 'svelte';
  import { invoke } from '../../lib/backend';
  import { edit, hasCapability, hub, setSetup, toast } from '../../lib/hub.svelte';
  import { effectiveSetups, profileByName } from '../../lib/model';
  import type { ModuleKind, Profile } from '../../lib/types';
  import { ui } from '../../lib/ui.svelte';
  import { saveApp } from '../settings/actions';
  import Button from '../ui/Button.svelte';
  import CheckRow from '../ui/CheckRow.svelte';
  import Chip from '../ui/Chip.svelte';
  import Kbd from '../ui/Kbd.svelte';
  import ProgressSteps from '../ui/ProgressSteps.svelte';
  import TemplateCard from '../ui/TemplateCard.svelte';
  import TemplatePreview from '../profiles/TemplatePreview.svelte';
  import { apply, build, STARTINGS, type StartingId } from './starting';
  import './trigger.svelte';

  interface Check {
    id: string;
    title: string;
    detail?: string;
    status: 'success' | 'warning' | 'danger' | 'off';
    label: string;
    /** A link or short instruction for a warning. */
    help?: string;
  }

  const STEPS = ['Welcome', 'Checks', 'Modules', 'Setup', 'Done'];
  /** Dev only: `?frstep=3` opens the flow at that step, for design checks. */
  let step = $state(import.meta.env.DEV ? Number(new URLSearchParams(location.search).get('frstep')) || 0 : 0);
  let checks = $state<Check[]>([]);
  let openClasses = $state<string[]>([]);
  let choice = $state<StartingId>('work');
  let created = $state<string | undefined>();
  let heading: HTMLElement | undefined = $state();

  onMount(async () => {
    checks = (await invoke<Check[]>('system_checks').catch(() => undefined)) ?? [
      { id: 'service', title: 'masterhubd service', detail: 'started with your session', status: hub.online ? 'success' : 'danger', label: hub.online ? 'Running' : 'Stopped' },
    ];
    openClasses = (await invoke<string[]>('open_window_classes').catch(() => undefined)) ?? [];
  });

  $effect(() => {
    void step;
    untrack(() => heading)?.focus();
  });

  const KINDS: Record<ModuleKind, string> = { base: 'Base', keypad: 'Keypad', dial: 'Dial', knob: 'Knobs', roller: 'Rollers', fader: 'Faders' };
  const where = (m: (typeof hub.modules)[number]) => (m.placement ? `col ${m.placement.col + 1}, row ${m.placement.row + 1}${m.placement.rotation ? `, ${m.placement.rotation}°` : ''}` : 'not placed');
  const hasWarnings = $derived(checks.some((c) => c.status === 'warning' || c.status === 'danger'));
  const found = $derived(build('work', openClasses).profiles.filter((p) => p.apps?.length).map((p) => p.name));

  /** The hub drawn on the modules step: the modules found, no keys yet. */
  const found_modules = $derived<Profile>({
    name: '',
    page: [{ name: 'Main', key: [] }],
    ...(hub.modules.some((m) => m.kind === 'dial') ? { dial: {} } : {}),
    ...(hub.modules.some((m) => m.kind === 'knob') ? { knob: [{ index: 0 }] } : {}),
    ...(hub.modules.some((m) => m.kind === 'fader') ? { fader: [{ index: 0 }] } : {}),
    ...(hub.modules.some((m) => m.kind === 'roller') ? { roller: [{ side: 'left' as const }] } : {}),
  });
  /** The first profile of what was just created, for the last step. */
  const createdProfile = $derived.by(() => {
    const s = created ? effectiveSetups(hub.config).find((x) => x.name === created) : undefined;
    return profileByName(hub.config, s?.profiles[0] ?? s?.fallback ?? 'Desktop') ?? profileByName(hub.config, 'Desktop');
  });

  function finish() {
    saveApp({ first_run_done: true }, 'finish first run');
    ui.firstRun = false;
  }

  function create() {
    let name: string | undefined;
    edit(
      (c) => {
        name = apply(c, choice, openClasses);
        c.app = { ...c.app, first_run_done: true };
      },
      { label: 'starting setup' },
    );
    created = name ?? 'Desktop';
    if (name && hasCapability('setups')) void setSetup(name);
    step = 4;
  }

  function open() {
    ui.firstRun = false;
    ui.go('editor');
  }

  const identify = () => {
    if (!hasCapability('identify')) return toast('This service cannot flash modules yet');
    for (const m of hub.modules) invoke('identify', { serial: m.serial }).catch(() => undefined);
  };
</script>

<div class="first" role="dialog" aria-modal="true" aria-label="Welcome to OpenMasterHUB">
  <div class="card">
  <header>
    <ProgressSteps count={STEPS.length} current={step} labels={STEPS} />
    {#if step < 4}<Button variant="ghost" size="sm" onclick={finish}>Skip setup</Button>{/if}
  </header>

  <div class="body">
    {#if step === 0}
      <h1 class="t-display" tabindex="-1" bind:this={heading}>Welcome to OpenMasterHUB</h1>
      <p class="lead">An open driver and editor for your MasterHUB. Setup takes about a minute.</p>
      <ul class="promises">
        <li><MousePointerClick size={20} /><span><b>Click the hub to edit</b><br /><span class="muted t-small">Every key, the dial, knobs and rollers</span></span></li>
        <li><Layers size={20} /><span><b>Profiles follow your apps</b><br /><span class="muted t-small">Or switch by hand while streaming</span></span></li>
        <li><FileText size={20} /><span><b>One plain text file</b><br /><span class="muted t-small">No account, nothing leaves this PC</span></span></li>
      </ul>
      <Button variant="primary" iconRight={ArrowRight} onclick={() => (step = 1)}>Get started</Button>
    {:else if step === 1}
      <h1 class="t-title" tabindex="-1" bind:this={heading}>Checking your system</h1>
      <p class="lead">Each check explains itself. You can continue with warnings.</p>
      <div class="checks">
        {#each checks as c (c.id)}
          <CheckRow title={c.title} detail={c.detail} status={c.status} statusLabel={c.label}>
            {#snippet action()}{#if c.help}<span class="t-small muted">{c.help}</span>{/if}{/snippet}
          </CheckRow>
        {/each}
      </div>
      <Button variant="primary" iconRight={ArrowRight} onclick={() => (step = 2)}>{hasWarnings ? 'Continue anyway' : 'Continue'}</Button>
    {:else if step === 2}
      <h1 class="t-title" tabindex="-1" bind:this={heading}>{hub.modules.length ? 'Found your hub' : 'No hub found yet'}</h1>
      <p class="lead">
        {hub.modules.length ? 'The service reports these modules. You can move them in the editor, and the drawing follows.' : 'Plug the MasterHUB in and the modules appear here. You can also continue and connect it later.'}
      </p>
      <div class="split">
        {#if hub.modules.length}<div class="hubview" aria-hidden="true"><TemplatePreview profile={found_modules} unit={30} /></div>{/if}
        <div class="modules">
          {#each hub.modules as m (m.serial)}
            <div class="module">
              <Check size={14} />
              <b>{KINDS[m.kind] ?? m.kind}</b>
              <span class="t-small muted">{where(m)}</span>
              {#if m.needs_replug}<Chip variant="warning">needs a replug</Chip>{/if}
            </div>
          {/each}
        </div>
      </div>
      <div class="buttons">
        <Button icon={ScanSearch} onclick={identify} disabled={!hub.modules.length}>Identify modules</Button>
        <Button variant="primary" iconRight={ArrowRight} onclick={() => (step = 3)}>{hub.modules.length ? 'Looks right' : 'Continue'}</Button>
      </div>
    {:else if step === 3}
      <h1 class="t-title" tabindex="-1" bind:this={heading}>Pick a starting setup</h1>
      <p class="lead">You can add the others later from Setups.</p>
      <div class="cards" role="radiogroup" aria-label="Starting setup">
        {#each STARTINGS as s (s.id)}
          <TemplateCard title={s.title} description={s.about} selected={choice === s.id} onclick={() => (choice = s.id)}>
            {#snippet preview()}<span class="glyph"><s.icon size={28} /></span>{/snippet}
          </TemplateCard>
        {/each}
      </div>
      {#if choice === 'work' && found.length}
        <p class="t-small muted">Found open now: {found.join(', ')}. Work will include matching profiles.</p>
      {/if}
      <Button variant="primary" iconRight={ArrowRight} onclick={create}>Create {STARTINGS.find((s) => s.id === choice)?.title}</Button>
    {:else}
      <div class="done"><Check size={28} /></div>
      <h1 class="t-title" tabindex="-1" bind:this={heading}>You’re set</h1>
      <p class="lead">Changes apply to the hub as you make them.</p>
      <div class="split">
        {#if createdProfile}<div class="hubview" aria-hidden="true"><TemplatePreview profile={createdProfile} unit={30} /></div>{/if}
        <div class="after">
          <p class="t-small muted">{created && created !== 'Desktop' ? `${created} is live. ` : ''}Try a key on the hub.</p>
          <ul class="tips">
            <li><Kbd>Ctrl</Kbd><Kbd>K</Kbd><span>finds anything</span></li>
            <li><Kbd>Ctrl</Kbd><Kbd>Z</Kbd><span>undoes any change</span></li>
          </ul>
        </div>
      </div>
      <Button variant="primary" iconRight={ArrowRight} onclick={open}>Open the editor</Button>
    {/if}
  </div>
  </div>
</div>

<style>
  .first {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: var(--space-4);
    background: var(--color-bg);
    animation: fade-in var(--motion-panel);
  }
  .card {
    width: min(840px, 100%);
    max-height: 100%;
    display: flex;
    flex-direction: column;
    padding: var(--space-6) 32px 28px;
    gap: var(--space-4);
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-popover);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    align-content: start;
    justify-items: start;
    gap: var(--space-4);
  }
  .split {
    display: flex;
    align-items: center;
    gap: var(--space-6);
    width: 100%;
  }
  .hubview {
    flex: none;
    padding: var(--space-3);
    background: var(--hub-base);
    border-radius: var(--radius-lg);
    pointer-events: none;
  }
  .split .modules,
  .after {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: var(--space-2);
  }
  .module :global(svg) {
    color: var(--color-success);
    flex: none;
  }
  h1,
  p {
    margin: 0;
  }
  h1:focus {
    outline: none;
  }
  .lead {
    color: var(--color-text-2);
  }
  .promises,
  .tips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-3);
    width: 100%;
  }
  .promises li {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
    background: var(--color-sunken);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-card);
  }
  .promises li :global(svg) {
    color: var(--color-accent);
    flex: none;
  }
  .checks,
  .modules,
  .cards {
    display: grid;
    gap: var(--space-2);
    width: 100%;
  }
  .cards {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-3);
  }
  .glyph {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--color-accent);
  }
  .module {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    background: var(--color-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-row);
  }
  .module b {
    min-width: 72px;
  }
  .buttons {
    display: flex;
    gap: var(--space-2);
  }
  .done {
    width: 56px;
    height: 56px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-pill);
    color: var(--color-success);
    background: var(--color-success-soft);
  }
  .tips li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-text-2);
  }
</style>

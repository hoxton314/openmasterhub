<!-- Claude Code integration (S24): hooks, `[status]` colours and sort, sessions now, the session
  keys profile and the pinned key. -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { edit, editProfile, hub, toast } from '../../lib/hub.svelte';
  import { integ, refreshHooks, refreshSessions, setHooks, STATUS_DEFAULTS } from '../../lib/integrations.svelte';
  import { setupsOf } from '../../lib/model';
  import type { Config, StatusLeds } from '../../lib/types';
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import ColorField from '../ui/ColorField.svelte';
  import Field from '../ui/Field.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import StatusDot from '../ui/StatusDot.svelte';
  import Toggle from '../ui/Toggle.svelte';

  onMount(() => {
    refreshHooks();
    refreshSessions();
    const t = setInterval(() => (refreshHooks(), refreshSessions()), 5000);
    return () => clearInterval(t);
  });

  const status = $derived(hub.config.status ?? {});
  const hooks = $derived(integ.hooks);

  function setStatus(patch: Partial<StatusLeds>, label: string) {
    edit(
      (c: Config) => {
        c.status = { ...c.status, ...patch };
        for (const k of Object.keys(c.status) as (keyof StatusLeds)[]) if (c.status[k] === undefined) delete c.status[k];
      },
      { label, coalesce: `status-${Object.keys(patch)[0]}` },
    );
  }

  const COLOURS = [
    { key: 'attention', label: 'Waiting for you', hint: 'Base LED flashes' },
    { key: 'ready', label: 'Ready', hint: 'Steady on the key' },
    { key: 'working', label: 'Working', hint: 'Mascot types on the key' },
  ] as const;

  const SORTS = [
    { value: 'urgent', label: 'Needs me' },
    { value: 'changed', label: 'Last changed' },
    { value: 'started', label: 'Opened' },
  ] as const;
  const SORT_HELP = { urgent: 'Waiting for you first, then finished, then working.', changed: 'The most recent status change first.', started: 'Oldest first, so keys stay where they are.' };

  const DOT = { attention: 'warning', ready: 'success', working: 'accent' } as const;
  const WORD = { attention: 'Waiting', ready: 'Ready', working: 'Working' } as const;

  async function hooksAction(install: boolean) {
    const err = await setHooks(install);
    toast(err ? { text: err, kind: 'danger' } : { text: install ? 'Hooks installed in ~/.claude/settings.json' : 'Hooks removed', kind: 'success' });
  }

  // The profile with session keys, and where it is used.
  const sessionProfile = $derived(hub.config.profile?.find((p) => [...(p.page ?? []), ...(p.folder ?? [])].some((pg) => pg.key?.some((k) => k.widget?.kind === 'claude_session'))));
  const usedIn = $derived(
    sessionProfile
      ? setupsOf(hub.config, sessionProfile.name).map((s) => {
          const i = s.profiles.indexOf(sessionProfile.name);
          return i >= 0 ? `${s.name} · rule ${i + 1}` : `${s.name} · fallback`;
        })
      : [],
  );
  const pinned = $derived(hub.config.pinned?.find((p) => p.press && 'profile' in p.press && sessionProfile && p.press.profile === sessionProfile.name));

  function ago(s: number | null) {
    if (s === null) return 'no event yet';
    if (s < 60) return `${s} s ago`;
    if (s < 3600) return `${Math.round(s / 60)} min ago`;
    return `${Math.round(s / 3600)} h ago`;
  }
</script>

<div class="page">
  <header>
    <div class="head">
      <h1 class="t-title">Claude Code</h1>
      {#if hooks === null}<Chip variant="neutral">Checking</Chip>
      {:else if hooks.installed}<Chip variant="success">On{integ.sessions.length ? ` · ${integ.sessions.length} session${integ.sessions.length === 1 ? '' : 's'}` : ''}</Chip>
      {:else}<Chip variant="warning">Hooks missing</Chip>{/if}
    </div>
    <p class="t-body muted">Base LED and keys follow your Claude Code sessions.</p>
  </header>

  <section aria-label="Hooks">
    <SectionLabel as="h2">Hooks</SectionLabel>
    <p class="t-small muted">Claude Code tells the service when a session changes state.</p>
    {#if hooks?.installed}
      <div class="line"><Chip variant="success">Installed</Chip><span class="t-small muted"><code class="t-mono">{hooks.path}</code> · {hooks.hooks} hooks</span></div>
      <div class="line">
        {#if hooks.last_event_secs !== null}<Chip variant="success">Receiving</Chip>{:else}<Chip variant="warning">Waiting</Chip>{/if}
        <span class="t-small muted">last event {ago(hooks.last_event_secs)}</span>
      </div>
      <div class="line">
        <Button size="sm" disabled={integ.hooksBusy} onclick={() => hooksAction(true)}>Reinstall hooks</Button>
        <Button size="sm" variant="danger" disabled={integ.hooksBusy} onclick={() => hooksAction(false)}>Remove</Button>
      </div>
    {:else if hooks}
      <div class="line"><Chip variant="warning">Not installed</Chip><span class="t-small muted">Adds 11 hooks to <code class="t-mono">{hooks.path}</code>; your own hooks stay.</span></div>
      <div class="line"><Button variant="primary" size="sm" disabled={integ.hooksBusy} onclick={() => hooksAction(true)}>Install hooks</Button></div>
    {/if}
  </section>

  <section aria-label="Colours">
    <SectionLabel as="h2">Colours</SectionLabel>
    <div class="line spread">
      <span class="t-body">Show status on the Base LED</span>
      <Toggle label="Show status on the Base LED" checked={status.enabled !== false} onchange={(v) => setStatus({ enabled: v ? undefined : false }, 'Base LED status')} />
    </div>
    {#each COLOURS as c (c.key)}
      <div class="line spread">
        <div>
          <div class="t-body">{c.label}</div>
          <div class="t-small muted">{c.hint}</div>
        </div>
        <ColorField
          label={c.label}
          value={status[c.key]}
          fallback={STATUS_DEFAULTS[c.key]}
          swatches={[STATUS_DEFAULTS[c.key]]}
          onchange={(v) => setStatus({ [c.key]: v }, `${c.label} colour`)}
        />
      </div>
    {/each}
  </section>

  <section aria-label="Sessions now">
    <SectionLabel as="h2">Sessions now</SectionLabel>
    {#if integ.sessions.length}
      <ol class="sessions">
        {#each integ.sessions as s, i (s.name)}
          <li>
            <span class="t-small faint num">{i + 1}</span>
            <span class="t-body grow">{s.name}</span>
            <StatusDot status={DOT[s.status]} label={WORD[s.status]} />
            <span class="t-small muted">{WORD[s.status]}</span>
          </li>
        {/each}
      </ol>
      <p class="t-small muted">Shown in the sort order below. The <strong>Claude</strong> widget shows one session per key; the <strong>session</strong> action brings it up, and <strong>sessions</strong> pages through them.</p>
    {:else}
      <p class="t-small muted">No Claude Code session is open{hooks && !hooks.installed ? ': install the hooks first' : ''}.</p>
    {/if}
    <Field label="Sort by" help={SORT_HELP[status.sort ?? 'urgent']}>
      <Segmented label="Sort sessions by" options={[...SORTS]} value={status.sort ?? 'urgent'} onchange={(v) => setStatus({ sort: v === 'urgent' ? undefined : v }, 'Session sort')} />
    </Field>
  </section>

  <section aria-label="Session keys profile">
    <SectionLabel as="h2">Session keys profile</SectionLabel>
    <p class="t-small muted">Each open session gets a key. The left roller pages through them when there are more sessions than keys.</p>
    {#if sessionProfile}
      <div class="line spread">
        <span class="t-body">{sessionProfile.name} <span class="muted">· {usedIn.join(', ') || 'in no setup'}</span></span>
        <Button size="sm" onclick={() => (editProfile(sessionProfile.name), ui.go('editor'))}>Open in editor</Button>
      </div>
    {:else}
      <p class="t-small muted">No profile has a Claude widget yet. Add the Claude profile from <button class="link" onclick={() => ui.openNewProfile({})}>New profile</button>.</p>
    {/if}
  </section>

  {#if pinned}
    <section aria-label="Pinned key">
      <SectionLabel as="h2">Pinned key</SectionLabel>
      <div class="line spread">
        <span class="t-body">Key {pinned.index + 1} switches to {sessionProfile?.name}{pinned.except?.length ? `, everywhere except ${pinned.except.join(', ')}` : ''}.</span>
        <Button size="sm" onclick={() => ui.go('global')}>Edit in Global</Button>
      </div>
    </section>
  {/if}
</div>

<style>
  .page {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 24px;
    max-width: 800px;
    width: 100%;
    margin: 0 auto;
    box-sizing: border-box;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  h1,
  p {
    margin: 0;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    background: var(--color-surface);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-hairline);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .spread {
    justify-content: space-between;
  }
  .grow {
    flex: 1;
  }
  code {
    padding: 1px 5px;
    background: var(--color-sunken);
    border-radius: var(--radius-sm);
  }
  .sessions {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .sessions li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    background: var(--color-sunken);
    border-radius: var(--radius-row);
  }
  .num {
    width: 16px;
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--color-accent);
    cursor: pointer;
    font: inherit;
    text-decoration: underline;
  }
</style>

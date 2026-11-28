<!-- Edits one action: a kind picker (with filter) and the editor for the chosen kind. Each kind keeps
     its last settings while you compare. Controls the running service cannot do are disabled with
     a hint (`hasCapability`). -->
<script lang="ts">
  import { untrack } from 'svelte';
  import { ChevronDown, CircleCheck, CircleX, Info, Play } from 'lucide-svelte';
  import { invoke } from '../../lib/backend';
  import { currentProfile, hasCapability, hub } from '../../lib/hub.svelte';
  import { effectiveSetups } from '../../lib/model/setups';
  import type { Action } from '../../lib/types';
  import Button from '../ui/Button.svelte';
  import KeyCapture from '../ui/KeyCapture.svelte';
  import NumberField from '../ui/NumberField.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import Select from '../ui/Select.svelte';
  import { KINDS, kindAvailable, kindIcon, kindLabel, kindOf, type Kind } from './actions';
  import AudioEditor from './AudioEditor.svelte';
  import Hint from './Hint.svelte';
  import RadioList from './RadioList.svelte';

  let {
    value,
    onchange,
    fader = false,
    liveValue,
    defaultVolume,
  }: {
    value: Action | undefined;
    onchange: (a: Action | undefined) => void;
    /** A fader action: `{value}` is the position (and a test run substitutes it). */
    fader?: boolean;
    /** The fader's live position, for the `{value}` hint. */
    liveValue?: number;
    /** What a fresh volume action says; faders follow the fader, knobs step. */
    defaultVolume?: string;
  } = $props();

  const volumeDefault = $derived(defaultVolume ?? (fader ? '{value}' : '+2'));
  const WINDOWS = navigator.userAgent.includes('Windows');
  const NOT_SUPPORTED = 'The running service is too old for this. Update masterhubd.';

  let kind = $state<Kind | undefined>(untrack(() => kindOf(value)));
  let picker = $state(untrack(() => !value));
  let filter = $state('');
  let filterBox: HTMLInputElement | undefined = $state();
  /** The last action of each kind, restored when switching back to it. */
  const drafts: Partial<Record<Kind, Action | undefined>> = {};

  $effect(() => {
    const k = kindOf(value);
    if (!k) return;
    if (k !== kind) kind = k;
    drafts[k] = JSON.parse(JSON.stringify(value));
  });
  $effect(() => {
    if (picker) filterBox?.focus();
  });

  const pages = $derived(currentProfile()?.page ?? []);
  const folders = $derived(currentProfile()?.folder ?? []);
  const profileNames = $derived((hub.config.profile ?? []).map((p) => p.name));
  const setupNames = $derived(effectiveSetups(hub.config).map((s) => s.name));
  const cycle = $derived(hasCapability('profile_cycle'));
  const setupAction = $derived(hasCapability('setup_action'));
  const pin = $derived(hasCapability('pin'));

  function initial(k: Kind): Action | undefined {
    switch (k) {
      case 'volume': return { volume: volumeDefault };
      case 'mute': return { mute: 'toggle' };
      case 'page': return { page: 'next' };
      case 'folder': return folders[0] ? { folder: folders[0].name } : undefined;
      case 'back': return { back: true };
      case 'profile': return cycle ? { profile: 'next' } : profileNames[0] ? { profile: profileNames[0] } : undefined;
      case 'setup': return setupAction ? { setup: 'next' } : pin ? { pin: 'toggle' } : undefined;
      case 'session': return { session: 0 };
      default: return undefined;
    }
  }

  function choose(k: Kind) {
    if (!kindAvailable(k)) return;
    kind = k;
    picker = false;
    filter = '';
    onchange(k in drafts ? drafts[k] : initial(k));
  }

  const shown = $derived(KINDS.filter((x) => x.label.toLowerCase().includes(filter.trim().toLowerCase())));
  function filterKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      const first = shown.find((x) => kindAvailable(x.k));
      if (first) choose(first.k);
    } else if (e.key === 'Escape' && (filter || value)) {
      e.stopPropagation();
      filter = '';
      picker = false;
    }
  }

  // ── keys ──
  const keys = $derived(value && 'keys' in value ? (typeof value.keys === 'string' ? { combo: value.keys, hold: undefined } : value.keys) : undefined);
  const setKeys = (combo: string, hold?: number) => onchange({ keys: hold ? { combo, hold } : combo });

  // ── command ──
  const cmd = $derived(value && 'cmd' in value ? value.cmd : '');
  let test = $state<{ ok: boolean; text: string } | undefined>();
  let testing = $state(false);
  async function runTest() {
    if (!cmd.trim()) return;
    testing = true;
    test = undefined;
    try {
      const r = await invoke<{ code: number | null; ms: number; output?: string }>('run_command_test', { cmd, ...(fader ? { value: liveValue ?? 50 } : {}) });
      const out = (r.output ?? '').trim();
      test = { ok: r.code === 0, text: `exit ${r.code ?? '?'} · ${r.ms} ms · ${out ? out.split('\n')[0].slice(0, 120) : 'no output'}` };
    } catch (e) {
      test = { ok: false, text: String(e).includes('unknown') || String(e).includes('not found') ? 'Test run needs a newer app' : String(e) };
    }
    testing = false;
  }

  // ── pages, folders, back ──
  type Nav = 'next' | 'prev' | 'goto' | 'folder' | 'back';
  const nav = $derived<Nav>(
    kind === 'folder' ? 'folder' : kind === 'back' ? 'back' : value && 'page' in value ? (value.page === 'next' ? 'next' : value.page === 'prev' ? 'prev' : 'goto') : 'next',
  );
  function setNav(n: Nav) {
    if (n === 'next') choose2('page', { page: 'next' });
    else if (n === 'prev') choose2('page', { page: 'prev' });
    else if (n === 'goto') choose2('page', pages[0] ? { page: pages[0].name } : undefined);
    else if (n === 'folder') choose2('folder', folders[0] ? { folder: folders[0].name } : undefined);
    else choose2('back', { back: true });
  }
  function choose2(k: Kind, a: Action | undefined) {
    kind = k;
    onchange(a);
  }

  // ── profile / setup ──
  const profileMode = $derived<'next' | 'prev' | 'name' | 'pick' | undefined>(
    value && 'profile' in value ? (value.profile === 'next' || value.profile === 'prev' || value.profile === 'pick' ? value.profile : 'name') : undefined,
  );
  let lastProfile = $state(untrack(() => (value && 'profile' in value ? value.profile : '')));
  $effect(() => {
    if (profileMode === 'name' && value && 'profile' in value) lastProfile = value.profile;
  });
  function setProfileMode(m: 'next' | 'prev' | 'name' | 'pick') {
    onchange({ profile: m === 'name' ? lastProfile && !['next', 'prev', 'pick'].includes(lastProfile) ? lastProfile : (profileNames[0] ?? '') : m });
  }

  type SetupMode = 'next' | 'prev' | 'name' | 'pick' | 'pin' | 'release';
  const setupMode = $derived<SetupMode | undefined>(
    value && 'setup' in value ? (value.setup === 'next' || value.setup === 'prev' || value.setup === 'pick' ? value.setup : 'name') : value && 'pin' in value ? (value.pin === 'release' ? 'release' : 'pin') : undefined,
  );
  let lastSetup = $state(untrack(() => (value && 'setup' in value ? value.setup : '')));
  $effect(() => {
    if (setupMode === 'name' && value && 'setup' in value) lastSetup = value.setup;
  });
  function setSetupMode(m: SetupMode) {
    if (m === 'pin' || m === 'release') onchange({ pin: m === 'pin' ? 'toggle' : 'release' });
    else onchange({ setup: m === 'name' ? (lastSetup && !['next', 'prev', 'pick'].includes(lastSetup) ? lastSetup : (setupNames[0] ?? '')) : m });
  }

  // ── Claude sessions ──
  const sessionMode = $derived<'bring' | 'page'>(value && 'sessions' in value ? 'page' : 'bring');
</script>

<div class="editor">
  <button type="button" class="kindbar" class:open={picker} aria-expanded={picker} onclick={() => (picker = !picker)}>
    {#if kind}{@const I = kindIcon(kind)}<span class="ico"><I size={15} /></span><span class="t-body name">{kindLabel(kind)}</span>{:else}<span class="t-body name none">Choose a kind</span>{/if}
    <ChevronDown size={14} />
  </button>

  {#if picker}
    <div class="picker">
      <input class="filter" bind:this={filterBox} bind:value={filter} placeholder="Filter kinds" aria-label="Filter kinds" onkeydown={filterKey} />
      <div class="grid" role="radiogroup" aria-label="Action kind">
        {#each shown as x (x.k)}
          {@const avail = kindAvailable(x.k)}
          <button type="button" class="kind" class:on={kind === x.k} disabled={!avail} title={avail ? undefined : NOT_SUPPORTED} role="radio" aria-checked={kind === x.k} onclick={() => choose(x.k)}>
            <x.icon size={16} />
            <span class="t-chip">{x.label}</span>
          </button>
        {/each}
        {#if !shown.length}<span class="t-small none">No kind matches.</span>{/if}
      </div>
      <Hint>Type to filter · each kind keeps its last settings while you compare</Hint>
    </div>
  {:else if kind === 'keys'}
    <KeyCapture
      value={keys?.combo}
      hold={keys?.hold}
      onchange={(c) => setKeys(c, keys?.hold)}
      onhold={hasCapability('hold') && keys ? (ms) => setKeys(keys.combo, ms) : undefined}
    />
  {:else if kind === 'cmd'}
    <div class="stack">
      <label class="cap t-small" for="cmd-text">Command</label>
      <textarea id="cmd-text" class="cmd" rows="3" spellcheck="false" value={cmd} placeholder={WINDOWS ? 'start notepad' : 'xdg-open ~'} oninput={(e) => onchange(e.currentTarget.value ? { cmd: e.currentTarget.value } : undefined)}></textarea>
      <div class="test">
        <Button size="sm" icon={Play} disabled={testing || !cmd.trim()} onclick={runTest}>{testing ? 'Running…' : 'Test run'}</Button>
      </div>
      {#if test}
        <div class="result" class:bad={!test.ok} role="status">
          {#if test.ok}<CircleCheck size={14} />{:else}<CircleX size={14} />{/if}<span class="t-mono">{test.text}</span>
        </div>
      {/if}
      <Hint>
        <Info size={12} class="inl" /> Runs with <code>{WINDOWS ? 'cmd /C' : 'sh -c'}</code> from your home folder. Faders replace <code>{'{value}'}</code> with 0–100.
        {#if fader && liveValue !== undefined}Right now: <b>{liveValue}</b>.{/if}
      </Hint>
    </div>
  {:else if kind === 'volume' || kind === 'mute'}
    <AudioEditor {value} {kind} defaultVolume={volumeDefault} {onchange} onkind={(k) => (kind = k)} />
  {:else if kind === 'page' || kind === 'folder' || kind === 'back'}
    <div class="stack">
      <Segmented
        full
        label="Page action"
        value={nav}
        onchange={setNav}
        options={[
          { value: 'next', label: 'Next' },
          { value: 'prev', label: 'Previous' },
          { value: 'goto', label: 'Go to' },
          { value: 'folder', label: 'Folder', disabled: !folders.length, title: folders.length ? undefined : 'This profile has no folders' },
          { value: 'back', label: 'Back' },
        ]}
      />
      {#if nav === 'goto'}
        <Select label="Page" value={value && 'page' in value ? value.page : undefined} onchange={(p) => onchange({ page: p })} options={pages.map((p, i) => ({ value: p.name, label: `${p.name} · page ${i + 1}` }))} />
      {:else if nav === 'folder'}
        <Select label="Folder" value={value && 'folder' in value ? value.folder : undefined} onchange={(f) => onchange({ folder: f })} options={folders.map((f) => ({ value: f.name, label: f.name }))} />
        <Hint>Folders open on top of the page. Their Back key returns here.</Hint>
      {:else if nav === 'back'}
        <Hint>Leaves the folder and shows the page below it.</Hint>
      {:else}
        <Hint>Next page skips folders.</Hint>
      {/if}
    </div>
  {:else if kind === 'profile' || kind === 'setup'}
    <div class="stack">
      <Segmented
        full
        label="Switch"
        value={kind}
        onchange={(k) => choose(k)}
        options={[{ value: 'profile', label: 'Profile' }, { value: 'setup', label: 'Setup', disabled: !kindAvailable('setup'), title: kindAvailable('setup') ? undefined : NOT_SUPPORTED }]}
      />
      {#if kind === 'profile'}
        <RadioList
          label="Profile switch"
          value={profileMode}
          onchange={setProfileMode}
          options={[
            { value: 'next', label: 'Next profile in the setup', disabled: !cycle, title: cycle ? undefined : NOT_SUPPORTED },
            { value: 'prev', label: 'Previous profile', disabled: !cycle, title: cycle ? undefined : NOT_SUPPORTED },
            { value: 'name', label: 'A specific profile' },
            { value: 'pick', label: 'Pick on the dial', desc: 'Turn to choose, press to switch', disabled: !cycle, title: cycle ? undefined : NOT_SUPPORTED },
          ]}
        >
          {#snippet detail(m)}
            {#if m === 'name'}
              <Select size="sm" value={value && 'profile' in value ? value.profile : undefined} onchange={(p) => onchange({ profile: p })} options={profileNames.map((n) => ({ value: n, label: n }))} />
            {:else if m === 'pick'}
              <Hint>Turn to choose, press to switch.</Hint>
            {/if}
          {/snippet}
        </RadioList>
        {#if !cycle}<Hint warn>Next, previous and pick need a newer service.</Hint>{/if}
      {:else}
        <RadioList
          label="Setup switch"
          value={setupMode}
          onchange={setSetupMode}
          options={[
            { value: 'next', label: 'Next setup', disabled: !setupAction, title: setupAction ? undefined : NOT_SUPPORTED },
            { value: 'prev', label: 'Previous setup', disabled: !setupAction, title: setupAction ? undefined : NOT_SUPPORTED },
            { value: 'name', label: 'A specific setup', disabled: !setupAction, title: setupAction ? undefined : NOT_SUPPORTED },
            { value: 'pick', label: 'Pick on the dial', disabled: !setupAction, title: setupAction ? undefined : NOT_SUPPORTED },
            { value: 'pin', label: 'Pin or release the live profile', disabled: !pin, title: pin ? 'For hybrid setups' : NOT_SUPPORTED },
            { value: 'release', label: 'Release the pin', disabled: !pin, title: pin ? 'For hybrid setups' : NOT_SUPPORTED },
          ]}
        >
          {#snippet detail(m)}
            {#if m === 'name'}
              <Select size="sm" value={value && 'setup' in value ? value.setup : undefined} onchange={(s) => onchange({ setup: s })} options={setupNames.map((n) => ({ value: n, label: n }))} />
            {:else if m === 'pin' || m === 'release'}
              <Hint>Hybrid setups: a pinned profile stays live until released.</Hint>
            {/if}
          {/snippet}
        </RadioList>
      {/if}
    </div>
  {:else if kind === 'session'}
    <div class="stack">
      <Segmented
        full
        label="Claude session action"
        value={sessionMode}
        onchange={(m) => onchange(m === 'bring' ? { session: 0 } : { sessions: 'next' })}
        options={[{ value: 'bring', label: 'Bring up session' }, { value: 'page', label: 'Page through' }]}
      />
      {#if value && 'sessions' in value}
        <Select value={value.sessions} onchange={(s) => onchange({ sessions: s })} options={[{ value: 'next', label: 'Next page' }, { value: 'prev', label: 'Previous page' }]} />
        <Hint>Scrolls the Claude session keys a page, when more sessions are open than keys. An unbound left roller does this on its own.</Hint>
      {:else}
        <div class="inline">
          <span class="t-body">Session</span>
          <div class="num"><NumberField value={(value && 'session' in value ? value.session : 0) + 1} min={1} max={15} onchange={(n) => onchange({ session: Math.max(0, (n ?? 1) - 1) })} /></div>
          <span class="t-small muted">in the sort order</span>
        </div>
        <Hint>Brings up that Claude Code session: its window, or the session in the Claude app. Status colours: green ready, blue working, amber waiting for you.</Hint>
      {/if}
    </div>
  {/if}
</div>

<style>
  .editor {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-width: 0;
    gap: 10px;
    padding-top: 8px;
  }
  .stack {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-width: 0;
    gap: 10px;
  }
  .kindbar {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: var(--size-control);
    padding: 0 10px;
    color: var(--color-text-1);
    background: var(--color-raised);
    border: var(--field-border);
    border-radius: var(--radius-control);
    cursor: pointer;
  }
  .kindbar:hover {
    border-color: var(--color-border-2);
  }
  .kindbar .ico {
    display: inline-flex;
    color: var(--color-accent);
  }
  .kindbar .name {
    flex: 1;
    font-weight: 600;
    text-align: left;
  }
  .kindbar :global(svg:last-child) {
    color: var(--color-text-3);
    transition: transform var(--motion-fast);
  }
  .kindbar.open :global(svg:last-child) {
    transform: rotate(180deg);
  }
  .none {
    color: var(--color-text-3);
  }
  .picker {
    display: grid;
    gap: 8px;
  }
  .filter {
    height: var(--size-control-sm);
    padding: 0 10px;
    font-family: var(--type-body-family);
    font-size: var(--type-small-size);
    color: var(--color-text-1);
    background: var(--color-sunken);
    border: var(--field-border);
    border-radius: var(--radius-control);
    outline: none;
  }
  .filter:focus {
    border-color: var(--color-accent);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(64px, 1fr));
    gap: 6px;
  }
  .kind {
    display: grid;
    justify-items: center;
    gap: 5px;
    padding: 9px 4px 7px;
    color: var(--color-text-2);
    background: var(--color-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-control);
    cursor: pointer;
  }
  .kind:hover:not(:disabled) {
    color: var(--color-text-1);
    border-color: var(--color-border-2);
  }
  .kind.on {
    color: var(--color-accent);
    border-color: var(--color-accent);
    background: var(--color-selection);
  }
  .kind:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .cap {
    color: var(--color-text-2);
  }
  .cmd {
    width: 100%;
    padding: 8px 10px;
    resize: vertical;
    font-family: var(--type-mono-family);
    font-size: var(--type-mono-size);
    line-height: 1.45;
    color: var(--color-text-1);
    background: var(--color-sunken);
    border: var(--field-border);
    border-radius: var(--radius-control);
    outline: none;
    user-select: text;
  }
  .cmd:focus {
    border-color: var(--color-accent);
  }
  .test {
    display: flex;
  }
  .result {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    color: var(--color-success);
    background: var(--color-sunken);
    border-radius: var(--radius-control);
  }
  .result span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text-1);
  }
  .result.bad {
    color: var(--color-danger);
  }
  .inline {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .num {
    width: 80px;
  }
</style>

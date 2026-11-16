<script lang="ts">
  import { untrack } from 'svelte';
  import { ArrowLeft, ArrowRight, Ban, Bot, ChevronDown, CornerUpLeft, Folder, Keyboard, LayoutGrid, Terminal, UserRound, Volume2, VolumeX } from 'lucide-svelte';
  import { invoke } from '../lib/backend';
  import { currentProfile, describe, edit, hub, toast } from '../lib/hub.svelte';
  import { ui } from '../lib/ui.svelte';
  import type { Action, AudioInfo, AudioTarget, Mute } from '../lib/types';

  let {
    title,
    value,
    onchange,
    fader = false,
  }: { title: string; value: Action | undefined; onchange: (a: Action | undefined) => void; fader?: boolean } = $props();
  const uid = $props.id();

  const KINDS = [
    { k: 'none', label: 'Nothing', icon: Ban },
    { k: 'keys', label: 'Shortcut', icon: Keyboard },
    { k: 'cmd', label: 'Command', icon: Terminal },
    { k: 'volume', label: 'Volume', icon: Volume2 },
    { k: 'mute', label: 'Mute', icon: VolumeX },
    { k: 'next', label: 'Next', icon: ArrowRight },
    { k: 'prev', label: 'Previous', icon: ArrowLeft },
    { k: 'page', label: 'Page', icon: LayoutGrid },
    { k: 'folder', label: 'Folder', icon: Folder },
    { k: 'back', label: 'Back', icon: CornerUpLeft },
    { k: 'profile', label: 'Profile', icon: UserRound },
    { k: 'session', label: 'Claude', icon: Bot },
  ] as const;
  /** Claude Code session slots, one per key. */
  const SLOTS = Array.from({ length: 15 }, (_, i) => i);
  type Kind = (typeof KINDS)[number]['k'];
  /** Same forms as `VolumeChange::parse` in masterhub-core. */
  const VOLUME = /^\s*([+-]\s*\d+|\d{1,2}|100|\{value\})\s*$/;
  const WINDOWS = navigator.userAgent.includes('Windows');
  /** What volume and mute act on. */
  const TARGETS = [
    { t: 'output', label: 'Default output' },
    { t: 'output-device', label: 'Output device' },
    { t: 'input', label: 'Microphone (default input)' },
    { t: 'input-device', label: 'Input device' },
    { t: 'app', label: 'App' },
    { t: 'group', label: 'App group' },
  ] as const;
  type TargetKind = (typeof TARGETS)[number]['t'];
  const NEW_GROUP = '\u0000new';

  function kindOf(a: Action | undefined): Kind {
    if (!a) return 'none';
    if ('cmd' in a) return 'cmd';
    if ('keys' in a) return 'keys';
    if ('volume' in a) return 'volume';
    if ('mute' in a) return 'mute';
    if ('page' in a) return a.page === 'next' ? 'next' : a.page === 'prev' ? 'prev' : 'page';
    if ('folder' in a) return 'folder';
    if ('back' in a) return 'back';
    if ('session' in a) return 'session';
    return 'profile';
  }
  function argOf(a: Action | undefined): string {
    if (!a) return '';
    if ('cmd' in a) return a.cmd;
    if ('keys' in a) return a.keys;
    if ('volume' in a) return typeof a.volume === 'string' ? a.volume : a.volume.level;
    if ('mute' in a) return typeof a.mute === 'string' ? a.mute : a.mute.state;
    if ('page' in a) return a.page === 'next' || a.page === 'prev' ? '' : a.page;
    if ('folder' in a) return a.folder;
    if ('profile' in a) return a.profile;
    if ('session' in a) return String(a.session);
    return '';
  }

  function targetOf(a: Action | undefined): [TargetKind, string] {
    const t: AudioTarget | undefined =
      a && 'volume' in a && typeof a.volume !== 'string' ? a.volume : a && 'mute' in a && typeof a.mute !== 'string' ? a.mute : undefined;
    const device = (d: string, k: TargetKind, named: TargetKind): [TargetKind, string] => (d === 'default' || !d ? [k, ''] : [named, d]);
    if (t?.input !== undefined) return device(t.input, 'input', 'input-device');
    if (t?.app !== undefined) return ['app', t.app];
    if (t?.group !== undefined) return ['group', t.group];
    if (t?.output !== undefined) return device(t.output, 'output', 'output-device');
    return ['output', ''];
  }
  function targetKeys(): AudioTarget | undefined {
    switch (target) {
      case 'output-device': return tname ? { output: tname } : undefined;
      case 'input': return { input: 'default' };
      case 'input-device': return tname ? { input: tname } : undefined;
      case 'app': return tname ? { app: tname } : undefined;
      case 'group': return tname ? { group: tname } : undefined;
      default: return undefined;
    }
  }

  let open = $state(false);
  // Rebuilt when the selection changes, so the initial value is all it needs.
  let kind = $state<Kind>(untrack(() => kindOf(value)));
  let arg = $state(untrack(() => argOf(value)));
  let target = $state<TargetKind>(untrack(() => targetOf(value)[0]));
  let tname = $state(untrack(() => targetOf(value)[1]));
  let audio = $state<AudioInfo>({ apps: [], outputs: [], inputs: [] });
  let audioLoaded = false;

  // Suggestions for devices and apps, once the editor shows an audio action.
  $effect(() => {
    if (!open || (kind !== 'volume' && kind !== 'mute') || audioLoaded) return;
    audioLoaded = true;
    invoke<AudioInfo>('audio_info')
      .then((a) => (audio = a))
      .catch(() => {});
  });

  const groups = $derived(hub.config.audio_group ?? []);
  const group = $derived(target === 'group' ? groups.find((g) => g.name === tname) : undefined);

  const icon = $derived(KINDS.find((x) => x.k === kindOf(value))!.icon);
  const names = $derived.by(() => {
    const p = currentProfile();
    if (kind === 'page') return (p?.page ?? []).map((x) => x.name);
    if (kind === 'folder') return (p?.folder ?? []).map((x) => x.name);
    if (kind === 'profile') return (hub.config.profile ?? []).map((x) => x.name);
    return [];
  });

  function build(): Action | undefined {
    switch (kind) {
      case 'cmd': return arg ? { cmd: arg } : undefined;
      case 'keys': return arg ? { keys: arg } : undefined;
      case 'volume': {
        if (!VOLUME.test(arg)) return undefined;
        const t = targetKeys();
        return { volume: t ? { level: arg, ...t } : arg };
      }
      case 'mute': {
        const state = (arg || 'toggle') as Mute;
        const t = targetKeys();
        return { mute: t ? { state, ...t } : state };
      }
      case 'next': return { page: 'next' };
      case 'prev': return { page: 'prev' };
      case 'page': return arg ? { page: arg } : undefined;
      case 'folder': return arg ? { folder: arg } : undefined;
      case 'back': return { back: true };
      case 'profile': return arg ? { profile: arg } : undefined;
      case 'session': return { session: Number(arg) || 0 };
      default: return undefined;
    }
  }

  function pick(k: Kind) {
    kind = k;
    if (k === 'page' || k === 'folder' || k === 'profile') arg = names[0] ?? '';
    else if (k === 'volume') arg = fader ? '{value}' : '+2';
    else if (k === 'mute') arg = 'toggle';
    else if (k === 'session') arg = '0';
    else if (k !== 'cmd' && k !== 'keys') arg = '';
    onchange(build());
  }

  function pickTarget(t: TargetKind) {
    target = t;
    if (t === 'group') tname = groups[0]?.name ?? '';
    else if (t === 'output-device') tname = audio.outputs[0] ?? '';
    else if (t === 'input-device') tname = audio.inputs[0] ?? '';
    else tname = '';
    if (t === 'group' && !groups.length) return newGroup();
    onchange(build());
  }

  function pickGroup(name: string) {
    if (name === NEW_GROUP) return newGroup();
    tname = name;
    onchange(build());
  }

  function newGroup() {
    ui.ask({
      title: 'New app group',
      body: 'One fader or key controls the volume of every app in the group, e.g. all your games.',
      placeholder: 'Games',
      ok: 'Create',
      done: (name) => {
        if (!name) return;
        if (groups.some((g) => g.name === name)) return toast('A group with that name already exists');
        // The group must exist before an action names it, or the service rejects the config.
        edit((c) => (c.audio_group = [...(c.audio_group ?? []), { name }]));
        target = 'group';
        tname = name;
        onchange(build());
      },
    });
  }

  function setGroupApps(text: string) {
    const apps = text.split(',').map((s) => s.trim()).filter(Boolean);
    edit((c) => {
      const g = (c.audio_group ?? []).find((x) => x.name === tname);
      if (g) g.apps = apps;
    });
  }

  function addGroupApp(app: string) {
    const apps = group?.apps ?? [];
    if (!apps.some((a) => a.toLowerCase() === app.toLowerCase())) setGroupApps([...apps, app].join(', '));
  }

  function setGroupSteam(steam: boolean) {
    edit((c) => {
      const g = (c.audio_group ?? []).find((x) => x.name === tname);
      if (g) g.steam = steam;
    });
  }
</script>

<div class="trigger" class:open>
  <button class="summary" onclick={() => (open = !open)}>
    <span class="ico" class:set={!!value}>{#if icon}{@const I = icon}<I size={14} />{/if}</span>
    <span class="txt">
      <span class="title">{title}</span>
      <span class="desc" class:none={!value}>{value ? describe(value) : 'Not set'}</span>
    </span>
    <span class="chev"><ChevronDown size={14} /></span>
  </button>
  {#if open}
    <div class="editor">
      <div class="kinds" role="radiogroup" aria-label="Action type">
        {#each KINDS as { k, label, icon: I }}
          <button class="kind" class:on={kind === k} role="radio" aria-checked={kind === k} onclick={() => pick(k)}>
            <I size={15} />
            <span>{label}</span>
          </button>
        {/each}
      </div>
      {#if kind === 'cmd' || kind === 'keys'}
        <input
          class="input mono"
          value={arg}
          placeholder={kind === 'keys' ? 'ctrl+shift+m · playpause · f13' : WINDOWS ? 'start notepad' : 'xdg-open ~'}
          oninput={(e) => { arg = e.currentTarget.value; onchange(build()); }}
        />
        <p class="hint">
          {#if kind === 'keys'}Modifiers and keys joined with +. Media keys: playpause, nextsong, previoussong, mute, volumeup.{:else if WINDOWS}Runs with cmd /C.{:else}Runs in a shell.{/if}
        </p>
      {:else if kind === 'volume'}
        <input
          class="input mono"
          value={arg}
          placeholder={'+2 · -2 · 40 · {value}'}
          oninput={(e) => { arg = e.currentTarget.value; onchange(build()); }}
        />
        <p class="hint">+2 or -2 changes the volume, 40 sets it, {'{value}'} follows a fader.</p>
      {:else if kind === 'mute'}
        <select class="select" value={arg} onchange={(e) => { arg = e.currentTarget.value; onchange(build()); }}>
          <option value="toggle">Toggle</option>
          <option value="on">Mute</option>
          <option value="off">Unmute</option>
        </select>
      {/if}
      {#if kind === 'volume' || kind === 'mute'}
        <div class="sub-label">Controls</div>
        <select class="select" value={target} onchange={(e) => pickTarget(e.currentTarget.value as TargetKind)}>
          {#each TARGETS as { t, label }}<option value={t}>{label}</option>{/each}
        </select>
        {#if target === 'output-device' || target === 'input-device'}
          {@const list = target === 'input-device' ? audio.inputs : audio.outputs}
          <input
            class="input"
            value={tname}
            list="{uid}-devices"
            placeholder={target === 'input-device' ? 'USB Microphone' : 'Headphones'}
            oninput={(e) => { tname = e.currentTarget.value; onchange(build()); }}
          />
          <datalist id="{uid}-devices">{#each list as d}<option value={d}></option>{/each}</datalist>
          <p class="hint">Device name as your system shows it; * is a wildcard.</p>
        {:else if target === 'app'}
          <input
            class="input mono"
            value={tname}
            list="{uid}-apps"
            placeholder={WINDOWS ? 'Discord.exe' : 'discord'}
            oninput={(e) => { tname = e.currentTarget.value; onchange(build()); }}
          />
          <datalist id="{uid}-apps">{#each audio.apps as a}<option value={a}></option>{/each}</datalist>
          {#if audio.apps.length}
            <div class="chips">
              {#each audio.apps as a}<button class="chip" class:on={a.toLowerCase() === tname.toLowerCase()} onclick={() => { tname = a; onchange(build()); }}>{a}</button>{/each}
            </div>
          {/if}
          <p class="hint">Program name, {WINDOWS ? '.exe optional' : 'as shown above while it plays'}; * is a wildcard. Apps that start later get the fader's level too.</p>
        {:else if target === 'group'}
          <select class="select" value={tname} onchange={(e) => pickGroup(e.currentTarget.value)}>
            {#each groups as g}<option value={g.name}>{g.name}</option>{/each}
            <option value={NEW_GROUP}>New group…</option>
          </select>
          {#if group}
            <input
              class="input mono"
              value={(group.apps ?? []).join(', ')}
              placeholder={WINDOWS ? 'cs2.exe, javaw.exe' : 'cs2, javaw'}
              onchange={(e) => setGroupApps(e.currentTarget.value)}
            />
            {#if audio.apps.length}
              <div class="chips">
                {#each audio.apps as a}<button class="chip" onclick={() => addGroupApp(a)}>+ {a}</button>{/each}
              </div>
            {/if}
            <label class="check">
              <input type="checkbox" checked={!!group.steam} onchange={(e) => setGroupSteam(e.currentTarget.checked)} />
              Everything Steam starts (games)
            </label>
            <p class="hint">Apps in “{group.name}”, separated by commas. Every action that uses the group follows these changes.</p>
          {/if}
        {/if}
      {:else if kind === 'session'}
        <select class="select" value={arg} onchange={(e) => { arg = e.currentTarget.value; onchange(build()); }}>
          {#each SLOTS as n}<option value={String(n)}>Session {n + 1}</option>{/each}
        </select>
        <p class="hint">Brings up that Claude Code session: its window, or the session in the Claude app. Sessions are numbered oldest first.</p>
      {:else if kind === 'page' || kind === 'folder' || kind === 'profile'}
        <select class="select" value={arg} onchange={(e) => { arg = e.currentTarget.value; onchange(build()); }}>
          {#each names as n}<option value={n}>{n}</option>{/each}
        </select>
      {/if}
    </div>
  {/if}
</div>

<style>
  .trigger {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
    overflow: hidden;
    transition: border-color 0.12s;
  }
  .trigger.open {
    border-color: var(--border-strong);
  }
  .summary {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    font: inherit;
    color: inherit;
    text-align: left;
    background: none;
    border: none;
    cursor: pointer;
  }
  .summary:hover {
    background: var(--bg-hover);
  }
  .ico {
    width: 26px;
    height: 26px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    border-radius: 6px;
    color: var(--text-3);
    background: var(--bg-input);
  }
  .ico.set {
    color: var(--accent-text);
    background: var(--accent-soft);
  }
  .txt {
    flex: 1;
    min-width: 0;
    display: grid;
    line-height: 1.3;
  }
  .title {
    font-weight: 500;
  }
  .desc {
    font-size: 12px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .desc.none {
    color: var(--text-3);
  }
  .chev {
    color: var(--text-3);
    transition: transform 0.15s var(--ease);
  }
  .open .chev {
    transform: rotate(180deg);
  }
  .editor {
    display: grid;
    gap: 8px;
    padding: 10px;
    border-top: 1px solid var(--border);
    animation: fade-in 0.15s ease-out;
  }
  .kinds {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
  }
  .kind {
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
    height: 30px;
    padding: 0 8px;
    font: inherit;
    font-size: 11.5px;
    color: var(--text-2);
    background: var(--bg-input);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
  }
  .kind:hover {
    color: var(--text);
    background: var(--bg-active);
  }
  .kind.on {
    color: var(--accent-text);
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
  }
  .hint {
    font-size: 11.5px;
    color: var(--text-3);
  }
  .sub-label {
    margin-top: 4px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-2);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip {
    height: 22px;
    padding: 0 8px;
    font: inherit;
    font-size: 11.5px;
    color: var(--text-2);
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 99px;
    cursor: pointer;
  }
  .chip:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }
  .chip.on {
    color: var(--accent-text);
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-2);
    cursor: pointer;
  }
</style>

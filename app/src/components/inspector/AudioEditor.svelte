<!-- Volume and mute actions: what changes, and what it acts on (output, microphone, app, group). -->
<script lang="ts">
  import { untrack } from 'svelte';
  import { Users } from 'lucide-svelte';
  import { invoke } from '../../lib/backend';
  import { edit, hub, toast } from '../../lib/hub.svelte';
  import { ui } from '../../lib/ui.svelte';
  import type { Action, AudioInfo, AudioTarget, Mute } from '../../lib/types';
  import Avatar from '../ui/Avatar.svelte';
  import Button from '../ui/Button.svelte';
  import NumberField from '../ui/NumberField.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import Select from '../ui/Select.svelte';
  import TextField from '../ui/TextField.svelte';
  import {
    audioParts, buildLevel, buildMute, buildVolume, levelMode, levelNumber, targetMode, VOLUME,
    type LevelMode, type TargetMode,
  } from './actions';
  import Hint from './Hint.svelte';

  let {
    value,
    kind,
    onchange,
    onkind,
    defaultVolume = '+2',
  }: {
    value: Action | undefined;
    kind: 'volume' | 'mute';
    onchange: (a: Action | undefined) => void;
    /** Switch between the Volume and Mute editors (the target carries over). */
    onkind: (k: 'volume' | 'mute') => void;
    /** What a fresh volume action says: `+2`, `-2` or `{value}`. */
    defaultVolume?: string;
  } = $props();

  const WINDOWS = navigator.userAgent.includes('Windows');
  const NEW_GROUP = '\u0000new';

  const parts = $derived(audioParts(value));
  const level = $derived(parts.level ?? defaultVolume);
  const lmode = $derived(levelMode(level));
  const mstate = $derived(parts.state ?? 'toggle');

  // The radio choice is local: "One app" with no app named yet is not yet a different action.
  let mode = $state<TargetMode>(untrack(() => targetMode(audioParts(value).target)));
  let dir = $state<'output' | 'input'>(untrack(() => (audioParts(value).target.input !== undefined ? 'input' : 'output')));
  $effect(() => {
    const m = targetMode(parts.target);
    if (m !== 'default') mode = m;
    if (parts.target.input !== undefined) dir = 'input';
    else if (parts.target.output !== undefined) dir = 'output';
  });

  let audio = $state<AudioInfo>({ apps: [], outputs: [], inputs: [] });
  $effect(() => {
    invoke<AudioInfo>('audio_info').then((a) => (audio = a)).catch(() => {});
  });

  const groups = $derived(hub.config.audio_group ?? []);
  const group = $derived(parts.target.group !== undefined ? groups.find((g) => g.name === parts.target.group) : undefined);
  const devices = $derived(dir === 'input' ? audio.inputs : audio.outputs);
  const deviceName = $derived((dir === 'input' ? parts.target.input : parts.target.output) ?? '');
  const appName = $derived(parts.target.app ?? '');

  function emit(target: AudioTarget, l = level, s: Mute = mstate) {
    if (kind === 'volume') {
      if (VOLUME.test(l)) onchange(buildVolume(l, target));
    } else onchange(buildMute(s, target));
  }

  function setMode(m: TargetMode) {
    mode = m;
    if (m === 'default') emit({});
    else if (m === 'device') emit({ [dir]: deviceName || 'default' });
    else if (m === 'app') emit(appName ? { app: appName } : {});
    else {
      if (!groups.length) return newGroup();
      emit({ group: parts.target.group ?? groups[0].name });
    }
  }
  const setDevice = (name: string) => emit({ [dir]: name || 'default' });
  const setApp = (name: string) => emit(name ? { app: name } : {});

  function setDir(d: 'output' | 'input') {
    dir = d;
    emit({ [d]: 'default' });
  }

  function setLevelMode(m: LevelMode) {
    const n = levelNumber(level);
    emit(parts.target, buildLevel(m, n || (m === 'set' ? 50 : 2)));
  }

  function switchKind(k: 'volume' | 'mute') {
    if (k === kind) return;
    onkind(k);
    onchange(k === 'volume' ? buildVolume(defaultVolume, parts.target) : buildMute('toggle', parts.target));
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
        edit((c) => (c.audio_group = [...(c.audio_group ?? []), { name }]), { label: 'new audio group' });
        mode = 'group';
        emit({ group: name });
      },
    });
  }
  function pickGroup(name: string) {
    if (name === NEW_GROUP) return newGroup();
    emit({ group: name });
  }

  const groupSummary = $derived(group ? [`${(group.apps ?? []).length} app${(group.apps ?? []).length === 1 ? '' : 's'}`, ...(group.steam ? ['Steam'] : [])].join(' + ') : '');
  const appsPlaying = $derived(audio.apps);
  const MODES: { m: TargetMode; label: string; desc?: string }[] = [
    { m: 'default', label: 'Default output', desc: 'Follows whatever your system uses' },
    { m: 'device', label: 'A specific output or microphone' },
    { m: 'app', label: 'One app' },
    { m: 'group', label: 'Audio group' },
  ];
</script>

<div class="audio">
  <Segmented
    full
    label="Volume or mute"
    value={kind}
    onchange={switchKind}
    options={[{ value: 'volume', label: 'Volume' }, { value: 'mute', label: 'Mute' }]}
  />

  {#if kind === 'volume'}
    <Segmented
      full
      label="What changes"
      value={lmode}
      onchange={setLevelMode}
      options={[{ value: 'up', label: 'Raise' }, { value: 'down', label: 'Lower' }, { value: 'set', label: 'Set to' }, { value: 'follow', label: 'Fader' }]}
    />
    {#if lmode !== 'follow'}
      <div class="inline">
        <span class="t-body">{lmode === 'set' ? 'Level' : 'Step per detent'}</span>
        <div class="num"><NumberField value={levelNumber(level)} min={0} max={100} unit="%" onchange={(v) => emit(parts.target, buildLevel(lmode, v ?? 0))} /></div>
      </div>
    {:else}
      <Hint>Follows the fader: <code>{'{value}'}</code> becomes its position, 0 to 100.</Hint>
    {/if}
  {:else}
    <Segmented
      full
      label="Mute state"
      value={mstate}
      onchange={(s) => emit(parts.target, level, s)}
      options={[{ value: 'toggle', label: 'Toggle' }, { value: 'on', label: 'Mute' }, { value: 'off', label: 'Unmute' }]}
    />
  {/if}

  <div class="t-label controls">Controls</div>
  <div class="radios" role="radiogroup" aria-label="What it controls">
    {#each MODES as o (o.m)}
      <div class="opt" class:on={mode === o.m}>
        <label class="radio">
          <input type="radio" name="target" checked={mode === o.m} onchange={() => setMode(o.m)} />
          <span class="t-body">{o.label}</span>
        </label>
        {#if mode === o.m}
          <div class="detail">
            {#if o.m === 'default'}
              <Hint>{o.desc}</Hint>
            {:else if o.m === 'device'}
              <Segmented
                label="Device kind"
                value={dir}
                onchange={setDir}
                options={[{ value: 'output', label: 'Output' }, { value: 'input', label: 'Microphone' }]}
              />
              <div class="chips">
                <button type="button" class="pick" class:on={deviceName === 'default' || !deviceName} onclick={() => setDevice('default')}>System default</button>
                {#each devices as d (d)}<button type="button" class="pick" class:on={d === deviceName} onclick={() => setDevice(d)}>{d}</button>{/each}
              </div>
              <TextField size="sm" value={deviceName === 'default' ? '' : deviceName} placeholder={dir === 'input' ? 'USB Microphone' : 'Headphones'} aria-label="Device name" onchange={(e) => setDevice(e.currentTarget.value.trim())} />
              <Hint>Device name as your system shows it; * is a wildcard.</Hint>
            {:else if o.m === 'app'}
              {#if appsPlaying.length}
                <div class="chips">
                  {#each appsPlaying as a (a)}
                    <button type="button" class="pick app" class:on={a.toLowerCase() === appName.toLowerCase()} onclick={() => setApp(a)}><Avatar name={a} size={16} />{a}</button>
                  {/each}
                </div>
              {/if}
              <TextField size="sm" mono value={appName} placeholder={WINDOWS ? 'Discord.exe' : 'discord'} aria-label="Program name" onchange={(e) => setApp(e.currentTarget.value.trim())} />
              <Hint>{appsPlaying.length ? 'Playing now first. ' : ''}Matches the program name on any output; * is a wildcard. Apps that start later follow too.</Hint>
            {:else}
              <Select
                size="sm"
                value={parts.target.group ?? groups[0]?.name ?? ''}
                onchange={pickGroup}
                options={[...groups.map((g) => ({ value: g.name, label: g.name })), { value: NEW_GROUP, label: 'New group…' }]}
              />
              {#if group}
                <div class="group">
                  <span class="t-chip"><Users size={13} />{group.name} · {groupSummary}</span>
                  <Button size="sm" variant="ghost" onclick={() => ui.go('global', { view: 'audio-groups' })}>Manage groups</Button>
                </div>
              {/if}
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .audio {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-width: 0;
    gap: 10px;
    padding-top: 8px;
  }
  .inline {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .num {
    width: 110px;
  }
  .controls {
    margin-top: 4px;
  }
  .radios {
    display: grid;
    gap: 4px;
  }
  .opt {
    padding: 6px 8px;
    border-radius: var(--radius-control);
  }
  .opt.on {
    background: var(--color-selection);
  }
  .radio {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  .radio input {
    margin: 0;
    accent-color: var(--color-accent-fill);
  }
  .detail {
    display: grid;
    gap: 8px;
    padding: 8px 0 2px 24px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .pick {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    height: var(--size-chip);
    padding: 0 9px 0 9px;
    font: inherit;
    font-size: var(--type-chip-size);
    font-weight: var(--type-chip-weight);
    color: var(--color-text-1);
    background: var(--color-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-chip);
    cursor: pointer;
  }
  .pick.app {
    padding-left: 3px;
  }
  .pick:hover {
    border-color: var(--color-border-2);
  }
  .pick.on {
    color: var(--color-accent);
    border-color: var(--color-accent);
  }
  .group {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .group .t-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--color-text-2);
  }
</style>

<script lang="ts">
  import { untrack } from 'svelte';
  import { ArrowLeft, ArrowRight, Ban, ChevronDown, CornerUpLeft, Folder, Keyboard, LayoutGrid, Terminal, UserRound, Volume2, VolumeX } from 'lucide-svelte';
  import { currentProfile, describe, hub } from '../lib/hub.svelte';
  import type { Action, Mute } from '../lib/types';

  let { title, value, onchange }: { title: string; value: Action | undefined; onchange: (a: Action | undefined) => void } = $props();

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
  ] as const;
  type Kind = (typeof KINDS)[number]['k'];
  /** Same forms as `VolumeChange::parse` in masterhub-core. */
  const VOLUME = /^\s*([+-]\s*\d+|\d{1,2}|100|\{value\})\s*$/;
  const WINDOWS = navigator.userAgent.includes('Windows');

  function kindOf(a: Action | undefined): Kind {
    if (!a) return 'none';
    if ('cmd' in a) return 'cmd';
    if ('keys' in a) return 'keys';
    if ('volume' in a) return 'volume';
    if ('mute' in a) return 'mute';
    if ('page' in a) return a.page === 'next' ? 'next' : a.page === 'prev' ? 'prev' : 'page';
    if ('folder' in a) return 'folder';
    if ('back' in a) return 'back';
    return 'profile';
  }
  function argOf(a: Action | undefined): string {
    if (!a) return '';
    if ('cmd' in a) return a.cmd;
    if ('keys' in a) return a.keys;
    if ('volume' in a) return a.volume;
    if ('mute' in a) return a.mute;
    if ('page' in a) return a.page === 'next' || a.page === 'prev' ? '' : a.page;
    if ('folder' in a) return a.folder;
    if ('profile' in a) return a.profile;
    return '';
  }

  let open = $state(false);
  // Rebuilt when the selection changes, so the initial value is all it needs.
  let kind = $state<Kind>(untrack(() => kindOf(value)));
  let arg = $state(untrack(() => argOf(value)));

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
      case 'volume': return VOLUME.test(arg) ? { volume: arg } : undefined;
      case 'mute': return { mute: (arg || 'toggle') as Mute };
      case 'next': return { page: 'next' };
      case 'prev': return { page: 'prev' };
      case 'page': return arg ? { page: arg } : undefined;
      case 'folder': return arg ? { folder: arg } : undefined;
      case 'back': return { back: true };
      case 'profile': return arg ? { profile: arg } : undefined;
      default: return undefined;
    }
  }

  function pick(k: Kind) {
    kind = k;
    if (k === 'page' || k === 'folder' || k === 'profile') arg = names[0] ?? '';
    else if (k === 'volume') arg = '+2';
    else if (k === 'mute') arg = 'toggle';
    else if (k !== 'cmd' && k !== 'keys') arg = '';
    onchange(build());
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
</style>

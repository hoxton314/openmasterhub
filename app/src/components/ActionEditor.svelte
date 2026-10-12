<script lang="ts">
  import { untrack } from 'svelte';
  import { currentProfile, describe, hub } from '../lib/hub.svelte';
  import type { Action } from '../lib/types';

  let { title, value, onchange }: { title: string; value: Action | undefined; onchange: (a: Action | undefined) => void } = $props();

  const KINDS = [
    ['none', 'Nothing'],
    ['cmd', 'Shell command'],
    ['keys', 'Key combination'],
    ['next', 'Next page'],
    ['prev', 'Previous page'],
    ['page', 'Go to page'],
    ['folder', 'Open folder'],
    ['back', 'Back (leave folder)'],
    ['profile', 'Switch profile'],
  ] as const;
  type Kind = (typeof KINDS)[number][0];

  function kindOf(a: Action | undefined): Kind {
    if (!a) return 'none';
    if ('cmd' in a) return 'cmd';
    if ('keys' in a) return 'keys';
    if ('page' in a) return a.page === 'next' ? 'next' : a.page === 'prev' ? 'prev' : 'page';
    if ('folder' in a) return 'folder';
    if ('back' in a) return 'back';
    return 'profile';
  }
  function argOf(a: Action | undefined): string {
    if (!a) return '';
    if ('cmd' in a) return a.cmd;
    if ('keys' in a) return a.keys;
    if ('page' in a) return a.page === 'next' || a.page === 'prev' ? '' : a.page;
    if ('folder' in a) return a.folder;
    if ('profile' in a) return a.profile;
    return '';
  }

  let open = $state(false);
  // The inspector rebuilds this component when the selection changes, so the initial value is
  // all it needs; afterwards it is the source of truth while editing.
  let kind = $state<Kind>(untrack(() => kindOf(value)));
  let arg = $state(untrack(() => argOf(value)));

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
      case 'next': return { page: 'next' };
      case 'prev': return { page: 'prev' };
      case 'page': return arg ? { page: arg } : undefined;
      case 'folder': return arg ? { folder: arg } : undefined;
      case 'back': return { back: true };
      case 'profile': return arg ? { profile: arg } : undefined;
      default: return undefined;
    }
  }

  function changeKind(k: Kind) {
    kind = k;
    if (k === 'page' || k === 'folder' || k === 'profile') arg = names[0] ?? '';
    else if (k !== 'cmd' && k !== 'keys') arg = '';
    onchange(build());
  }
</script>

<div class="row expander" class:open>
  <button class="head" onclick={() => (open = !open)}>
    <span class="title">{title}</span>
    <span class="sub">{describe(value)}</span>
    <span class="chev">{open ? '▾' : '▸'}</span>
  </button>
  {#if open}
    <div class="body">
      <label>
        <span>Action</span>
        <select value={kind} onchange={(e) => changeKind(e.currentTarget.value as Kind)}>
          {#each KINDS as [k, label]}<option value={k}>{label}</option>{/each}
        </select>
      </label>
      {#if kind === 'cmd' || kind === 'keys'}
        <label>
          <span>{kind === 'keys' ? 'Keys' : 'Command'}</span>
          <input
            value={arg}
            placeholder={kind === 'keys' ? 'ctrl+shift+m, playpause, f13…' : 'wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle'}
            oninput={(e) => { arg = e.currentTarget.value; onchange(build()); }}
          />
        </label>
      {:else if kind === 'page' || kind === 'folder' || kind === 'profile'}
        <label>
          <span>Target</span>
          <select value={arg} onchange={(e) => { arg = e.currentTarget.value; onchange(build()); }}>
            {#each names as n}<option value={n}>{n}</option>{/each}
          </select>
        </label>
      {/if}
    </div>
  {/if}
</div>

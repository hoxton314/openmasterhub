<!-- Rule tester: type a window class or .exe, see which profile the setup would switch to. -->
<script lang="ts">
  import { AppWindow, Info, Search, TriangleAlert } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import { hub } from '../../lib/hub.svelte';
  import { matchApp, type EffectiveSetup } from '../../lib/model';
  import InspectorSection from '../ui/InspectorSection.svelte';
  import TextField from '../ui/TextField.svelte';
  import { refreshWindows, windows } from './windows.svelte';

  let { setup, value = $bindable('') }: { setup: EffectiveSetup; value: string } = $props();

  onMount(refreshWindows);
  const cls = $derived(value.trim());
  const match = $derived(cls ? matchApp(hub.config, setup, cls) : undefined);
  const first = $derived(match?.shadowed[0]);
  const pinned = $derived(setup.mode === 'hybrid' && hub.state.setup === setup.name ? hub.state.pinned : undefined);

  /** What a window would switch to in this setup; "not in Work" when no rule and no fallback. */
  function outcome(c: string) {
    const m = matchApp(hub.config, setup, c);
    return m.kind === 'rule' ? m.profile!.name : m.kind === 'fallback' ? m.profile!.name : `not in ${setup.name}`;
  }
</script>

<InspectorSection>
  <h2 class="t-heading">Rule tester</h2>
  <label class="tfield">
    <span class="t-small muted">Window class or .exe</span>
    <span class="sbox">
      <Search size={14} />
      <TextField bind:value mono placeholder="e.g. com.mitchellh.ghostty" aria-label="Window class or .exe" spellcheck={false} />
    </span>
  </label>

  {#if match}
    <div class="result" class:none={match.kind === 'none'} aria-live="polite">
      <span class="t-small">{match.kind === 'none' ? 'No profile for this window' : 'Would switch to'}</span>
      {#if match.profile}<b class="name">{match.profile.name}</b>{/if}
      <span class="why t-chip">
        {#if match.kind === 'rule'}Rule {match.hit!.index + 1} · <code>{match.hit!.pattern}</code>{:else if match.kind === 'fallback'}Fallback · nothing above matches{:else}No rule matches and the setup has no fallback{/if}
      </span>
    </div>
    {#if first}
      <p class="note warn t-small">
        <TriangleAlert size={14} />
        <span>{first.profile.name} (rule {first.index + 1} · <code>{first.pattern}</code>) also matches, but comes later.</span>
      </p>
    {/if}
    {#if pinned}
      <p class="note t-small"><Info size={14} /><span>{pinned.profile} is pinned: rules resume when you release it.</span></p>
    {/if}
  {:else}
    <p class="hint t-small muted">Type a class, or pick one of the open windows below.</p>
  {/if}
</InspectorSection>

<InspectorSection title="Open windows" divider={false}>
  <div class="windows">
    {#each windows.open as w (w.class)}
      <button type="button" class="win" class:on={cls.toLowerCase() === w.class.toLowerCase()} title={w.title} onclick={() => (value = w.class)}>
        <AppWindow size={13} />
        <code>{w.class}</code>
        <span class="to">{outcome(w.class)}</span>
      </button>
    {:else}
      <span class="t-small muted">{windows.loaded ? 'No windows found.' : 'Looking for windows…'}</span>
    {/each}
  </div>
  <p class="note t-chip faint"><Info size={13} /><span>Window class on Linux (Hyprland, KDE), the .exe name on Windows. Focus tracking depends on your desktop.</span></p>
</InspectorSection>

<style>
  .tfield {
    display: grid;
    gap: 5px;
  }
  .sbox {
    position: relative;
    display: block;
  }
  .sbox :global(svg) {
    position: absolute;
    left: 10px;
    top: 50%;
    z-index: 1;
    translate: 0 -50%;
    color: var(--color-text-3);
    pointer-events: none;
  }
  .sbox :global(input) {
    padding-left: 30px;
  }
  .result {
    display: grid;
    gap: 2px;
    padding: 12px;
    color: var(--color-text-1);
    background: var(--color-selection);
    border-radius: var(--radius-panel);
  }
  .result .name {
    font: var(--type-heading-weight) var(--type-heading-size) / var(--type-heading-lh) var(--type-heading-family);
    text-transform: var(--type-heading-case);
  }
  .result .why {
    color: var(--color-text-2);
  }
  .none .name {
    color: var(--color-text-2);
  }
  .note {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    color: var(--color-text-2);
  }
  .note :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .warn {
    color: var(--color-warning);
  }
  code {
    font-family: var(--type-mono-family);
  }
  .windows {
    display: grid;
    gap: 2px;
  }
  .win {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    padding: 0 8px;
    text-align: left;
    color: var(--color-text-1);
    background: none;
    border: 0;
    border-radius: var(--radius-row);
    cursor: pointer;
  }
  .win:hover,
  .win.on {
    background: var(--color-selection);
  }
  .win :global(svg) {
    flex: none;
    color: var(--color-text-2);
  }
  .win code {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: var(--type-mono-size);
  }
  .to {
    flex: none;
    font-size: var(--type-chip-size);
    color: var(--color-text-3);
  }
</style>

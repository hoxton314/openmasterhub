<!-- "Press · Ctrl + S" / "Release · Nothing": a trigger and its action summary; opens the editor. -->
<script lang="ts">
  import { ChevronRight } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import type { Icon } from './icon';
  import KeyCombo from './KeyCombo.svelte';

  let {
    label,
    icon: Ic,
    summary,
    combo,
    active = false,
    onclick,
    children,
  }: { label: string; icon?: Icon; summary?: string; combo?: string; active?: boolean; onclick?: () => void; children?: Snippet } = $props();
</script>

<div class="trigger" class:active>
  <button type="button" aria-expanded={active} {onclick}>
    {#if Ic}<span class="icon"><Ic size={15} /></span>{/if}
    <span class="name">{label}</span>
    <span class="what">
      {#if combo}<KeyCombo {combo} />{:else if summary}<span class="summary">{summary}</span>{:else}<span class="none">Nothing</span>{/if}
    </span>
    <ChevronRight size={14} class="chev" />
  </button>
  {#if active && children}<div class="body">{@render children()}</div>{/if}
</div>

<style>
  .trigger {
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-panel);
  }
  button {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    height: 40px;
    padding: 0 10px;
    text-align: left;
    background: transparent;
    border: 0;
    border-radius: inherit;
    cursor: pointer;
  }
  button:hover {
    background: var(--color-selection);
  }
  .icon {
    display: inline-flex;
    color: var(--color-text-2);
  }
  .name {
    width: 74px;
    flex: none;
    font-weight: 600;
  }
  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }
  .summary {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .none {
    font-size: var(--type-small-size);
    color: var(--color-text-3);
  }
  button :global(.chev) {
    flex: none;
    color: var(--color-text-3);
    transition: transform var(--motion-fast);
  }
  .active button :global(.chev) {
    transform: rotate(90deg);
  }
  .body {
    padding: 4px 10px 12px;
    border-top: 1px solid var(--color-border);
  }
</style>

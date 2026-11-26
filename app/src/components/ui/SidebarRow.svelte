<!--
  A profile row: avatar, name, trailing meta (LIVE, mono glob, "Fallback", integration plug,
  conflict). Selected = selection tint (Signal: inverted). Drag handle on hover when `draggable`;
  a locked row (the fallback) shows a lock instead.
-->
<script lang="ts">
  import { GripVertical, Lock, Plug, TriangleAlert } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  import Avatar from './Avatar.svelte';
  import LiveBadge from './LiveBadge.svelte';

  let {
    name,
    color,
    selected = false,
    live = false,
    glob,
    fallback = false,
    integration = false,
    conflict,
    locked = false,
    draggable = false,
    collapsed = false,
    onclick,
    trailing,
    ...rest
  }: Omit<HTMLAttributes<HTMLDivElement>, 'onclick'> & {
    name: string;
    color?: string;
    selected?: boolean;
    live?: boolean;
    /** The first app rule pattern, shown mono (max 92 px). */
    glob?: string;
    fallback?: boolean;
    integration?: boolean;
    /** Tooltip of a rule conflict warning. */
    conflict?: string;
    locked?: boolean;
    draggable?: boolean;
    /** Avatar-only (collapsed sidebar). */
    collapsed?: boolean;
    onclick?: () => void;
    trailing?: Snippet;
  } = $props();
</script>

<div class="row" class:selected class:collapsed role="option" aria-selected={selected} tabindex="-1" {...rest}>
  {#if draggable && !locked && !collapsed}<span class="grip" aria-hidden="true"><GripVertical size={12} /></span>{/if}
  <button type="button" class="main" title={collapsed ? name : undefined} aria-label={collapsed ? name : undefined} {onclick}>
    <span class="av"><Avatar {name} {color} size={collapsed ? 28 : 20} />{#if collapsed && live}<span class="dot"></span>{/if}</span>
    {#if !collapsed}
      <span class="name">{name}</span>
      <span class="meta">
        {#if conflict}<span class="conflict" title={conflict}><TriangleAlert size={13} /></span>{/if}
        {#if integration}<span class="plug" title="Integration"><Plug size={13} /></span>{/if}
        {#if live}<LiveBadge />{:else if fallback}<span class="fb">Fallback</span>{:else if glob}<span class="glob">{glob}</span>{/if}
        {#if locked}<span class="lock" title="Always last"><Lock size={12} /></span>{/if}
      </span>
    {/if}
  </button>
  {#if trailing && !collapsed}<span class="trailing">{@render trailing()}</span>{/if}
</div>

<style>
  .row {
    position: relative;
    display: flex;
    align-items: center;
    height: var(--size-row-sidebar);
    border-radius: var(--radius-sidebar-row);
    color: var(--color-text-1);
    outline: none;
  }
  .row:hover:not(.selected) {
    background: var(--color-selection);
  }
  .selected {
    background: var(--row-selected-bg);
    color: var(--row-selected-fg);
    --live-color: var(--row-selected-live);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 100%;
    padding: 0 8px;
    text-align: left;
    font-weight: 500;
    color: inherit;
    background: none;
    border: 0;
    border-radius: inherit;
    cursor: pointer;
  }
  .selected .main {
    font-weight: 600;
  }
  .collapsed {
    height: auto;
    justify-content: center;
  }
  .collapsed .main {
    justify-content: center;
    padding: 4px;
  }
  .av {
    position: relative;
    display: inline-flex;
  }
  .dot {
    position: absolute;
    right: -3px;
    bottom: -3px;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--live-color);
    box-shadow: 0 0 0 2px var(--color-bg);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
  .glob {
    max-width: 92px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 400 var(--type-micro-size) / 1 var(--type-mono-family);
    color: var(--color-text-3);
  }
  .fb {
    font-size: var(--type-label-size);
    color: var(--color-text-3);
  }
  .selected .glob,
  .selected .fb,
  .selected .lock,
  .selected .plug {
    color: var(--row-selected-meta);
  }
  .plug {
    display: inline-flex;
    color: var(--color-accent);
  }
  .conflict {
    display: inline-flex;
    color: var(--color-warning);
  }
  .lock {
    display: inline-flex;
    color: var(--color-text-3);
  }
  .grip {
    position: absolute;
    left: -10px;
    display: none;
    color: var(--color-text-3);
    cursor: grab;
  }
  .row:hover .grip {
    display: inline-flex;
  }
  .trailing {
    display: inline-flex;
    padding-right: 4px;
  }
</style>

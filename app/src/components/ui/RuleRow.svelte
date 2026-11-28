<!--
  An app rule in a setup: index, avatar, name, glob chips, "Linked · also in Play", conflict line,
  drag handle, menu. The fallback row sits apart under "Fallback · always last", with a lock.
-->
<script lang="ts">
  import { GripVertical, Lock, TriangleAlert } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  import Avatar from './Avatar.svelte';
  import Chip from './Chip.svelte';

  let {
    index,
    name,
    color,
    globs = [],
    linked,
    conflict,
    fallback = false,
    menu,
    actions,
    ...rest
  }: HTMLAttributes<HTMLDivElement> & {
    /** 1-based rule number; omitted for the fallback. */
    index?: number;
    name: string;
    color?: string;
    globs?: string[];
    /** e.g. "Linked · also in Play". */
    linked?: string;
    /** Conflict explanation, shown as a warning line. */
    conflict?: string;
    fallback?: boolean;
    menu?: Snippet;
    /** Extra buttons before the menu (e.g. "Add app"). */
    actions?: Snippet;
  } = $props();
</script>

<div class="rule" class:fallback {...rest}>
  <div class="line">
    <span class="handle" aria-hidden="true">{#if fallback}<Lock size={14} />{:else}<GripVertical size={14} />{/if}</span>
    <span class="index">{index ?? ''}</span>
    <Avatar {name} {color} size={26} />
    <b class="name">{name}</b>
    <span class="globs">
      {#each globs as g}<Chip variant="glob" icon={null}>{g}</Chip>{:else}<span class="none">{fallback ? 'When nothing matches' : 'No apps yet'}</span>{/each}
      {@render actions?.()}
    </span>
    {#if linked}<Chip variant="linked">{linked}</Chip>{/if}
    {@render menu?.()}
  </div>
  {#if conflict}<div class="conflict"><TriangleAlert size={13} />{conflict}</div>{/if}
</div>

<style>
  .rule {
    display: grid;
    gap: 6px;
    padding: 10px 12px;
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-panel);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .handle {
    display: inline-flex;
    color: var(--color-text-3);
    cursor: grab;
  }
  .fallback .handle {
    cursor: default;
  }
  .index {
    width: 18px;
    font: var(--type-mono-weight) var(--type-mono-size) var(--type-mono-family);
    color: var(--color-text-3);
  }
  .name {
    width: 120px;
    flex: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .globs {
    flex: 1;
    min-width: 90px;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .none {
    font-size: var(--type-small-size);
    color: var(--color-text-3);
  }
  .conflict {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-left: 64px;
    font-size: var(--type-chip-size);
    color: var(--color-warning);
  }
</style>

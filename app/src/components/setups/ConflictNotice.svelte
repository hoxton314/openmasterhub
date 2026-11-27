<!-- "Claude’s claude-* can never win: …" under the row that cannot win, with the fixes next to it. -->
<script lang="ts">
  import { conflictText, fixLabel, type ConflictFix, type RuleConflict } from '../../lib/model/conflicts';
  import Banner from '../ui/Banner.svelte';
  import Button from '../ui/Button.svelte';

  let { conflict, onfix }: { conflict: RuleConflict; onfix: (f: ConflictFix) => void } = $props();
  const text = $derived(conflictText(conflict));
</script>

<Banner kind="warning">
  <span class="msg">{text.head}{#if text.witness}&nbsp;<code>{text.witness}</code>{/if}{text.tail ? (text.witness ? '' : ' ') + text.tail : ''}</span>
  {#snippet actions()}
    {#each conflict.fixes as f, i (i)}
      <Button size="sm" variant={i === 0 ? 'secondary' : 'ghost'} onclick={() => onfix(f)} title={f.kind === 'narrow' ? `Replace ${f.from} with ${f.to.join(', ')}` : undefined}>{fixLabel(f)}</Button>
    {/each}
  {/snippet}
</Banner>

<style>
  .msg {
    font-size: var(--type-small-size);
  }
  code {
    font: var(--type-mono-weight) var(--type-mono-size) var(--type-mono-family);
  }
</style>

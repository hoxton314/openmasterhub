<!-- "Copy to…": copies the selected control's face and actions to another control, page or profile. -->
<script lang="ts">
  import { Copy } from 'lucide-svelte';
  import { toastUndo } from '../../lib/hub.svelte';
  import type { ControlRef } from '../../lib/model/precedence';
  import Button from '../ui/Button.svelte';
  import Menu, { type MenuEntry } from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';
  import { copyPlan } from './control.svelte';

  let { ctl, disabled = false }: { ctl: ControlRef; disabled?: boolean } = $props();
  let open = $state(false);

  const plan = $derived(copyPlan(ctl, (text) => toastUndo(text)));
  const menu = $derived.by(() => {
    const items: MenuEntry[] = [];
    const add = (heading: string, list: { label: string; hint?: string; run: () => void }[]) => {
      if (!list.length) return;
      if (items.length) items.push({ separator: true });
      items.push({ heading });
      for (const t of list) items.push({ label: t.label, hint: t.hint, onselect: t.run });
    };
    add('Same kind', plan.siblings);
    add('Other pages of this profile', plan.pages);
    add('Other profiles', plan.profiles);
    return items;
  });
</script>

<Popover bind:open placement="top-start" width={248} label="Copy to" role="dialog">
  {#snippet trigger({ toggle, attrs })}
    <Button size="sm" icon={Copy} {disabled} {...attrs} onclick={toggle}>Copy to…</Button>
  {/snippet}
  {#snippet children({ close })}
    {#if plan.keys.length}
      <div class="keys">
        <div class="t-label">Another key on this page</div>
        <div class="grid">
          {#each Array.from({ length: 15 }, (_, i) => i) as i (i)}
            {@const t = plan.keys.find((k) => k.index === i)}
            <button type="button" class="cell" class:self={!t} disabled={!t} aria-label="Key {i + 1}" onclick={() => { t?.run(); close(); }}>{i + 1}</button>
          {/each}
        </div>
      </div>
    {/if}
    <Menu items={menu} onclose={close} label="Copy to" />
    {#if !plan.keys.length && !menu.length}<p class="t-small empty">Nowhere else to copy this to.</p>{/if}
  {/snippet}
</Popover>

<style>
  .keys {
    display: grid;
    gap: 8px;
    padding: 10px 12px 6px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 4px;
  }
  .cell {
    height: 28px;
    font: inherit;
    font-size: var(--type-small-size);
    color: var(--color-text-1);
    background: var(--color-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .cell:hover:not(:disabled) {
    border-color: var(--color-accent);
    color: var(--color-accent);
  }
  .cell.self {
    color: var(--color-text-3);
    border-style: dashed;
    cursor: default;
  }
  .empty {
    margin: 0;
    padding: 12px;
    color: var(--color-text-3);
  }
</style>

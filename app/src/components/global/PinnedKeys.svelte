<!-- Pinned keys: the same slot on every page of every profile. Shown on S26. -->
<script lang="ts">
  import { Pin, Plus, X } from 'lucide-svelte';
  import { describe, edit, hub, toastUndo } from '../../lib/hub.svelte';
  import { effectiveSetups } from '../../lib/model/setups';
  import type { PinnedKey } from '../../lib/types';
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Menu, { type MenuEntry } from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import Banner from '../ui/Banner.svelte';
  import { hasCapability } from '../../lib/hub.svelte';

  const pins = $derived(hub.config.pinned ?? []);
  const supportsSetups = $derived(hasCapability('except_setups'));

  function patch(index: number, f: (k: PinnedKey) => void, label: string) {
    edit((c) => {
      const k = c.pinned?.find((x) => x.index === index);
      if (k) f(k);
    }, { label });
  }

  function add() {
    const used = new Set(pins.map((p) => p.index));
    const free = Array.from({ length: 15 }, (_, i) => i).filter((i) => !used.has(i));
    if (!free.length) return ui.toast('All 15 keys are already pinned');
    ui.ask({
      title: 'Pin a key',
      body: `Which key? 1 to 15, counted along the rows. Free: ${free.map((i) => i + 1).join(', ')}.`,
      value: String(free[0] + 1),
      ok: 'Pin',
      done: (v) => {
        const index = Number(v) - 1;
        if (!free.includes(index)) return ui.toast({ text: 'That key is not free', kind: 'warning' });
        edit((c) => (c.pinned = [...(c.pinned ?? []), { index, label: 'Pinned' }].sort((a, b) => a.index - b.index)), { label: 'pin key' });
        ui.go('editor');
        hub.selection = { kind: 'key', index };
      },
    });
  }

  function remove(k: PinnedKey) {
    edit((c) => (c.pinned = (c.pinned ?? []).filter((x) => x.index !== k.index)), { label: 'unpin key' });
    toastUndo(`Unpinned key ${k.index + 1}`);
  }

  function exceptions(k: PinnedKey): MenuEntry[] {
    const profiles = (hub.config.profile ?? []).filter((p) => !k.except?.includes(p.name));
    const setups = supportsSetups ? effectiveSetups(hub.config).filter((s) => !s.implicit && !k.except_setups?.includes(s.name)) : [];
    const entries: MenuEntry[] = [];
    if (setups.length) {
      entries.push({ heading: 'Every profile of a setup' });
      for (const s of setups) entries.push({ label: s.name, onselect: () => patch(k.index, (x) => (x.except_setups = [...(x.except_setups ?? []), s.name]), 'change pinned key') });
    }
    if (profiles.length) {
      entries.push({ heading: 'A profile' });
      for (const p of profiles) entries.push({ label: p.name, onselect: () => patch(k.index, (x) => (x.except = [...(x.except ?? []), p.name]), 'change pinned key') });
    }
    return entries;
  }
</script>

<section>
  <div class="head">
    <SectionLabel as="h2">Pinned keys</SectionLabel>
    <Button size="sm" icon={Plus} onclick={add}>Pin a key</Button>
  </div>
  <p class="t-small muted">Shown in the same slot on every page of every profile. They win over profile keys, but not over a running game mode.</p>

  {#each pins as k (k.index)}
    <div class="pin">
      <div class="top">
        <Pin size={16} />
        <b class="grow">Key {k.index + 1}{k.label ? ` · ${k.label}` : ''}</b>
        <Button size="sm" variant="ghost" onclick={() => { ui.go('editor'); hub.selection = { kind: 'key', index: k.index }; }}>Edit key</Button>
        <IconButton icon={X} label="Unpin key {k.index + 1}" size="sm" onclick={() => remove(k)} />
      </div>
      <p class="t-small muted">{describe(k.press)}</p>
      <div class="except">
        <span class="t-small muted">Shown in every profile except</span>
        {#each k.except_setups ?? [] as s (s)}
          <button type="button" class="x" title="Show again in {s}" onclick={() => patch(k.index, (x) => (x.except_setups = x.except_setups?.filter((n) => n !== s)), 'change pinned key')}>
            <Chip variant="outline" icon={X}>{s} · all</Chip>
          </button>
        {/each}
        {#each k.except ?? [] as p (p)}
          <button type="button" class="x" title="Show again in {p}" onclick={() => patch(k.index, (x) => (x.except = x.except?.filter((n) => n !== p)), 'change pinned key')}>
            <Chip variant="outline" icon={X}>{p}</Chip>
          </button>
        {/each}
        <Popover placement="bottom-start" width={220} role="menu" label="Hide key {k.index + 1} in…">
          {#snippet trigger({ toggle, attrs })}
            <Button size="sm" variant="ghost" icon={Plus} onclick={toggle} {...attrs}>Add</Button>
          {/snippet}
          {#snippet children({ close })}
            <Menu items={exceptions(k)} onclose={close} />
          {/snippet}
        </Popover>
      </div>
    </div>
  {:else}
    <EmptyState icon={Pin} title="No pinned keys" body="Pin a key to keep it in the same place in every profile, for example the Claude session key." actionLabel="Pin a key" onaction={add} />
  {/each}
  {#if pins.length && !supportsSetups && (hub.config.setup?.length ?? 0) > 0}
    <Banner kind="info">The running service cannot hide pinned keys per setup yet.</Banner>
  {/if}
</section>

<style>
  section {
    display: grid;
    gap: var(--space-3);
  }
  p {
    margin: 0;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .pin {
    display: grid;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-card);
  }
  .top {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .grow {
    flex: 1;
  }
  .except {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }
  .x {
    border: 0;
    padding: 0;
    background: none;
    cursor: pointer;
    color: inherit;
  }
</style>

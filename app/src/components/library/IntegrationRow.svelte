<!-- One integration in the library list: icon tile, name, status chip and Open (S22). -->
<script lang="ts">
  import { Gamepad2, Plug, Sparkles } from 'lucide-svelte';
  import type { IntegrationEntry, IntegrationState } from '../../lib/integrations.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';

  let { entry, onopen }: { entry: IntegrationEntry; onopen: () => void } = $props();

  const VARIANT: Record<IntegrationState, 'success' | 'warning' | 'danger' | 'off' | 'neutral'> = {
    checking: 'neutral',
    off: 'off',
    needs_setup: 'warning',
    on: 'success',
    running: 'success',
    bridge_down: 'warning',
    error: 'danger',
  };
  const Ic = $derived(entry.type === 'game' ? Gamepad2 : entry.type === 'claude' ? Sparkles : Plug);
</script>

<div class="row">
  <span class="tile" aria-hidden="true"><Ic size={18} /></span>
  <div class="text">
    <div class="t-heading name">{entry.title}</div>
    <div class="t-small muted">{entry.subtitle}</div>
  </div>
  <Chip variant={VARIANT[entry.state]}>{entry.stateLabel}</Chip>
  <Button size="sm" onclick={onopen} aria-label="Open {entry.title}">Open</Button>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: var(--color-surface);
    border-radius: var(--radius-row);
    box-shadow: var(--shadow-hairline);
  }
  .tile {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    flex: none;
    color: var(--color-accent);
    background: var(--color-raised);
    border-radius: var(--radius-lg);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .name {
    color: var(--color-text-1);
  }
</style>

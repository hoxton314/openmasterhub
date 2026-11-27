<!-- One integration card in the library grid: icon tile, name, status chip and Open (S22). -->
<script lang="ts">
  import { ArrowRight, Gamepad2, Plug, Sparkles } from 'lucide-svelte';
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

<div class="card">
  <div class="top">
    <span class="tile" aria-hidden="true"><Ic size={20} /></span>
    <div class="text">
      <div class="t-heading name">{entry.title}</div>
      <div class="t-small muted">{entry.subtitle}</div>
    </div>
  </div>
  <div class="foot">
    <Chip variant={VARIANT[entry.state]}>{entry.stateLabel}</Chip>
    <Button variant="ghost" size="sm" iconRight={ArrowRight} onclick={onopen} aria-label="Open {entry.title}">Open</Button>
  </div>
</div>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    background: var(--color-surface);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-hairline);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .tile {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
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

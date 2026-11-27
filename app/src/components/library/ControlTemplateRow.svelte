<!-- A control template as a compact row: module icon, name, the module it needs, and Use (S22). -->
<script lang="ts">
  import { Activity, CircleDot, Grid2x2, SlidersVertical, Volume2 } from 'lucide-svelte';
  import type { ModuleKind } from '../../lib/types';
  import Button from '../ui/Button.svelte';
  import type { Icon } from '../ui/icon';

  let { title, module, label, onuse }: { title: string; module?: ModuleKind; label: string; onuse: () => void } = $props();

  const ICON: Partial<Record<ModuleKind, Icon>> = { fader: SlidersVertical, dial: CircleDot, knob: Volume2, roller: Activity, keypad: Grid2x2 };
  const Ic = $derived((module && ICON[module]) || Grid2x2);
</script>

<div class="row">
  <span class="ic" aria-hidden="true"><Ic size={18} /></span>
  <div class="text">
    <div class="t-heading name">{title}</div>
    <div class="t-small muted">{label}</div>
  </div>
  <Button size="sm" onclick={onuse} aria-label="Use {title}">Use</Button>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    background: var(--color-surface);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-hairline);
  }
  .ic {
    display: grid;
    place-items: center;
    flex: none;
    color: var(--color-text-2);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .name {
    color: var(--color-text-1);
  }
</style>

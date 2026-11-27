<!-- Reset and Copy to… under a control's editors. -->
<script lang="ts">
  import { RotateCcw } from 'lucide-svelte';
  import { toastUndo } from '../../lib/hub.svelte';
  import type { ControlRef } from '../../lib/model/precedence';
  import Button from '../ui/Button.svelte';
  import CopyTo from './CopyTo.svelte';
  import { resetControl } from './control.svelte';

  let { ctl, label, disabled = false, resetLabel }: { ctl: ControlRef; label: string; disabled?: boolean; resetLabel?: string } = $props();
</script>

<div class="foot">
  <div class="row">
    <Button size="sm" variant="ghost" icon={RotateCcw} {disabled} onclick={() => { resetControl(ctl); toastUndo(`Reset ${label}`); }}>{resetLabel ?? `Reset ${label.toLowerCase()}`}</Button>
    <CopyTo {ctl} {disabled} />
  </div>
</div>

<style>
  /* Pinned to the bottom of the inspector (and sticky while the editors scroll). */
  .foot {
    position: sticky;
    bottom: 0;
    margin-top: auto;
    padding: 12px var(--size-panel-pad);
    background: var(--color-surface);
    border-top: var(--panel-border-width) solid var(--color-border);
  }
  .row {
    justify-content: space-between;
    display: flex;
    gap: 8px;
    align-items: center;
  }
</style>

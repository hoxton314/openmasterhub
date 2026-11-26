<!-- One toast: 16 px status icon, text, optional ghost action (Undo, Details). See shell/ToastHost. -->
<script lang="ts">
  import { CircleAlert, CircleCheck, Info, TriangleAlert, X } from 'lucide-svelte';
  import type { ToastAction, ToastKind } from '../../lib/ui.svelte';
  import Button from './Button.svelte';
  import IconButton from './IconButton.svelte';

  let { text, kind = 'info', action, leaving = false, ondismiss }: { text: string; kind?: ToastKind; action?: ToastAction; leaving?: boolean; ondismiss?: () => void } = $props();
  const Ic = $derived({ info: Info, success: CircleCheck, warning: TriangleAlert, danger: CircleAlert }[kind]);
</script>

<div class="toast {kind}" class:leaving role={kind === 'danger' ? 'alert' : 'status'}>
  <span class="icon"><Ic size={16} /></span>
  <span class="text">{text}</span>
  {#if action}
    <Button variant="ghost" size="sm" onclick={() => { action.run(); ondismiss?.(); }}>{action.label}</Button>
  {/if}
  {#if ondismiss}<IconButton icon={X} size="sm" label="Dismiss" onclick={ondismiss} />{/if}
</div>

<style>
  .toast {
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: 520px;
    padding: 8px 8px 8px 12px;
    color: var(--color-text-1);
    background: var(--color-raised);
    border-radius: var(--radius-toast);
    box-shadow: var(--shadow-popover);
    animation: rise var(--motion-toast-in);
  }
  .leaving {
    animation: sink var(--motion-toast-out) forwards;
  }
  .icon {
    display: inline-flex;
    color: var(--color-text-2);
  }
  .success .icon {
    color: var(--color-success);
  }
  .warning .icon {
    color: var(--color-warning);
  }
  .danger .icon {
    color: var(--color-danger);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
</style>

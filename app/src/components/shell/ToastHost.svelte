<!-- The toast stack, bottom centre. -->
<script lang="ts">
  import { ui } from '../../lib/ui.svelte';
  import Toast from '../ui/Toast.svelte';
</script>

<div class="host" aria-live="polite">
  {#each ui.toasts as t (t.id)}
    <Toast text={t.text} kind={t.kind} action={t.action} leaving={t.leaving} ondismiss={() => ui.dismiss(t.id)} />
  {/each}
</div>

<style>
  .host {
    position: fixed;
    left: 50%;
    bottom: 20px;
    z-index: 90;
    display: grid;
    justify-items: center;
    gap: 8px;
    transform: translateX(-50%);
    pointer-events: none;
  }
  .host > :global(*) {
    pointer-events: auto;
  }
</style>

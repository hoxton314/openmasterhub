<!-- "Setup triggers": the global shortcut that switches to each setup, and the other ways in. -->
<script lang="ts">
  import { effectiveSetups } from '../../lib/model';
  import { hub } from '../../lib/hub.svelte';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import KeyCapture from '../ui/KeyCapture.svelte';
  import { updateSetup } from './ops';

  let { setup, onclose }: { /** Only this setup; omitted = all of them. */ setup?: string; onclose: () => void } = $props();
  const list = $derived(effectiveSetups(hub.config).filter((s) => !setup || s.name === setup));

  const set = (name: string, combo: string | undefined) => updateSetup(name, (s) => (combo ? (s.shortcut = combo) : delete s.shortcut), 'change setup shortcut');
</script>

<Dialog title="Setup triggers" subtitle="Shortcuts, the tray and the palette switch at once. From the hub it takes a long-press on the dial, then a press to confirm." width={520} {onclose}>
  {#each list as s (s.name)}
    <div class="one">
      <KeyCapture label="Switch to {s.name}" value={s.shortcut} requireModifier onchange={(v) => set(s.name, v)} />
      {#if s.shortcut}<Button size="sm" variant="ghost" onclick={() => set(s.name, undefined)}>Remove</Button>{/if}
    </div>
  {/each}
  <p class="t-chip faint">The tray menu and the command palette (Ctrl K) always list every setup. Global shortcuts work on X11 and Windows; on Wayland, bind a key to <code>masterhubd ctl setup</code>.</p>
  {#snippet footer()}<Button variant="primary" onclick={onclose}>Done</Button>{/snippet}
</Dialog>

<style>
  .one {
    display: grid;
    gap: 4px;
    justify-items: start;
  }
  .one :global(.capture) {
    width: 100%;
  }
  code {
    font-family: var(--type-mono-family);
  }
</style>

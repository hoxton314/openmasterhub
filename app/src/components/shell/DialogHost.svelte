<!-- Renders `ui.ask` / `ui.confirm` dialogs (text prompt with suggestions, or a confirmation). -->
<script lang="ts">
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import TextField from '../ui/TextField.svelte';

  let value = $state('');
  $effect(() => {
    value = ui.dialog?.value ?? '';
  });

  const close = () => (ui.dialog = null);
  function submit() {
    const d = ui.dialog;
    if (!d) return;
    close();
    d.done(value.trim());
  }
  function addSuggestion(s: string) {
    const items = value.split(',').map((x) => x.trim()).filter(Boolean);
    if (!items.includes(s)) items.push(s);
    value = items.join(', ');
  }
</script>

{#if ui.dialog}
  {@const d = ui.dialog}
  <Dialog title={d.title} subtitle={d.input ? d.body : undefined} onclose={close}>
    {#if !d.input && d.body}<p>{d.body}</p>{/if}
    {#if d.input}
      <!-- svelte-ignore a11y_autofocus -->
      <TextField bind:value autofocus placeholder={d.placeholder} aria-label={d.title} onkeydown={(e) => e.key === 'Enter' && submit()} />
    {/if}
    {#if d.suggestions?.length}
      <div class="suggest">
        <span class="t-label">Open windows</span>
        <div class="chips">
          {#each d.suggestions as s}<button type="button" class="chip" onclick={() => addSuggestion(s)}>{s}</button>{/each}
        </div>
      </div>
    {/if}
    {#snippet footer()}
      <Button variant="ghost" onclick={close}>Cancel</Button>
      <Button variant={d.danger ? 'danger' : 'primary'} onclick={submit}>{d.ok}</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .suggest {
    display: grid;
    gap: 6px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    height: var(--size-chip);
    padding: 0 9px;
    font: var(--type-mono-weight) var(--type-chip-size) var(--type-mono-family);
    color: var(--color-text-2);
    background: var(--color-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-chip);
    cursor: pointer;
  }
  .chip:hover {
    color: var(--color-text-1);
    border-color: var(--color-border-2);
  }
</style>

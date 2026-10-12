<script lang="ts">
  import { ui } from '../lib/ui.svelte';

  let value = $state('');
  $effect(() => {
    value = ui.dialog?.value ?? '';
  });

  function close() {
    ui.dialog = null;
  }
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
  <div class="scrim" role="presentation" onclick={close}>
    <div class="dialog" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.key === 'Escape' && close()}>
      <h2>{d.title}</h2>
      {#if d.body}<p>{d.body}</p>{/if}
      {#if d.input}
        <!-- svelte-ignore a11y_autofocus -->
        <input class="input" autofocus placeholder={d.placeholder} bind:value onkeydown={(e) => e.key === 'Enter' && submit()} />
      {/if}
      {#if d.suggestions?.length}
        <div class="suggest">
          <span>Open windows</span>
          <div class="chips">
            {#each d.suggestions as s}<button class="chip" onclick={() => addSuggestion(s)}>{s}</button>{/each}
          </div>
        </div>
      {/if}
      <div class="actions">
        <button class="btn ghost" onclick={close}>Cancel</button>
        <button class="btn {d.danger ? 'danger' : 'primary'}" onclick={submit}>{d.ok}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: start center;
    padding-top: 18vh;
    background: rgb(0 0 0 / 0.5);
    animation: fade-in 0.12s ease-out;
  }
  .dialog {
    width: 420px;
    padding: 18px;
    display: grid;
    gap: 12px;
    background: var(--bg-raised);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-pop);
    animation: pop 0.14s var(--ease);
  }
  h2 {
    font-size: 15px;
    font-weight: 600;
  }
  p {
    color: var(--text-2);
  }
  .input {
    height: 34px;
  }
  .suggest span {
    font-size: 12px;
    color: var(--text-2);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 6px;
  }
  .chip {
    height: 24px;
    padding: 0 9px;
    font: inherit;
    font-size: 12px;
    color: var(--text-2);
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 99px;
    cursor: pointer;
  }
  .chip:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 4px;
  }
  .btn.danger {
    background: var(--danger);
    color: white;
    border-color: transparent;
  }
</style>

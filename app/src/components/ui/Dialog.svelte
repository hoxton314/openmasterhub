<!--
  Modal dialog: w420 raised, title + optional subtitle and close button, body (text2), footer
  right-aligned. Escape and the scrim close it. Destructive dialogs name what is affected.
-->
<script lang="ts">
  import { X } from 'lucide-svelte';
  import type { Snippet } from 'svelte';
  import IconButton from './IconButton.svelte';

  let {
    open = true,
    title,
    subtitle,
    width,
    onclose,
    children,
    footer,
  }: { open?: boolean; title: string; subtitle?: string; width?: number; onclose: () => void; children?: Snippet; footer?: Snippet } = $props();

  let box: HTMLElement | undefined = $state();
  let previous: Element | null = null;
  $effect(() => {
    if (!open || !box) return;
    previous = document.activeElement;
    (box.querySelector<HTMLElement>('[autofocus], input, select, textarea') ?? box).focus();
    return () => (previous as HTMLElement | null)?.focus?.();
  });

  /** Keep Tab inside the dialog. */
  function trap(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onclose();
      return;
    }
    if (e.key !== 'Tab' || !box) return;
    const f = [...box.querySelectorAll<HTMLElement>('button, input, select, textarea, [tabindex]:not([tabindex="-1"])')].filter((x) => !x.hasAttribute('disabled'));
    if (!f.length) return;
    const [first, last] = [f[0], f[f.length - 1]];
    if (e.shiftKey && document.activeElement === first) (e.preventDefault(), last.focus());
    else if (!e.shiftKey && document.activeElement === last) (e.preventDefault(), first.focus());
  }
</script>

{#if open}
  <div class="scrim" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && onclose()}>
    <div class="dialog" role="dialog" aria-modal="true" aria-label={title} tabindex="-1" style={width ? `width:${width}px` : ''} bind:this={box} onkeydown={trap}>
      <header>
        <div class="titles">
          <h2>{title}</h2>
          {#if subtitle}<p class="subtitle">{subtitle}</p>{/if}
        </div>
        <IconButton icon={X} label="Close" onclick={onclose} />
      </header>
      {#if children}<div class="body">{@render children()}</div>{/if}
      {#if footer}<footer>{@render footer()}</footer>{/if}
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: start center;
    padding: 16vh 16px 16px;
    background: var(--color-overlay);
    animation: fade-in var(--motion-fast);
  }
  .dialog {
    width: var(--size-dialog-width);
    max-width: 100%;
    max-height: 76vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--color-dialog);
    border: var(--size-dialog-border) solid var(--color-border-2);
    border-radius: var(--radius-dialog);
    box-shadow: var(--shadow-dialog);
    animation: pop var(--motion-panel);
    outline: none;
  }
  header {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 18px 20px 14px;
    border-bottom: 1px solid var(--color-border);
  }
  .titles {
    flex: 1;
    display: grid;
    gap: 4px;
  }
  h2 {
    font: var(--type-heading-weight) calc(var(--type-heading-size) - 1px) / var(--type-heading-lh) var(--type-heading-family);
    text-transform: var(--type-heading-case);
    letter-spacing: var(--type-heading-tracking);
  }
  .subtitle {
    font-size: var(--type-small-size);
    color: var(--color-text-2);
  }
  .body {
    padding: 16px 20px;
    overflow-y: auto;
    display: grid;
    gap: 12px;
    color: var(--color-text-2);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--color-border);
    background: var(--color-surface);
  }
</style>

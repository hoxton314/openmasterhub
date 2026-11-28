<!--
  Popover anchored to its trigger, drawn in a fixed layer so scrolling panels do not clip it.
  Closes on outside click and Escape.

  <Popover bind:open>
    {#snippet trigger({ toggle, attrs })}<Button {...attrs} onclick={toggle}>Why?</Button>{/snippet}
    {#snippet children({ close })}…{/snippet}
  </Popover>
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  type Placement = 'bottom-start' | 'bottom-end' | 'top-start' | 'top-end' | 'right-start' | 'left-start';
  let {
    open = $bindable(false),
    placement = 'bottom-start',
    width,
    label,
    role = 'dialog',
    onclose,
    trigger,
    children,
  }: {
    open?: boolean;
    placement?: Placement;
    width?: number;
    label?: string;
    role?: 'dialog' | 'menu' | 'listbox';
    onclose?: () => void;
    trigger: Snippet<[{ toggle: () => void; open: boolean; attrs: Record<string, string | boolean> }]>;
    children: Snippet<[{ close: () => void }]>;
  } = $props();

  let anchor: HTMLElement | undefined = $state();
  let pop: HTMLElement | undefined = $state();
  let pos = $state({ x: 0, y: 0 });

  function close() {
    if (!open) return;
    open = false;
    onclose?.();
  }
  const toggle = () => (open ? close() : (open = true));

  function place() {
    if (!anchor || !pop) return;
    const a = anchor.getBoundingClientRect();
    const p = pop.getBoundingClientRect();
    const gap = 6;
    let x = placement.endsWith('end') ? a.right - p.width : a.left;
    let y = placement.startsWith('top') ? a.top - p.height - gap : a.bottom + gap;
    if (placement === 'right-start') [x, y] = [a.right + gap, a.top];
    if (placement === 'left-start') [x, y] = [a.left - p.width - gap, a.top];
    if (y + p.height > innerHeight - 8 && placement.startsWith('bottom')) y = Math.max(8, a.top - p.height - gap);
    pos = { x: Math.max(8, Math.min(x, innerWidth - p.width - 8)), y: Math.max(8, Math.min(y, innerHeight - p.height - 8)) };
  }

  $effect(() => {
    if (!open || !pop) return;
    place();
    const ro = new ResizeObserver(place);
    ro.observe(pop);
    addEventListener('resize', place);
    addEventListener('scroll', place, true);
    return () => {
      ro.disconnect();
      removeEventListener('resize', place);
      removeEventListener('scroll', place, true);
    };
  });

  function outside(e: PointerEvent) {
    const t = e.target as Node;
    if (open && !anchor?.contains(t) && !pop?.contains(t)) close();
  }
  function key(e: KeyboardEvent) {
    if (open && e.key === 'Escape') {
      e.preventDefault();
      close();
      anchor?.querySelector<HTMLElement>('button, [tabindex]')?.focus();
    }
  }
</script>

<svelte:window onpointerdown={outside} onkeydown={key} />

<span class="anchor" bind:this={anchor}>
  {@render trigger({ toggle, open, attrs: { 'aria-haspopup': role === 'dialog' ? 'dialog' : role, 'aria-expanded': open } })}
</span>
{#if open}
  <div class="pop" bind:this={pop} {role} aria-label={label} tabindex="-1" style="left:{pos.x}px;top:{pos.y}px;{width ? `width:${width}px` : ''}">
    {@render children({ close })}
  </div>
{/if}

<style>
  .anchor {
    display: inline-flex;
    min-width: 0;
  }
  .pop {
    position: fixed;
    z-index: 70;
    min-width: 180px;
    max-height: min(480px, calc(100vh - 16px));
    overflow-y: auto;
    padding: 4px;
    color: var(--color-text-1);
    background: var(--color-raised);
    border-radius: var(--radius-popover);
    box-shadow: var(--shadow-popover);
    animation: pop var(--motion-fast);
    outline: none;
  }
</style>

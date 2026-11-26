<!-- Text tabs (gallery: Templates · Integrations · My profiles · Import file · Blank). -->
<script lang="ts" generics="T extends string">
  let { tabs, value, onchange, label }: { tabs: { value: T; label: string; count?: number }[]; value: T; onchange: (v: T) => void; label?: string } = $props();

  function key(e: KeyboardEvent, i: number) {
    const d = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const next = tabs[(i + d + tabs.length) % tabs.length];
    onchange(next.value);
    (e.currentTarget as HTMLElement).parentElement?.querySelector<HTMLElement>(`[data-tab="${next.value}"]`)?.focus();
  }
</script>

<div class="tabs" role="tablist" aria-label={label}>
  {#each tabs as t, i (t.value)}
    <button type="button" role="tab" data-tab={t.value} aria-selected={t.value === value} tabindex={t.value === value ? 0 : -1} onclick={() => onchange(t.value)} onkeydown={(e) => key(e, i)}>
      {t.label}{#if t.count !== undefined}<span class="count">{t.count}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    gap: 18px;
    border-bottom: 1px solid var(--color-border);
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    padding: 0 2px;
    font-weight: 500;
    color: var(--color-text-2);
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    cursor: pointer;
  }
  button:hover {
    color: var(--color-text-1);
  }
  [aria-selected='true'] {
    color: var(--color-text-1);
    font-weight: 600;
    border-bottom-color: var(--color-accent);
  }
  .count {
    font-size: var(--type-label-size);
    color: var(--color-text-3);
  }
</style>

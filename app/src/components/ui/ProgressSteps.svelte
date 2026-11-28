<!-- First-run step dots: done, current, upcoming. -->
<script lang="ts">
  let { count, current, labels }: { count: number; current: number; labels?: string[] } = $props();
</script>

<ol class="steps" aria-label="Step {current + 1} of {count}">
  {#each Array(count) as _, i}
    <li class:done={i < current} class:current={i === current} aria-current={i === current ? 'step' : undefined} title={labels?.[i]}>
      <span class="visually-hidden">{labels?.[i] ?? `Step ${i + 1}`}</span>
    </li>
  {/each}
</ol>

<style>
  .steps {
    display: flex;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    width: 8px;
    height: 8px;
    border-radius: var(--radius-pill);
    background: var(--color-border-2);
    transition: width var(--motion-panel), background var(--motion-panel);
  }
  .done {
    background: var(--color-accent);
  }
  .current {
    width: 22px;
    background: var(--color-accent-fill);
  }
</style>

<!-- A small status dot with a soft halo. Pair it with text: status is never colour alone. -->
<script lang="ts">
  let { status = 'success', size = 7, halo = true, label }: { status?: 'success' | 'warning' | 'danger' | 'accent' | 'off'; size?: number; halo?: boolean; label?: string } = $props();
</script>

<span class="dot {status}" class:halo style="--d:{size}px" role={label ? 'img' : undefined} aria-label={label} aria-hidden={label ? undefined : 'true'}></span>

<style>
  .dot {
    display: inline-block;
    flex: none;
    width: var(--d);
    height: var(--d);
    border-radius: 50%;
    background: var(--c);
  }
  .halo {
    box-shadow: 0 0 0 3px var(--h);
  }
  .success {
    --c: var(--live-color);
    --h: var(--live-halo);
  }
  .warning {
    --c: var(--color-warning);
    --h: var(--color-warning-soft);
  }
  .danger {
    --c: var(--color-danger);
    --h: var(--color-danger-soft);
  }
  .accent {
    --c: var(--color-accent);
    --h: var(--color-accent-soft);
  }
  .off {
    --c: var(--color-text-3);
    --h: transparent;
  }
</style>

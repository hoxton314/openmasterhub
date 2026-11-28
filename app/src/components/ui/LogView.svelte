<!-- Mono log lines coloured by level, with Copy log / Open log folder. -->
<script lang="ts" module>
  export type LogLine = string | { level?: 'error' | 'warn' | 'info' | 'debug'; text: string };
</script>

<script lang="ts">
  import { Copy, FolderOpen } from 'lucide-svelte';
  import Button from './Button.svelte';

  let { lines, height = 220, onopenfolder }: { lines: LogLine[]; height?: number; onopenfolder?: () => void } = $props();

  const parsed = $derived(
    lines.map((l) => {
      if (typeof l !== 'string') return { level: l.level ?? 'info', text: l.text };
      const m = l.match(/\b(ERROR|WARN|INFO|DEBUG|TRACE)\b/);
      return { level: (m?.[1].toLowerCase().replace('trace', 'debug') ?? 'info') as string, text: l };
    }),
  );
  let copied = $state(false);
  async function copy() {
    await navigator.clipboard?.writeText(parsed.map((l) => l.text).join('\n'));
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }
</script>

<div class="log">
  <pre style="max-height:{height}px">{#each parsed as l}<span class={l.level}>{l.text}
</span>{/each}</pre>
  <div class="actions">
    <Button size="sm" icon={Copy} onclick={copy}>{copied ? 'Copied' : 'Copy log'}</Button>
    {#if onopenfolder}<Button size="sm" variant="ghost" icon={FolderOpen} onclick={onopenfolder}>Open log folder</Button>{/if}
  </div>
</div>

<style>
  .log {
    display: grid;
    gap: 8px;
  }
  pre {
    margin: 0;
    padding: 10px 12px;
    overflow: auto;
    font: var(--type-mono-weight) var(--type-mono-size) / 1.5 var(--type-mono-family);
    color: var(--color-text-2);
    background: var(--color-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-well);
    user-select: text;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .error {
    color: var(--color-danger);
  }
  .warn {
    color: var(--color-warning);
  }
  .debug {
    color: var(--color-text-3);
  }
  .actions {
    display: flex;
    gap: 6px;
  }
</style>

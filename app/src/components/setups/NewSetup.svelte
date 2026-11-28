<!-- "New setup": start from a template or from scratch. Picking one creates the setup and opens it. -->
<script lang="ts">
  import { AppWindow, Gamepad2, Plus, Radio } from 'lucide-svelte';
  import { hub } from '../../lib/hub.svelte';
  import { listTemplates, moduleWarning, type Template } from '../../lib/templates';
  import type { Icon } from '../ui/icon';
  import { createFromTemplate } from './ops';

  let { onpick }: { onpick: (setup: string) => void } = $props();

  const ICONS: Record<string, Icon> = { 'setup-creative': AppWindow, 'setup-streaming': Radio, 'setup-games': Gamepad2, 'setup-blank': Plus };
  const TITLES: Record<string, string> = { 'setup-creative': 'Creative apps', 'setup-streaming': 'Streaming', 'setup-games': 'Games', 'setup-blank': 'Blank setup' };
  const ABOUT: Record<string, string> = {
    'setup-creative': 'Blender, Godot, DAW · follow apps',
    'setup-streaming': 'OBS scenes · manual',
    'setup-games': 'Your games + game modes',
    'setup-blank': 'Choose the mode yourself',
  };
  const ORDER = ['setup-creative', 'setup-streaming', 'setup-games', 'setup-blank'];
  const templates = $derived(listTemplates('setup').sort((a, b) => ORDER.indexOf(a.id) - ORDER.indexOf(b.id)));

  function pick(t: Template) {
    const name = createFromTemplate(t);
    if (name) onpick(name);
  }
</script>

<div class="list">
  {#each templates as t (t.id)}
    {@const Ic = ICONS[t.id] ?? Plus}
    {@const warn = moduleWarning(t, hub.modules)}
    <button type="button" class="tpl" onclick={() => pick(t)}>
      <span class="ic"><Ic size={16} /></span>
      <span class="text">
        <b>{TITLES[t.id] ?? t.name}</b>
        <span class="t-small muted">{ABOUT[t.id] ?? t.about}</span>
        {#if warn}<span class="warn t-chip">{warn}</span>{/if}
      </span>
    </button>
  {/each}
</div>

<style>
  .list {
    display: grid;
    gap: 8px;
  }
  .tpl {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    text-align: left;
    color: var(--color-text-1);
    background: var(--color-surface);
    border: var(--panel-border-width) solid var(--color-border);
    border-radius: var(--radius-panel);
    cursor: pointer;
  }
  .tpl:hover {
    border-color: var(--color-border-2);
    background: var(--color-selection);
  }
  .ic {
    display: inline-flex;
    flex: none;
    color: var(--color-text-1);
  }
  .text {
    display: grid;
    gap: 1px;
    min-width: 0;
  }
  .warn {
    color: var(--color-warning);
  }
</style>

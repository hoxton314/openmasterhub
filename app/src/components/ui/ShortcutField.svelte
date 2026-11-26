<!-- Records a global shortcut (setup switching, pin): needs a modifier, no hold, no media keys. -->
<script lang="ts">
  import { X } from 'lucide-svelte';
  import IconButton from './IconButton.svelte';
  import KeyCapture from './KeyCapture.svelte';

  let { value, onchange, label = 'Shortcut' }: { value: string | undefined; onchange: (combo: string | undefined) => void; label?: string } = $props();
</script>

<div class="sc">
  <KeyCapture {value} {label} fallback={false} requireModifier onchange={(c) => onchange(c)} />
  {#if value}<span class="clear"><IconButton icon={X} size="sm" label="Remove shortcut" onclick={() => onchange(undefined)} /></span>{/if}
</div>

<style>
  .sc {
    position: relative;
  }
  .clear {
    position: absolute;
    top: 0;
    right: 0;
  }
</style>

<!-- "Switching": how the setup picks its profile (follow / manual / hybrid). -->
<script lang="ts">
  import { hasCapability } from '../../lib/hub.svelte';
  import type { SetupMode } from '../../lib/types';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import { MODES } from './ops';

  let { mode, onchange }: { mode: SetupMode; onchange: (m: SetupMode) => void } = $props();
  const options = $derived(
    MODES.map((m) => ({
      value: m.value,
      label: m.label,
      disabled: m.value === 'hybrid' && !hasCapability('pin'),
      title: m.value === 'hybrid' && !hasCapability('pin') ? 'This service version cannot pin a profile yet' : undefined,
    })),
  );
</script>

<div class="switching">
  <SectionLabel>Switching</SectionLabel>
  <Segmented label="Switching mode" {options} value={mode} {onchange} />
</div>

<style>
  .switching {
    display: grid;
    justify-items: start;
    gap: 8px;
  }
</style>

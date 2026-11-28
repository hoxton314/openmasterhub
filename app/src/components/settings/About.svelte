<!-- Settings › About: version, licence, source. -->
<script lang="ts">
  import { ExternalLink } from 'lucide-svelte';
  import pkg from '../../../package.json';
  import { hasCapability } from '../../lib/hub.svelte';
  import { invoke } from '../../lib/backend';
  import { onMount } from 'svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import Page from './Page.svelte';
  import Row from './Row.svelte';
  import { openPath } from './actions';

  const SOURCE = 'https://github.com/hoxton314/openmasterhub';
  let service = $state<string | undefined>();
  onMount(async () => {
    if (!hasCapability('system_info')) return;
    const info = await invoke<{ version?: string }>('system_info').catch(() => undefined);
    service = info?.version;
  });
</script>

<Page title="About" about="An open driver, service and editor for the Cooler Master MasterHUB. Not affiliated with Cooler Master.">
  <div class="list">
    <Row label="OpenMasterHUB"><Chip variant="neutral" icon={null}>{pkg.version}</Chip></Row>
    {#if service}<Row label="Service (masterhubd)"><Chip variant="neutral" icon={null}>{service}</Chip></Row>{/if}
    <Row label="Licence" help="Free to use, change and share"><Chip variant="outline" icon={null}>MIT</Chip></Row>
    <Row label="Source code" help={SOURCE}>
      <Button size="sm" icon={ExternalLink} onclick={() => openPath(SOURCE)}>Open</Button>
    </Row>
  </div>
</Page>

<style>
  .list {
    display: grid;
  }
</style>

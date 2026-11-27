<!-- Settings › Service: autostart, log location, restart. -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { RotateCw } from 'lucide-svelte';
  import { hub, toast } from '../../lib/hub.svelte';
  import Button from '../ui/Button.svelte';
  import LogView from '../ui/LogView.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import StatusDot from '../ui/StatusDot.svelte';
  import Toggle from '../ui/Toggle.svelte';
  import { openPath, run } from './actions';
  import Page from './Page.svelte';
  import Row from './Row.svelte';

  let autostart = $state(true);
  let logDir = $state('');
  let lines = $state<string[]>([]);

  async function refresh() {
    autostart = (await run<boolean>('service_autostart')) ?? autostart;
    logDir = (await run<string>('service_log_dir')) ?? '';
    lines = (await run<string[]>('service_log', { lines: 40 })) ?? [];
  }
  onMount(refresh);

  async function setAutostart(on: boolean) {
    autostart = on;
    await run('set_service_autostart', { on }, 'Could not change autostart');
  }
  async function restart() {
    await run('restart_service', undefined, 'Could not restart the service');
    toast('Restarting the service');
    setTimeout(refresh, 1500);
  }
</script>

<Page title="Service" about="masterhubd is the background service that talks to the hub. The app only edits your config.">
  <section>
    <SectionLabel as="h3">Status</SectionLabel>
    <Row label={hub.online ? 'Service running' : 'Service stopped'} help={hub.online ? `${hub.modules.length} modules connected` : 'Settings still work; the hub does not follow your changes'}>
      <StatusDot status={hub.online ? 'success' : 'danger'} />
      <Button icon={RotateCw} onclick={restart}>Restart service</Button>
    </Row>
    <Row label="Start with my session" help="Starts the service when you log in, so the hub is ready">
      <Toggle title="Start with my session" checked={autostart} onchange={setAutostart} />
    </Row>
  </section>
  <section>
    <SectionLabel as="h3">Log</SectionLabel>
    {#if logDir}<p class="t-small muted"><code>{logDir}</code></p>{/if}
    <LogView {lines} onopenfolder={logDir ? () => openPath(logDir) : undefined} />
  </section>
</Page>

<style>
  section {
    display: grid;
    gap: var(--space-3);
  }
  p {
    margin: 0;
  }
</style>

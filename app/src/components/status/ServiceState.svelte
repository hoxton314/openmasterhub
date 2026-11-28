<!-- The service is not reachable (S05): stopped, starting, or crashed with its log. Settings stay available. -->
<script lang="ts">
  import { CircleAlert, Loader, Power, RotateCw, Settings } from 'lucide-svelte';
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '../../lib/backend';
  import { toast } from '../../lib/hub.svelte';
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import LogView from '../ui/LogView.svelte';

  type Phase = 'stopped' | 'starting' | 'crashed';
  interface Status {
    state: Phase;
    /** Short reason for a crash, in plain words. */
    reason?: string;
  }

  let status = $state<Status>({ state: 'stopped' });
  let lines = $state<string[]>([]);
  let showLog = $state(false);
  let timer: ReturnType<typeof setInterval> | undefined;

  async function poll() {
    status = (await invoke<Status>('service_status').catch(() => undefined)) ?? (status.state === 'starting' ? status : { state: 'stopped' });
    if (status.state !== 'stopped' || showLog) lines = (await invoke<string[]>('service_log', { lines: 40 }).catch(() => undefined)) ?? lines;
  }
  onMount(() => {
    poll();
    timer = setInterval(poll, 2000);
  });
  onDestroy(() => clearInterval(timer));

  async function start() {
    status = { state: 'starting' };
    try {
      await invoke(status.state === 'starting' && lines.length ? 'restart_service' : 'start_service');
    } catch (e) {
      status = { state: 'stopped' };
      toast({ text: `Could not start the service: ${e}`, kind: 'danger' });
    }
  }
  async function restart() {
    status = { state: 'starting' };
    await invoke('restart_service').catch((e) => toast({ text: `Could not restart the service: ${e}`, kind: 'danger' }));
  }
  async function openLogFolder() {
    const dir = await invoke<string>('service_log_dir').catch(() => undefined);
    if (dir) await invoke('open_path', { path: dir }).catch(() => undefined);
  }
</script>

<div class="state" role="status" aria-live="polite">
  <div class="badge" class:bad={status.state === 'crashed'}>
    {#if status.state === 'starting'}<span class="spin"><Loader size={22} /></span>
    {:else if status.state === 'crashed'}<CircleAlert size={22} />
    {:else}<Power size={22} />{/if}
  </div>

  {#if status.state === 'starting'}
    <h1 class="t-section">Starting the service</h1>
    <p>Opening the hub and loading your config. This takes a second or two.</p>
    <Button variant="ghost" onclick={() => (showLog = !showLog)}>{showLog ? 'Hide log' : 'Show log'}</Button>
  {:else if status.state === 'crashed'}
    <h1 class="t-section">The service stopped unexpectedly</h1>
    <p>{status.reason ? `It ran into ${status.reason}. ` : ''}Your config is safe. Restarting usually fixes it.</p>
    <div class="buttons">
      <Button variant="primary" icon={RotateCw} onclick={restart}>Restart service</Button>
    </div>
  {:else}
    <h1 class="t-section">The MasterHUB service isn’t running</h1>
    <p>The app edits your config, but only the service talks to the hub. It starts with your session unless you turned that off.</p>
    <div class="buttons">
      <Button variant="primary" icon={Power} onclick={start}>Start service</Button>
      <Button variant="ghost" icon={Settings} onclick={() => ui.go('settings', { view: 'service' })}>Autostart settings</Button>
    </div>
  {/if}

  {#if (status.state === 'crashed' || showLog) && lines.length}
    <div class="log"><LogView {lines} height={180} onopenfolder={openLogFolder} /></div>
  {/if}
</div>

<style>
  .state {
    margin: auto;
    max-width: 480px;
    padding: var(--space-6);
    display: grid;
    justify-items: center;
    gap: var(--space-3);
    text-align: center;
  }
  .badge {
    width: 48px;
    height: 48px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-card);
    color: var(--color-text-2);
    background: var(--color-raised);
    box-shadow: var(--shadow-hairline);
  }
  .badge.bad {
    color: var(--color-danger);
    background: var(--color-danger-soft);
  }
  .spin {
    display: grid;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(calc(360deg * var(--motion-scale, 1)));
    }
  }
  h1,
  p {
    margin: 0;
  }
  p {
    color: var(--color-text-2);
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .log {
    width: 100%;
    text-align: left;
    margin-top: var(--space-3);
  }
</style>

<!-- Hardware problems (S06): no modules, a module that needs a replug, the vendor app running. Mount it in place of the hub drawing while `hardwareIssue()` says there is one. -->
<script lang="ts">
  import { CircleOff, Plug, RefreshCw, TriangleAlert, X } from 'lucide-svelte';
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '../../lib/backend';
  import { hasCapability, hub, reload, toast } from '../../lib/hub.svelte';
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import { hardwareIssue, vendorApp, type VendorApp } from './hardware';

  let vendor = $state<VendorApp | null>(null);
  let timer: ReturnType<typeof setInterval> | undefined;
  let looking = $state(false);
  const issue = $derived(hardwareIssue(vendor));
  const stuck = $derived(hub.modules.filter((m) => m.needs_replug));

  onMount(() => {
    const check = async () => (vendor = await vendorApp());
    check();
    timer = setInterval(check, 3000);
  });
  onDestroy(() => clearInterval(timer));

  async function lookAgain() {
    looking = true;
    await reload().catch(() => undefined);
    looking = false;
    if (!hub.modules.length) toast('Still no modules found');
  }
  async function closeVendor() {
    if (!vendor) return;
    const { name } = vendor;
    ui.confirm({
      title: `Close ${name}?`,
      body: 'It will quit without asking. Unsaved settings in it are lost. After that, modules it left in their built-in mode may need a replug.',
      ok: `Close ${name}`,
      danger: false,
      done: async () => {
        await invoke('close_vendor_app').catch((e) => toast({ text: `Could not close it: ${e}`, kind: 'danger' }));
        vendor = await vendorApp();
      },
    });
  }
  const identify = (serial: string) => invoke('identify', { serial }).catch(() => undefined);
</script>

{#if issue}
  <div class="state" role="status">
    {#if issue === 'vendor-app' && vendor}
      <div class="badge warn"><TriangleAlert size={22} /></div>
      <h1 class="t-section">The vendor MasterHUB app is running</h1>
      <p>Both apps try to drive the hub at once, which can leave modules stuck. Close the other app to continue.</p>
      <div class="proc">
        <b class="t-mono">{vendor.name}</b><span class="muted t-small">PID {vendor.pid}</span><Chip variant="warning" icon={null}>running</Chip>
      </div>
      <div class="buttons">
        <Button variant="primary" icon={X} onclick={closeVendor}>Close {vendor.name}</Button>
        <Button variant="ghost" onclick={async () => (vendor = await vendorApp())}>I closed it</Button>
      </div>
    {:else if issue === 'replug'}
      <div class="badge warn"><Plug size={22} /></div>
      <h1 class="t-section">{stuck.length === 1 ? `The ${stuck[0].kind} needs a replug` : 'Some modules need a replug'}</h1>
      <p>{stuck.length === 1 ? 'It is' : 'They are'} still in the built-in mode (the logo animation keeps running), so {stuck.length === 1 ? 'it ignores' : 'they ignore'} the app. This happens after the vendor app or a firmware reset.</p>
      <ol>
        <li><span class="n">1</span>Unplug the USB cable</li>
        <li><span class="n">2</span>Wait 3 seconds</li>
        <li><span class="n">3</span>Plug it back in</li>
      </ol>
      <div class="buttons">
        <Button variant="primary" onclick={lookAgain}>I replugged it</Button>
        {#if hasCapability('identify')}
          {#each stuck as m (m.serial)}<Button variant="ghost" onclick={() => identify(m.serial)}>Identify {m.kind}</Button>{/each}
        {/if}
      </div>
    {:else}
      <div class="badge"><CircleOff size={22} /></div>
      <h1 class="t-section">No modules connected</h1>
      <p>The service is running but sees no MasterHUB. Check the USB cable and that the Base has power.</p>
      <p class="t-small">On Linux, the udev rule must be installed:</p>
      <pre class="t-mono">sudo cp 70-masterhub.rules /etc/udev/rules.d/</pre>
      <div class="buttons">
        <Button variant="primary" icon={RefreshCw} onclick={lookAgain} disabled={looking}>Look again</Button>
      </div>
    {/if}
  </div>
{/if}

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
  .badge.warn {
    color: var(--color-warning);
    background: var(--color-warning-soft);
  }
  h1,
  p {
    margin: 0;
  }
  p {
    color: var(--color-text-2);
  }
  pre {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    max-width: 100%;
    overflow-x: auto;
    color: var(--color-text-2);
    background: var(--color-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-well);
    user-select: text;
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-2);
    text-align: left;
  }
  li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .n {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-pill);
    color: var(--color-accent);
    background: var(--color-accent-soft);
    font-weight: 600;
  }
  .proc {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-row);
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
</style>

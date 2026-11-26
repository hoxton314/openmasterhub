<!-- WP6 owns this file. Wave 0 stand-in: the pre-redesign "service isn't running" screen. -->
<script lang="ts">
  import { Power } from 'lucide-svelte';
  import { invoke } from '../../lib/backend';
  import { toast } from '../../lib/hub.svelte';
  import Button from '../ui/Button.svelte';
</script>

<div class="offline">
  <div class="badge"><Power size={22} /></div>
  <h1 class="t-section">The MasterHUB service isn’t running</h1>
  <p>OpenMasterHUB configures <code>masterhubd</code>, the background service that drives the hardware. Start it, or run <code>masterhubd</code> in a terminal.</p>
  <Button variant="primary" onclick={() => invoke('start_service').catch((e) => toast(String(e)))}>Start service</Button>
</div>

<style>
  .offline {
    margin: auto;
    max-width: 420px;
    display: grid;
    justify-items: center;
    gap: 10px;
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
  p {
    color: var(--color-text-2);
    margin-bottom: 8px;
  }
</style>

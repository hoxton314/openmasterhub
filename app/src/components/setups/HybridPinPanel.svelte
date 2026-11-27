<!-- Hybrid setups: who is pinned and since when, with Release now (or Pin the live profile). -->
<script lang="ts">
  import { Pin, PinOff } from 'lucide-svelte';
  import { invoke } from '../../lib/backend';
  import { hasCapability, hub, toast } from '../../lib/hub.svelte';
  import KeyCombo from '../ui/KeyCombo.svelte';
  import Banner from '../ui/Banner.svelte';
  import Button from '../ui/Button.svelte';

  let { setup, shortcut }: { setup: string; shortcut?: string } = $props();

  const pinned = $derived(hub.state.pinned);
  let now = $state(Date.now());
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 15_000);
    return () => clearInterval(t);
  });
  const ago = $derived.by(() => {
    if (!pinned?.since) return '';
    const m = Math.max(0, Math.round((now / 1000 - pinned.since) / 60));
    return m < 1 ? 'just now' : m < 60 ? `${m} min ago` : `${Math.round(m / 60)} h ago`;
  });

  async function release() {
    try {
      await invoke('release_pin');
    } catch (e) {
      toast(`Could not release the pin: ${e}`);
    }
  }
  async function pin() {
    try {
      await invoke('pin_profile', { name: hub.state.profile });
    } catch (e) {
      toast(`Could not pin: ${e}`);
    }
  }
  const active = $derived(hub.state.setup === setup);
</script>

{#if pinned && active}
  <Banner kind="accent" icon={Pin}>
    <b>{pinned.profile} is pinned</b>
    <div class="sub">
      {#if ago}Pinned {ago}. {/if}Rules resume when you release it.
    </div>
    {#snippet actions()}<Button size="sm" icon={PinOff} onclick={release}>Release now</Button>{/snippet}
  </Banner>
{:else}
  <Banner kind="info" icon={Pin}>
    <b>Nothing is pinned</b>
    <div class="sub">
      {#if active}The app rules choose the profile.{:else}This setup is not active on the hub.{/if}
      {#if shortcut}&nbsp;Pin the live profile with <KeyCombo combo={shortcut} />.{/if}
    </div>
    {#snippet actions()}
      {#if active && hasCapability('pin')}<Button size="sm" icon={Pin} onclick={pin}>Pin {hub.state.profile}</Button>{/if}
    {/snippet}
  </Banner>
{/if}

<style>
  .sub {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
    font-size: var(--type-small-size);
    color: var(--color-text-2);
  }
</style>

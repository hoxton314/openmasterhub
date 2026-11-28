<!-- What an integration is drawing right now (the S03 caption under the hub). -->
<script lang="ts">
  import { Plug } from 'lucide-svelte';
  import { hub, isEditingLive } from '../../lib/hub.svelte';

  const claims = $derived(isEditingLive() ? hub.claims.filter((c) => c.active) : []);
  const names = $derived(
    claims.map((c) => {
      const o = c.owns ?? {};
      const parts = [(o.keys?.length ?? 0) >= 15 && 'keypad', o.dial && 'dial', (o.knobs?.length ?? 0) >= 3 && 'knobs', (o.rollers?.length ?? 0) >= 2 && 'rollers', (o.faders?.length ?? 0) >= 5 && 'faders'].filter(Boolean) as string[];
      return { name: c.name, owns: parts.join(' and '), app: c.apps?.[0] ?? hub.state.active_app?.split('.').pop() };
    }),
  );
  const unclaimed = $derived(['knobs', 'rollers', 'faders'].filter((k) => !names.some((n) => n.owns.includes(k))));
</script>

{#each names as n}
  <p class="integration">
    <Plug size={13} />
    <span>{n.name} draws {n.owns || 'some controls'}{n.app ? ` while ${n.app} is focused` : ''}.{n.owns && unclaimed.length ? ` ${unclaimed.slice(0, 2).join(' and ').replace(/^./, (c) => c.toUpperCase())} stay yours.` : ''}</span>
  </p>
{/each}

<style>
  .integration {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--color-claim);
  }
</style>

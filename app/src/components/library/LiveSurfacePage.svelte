<!-- Live surface clients (S25): every claim on the hub with its controls, priority and the apps it
  applies to. Release needs the service's `release_claim` capability; priority, pid and apps need
  `claim_details`. -->
<script lang="ts">
  import { Plug } from 'lucide-svelte';
  import { hub, hasCapability, toast } from '../../lib/hub.svelte';
  import { controlLabels, releaseClaim } from '../../lib/integrations.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import EmptyState from '../ui/EmptyState.svelte';

  const canRelease = $derived(hasCapability('release_claim'));
  const details = $derived(hasCapability('claim_details'));
  const windows = typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent);
  const sorted = $derived([...hub.claims].sort((a, b) => (b.priority ?? 0) - (a.priority ?? 0) || b.claim - a.claim));

  async function release(id: number, name: string) {
    const err = await releaseClaim(id);
    toast(err ? { text: err, kind: 'danger' } : `Released ${name}`);
  }
  async function releaseAll() {
    for (const c of hub.claims) await releaseClaim(c.claim);
    toast('Released every client');
  }
</script>

<div class="page">
  <header>
    <div class="head">
      <div>
        <h1 class="t-title">Live surface</h1>
        <p class="t-body muted">Programs drawing on the hub right now.</p>
      </div>
      {#if canRelease && sorted.length}<Button variant="danger" size="sm" onclick={releaseAll}>Release all</Button>{/if}
    </div>
  </header>

  <p class="t-small muted">
    Programs connect over the local socket <code class="t-mono">{windows ? 'the service’s local pipe' : '$XDG_RUNTIME_DIR/masterhubd.sock'}</code>.
    A higher priority wins a control; when a claim ends, your config comes back.
  </p>

  {#if sorted.length}
    <table>
      <thead>
        <tr class="t-label faint">
          <th scope="col">Program</th>
          <th scope="col">Claims</th>
          {#if details}<th scope="col" class="num">Priority</th><th scope="col">Only while</th>{/if}
          {#if canRelease}<th scope="col"><span class="visually-hidden">Release</span></th>{/if}
        </tr>
      </thead>
      <tbody>
        {#each sorted as c (c.claim)}
          <tr>
            <td>
              <div class="t-heading">{c.name}</div>
              {#if details && (c.program || c.pid)}<div class="t-small muted">{[c.program, c.pid ? `pid ${c.pid}` : ''].filter(Boolean).join(' · ')}</div>{/if}
              {#if !c.active}<div class="t-small faint">Not active right now</div>{/if}
            </td>
            <td>
              <div class="chips">{#each controlLabels(c.owns) as l}<Chip variant="integration" icon={null}>{l}</Chip>{/each}</div>
            </td>
            {#if details}
              <td class="num t-body">{c.priority ?? 0}</td>
              <td class="t-small">{#if c.apps?.length}<code class="t-mono">{c.apps.join(', ')}</code>{:else}<span class="muted">any app</span>{/if}</td>
            {/if}
            {#if canRelease}<td class="act"><Button size="sm" onclick={() => release(c.claim, c.name)} aria-label="Release {c.name}">Release</Button></td>{/if}
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <EmptyState icon={Plug} title="No programs are connected" body="Integrations and scripts show up here while they run." />
  {/if}
</div>

<style>
  .page {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 24px;
    max-width: 960px;
    width: 100%;
    margin: 0 auto;
    box-sizing: border-box;
  }
  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  h1,
  p {
    margin: 0;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    background: var(--color-surface);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-hairline);
  }
  th,
  td {
    padding: 10px 12px;
    text-align: left;
    vertical-align: middle;
  }
  tbody tr {
    border-top: var(--panel-border-width) solid var(--color-border);
  }
  .num {
    text-align: right;
    width: 80px;
  }
  .act {
    text-align: right;
    width: 90px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  code {
    padding: 1px 5px;
    background: var(--color-sunken);
    border-radius: var(--radius-sm);
  }
</style>

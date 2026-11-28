<!-- The state block on an integration's page: checking, not found, running, bridge disconnected,
  error (S23). Driven by the plan and the integration's status. -->
<script lang="ts">
  import { FolderSearch, LoaderCircle, TriangleAlert } from 'lucide-svelte';
  import type { IntegrationState } from '../../lib/integrations.svelte';
  import type { GamePlan } from '../../lib/types';
  import Banner from '../ui/Banner.svelte';
  import Button from '../ui/Button.svelte';
  import CheckRow from '../ui/CheckRow.svelte';

  let {
    state,
    plan,
    error,
    onbrowse,
    onretry,
    onsetup,
    onturnoff,
  }: {
    state: IntegrationState;
    plan: GamePlan | null;
    error?: string;
    onbrowse: () => void;
    onretry: () => void;
    onsetup: () => void;
    onturnoff: () => void;
  } = $props();
</script>

{#if !plan}
  <Banner icon={LoaderCircle}>
    <span role="status"><strong>Checking</strong> · Looking for the game in Steam libraries…</span>
  </Banner>
{:else if plan.error && !plan.path}
  <Banner kind="warning" icon={FolderSearch}>
    <span><strong>Folder not found</strong> · {plan.error}</span>
    {#snippet actions()}<Button size="sm" onclick={onbrowse}>Browse…</Button>{/snippet}
  </Banner>
{:else if plan.error}
  <Banner kind="danger"><span><strong>Cannot set up</strong> · {plan.error}</span></Banner>
{/if}

{#if state === 'error'}
  <Banner kind="danger" icon={TriangleAlert}>
    <span role="alert"><strong>Error</strong> · {error ?? 'The last setup did not finish.'}</span>
    {#snippet actions()}<Button size="sm" onclick={onsetup}>Set up again</Button>{/snippet}
  </Banner>
{:else if state === 'bridge_down'}
  <Banner kind="warning">
    <span><strong>Bridge disconnected</strong> · The mod is not answering. Start the game once so it loads, then retry.</span>
    {#snippet actions()}<Button size="sm" onclick={onretry}>Retry</Button>{/snippet}
  </Banner>
{:else if state === 'running'}
  <CheckRow title="On · running" detail="Bridge connected · game focused" status="success" statusLabel="Running">
    {#snippet action()}<Button size="sm" onclick={onturnoff}>Turn off</Button>{/snippet}
  </CheckRow>
{:else if state === 'on'}
  <CheckRow title="On" detail="Waiting for the game to be focused" status="success" statusLabel="On">
    {#snippet action()}<Button size="sm" onclick={onturnoff}>Turn off</Button>{/snippet}
  </CheckRow>
{:else if plan && !plan.error}
  <CheckRow title={state === 'needs_setup' ? 'Needs setup' : 'Off'} detail="Nothing is installed or changed until you approve the plan." status={state === 'needs_setup' ? 'warning' : 'off'} statusLabel={state === 'needs_setup' ? 'Needs setup' : 'Off'} />
{/if}

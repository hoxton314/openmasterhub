<!-- One setup: the setup list on the left, switching mode and its profiles in the middle, the rule tester on the right (follow). -->
<script lang="ts">
  import { ArrowRight, Ellipsis, FlaskConical, Keyboard, Play, Plus } from 'lucide-svelte';
  import { hub } from '../../lib/hub.svelte';
  import { effectiveSetups, type EffectiveSetup } from '../../lib/model';
  import { ui } from '../../lib/ui.svelte';
  import Banner from '../ui/Banner.svelte';
  import Button from '../ui/Button.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Menu, { type MenuEntry } from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import StatusDot from '../ui/StatusDot.svelte';
  import Layout from '../shell/Layout.svelte';
  import AreaHeader from './AreaHeader.svelte';
  import HybridPinPanel from './HybridPinPanel.svelte';
  import ModeSwitcher from './ModeSwitcher.svelte';
  import OrderList from './OrderList.svelte';
  import ReleasePolicy from './ReleasePolicy.svelte';
  import RuleEditorDialog from './RuleEditorDialog.svelte';
  import RuleList from './RuleList.svelte';
  import RuleTester from './RuleTester.svelte';
  import SetupTriggers from './SetupTriggers.svelte';
  import TriggerPicker from './TriggerPicker.svelte';
  import { deleteSetup, duplicateSetup, makeActive, modeInfo, openInEditor, renameSetup, setMode, updateSetup } from './ops';

  let { setup, onnew, onselect }: { setup: EffectiveSetup; onnew: () => void; onselect: (name: string) => void } = $props();

  const all = $derived(effectiveSetups(hub.config));
  const activeName = $derived(hub.state.setup ?? all[0]?.name);
  const active = $derived(activeName === setup.name);
  const LINES = { follow: 'switches by the focused app', manual: 'you switch, focus does nothing', hybrid: 'follows apps, with manual override' } as const;
  const subtitle = $derived(`${active ? 'Active on the hub · ' : ''}${LINES[setup.mode]}`);

  let testClass = $state(hub.state.active_app ?? '');
  let ruleEdit = $state<{ profile: string; index?: number } | null>(null);
  let triggers = $state(false);

  const menu = $derived<MenuEntry[]>([
    { label: 'Rename…', onselect: () => renameSetup(setup.name) },
    { label: 'Duplicate', onselect: () => duplicateSetup(setup.name) },
    { separator: true },
    { label: 'Delete…', danger: true, disabled: all.length < 2, onselect: () => deleteSetup(setup.name) },
  ]);
  const mode = $derived(modeInfo(setup.mode));
</script>

{#snippet tester()}
  <RuleTester {setup} bind:value={testClass} />
{/snippet}

<Layout inspectorLabel="Rule tester" inspector={setup.mode === 'follow' ? tester : undefined}>
  {#snippet sidebar({ collapsed })}
    <div class="side" class:collapsed>
      {#if !collapsed}<div class="side-label"><SectionLabel>Setups</SectionLabel></div>{/if}
      <nav aria-label="Setups">
        {#each all as s (s.name)}
          {@const M = modeInfo(s.mode).icon}
          <button type="button" class="srow" class:on={s.name === setup.name} aria-current={s.name === setup.name ? 'page' : undefined} title={collapsed ? s.name : undefined} onclick={() => onselect(s.name)}>
            <M size={16} />
            {#if !collapsed}<span class="n">{s.name}</span>{/if}
            {#if s.name === activeName}<StatusDot size={7} label="Active on the hub" />{/if}
          </button>
        {/each}
        <button type="button" class="srow add" title={collapsed ? 'New setup' : undefined} aria-label="New setup" onclick={onnew}>
          <Plus size={16} />{#if !collapsed}<span class="n">New setup</span>{/if}
        </button>
      </nav>
    </div>
  {/snippet}

  {#snippet main()}
    <AreaHeader title={setup.name} {subtitle}>
      {#snippet actions()}
        {#if ui.layout.inspectorDrawer && setup.mode === 'follow'}
          <Button icon={FlaskConical} onclick={() => (ui.drawer = !ui.drawer)}>Rule tester</Button>
        {/if}
        <Button icon={Keyboard} onclick={() => (triggers = true)}>Setup triggers</Button>
        {#if !active}<Button variant="primary" icon={Play} onclick={() => makeActive(setup.name)}>Make active</Button>{/if}
        <Button variant={active ? 'primary' : 'secondary'} icon={ArrowRight} onclick={() => openInEditor(setup.name)}>Open in editor</Button>
        <Popover placement="bottom-end" role="menu" label="Setup menu">
          {#snippet trigger({ toggle, attrs })}<IconButton icon={Ellipsis} variant="secondary" label="Setup menu" {...attrs} onclick={toggle} />{/snippet}
          {#snippet children({ close })}<Menu items={menu} onclose={close} label="Setup menu" />{/snippet}
        </Popover>
      {/snippet}
    </AreaHeader>

    <div class="scroll">
      {#if setup.implicit}
        <Banner kind="info">This config has no setups yet, so Work shows the one the service derives. The first change you make saves it as <b>{setup.name}</b>.</Banner>
      {/if}
      <div class="stack">
        <ModeSwitcher mode={setup.mode} onchange={(m) => setMode(setup.name, m)} />

        {#if setup.mode === 'follow'}
          <RuleList {setup} {testClass} onrule={(profile, index) => (ruleEdit = { profile, index })} />
        {:else if setup.mode === 'manual'}
          <div class="two">
            <OrderList {setup} />
            <TriggerPicker {setup} kind="manual" />
          </div>
        {:else}
          <HybridPinPanel setup={setup.name} shortcut={setup.pin} />
          <div class="two">
            <RuleList {setup} {testClass} title="App rules" onrule={(profile, index) => (ruleEdit = { profile, index })} />
            <div class="col">
              <section class="rel">
                <h2 class="t-heading">Release the pin</h2>
                <p class="t-small muted">A manual switch pins a profile over the app rules. Choose when the rules take back over.</p>
                <ReleasePolicy
                  value={setup.release ?? 'same_trigger'}
                  minutes={setup.release_after}
                  onchange={(v) => updateSetup(setup.name, (s) => ((s.release = v), v === 'timeout' && (s.release_after ??= 10)), 'change release policy')}
                  onminutes={(m) => updateSetup(setup.name, (s) => (s.release_after = m), 'change release time', `release-${setup.name}`)}
                />
              </section>
              <TriggerPicker {setup} kind="hybrid" />
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/snippet}

</Layout>

{#if ruleEdit}
  <RuleEditorDialog {setup} profile={ruleEdit.profile} index={ruleEdit.index} onclose={() => (ruleEdit = null)} />
{/if}
{#if triggers}<SetupTriggers setup={setup.name} onclose={() => (triggers = false)} />{/if}

<style>
  .side {
    display: grid;
    align-content: start;
    gap: 6px;
    padding: 16px 10px;
  }
  .side-label {
    padding: 0 8px;
  }
  nav {
    display: grid;
    gap: 2px;
  }
  .srow {
    display: flex;
    align-items: center;
    gap: 10px;
    height: var(--size-row-sidebar);
    padding: 0 10px;
    font-weight: 500;
    text-align: left;
    color: var(--color-text-1);
    background: none;
    border: 0;
    border-radius: var(--radius-sidebar-row);
    cursor: pointer;
  }
  .srow:hover:not(.on) {
    background: var(--color-selection);
  }
  .srow.on {
    background: var(--row-selected-bg);
    color: var(--row-selected-fg);
    --live-color: var(--row-selected-live);
    font-weight: 600;
  }
  .srow :global(svg) {
    flex: none;
  }
  .n {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .add {
    color: var(--color-text-2);
  }
  .collapsed .srow {
    justify-content: center;
    padding: 0;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    align-content: start;
    gap: 14px;
    padding: 20px 32px 32px;
  }
  .stack {
    display: grid;
    gap: 18px;
    align-content: start;
  }
  .two {
    display: grid;
    grid-template-columns: minmax(0, 1.35fr) minmax(300px, 1fr);
    gap: 28px;
    align-items: start;
  }
  .col {
    display: grid;
    gap: 18px;
  }
  .rel {
    display: grid;
    gap: 8px;
  }
</style>

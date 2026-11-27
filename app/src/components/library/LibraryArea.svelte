<!-- The Library area (S22): filters on the left; integrations and templates in the middle; an
  integration's plan, the Claude Code page and the live surface as sub-views. -->
<script lang="ts">
  import { Library as LibraryIcon } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import { games } from '../../lib/games.svelte';
  import { currentProfile, edit, hub, profileMut, undo } from '../../lib/hub.svelte';
  import { integrationEntries, refreshIntegrations, type IntegrationEntry } from '../../lib/integrations.svelte';
  import { applyControls, instantiate, listTemplates, moduleWarning, needsLabels, type Template } from '../../lib/templates';
  import type { ModuleKind, Profile } from '../../lib/types';
  import { ui } from '../../lib/ui.svelte';
  import { createFromTemplate } from '../setups/ops';
  import Layout from '../shell/Layout.svelte';
  import Breadcrumb, { type Crumb } from '../ui/Breadcrumb.svelte';
  import TemplatePreview from '../profiles/TemplatePreview.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import TemplateCard from '../ui/TemplateCard.svelte';
  import ClaudeCodePage from './ClaudeCodePage.svelte';
  import Filters, { anyFilter, emptyFilters, type Kind, type LibraryFilters } from './Filters.svelte';
  import IntegrationPlan from './IntegrationPlan.svelte';
  import IntegrationRow from './IntegrationRow.svelte';
  import LiveSurfacePage from './LiveSurfacePage.svelte';

  const templates: Template[] = (['profile', 'setup', 'control'] as const).flatMap((k) => listTemplates(k));

  interface Item {
    kind: Kind;
    key: string;
    name: string;
    about: string;
    category: string;
    needs: ModuleKind[];
    installed: boolean;
    integration?: IntegrationEntry;
    template?: Template;
  }

  const MODULE_LABEL: Record<string, string> = { keypad: 'Keypad', dial: 'Dial', fader: 'Faders', knob: 'Knobs', roller: 'Rollers', base: 'Base' };
  const KIND_LABEL: Record<Kind, string> = { profile: 'Profiles', setup: 'Setups', control: 'Control templates', integration: 'Integrations' };
  const KINDS: Kind[] = ['profile', 'setup', 'control', 'integration'];

  let filters = $state<LibraryFilters>(emptyFilters());

  onMount(() => {
    refreshIntegrations();
  });

  const names = $derived(new Set([...(hub.config.profile ?? []).map((p) => p.name), ...(hub.config.setup ?? []).map((s) => s.name)]));
  const items = $derived<Item[]>([
    ...integrationEntries().map((e) => ({ kind: 'integration' as const, key: `i-${e.id}`, name: e.title, about: e.subtitle, category: e.category, needs: e.needs, installed: e.installed, integration: e })),
    ...templates.map((t) => ({ kind: t.kind, key: `t-${t.id}`, name: t.name, about: t.about ?? '', category: t.category ?? 'Other', needs: t.needs ?? [], installed: t.kind !== 'control' && names.has(t.name), template: t })),
  ]);

  function matches(it: Item, f: LibraryFilters, skip?: keyof LibraryFilters) {
    if (skip !== 'kinds' && f.kinds.length && !f.kinds.includes(it.kind)) return false;
    if (skip !== 'needs' && f.needs.length && !f.needs.some((n) => it.needs.includes(n))) return false;
    if (skip !== 'categories' && f.categories.length && !f.categories.includes(it.category)) return false;
    if (skip !== 'states' && f.states.length && !f.states.includes(it.installed ? 'installed' : 'available')) return false;
    return true;
  }
  const shown = $derived(items.filter((i) => matches(i, filters)));
  // Counts ignore the group's own selection, so you see what adding one more option gives.
  const count = (skip: keyof LibraryFilters, test: (i: Item) => boolean) => items.filter((i) => matches(i, filters, skip) && test(i)).length;

  const kindOptions = $derived(KINDS.map((k) => ({ value: k, label: KIND_LABEL[k], count: count('kinds', (i) => i.kind === k) })));
  const needOptions = $derived((['keypad', 'dial', 'fader', 'knob'] as ModuleKind[]).map((m) => ({ value: m, label: MODULE_LABEL[m], count: count('needs', (i) => i.needs.includes(m)) })));
  const categoryOptions = $derived([...new Set(items.map((i) => i.category))].sort().map((c) => ({ value: c, label: c, count: count('categories', (i) => i.category === c) })));
  const stateOptions = $derived([
    { value: 'installed', label: 'Installed', count: count('states', (i) => i.installed) },
    { value: 'available', label: 'Available', count: count('states', (i) => !i.installed) },
  ]);

  /** What a template draws in its card: a profile as is, a control template as a bare profile. */
  const previewOf = (t: Template): Profile | undefined => (t.kind === 'profile' ? t.profile : t.kind === 'control' ? ({ name: t.name, ...t.controls } as Profile) : undefined);

  const route = $derived(ui.route);
  const game = $derived(route?.view === 'integration' ? games.list.find((g) => g.game === route.id) : undefined);

  function open(e: IntegrationEntry) {
    if (e.type === 'claude') ui.open({ view: 'claude-code' });
    else if (e.type === 'live') ui.open({ view: 'live-surface' });
    else ui.open({ view: 'integration', id: e.id });
  }

  /** Profile templates open the new-profile flow on that template; setups and controls are added right away, with Undo. */
  function use(t: Template) {
    if (t.kind === 'profile') return ui.openNewProfile({ template: t.id });
    if (t.kind === 'setup') {
      const name = createFromTemplate(t);
      if (!name) return;
      ui.go('setups', { view: 'setup-detail', setup: name });
      return ui.toast({ text: `Added the setup ${name}`, kind: 'success', action: { label: 'Undo', run: () => undo() } });
    }
    const target = currentProfile()?.name;
    if (!target) return ui.toast({ text: 'There is no profile to apply this to yet. Create one first.', kind: 'warning' });
    edit((c) => {
      const p = profileMut(c);
      applyControls(p, instantiate(t, c).controls ?? {});
    }, { label: `apply ${t.name}` });
    ui.go('editor');
    ui.toast({ text: `Applied ${t.name} to ${target}`, kind: 'success', action: { label: 'Undo', run: () => undo() } });
  }

  const crumbs = $derived.by<Crumb[]>(() => {
    const root: Crumb = { label: 'Library', icon: LibraryIcon, onclick: () => ui.go('library', null) };
    if (!route) return [root];
    if (route.view === 'integration') return [root, { label: 'Integrations', onclick: () => ui.go('library', null) }, { label: game?.title ?? String(route.id) }];
    return [root, { label: 'Integrations', onclick: () => ui.go('library', null) }, { label: route.view === 'claude-code' ? 'Claude Code' : 'Live surface' }];
  });

  const sections = $derived(
    (['integration', 'profile', 'setup', 'control'] as Kind[])
      .map((k) => ({ kind: k, items: shown.filter((i) => i.kind === k) }))
      .filter((s) => s.items.length),
  );
  const TITLE: Record<Kind, string> = { integration: 'Integrations', profile: 'Profile templates', setup: 'Setup templates', control: 'Control templates' };
  const SUB: Record<Kind, string> = { integration: 'Bring their own behaviour · approved one by one', profile: '', setup: '', control: '' };
</script>

<Layout>
  {#snippet sidebar()}
    <Filters {filters} kinds={kindOptions} needs={needOptions} categories={categoryOptions} states={stateOptions} onchange={(f) => (filters = f)} />
  {/snippet}
  {#snippet topbar()}
    <Breadcrumb items={crumbs} />
  {/snippet}
  {#snippet main()}
    {#if route?.view === 'integration'}
      {#if game}<IntegrationPlan {game} />{:else}<EmptyState title="Integration not found" body="This game mode is not offered by the installed masterhub-games." actionLabel="Back to the library" onaction={() => ui.go('library', null)} />{/if}
    {:else if route?.view === 'claude-code'}
      <ClaudeCodePage />
    {:else if route?.view === 'live-surface'}
      <LiveSurfacePage />
    {:else}
      <div class="page">
        <header>
          <h1 class="t-title">Library</h1>
          <p class="t-body muted">Templates and integrations. Nothing here is downloaded; it ships with the app or comes from your files.</p>
        </header>
        {#each sections as s (s.kind)}
          <section aria-label={TITLE[s.kind]}>
            <SectionLabel as="h2">{TITLE[s.kind]}</SectionLabel>
            {#if SUB[s.kind]}<p class="t-small muted sub">{SUB[s.kind]}</p>{/if}
            {#if s.kind === 'integration'}
              <div class="list">
                {#each s.items as it (it.key)}<IntegrationRow entry={it.integration!} onopen={() => open(it.integration!)} />{/each}
              </div>
            {:else}
              <div class="grid">
                {#each s.items as it (it.key)}
                  <TemplateCard title={it.name} description={it.about} modules={needsLabels(it.template!)} warning={moduleWarning(it.template!, hub.modules)} onclick={() => use(it.template!)}>
                    {#snippet preview()}<TemplatePreview profile={previewOf(it.template!)} unit={18} />{/snippet}
                  </TemplateCard>
                {/each}
              </div>
            {/if}
          </section>
        {/each}
        {#if !sections.length}
          <EmptyState title="Nothing matches" body="No integration or template fits these filters." actionLabel={anyFilter(filters) ? 'Clear filters' : undefined} onaction={() => (filters = emptyFilters())} />
        {/if}
      </div>
    {/if}
  {/snippet}
</Layout>

<style>
  .page {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 24px;
  }
  h1,
  p {
    margin: 0;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .sub {
    margin-top: -6px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 12px;
  }
</style>

<!--
  New profile (S19, S19b): a gallery of templates in two steps, then name, app rule, base profile
  and which setups get it. Mounted by App while `ui.newProfile` is set; closes by clearing it.
-->
<script lang="ts">
  import { ArrowLeft, FilePlus2, FileUp, Link2, Plug, Plus, X } from 'lucide-svelte';
  import { onMount, untrack } from 'svelte';
  import { invoke } from '../../lib/backend';
  import { edit, editProfile, hub, editor, toastUndo } from '../../lib/hub.svelte';
  import { activeSetup, effectiveSetups, globMatch, looseProfiles, setupByName } from '../../lib/model';
  import { getTemplate, instantiate, listTemplates, missingModules, moduleWarning, needsLabels, uniqueName, type Template } from '../../lib/templates';
  import type { GameStatus, ModuleKind, Profile } from '../../lib/types';
  import { ui } from '../../lib/ui.svelte';
  import Avatar from '../ui/Avatar.svelte';
  import Banner from '../ui/Banner.svelte';
  import Button from '../ui/Button.svelte';
  import Card from '../ui/Card.svelte';
  import Chip from '../ui/Chip.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import Field from '../ui/Field.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import Select from '../ui/Select.svelte';
  import Tabs from '../ui/Tabs.svelte';
  import TemplateCard from '../ui/TemplateCard.svelte';
  import TextField from '../ui/TextField.svelte';
  import { insertProfile, addToSetup, joinNames } from './profileOps';
  import TemplatePreview from './TemplatePreview.svelte';

  /** What the wizard starts from: a template, a blank profile, an imported one or one no setup uses. */
  interface Source {
    key: string;
    name: string;
    about?: string;
    needs: ModuleKind[];
    profile: Profile;
    template?: Template;
    /** An existing profile that no setup uses: it is added, not created. */
    existing?: boolean;
  }

  type Tab = 'templates' | 'integrations' | 'mine' | 'import' | 'blank';

  const req = ui.newProfile ?? {};
  const config = $derived(hub.config);
  const setups = $derived(effectiveSetups(config));
  const setupName = untrack(() => req.setup ?? (editor.setup || activeSetup(hub.config, hub.state).name));
  const setup = $derived(setupByName(config, setupName) ?? setups[0]);
  const others = $derived(setups.filter((s) => s.name !== setup?.name));

  let tab = $state<Tab>('templates');
  let step = $state<1 | 2>(1);
  let picked = $state<Source | null>(null);
  let name = $state('');
  let apps = $state<string[]>([]);
  let base = $state('');
  let position = $state(0);
  let typed = $state('');
  let linkOn = $state(false);
  let linkTo = $state('');
  let windows = $state<string[]>([]);
  let games = $state<GameStatus[]>([]);
  let imported = $state<Profile[] | null>(null);

  onMount(() => {
    invoke<string[]>('open_window_classes').then((w) => (windows = w ?? [])).catch(() => {});
    invoke<GameStatus[]>('games_list').then((g) => (games = g ?? [])).catch(() => {});
    picked = fromTemplate(getTemplate(req.template ?? '') ?? listTemplates('profile')[0]);
  });

  const templates = $derived(listTemplates('profile'));
  const loose = $derived(looseProfiles(config));
  const blank = $derived<Source>({ key: 'blank', name: 'New profile', about: 'An empty profile with one page; everything else comes from its base.', needs: ['keypad'], profile: { name: 'New profile', page: [{ name: 'Main' }] } });

  function fromTemplate(t: Template | undefined): Source | null {
    if (!t?.profile) return null;
    return { key: t.id, name: t.name, about: t.about, needs: t.needs, profile: t.profile, template: t };
  }

  function pick(s: Source | null) {
    picked = s;
  }

  function pickTab(t: Tab) {
    tab = t;
    if (t === 'templates') picked = picked?.template ? picked : fromTemplate(templates[0]);
    else if (t === 'blank') picked = blank;
    else if (t === 'mine') picked = picked?.existing ? picked : null;
    else if (t === 'import') picked = imported && picked && !picked.template && !picked.existing && picked.key.startsWith('import:') ? picked : null;
    else picked = null;
  }

  async function chooseFile() {
    try {
      const found = await invoke<Profile[] | null>('import_toml');
      if (!found) return;
      imported = found;
      pick(fromImport(found[0]));
      if (!found.length) ui.toast({ text: 'No profiles in that file.', kind: 'warning' });
    } catch (e) {
      ui.toast({ text: `Could not import: ${e}`, kind: 'danger' });
    }
  }

  function fromImport(p: Profile | undefined): Source | null {
    if (!p) return null;
    return { key: `import:${p.name}`, name: p.name, about: p.about, needs: usedModules(p), profile: p };
  }

  function usedModules(p: Profile): ModuleKind[] {
    const out: ModuleKind[] = [];
    if (p.page?.length || p.folder?.length) out.push('keypad');
    if (p.dial) out.push('dial');
    if (p.knob?.length) out.push('knob');
    if (p.fader?.length) out.push('fader');
    if (p.roller?.length) out.push('roller');
    return out;
  }

  const warn = (needs: ModuleKind[], t?: Template) => (t ? moduleWarning(t, hub.modules) : moduleWarning({ needs } as Template, hub.modules));

  // ── step 2 ──
  const takenNames = $derived(new Set((config.profile ?? []).map((p) => p.name)));
  const nameError = $derived(!name.trim() ? 'Give the profile a name.' : !picked?.existing && takenNames.has(name.trim()) ? `There is already a profile called “${name.trim()}”.` : undefined);
  const baseProfile = $derived((config.profile ?? []).find((p) => p.name === base));
  const baseOptions = $derived([{ value: '', label: 'None' }, ...(config.profile ?? []).filter((p) => p.name !== picked?.profile.name).map((p) => ({ value: p.name, label: p.name }))]);
  const positionCount = $derived((setup?.profiles.length ?? 0) + 1);
  const positionOptions = $derived(Array.from({ length: positionCount }, (_, i) => ({ value: i, label: i === 0 && positionCount > 1 ? '1 · first' : i === positionCount - 1 && positionCount > 1 ? `${i + 1} · last` : String(i + 1) })));
  const manual = $derived(setup?.mode === 'manual');
  const ruleWord = $derived(manual ? 'Position' : 'Rule');
  const preview = $derived.by(() => {
    if (!picked) return undefined;
    return picked.template ? instantiate(picked.template, config, { name: name || picked.name, apps }).profiles[0] : { ...picked.profile, name: name || picked.name };
  });

  /** The controls the profile leaves empty that its base sets, in words. */
  const inheritedHelp = $derived.by(() => {
    const p = preview;
    const b = baseProfile;
    if (!b) return 'Nothing is inherited: controls left empty stay empty.';
    const names = [!p?.dial && b.dial ? 'dial' : '', !p?.knob?.length && b.knob?.length ? 'knobs' : '', !p?.fader?.length && b.fader?.length ? 'faders' : '', !p?.roller?.length && b.roller?.length ? 'rollers' : ''].filter(Boolean);
    return names.length ? `Controls the profile leaves empty (${names.join(', ')}) come from ${b.name}.` : `${b.name} has nothing this profile leaves empty.`;
  });

  function next() {
    if (!picked) return;
    name = picked.existing ? picked.profile.name : uniqueName(takenNames, picked.name);
    apps = [...(picked.profile.apps ?? [])];
    base = picked.profile.base ?? (picked.existing ? '' : setup && setup.mode !== 'manual' && setup.fallback && setup.fallback !== picked.profile.name ? setup.fallback : '');
    position = Math.min(req.position ?? (manual ? setup?.profiles.length ?? 0 : 0), setup?.profiles.length ?? 0);
    linkOn = false;
    linkTo = others[0]?.name ?? '';
    typed = '';
    step = 2;
  }

  function addApp(v: string) {
    const g = v.trim();
    if (g && !apps.includes(g)) apps = [...apps, g];
    typed = '';
  }

  const matches = (g: string) => windows.filter((w) => globMatch(g, w)).length;

  function create() {
    if (!picked || !setup || nameError) return;
    const targets = [setup.name, ...(linkOn && linkTo ? [linkTo] : [])];
    const finalName = name.trim();
    if (picked.existing) {
      edit(
        (c) => {
          const p = (c.profile ?? []).find((x) => x.name === picked!.profile.name);
          if (p) {
            if (apps.length) p.apps = [...apps];
            else delete p.apps;
            if (base) p.base = base;
            else delete p.base;
          }
          for (const s of targets) addToSetup(c, finalName, s, position);
        },
        { label: `Add ${finalName}` },
      );
    } else {
      const made = picked.template
        ? instantiate(picked.template, config, { name: finalName, apps, base: base || undefined }).profiles[0]
        : (() => {
            const p = JSON.parse(JSON.stringify(picked.profile)) as Profile;
            p.name = finalName;
            if (apps.length) p.apps = [...apps];
            else delete p.apps;
            if (base) p.base = base;
            else delete p.base;
            return p;
          })();
      edit((c) => insertProfile(c, made, targets, position), { label: `New profile ${finalName}` });
    }
    editProfile(finalName);
    ui.newProfile = null;
    ui.go('editor', null);
    toastUndo(`Created “${finalName}” in ${joinNames(targets)}. Click a key to set it.`);
  }

  function close() {
    ui.newProfile = null;
  }

  const TABS = $derived([
    { value: 'templates' as Tab, label: 'Templates' },
    { value: 'integrations' as Tab, label: 'Integrations' },
    { value: 'mine' as Tab, label: 'My profiles', count: loose.length },
    { value: 'import' as Tab, label: 'Import file' },
    { value: 'blank' as Tab, label: 'Blank' },
  ]);

  const footerNote = $derived(picked ? `${picked.name} · uses ${needsLabels({ needs: picked.needs } as Template).join(', ').toLowerCase()}` : 'Pick one to continue');
</script>

{#if step === 1}
  <Dialog title="New profile in {setup?.name ?? 'your setup'}" subtitle="Start from a template. Blank is always the last tab." width={1120} onclose={close}>
    <div class="gallery">
      <Tabs tabs={TABS} value={tab} onchange={pickTab} label="Start from" />

      {#if tab === 'templates'}
        <div class="grid">
          {#each templates as t (t.id)}
            <TemplateCard title={t.name} description={t.about} modules={needsLabels(t)} warning={moduleWarning(t, hub.modules)} selected={picked?.key === t.id} onclick={() => pick(fromTemplate(t))}>
              {#snippet preview()}<TemplatePreview profile={t.profile} unit={18} />{/snippet}
            </TemplateCard>
          {/each}
        </div>
      {:else if tab === 'integrations'}
        {#if games.length}
          <div class="grid">
            {#each games as g (g.game)}
              <Card padding={12} label={g.title}>
                <b>{g.title}</b>
                <span class="muted t-small">{g.about}</span>
                <span class="row">
                  <Chip variant={g.enabled ? 'success' : 'integration'}>{g.enabled ? 'Set up' : 'Integration'}</Chip>
                  <Button size="sm" icon={Plug} onclick={() => (close(), ui.go('library'))}>Open in Library</Button>
                </span>
              </Card>
            {/each}
          </div>
          <p class="muted t-small">Game profiles read artwork from your own install, so they are set up in the Library.</p>
        {:else}
          <EmptyState icon={Plug} title="No integrations here" body="Game modes and Claude Code are set up in the Library." actionLabel="Open the Library" onaction={() => (close(), ui.go('library'))} />
        {/if}
      {:else if tab === 'mine'}
        {#if loose.length}
          <div class="grid">
            {#each loose as p (p.name)}
              <TemplateCard
                title={p.name}
                description={p.about ?? 'Not used by any setup'}
                modules={needsLabels({ needs: usedModules(p) } as Template)}
                selected={picked?.key === `mine:${p.name}`}
                onclick={() => pick({ key: `mine:${p.name}`, name: p.name, about: p.about, needs: usedModules(p), profile: p, existing: true })}
              >
                {#snippet preview()}<TemplatePreview profile={p} unit={18} />{/snippet}
              </TemplateCard>
            {/each}
          </div>
        {:else}
          <EmptyState icon={FilePlus2} title="Every profile is in a setup" body="Profiles that no setup uses show up here, so you can add them to {setup?.name ?? 'this setup'}." />
        {/if}
      {:else if tab === 'import'}
        {#if imported?.length}
          <div class="grid">
            {#each imported as p (p.name)}
              <TemplateCard title={p.name} description={p.about ?? 'From a .toml file'} modules={needsLabels({ needs: usedModules(p) } as Template)} warning={warn(usedModules(p))} selected={picked?.key === `import:${p.name}`} onclick={() => pick(fromImport(p))}>
                {#snippet preview()}<TemplatePreview profile={p} unit={18} />{/snippet}
              </TemplateCard>
            {/each}
          </div>
          <div><Button size="sm" icon={FileUp} onclick={chooseFile}>Choose another file</Button></div>
        {:else}
          <EmptyState icon={FileUp} title="Import a profile" body="Pick a .toml file exported from OpenMasterHUB, or a config with profiles in it." actionLabel="Choose a .toml file" onaction={chooseFile} />
        {/if}
      {:else}
        <div class="grid one">
          <TemplateCard title="Blank" description={blank.about} modules={['Keypad']} selected={picked?.key === 'blank'} onclick={() => pick(blank)}>
            {#snippet preview()}<TemplatePreview profile={blank.profile} unit={18} />{/snippet}
          </TemplateCard>
        </div>
      {/if}
    </div>
    {#snippet footer()}
      <span class="note">{footerNote}</span>
      <Button onclick={close}>Cancel</Button>
      <Button variant="primary" disabled={!picked} onclick={next}>Next: name and rules</Button>
    {/snippet}
  </Dialog>
{:else if picked}
  <Dialog title="Name it and choose when it applies" subtitle="Step 2 of 2 · {picked.name} {picked.template ? 'template' : 'profile'}" width={920} onclose={close}>
    <div class="wizard">
      <div class="form">
        <TextField bind:value={name} label="Name" error={nameError} disabled={picked.existing} />

        <Field label="When it applies in {setup?.name}">
          {#if apps.length}
            <ul class="rules">
              {#each apps as g (g)}
                <li>
                  <Avatar name={g} size={18} />
                  <Chip variant="glob">{g}</Chip>
                  <span class="muted t-small">{matches(g) === 1 ? 'matches 1 open window' : matches(g) ? `matches ${matches(g)} open windows` : 'no open window matches'}</span>
                  <IconButton icon={X} size="sm" label="Remove {g}" onclick={() => (apps = apps.filter((x) => x !== g))} />
                </li>
              {/each}
            </ul>
          {:else}
            <p class="muted t-small">No rule yet: it will not apply on its own.{manual ? ' It is reached by cycling.' : ''}</p>
          {/if}
          <div class="add">
            <TextField mono size="sm" placeholder="window class or exe, * allowed" bind:value={typed} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), addApp(typed))} />
            <Button size="sm" icon={Plus} disabled={!typed.trim()} onclick={() => addApp(typed)}>Add</Button>
          </div>
          {#if windows.some((w) => !apps.includes(w))}
            <div class="suggest" aria-label="Open windows">
              <span class="muted t-small">Open now:</span>
              {#each windows.filter((w) => !apps.includes(w)) as w (w)}
                <button type="button" class="pill" onclick={() => addApp(w)}><Plus size={11} />{w}</button>
              {/each}
            </div>
          {/if}
          <p class="muted t-small">Goes to {ruleWord.toLowerCase()} {position + 1}. You can reorder later.</p>
          {#if positionCount > 1}<Select size="sm" value={position} options={positionOptions} onchange={(v) => (position = v)} label={ruleWord} />{/if}
        </Field>

        <Select
          label="Based on"
          value={base}
          options={baseOptions}
          onchange={(v) => (base = v)}
          help={inheritedHelp}
        />

        <Field label="Add to">
          <Segmented
            full
            label="Add to"
            value={linkOn ? 'linked' : 'only'}
            options={[
              { value: 'only', label: `Only ${setup?.name}` },
              { value: 'linked', label: others.length ? `${setup?.name} and ${linkTo || others[0].name}, linked` : 'No other setup', icon: Link2, disabled: !others.length },
            ]}
            onchange={(v) => (linkOn = v === 'linked')}
          />
          {#if linkOn && others.length > 1}
            <Select size="sm" value={linkTo} options={others.map((s) => ({ value: s.name, label: s.name }))} onchange={(v) => (linkTo = v)} />
          {/if}
          <p class="muted t-small">{linkOn ? 'One profile, edits show in both.' : 'Only this setup gets it. You can link it later.'}</p>
        </Field>
      </div>

      <aside class="side" aria-label="Preview">
        <div class="well">{#if preview}<TemplatePreview profile={preview} base={baseProfile} unit={26} />{/if}</div>
        <span class="muted t-small">Preview{baseProfile ? ` · link marks come from ${baseProfile.name}` : ''}</span>
        {#if warn(picked.needs, picked.template)}<Banner kind="warning">{warn(picked.needs, picked.template)}</Banner>{/if}
      </aside>
    </div>
    {#snippet footer()}
      <Button variant="ghost" icon={ArrowLeft} onclick={() => (step = 1)}>Back</Button>
      <span class="note"></span>
      <Button onclick={close}>Cancel</Button>
      <Button variant="primary" disabled={!!nameError} onclick={create}>{picked?.existing ? 'Add profile' : 'Create profile'}</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .gallery {
    display: grid;
    gap: 14px;
    align-content: start;
    min-height: 420px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
  }
  .grid.one {
    grid-template-columns: minmax(0, 260px);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .note {
    flex: 1;
    align-self: center;
    font-size: var(--type-small-size);
    color: var(--color-text-2);
    text-align: left;
  }
  .wizard {
    display: flex;
    gap: 24px;
    min-height: 380px;
  }
  .form {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 16px;
    align-content: start;
  }
  .side {
    flex: none;
    width: 300px;
    display: grid;
    gap: 10px;
    align-content: start;
    justify-items: start;
  }
  .well {
    display: grid;
    place-items: center;
    width: 100%;
    min-height: 220px;
    padding: 12px;
    background: var(--hub-base);
    border-radius: var(--radius-well);
    box-shadow: var(--hub-base-shadow);
  }
  .rules {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .rules li {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .rules .muted {
    flex: 1;
  }
  .add {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
  }
  .suggest {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 8px;
    font: inherit;
    font-family: var(--font-mono);
    font-size: var(--type-chip-size);
    color: var(--color-text-2);
    background: var(--color-surface-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-chip);
    cursor: pointer;
  }
  .pill:hover {
    color: var(--color-text-1);
    border-color: var(--color-border-2);
  }
</style>

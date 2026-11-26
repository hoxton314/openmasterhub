<!--
  Profile settings (S20): name, base, where it is used (linked or not), LED colours, export and
  delete. Opened as the `profile-settings` route of the editor (`profile` param, else the profile
  being edited).
-->
<script lang="ts">
  import { ArrowLeft, Copy, FileDown, LayoutTemplate, Link2, Plus, Trash2 } from 'lucide-svelte';
  import { invoke } from '../../lib/backend';
  import { edit, editProfile, editor, hub, toastUndo } from '../../lib/hub.svelte';
  import { baseChain, effectiveSetups, isLinked, materialiseSetups, renameProfile } from '../../lib/model';
  import type { Color, Leds, ModuleKind, Profile } from '../../lib/types';
  import { ui } from '../../lib/ui.svelte';
  import Avatar from '../ui/Avatar.svelte';
  import Button from '../ui/Button.svelte';
  import Card from '../ui/Card.svelte';
  import Chip from '../ui/Chip.svelte';
  import ColorField from '../ui/ColorField.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import Field from '../ui/Field.svelte';
  import Menu, { type MenuEntry } from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import Select from '../ui/Select.svelte';
  import TextField from '../ui/TextField.svelte';
  import { deleteProfile, joinNames, profileReferences, separateCopy, addToSetup, usage } from './profileOps';

  const config = $derived(hub.config);
  const name = $derived(String(ui.route?.profile ?? editor.profile));
  const profile = $derived((config.profile ?? []).find((p) => p.name === name));
  const uses = $derived(usage(config, name));
  const linked = $derived(isLinked(config, name));
  const setups = $derived(effectiveSetups(config));
  const usedBy = $derived(uses.map((u) => u.setup));
  const copyTarget = $derived((uses.find((u) => u.setup !== editor.setup) ?? uses[1])?.setup ?? '');
  const free = $derived(setups.filter((s) => !usedBy.includes(s.name)));
  const globalLeds = $derived(config.leds ?? {});

  let draft = $state('');
  let draftFor = '';
  $effect(() => {
    if (draftFor !== name) {
      draftFor = name;
      draft = name;
    }
  });
  const nameError = $derived(!draft.trim() ? 'A profile needs a name.' : draft.trim() !== name && (config.profile ?? []).some((p) => p.name === draft.trim()) ? `There is already a profile called “${draft.trim()}”.` : undefined);

  function rename() {
    const next = draft.trim();
    if (nameError || next === name) {
      if (nameError) draft = name;
      return;
    }
    const wasEditing = editor.profile === name;
    edit((c) => renameProfile(c, name, next), { label: `Rename ${name}` });
    if (wasEditing) editor.profile = next;
    ui.open({ ...(ui.route ?? { view: 'profile-settings' }), view: 'profile-settings', profile: next });
    draftFor = next;
  }

  const baseOptions = $derived([
    { value: '', label: 'None' },
    ...(config.profile ?? []).filter((p) => p.name !== name && !baseChain(config, p.name).some((b) => b.name === name)).map((p) => ({ value: p.name, label: p.name })),
  ]);
  const inherited = $derived.by(() => {
    const b = profile?.base ? (config.profile ?? []).find((p) => p.name === profile.base) : undefined;
    if (!b || !profile) return '';
    const list = [!profile.dial && b.dial ? 'dial' : '', !profile.knob?.length && b.knob?.length ? 'knobs' : '', !profile.fader?.length && b.fader?.length ? 'faders' : '', !profile.roller?.length && b.roller?.length ? 'rollers' : ''].filter(Boolean);
    return list.length ? `${b.name} gives ${list.join(', ')}.` : '';
  });

  function setBase(v: string) {
    edit(
      (c) => {
        const p = (c.profile ?? []).find((x) => x.name === name);
        if (!p) return;
        if (v) p.base = v;
        else delete p.base;
      },
      { label: 'Change base' },
    );
  }

  // ── LEDs ──
  const first = (v: Color | Color[] | undefined) => (Array.isArray(v) ? v[0] : v);

  function setLeds(f: (l: Leds) => void, label: string, coalesce?: string) {
    edit(
      (c) => {
        const p = (c.profile ?? []).find((x) => x.name === name);
        if (!p) return;
        p.leds ??= {};
        f(p.leds);
        if (Object.values(p.leds).every((v) => v === undefined)) delete p.leds;
      },
      { label, coalesce },
    );
  }

  function setLed(key: 'base' | 'knob' | 'fader', v: Color | undefined) {
    setLeds((l) => (l[key] = v), `Change ${key} LEDs`, `led-${key}-${name}`);
  }

  // ── linked / copy ──
  function copyFor(setup: string) {
    ui.ask({
      title: `Make a separate copy for ${setup}`,
      body: `${setup} gets its own copy of ${name}. Edits to one no longer show in the other.`,
      value: `${name} (${setup})`,
      ok: 'Make copy',
      done: (v) => {
        let made = '';
        edit((c) => (made = separateCopy(c, name, setup, v)?.name ?? ''), { label: `Copy ${name} for ${setup}` });
        if (made) {
          editProfile(made);
          ui.open({ view: 'profile-settings', profile: made });
          toastUndo(`${setup} now has its own “${made}”.`);
        }
      },
    });
  }

  const copyItems = $derived<MenuEntry[]>(uses.map((u) => ({ label: `For ${u.setup}`, icon: Copy, onselect: () => copyFor(u.setup) })));
  const addItems = $derived<MenuEntry[]>(free.map((s) => ({ label: s.name, icon: Plus, onselect: () => addTo(s.name) })));
  let copyOpen = $state(false);
  let addOpen = $state(false);

  function addTo(setup: string) {
    edit((c) => addToSetup(c, name, setup), { label: `Add ${name} to ${setup}` });
    toastUndo(`Added “${name}” to ${setup}. It is now linked.`);
  }

  // ── export / template ──
  const plain = () => $state.snapshot(profile) as Profile;

  async function exportToml() {
    try {
      const path = await invoke<string | null>('export_toml', { profile: plain() });
      if (path) ui.toast({ text: `Exported to ${path}`, kind: 'success' });
    } catch (e) {
      ui.toast({ text: `Could not export: ${e}`, kind: 'danger' });
    }
  }

  function saveTemplate() {
    if (!profile) return;
    ui.ask({
      title: 'Save as template',
      body: 'Keeps the pages and controls, not the app rule. It shows in the Library and in the new-profile gallery.',
      value: name,
      ok: 'Save template',
      done: async (v) => {
        const p = plain();
        delete p.apps;
        delete p.base;
        p.name = v.trim() || name;
        const needs: ModuleKind[] = [];
        if (p.page?.length || p.folder?.length) needs.push('keypad');
        if (p.dial) needs.push('dial');
        if (p.knob?.length) needs.push('knob');
        if (p.fader?.length) needs.push('fader');
        if (p.roller?.length) needs.push('roller');
        const id = p.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'template';
        try {
          await invoke('template_save', { template: { id, kind: 'profile', name: p.name, about: p.about ?? '', category: 'My templates', needs, profile: p } });
          ui.toast({ text: `Saved the template “${p.name}”.`, kind: 'success' });
        } catch (e) {
          ui.toast({ text: `Could not save the template: ${e}`, kind: 'danger' });
        }
      },
    });
  }

  // ── delete ──
  function remove() {
    const refs = profileReferences(config, name);
    const parts: string[] = [];
    if (refs.setups.length) parts.push(`It is used in ${joinNames(refs.setups)}.`);
    if (refs.bases.length) parts.push(`${joinNames(refs.bases)} ${refs.bases.length > 1 ? 'are' : 'is'} based on it and will lose their base.`);
    if (refs.actions) parts.push(`${refs.actions} key${refs.actions > 1 ? 's switch' : ' switches'} to it.`);
    ui.confirm({
      title: `Delete ${name}?`,
      body: `${parts.join(' ')} Ctrl+Z brings it back.`.trim(),
      ok: `Delete ${name}`,
      done: () => {
        const rest = (config.profile ?? []).filter((p) => p.name !== name);
        edit((c) => deleteProfile(c, name), { label: `Delete ${name}` });
        if (rest.length) editProfile(rest.find((p) => p.name === hub.state.profile)?.name ?? rest[0].name);
        ui.open(null);
        toastUndo(`Deleted “${name}”.`);
      },
    });
  }

  const back = () => ui.open(null);
</script>

<div class="page">
  {#if !profile}
    <EmptyState title="No such profile" body="“{name}” is not in the config." actionLabel="Back to editor" onaction={back} />
  {:else}
    <header>
      <Avatar {name} color={profile.color} size={28} />
      <div class="titles">
        <h1 class="t-title">{name} · profile settings</h1>
        {#if linked}<Chip variant="linked">Linked in {joinNames(usedBy)}</Chip>{:else if uses.length}<span class="muted t-small">Used in {uses[0].setup}</span>{:else}<span class="muted t-small">Not used by any setup</span>{/if}
      </div>
      <Button icon={ArrowLeft} onclick={back}>Back to editor</Button>
    </header>

    <section>
      <TextField label="Name" bind:value={draft} error={nameError} onblur={rename} onkeydown={(e) => e.key === 'Enter' && (e.currentTarget as HTMLInputElement).blur()} />
      <Select label="Based on" value={profile.base ?? ''} options={baseOptions} onchange={setBase} help={inherited || (profile.base ? '' : 'Controls this profile leaves empty stay empty.')} />
    </section>

    <section>
      <SectionLabel as="h2">Used in</SectionLabel>
      {#if uses.length}
        <ul class="uses">
          {#each uses as u (u.setup)}
            <li><b>{u.setup}</b><span class="muted">·</span><span class="muted">{u.where}</span></li>
          {/each}
        </ul>
      {:else}
        <p class="muted t-small">No setup uses this profile yet, so it never applies.</p>
      {/if}
      {#if linked}
        <p class="muted t-small"><Link2 size={13} /> Linked: one profile, {uses.length === 2 ? 'two setups' : `${uses.length} setups`}. Edits show in {uses.length === 2 ? 'both' : 'all'}.</p>
      {/if}
      <div class="row">
        {#if linked}
          {#if uses.length === 2}
            <Button icon={Copy} onclick={() => copyFor(copyTarget)}>Make a separate copy for {copyTarget}</Button>
          {:else}
            <Popover bind:open={copyOpen} role="menu" label="Make a separate copy">
              {#snippet trigger({ toggle, attrs })}<Button icon={Copy} {...attrs} onclick={toggle}>Make a separate copy…</Button>{/snippet}
              {#snippet children({ close })}<Menu items={copyItems} onclose={close} />{/snippet}
            </Popover>
          {/if}
        {/if}
        {#if free.length}
          <Popover bind:open={addOpen} role="menu" label="Add to a setup">
            {#snippet trigger({ toggle, attrs })}<Button icon={Plus} {...attrs} onclick={toggle}>Add to a setup</Button>{/snippet}
            {#snippet children({ close })}<Menu items={addItems} onclose={close} />{/snippet}
          </Popover>
        {/if}
      </div>
    </section>

    <section>
      <SectionLabel as="h2">LED colours</SectionLabel>
      <Field label="Base ring" inline>
        <ColorField value={profile.leds?.base} fallback={globalLeds.base ?? '#5960D6'} onchange={(v) => setLed('base', v)} />
      </Field>
      <Field label="Knob rings" inline>
        <div class="led">
          <Segmented
            label="Knob rings"
            value={profile.leds?.knob ? 'own' : 'global'}
            options={[{ value: 'global', label: 'Global' }, { value: 'own', label: 'Own' }]}
            onchange={(v) => setLed('knob', v === 'own' ? (first(globalLeds.knob) ?? '#5960D6') : undefined)}
          />
          {#if profile.leds?.knob}<ColorField value={first(profile.leds.knob)} fallback="#5960D6" resettable={false} onchange={(v) => setLed('knob', v)} />{/if}
        </div>
      </Field>
      <Field label="Fader tracks" inline>
        <div class="led">
          <Segmented
            label="Fader tracks"
            value={profile.leds?.fader ? 'own' : 'global'}
            options={[{ value: 'global', label: 'Global' }, { value: 'own', label: 'Own' }]}
            onchange={(v) => setLed('fader', v === 'own' ? (first(globalLeds.fader) ?? '#5960D6') : undefined)}
          />
          {#if profile.leds?.fader}<ColorField value={first(profile.leds.fader)} fallback="#5960D6" resettable={false} onchange={(v) => setLed('fader', v)} />{/if}
        </div>
      </Field>
    </section>

    <section>
      <SectionLabel as="h2">Export</SectionLabel>
      <div class="row">
        <Button icon={FileDown} onclick={exportToml}>Export as .toml</Button>
        <Button icon={LayoutTemplate} onclick={saveTemplate}>Save as template</Button>
      </div>
    </section>

    <section>
      <SectionLabel as="h2">Delete</SectionLabel>
      <Card padding={14}>
        <div class="danger">
          <div class="grow">
            <b>Delete {name}</b>
            <p class="muted t-small">{uses.length ? `Removes it from ${joinNames(usedBy)}.` : 'Removes it from the config.'} Ctrl+Z restores it.</p>
          </div>
          <Button variant="danger" icon={Trash2} onclick={remove}>Delete {name}</Button>
        </div>
      </Card>
    </section>
  {/if}
</div>

<style>
  .page {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    align-content: start;
    gap: 22px;
    width: 100%;
    max-width: 640px;
    margin: 0 auto;
    padding: 24px 24px 48px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .titles {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 4px;
    justify-items: start;
  }
  section {
    display: grid;
    gap: 10px;
    padding-bottom: 20px;
    border-bottom: 1px solid var(--color-border);
  }
  section:last-child {
    border-bottom: 0;
  }
  .uses {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .uses li {
    display: flex;
    gap: 8px;
  }
  .row,
  .led {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  p {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .danger {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .grow {
    flex: 1;
    display: grid;
    gap: 2px;
  }
  .grow p {
    display: block;
  }
</style>

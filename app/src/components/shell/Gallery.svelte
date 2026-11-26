<!--
  Component gallery (`?gallery`, dev builds only): every ui/ component, once per theme, each column
  scoped to its theme with `themeStyle()`. Use it to check a component or a theme at a glance.
-->
<script lang="ts">
  import { Bell, FolderOpen, Keyboard, Layers, LayoutGrid, Pencil, Plus, Settings, Trash2, Undo2, Users } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import { FACE_BACKGROUNDS } from '../../lib/palette';
  import { loadUserThemes, theme, themeStyle } from '../../lib/theme/theme.svelte';
  import { ui } from '../../lib/ui.svelte';
  import {
    Avatar, Badge, Banner, Breadcrumb, Button, CanvasMarker, Card, CheckRow, Chip, ClaimOutline, ColorField, DialCard, Dialog, EmptyState,
    IconButton, InspectorSection, Kbd, KeyCapture, KeyCombo, KeyFace, LiveBadge, LogView, MarkerLegend, Menu, NumberField, Popover,
    ProgressSteps, RailItem, RuleRow, SectionLabel, Segmented, Select, ShortcutField, SidebarRow, StatusDot, Tabs, TemplateCard, TextField,
    Toast, Toggle, TreeRow, TriggerRow,
  } from '../ui';
  import ToastHost from './ToastHost.svelte';

  onMount(() => {
    loadUserThemes();
  });

  const params = new URLSearchParams(location.search);
  const only = params.get('gallery');
  const ids = $derived(only ? only.split(',') : ['dark', 'light', 'signal']);

  // Per-column demo state.
  const demo = () => ({ seg: 'image', on: true, text: 'Frame sel.', num: 250 as number | undefined, color: FACE_BACKGROUNDS[2] as string | undefined, combo: 'ctrl+shift+t', hold: undefined as number | undefined, sc: 'ctrl+alt+1' as string | undefined, tab: 'templates', dialog: false, menu: false, trig: true });
  const state = $state(Object.fromEntries([...new Set(['dark', 'light', 'signal', ...(only?.split(',') ?? [])])].map((id) => [id, demo()])));
  const s = (id: string) => state[id] ?? demo();
</script>

<div class="gallery">
  {#each ids as id (id)}
    {@const st = s(id)}
    <div class="col" style={themeStyle(id)}>
      <h1 class="t-title">{theme.list.find((t) => t.id === id)?.name ?? id}</h1>

      <section>
        <SectionLabel>Buttons</SectionLabel>
        <div class="row">
          <Button variant="primary" icon={Plus}>Primary</Button>
          <Button>Secondary</Button>
          <Button variant="ghost">Ghost</Button>
          <Button variant="danger" icon={Trash2}>Delete</Button>
        </div>
        <div class="row">
          <Button size="sm" variant="primary">Small</Button>
          <Button size="sm" disabled>Disabled</Button>
          <IconButton icon={Undo2} label="Undo" />
          <IconButton icon={Pencil} label="Edit" variant="secondary" />
          <IconButton icon={Bell} label="Active" active />
        </div>
      </section>

      <section>
        <SectionLabel>Segmented and toggles</SectionLabel>
        <Segmented label="Face" value={st.seg} onchange={(v) => (st.seg = v)} options={[{ value: 'label', label: 'Icon & label' }, { value: 'image', label: 'Image' }, { value: 'widget', label: 'Widget' }]} />
        <Segmented full label="Tap or hold" value={st.on ? 'tap' : 'hold'} onchange={(v) => (st.on = v === 'tap')} options={[{ value: 'tap', label: 'Tap' }, { value: 'hold', label: 'Hold' }]} />
        <div class="row">
          <Toggle checked={st.on} onchange={(v) => (st.on = v)} title="Toggle" />
          <Toggle checked={!st.on} onchange={(v) => (st.on = !v)} title="Toggle" />
          <Toggle label="Follow live" checked={st.on} onchange={(v) => (st.on = v)} />
        </div>
      </section>

      <section>
        <SectionLabel>Chips and status</SectionLabel>
        <div class="row wrap">
          <Chip icon={null}>blender</Chip>
          <Chip variant="glob">Godot*.exe</Chip>
          <Chip variant="linked">Linked</Chip>
          <Chip variant="integration">Integration</Chip>
          <Chip variant="success">Bridge connected</Chip>
          <Chip variant="warning">Needs setup</Chip>
          <Chip variant="danger">Error</Chip>
          <Chip variant="off">Off</Chip>
        </div>
        <div class="row">
          <LiveBadge />
          <StatusDot status="success" label="Running" /><span class="t-small">Running</span>
          <StatusDot status="warning" label="Waiting" /><span class="t-small">Waiting</span>
          <Badge>+1</Badge><Badge tone="accent">2 of 4</Badge><Badge tone="warning">3</Badge>
        </div>
      </section>

      <section>
        <SectionLabel>Fields</SectionLabel>
        <TextField label="Label" bind:value={st.text} help="Shown on the key." />
        <TextField label="App rule" mono value="Godot*.exe" error="Matches nothing open right now" />
        <Select label="Widget" value="cpu_usage" onchange={() => {}} options={[{ value: 'clock', label: 'Clock' }, { value: 'cpu_usage', label: 'CPU usage' }, { value: 'ram_usage', label: 'RAM usage' }]} />
        <div class="row">
          <NumberField label="Hold" value={st.num} unit="ms" onchange={(v) => (st.num = v)} />
          <ColorField label="Background" value={st.color} onchange={(v) => (st.color = v)} />
        </div>
      </section>

      <section>
        <SectionLabel>Keys</SectionLabel>
        <div class="row"><Kbd>Ctrl</Kbd><KeyCombo combo="ctrl+alt+f12" /><KeyCombo combo={undefined} /></div>
        <KeyCapture value={st.combo} onchange={(c) => (st.combo = c)} hold={st.hold} onhold={(h) => (st.hold = h)} />
        <ShortcutField label="Switch to Work" value={st.sc} onchange={(c) => (st.sc = c)} />
      </section>

      <section>
        <SectionLabel>Trigger rows</SectionLabel>
        <TriggerRow label="Press" icon={Keyboard} combo="ctrl+s" active={st.trig} onclick={() => (st.trig = !st.trig)}>
          <span class="t-small muted">The action editor opens here.</span>
        </TriggerRow>
        <TriggerRow label="Release" />
        <TriggerRow label="Turn right" summary="Output volume +2%" />
      </section>

      <section>
        <SectionLabel>Sidebar</SectionLabel>
        <div class="list" role="listbox" aria-label="Profiles">
          <SidebarRow name="Blender" selected live />
          <SidebarRow name="Godot" glob="godot" draggable />
          <SidebarRow name="Cyberpunk 2077" integration glob="steam_app_…" />
          <SidebarRow name="Claude" conflict="Ghostty is above and matches the same window" glob="claude-*" />
          <SidebarRow name="Desktop" fallback locked />
        </div>
        <div class="list">
          <TreeRow name="Views" selected meta={1} />
          <TreeRow name="Modeling" meta={2} />
          <TreeRow name="Shading" kind="folder" meta="folder" />
          <TreeRow name="Add page or folder" kind="add" />
        </div>
        <div class="row"><SidebarRow name="Blender" collapsed live /><SidebarRow name="Godot" collapsed /><Avatar name="Obsidian" size={28} /></div>
      </section>

      <section>
        <SectionLabel>Rules</SectionLabel>
        <RuleRow index={2} name="Godot" globs={['godot', 'Godot*.exe']} />
        <RuleRow index={3} name="Discord" globs={['Discord.exe']} linked="Linked · also in Play" />
        <RuleRow index={6} name="Claude" globs={['claude-*']} conflict="Can never win for com.mitchellh.ghostty: Ghostty (rule 4) matches it first." />
        <RuleRow name="Desktop" fallback />
      </section>

      <section>
        <SectionLabel>Navigation</SectionLabel>
        <div class="row rail">
          <RailItem icon={LayoutGrid} label="Editor" active onclick={() => {}} />
          <RailItem icon={Layers} label="Setups" onclick={() => {}} />
          <RailItem icon={Settings} label="Settings" badge onclick={() => {}} />
        </div>
        <Breadcrumb items={[{ label: 'Work', onclick: () => {} }, { label: 'Blender', live: true, onclick: () => {} }, { label: 'Views', icon: LayoutGrid, menu: true, onclick: () => {} }]} />
        <Tabs label="Gallery" value={st.tab} onchange={(v) => (st.tab = v)} tabs={[{ value: 'templates', label: 'Templates' }, { value: 'integrations', label: 'Integrations', count: 4 }, { value: 'mine', label: 'My profiles' }, { value: 'blank', label: 'Blank' }]} />
        <div class="row">
          <Popover bind:open={st.menu} role="menu" label="Page">
            {#snippet trigger({ toggle, attrs })}<Button {...attrs} onclick={toggle}>Menu</Button>{/snippet}
            {#snippet children({ close })}
              <Menu onclose={close} items={[{ heading: 'Page' }, { label: 'Rename', icon: Pencil, shortcut: 'f2', onselect: () => {} }, { label: 'Open folder', icon: FolderOpen, onselect: () => {} }, { separator: true }, { label: 'Delete', icon: Trash2, danger: true, onselect: () => {} }]} />
            {/snippet}
          </Popover>
          <Button onclick={() => (st.dialog = true)}>Dialog</Button>
          <Button variant="ghost" onclick={() => ui.toast({ text: 'Key 7 reset', action: { label: 'Undo', run: () => {} } })}>Toast</Button>
        </div>
        <Dialog open={st.dialog} title="Delete Discord?" subtitle="It is used in Work and Play." onclose={() => (st.dialog = false)}>
          <p>Both setups lose it. Ctrl+Z restores it during this session.</p>
          {#snippet footer()}
            <Button onclick={() => (st.dialog = false)}>Cancel</Button>
            <Button variant="danger" onclick={() => (st.dialog = false)}>Delete</Button>
          {/snippet}
        </Dialog>
      </section>

      <section>
        <SectionLabel>Feedback</SectionLabel>
        <Toast text="Key 7 reset" action={{ label: 'Undo', run: () => {} }} />
        <Toast text="Could not read wallpaper.gif" kind="danger" action={{ label: 'Details', run: () => {} }} />
        <Banner kind="accent">You are editing <b>Godot</b>. Blender is live on the hub.{#snippet actions()}<Button size="sm">Show on hub</Button>{/snippet}</Banner>
        <Banner kind="claim">Cyberpunk 2077 mode owns the keypad while the game is focused.</Banner>
        <EmptyState icon={Users} title="No profiles in this setup" body="Start from a template. Blank is the last tab." actionLabel="New profile" onaction={() => {}} />
        <ProgressSteps count={5} current={2} />
        <CheckRow title="Service" detail="masterhubd 0.9.0" status="success" statusLabel="Running" />
        <CheckRow title="Focus tracking" detail="Hyprland" status="warning" statusLabel="Limited" />
        <LogView lines={['INFO masterhubd started', 'WARN keypad: no reply to GET 0x03', 'ERROR dial: device busy']} height={90} />
      </section>

      <section>
        <SectionLabel>Cards</SectionLabel>
        <Card><b>Work</b><span class="t-small muted">Follow the focused app · 7 profiles</span></Card>
        <div class="row cards">
          <TemplateCard title="Blender" description="Views, shading, render" modules={['Keypad', 'Dial', 'Knobs']} selected>
            {#snippet preview()}<DialCard name="Blender" sub="Work · by app" size={100} />{/snippet}
          </TemplateCard>
          <TemplateCard title="Volume mixer" description="Five apps on five faders" modules={['Faders']} warning="Uses faders, you have none" />
        </div>
      </section>

      <section class="canvas">
        <SectionLabel>Canvas (hub hardware)</SectionLabel>
        <div class="row markers">
          <span class="key marker-selected"></span>
          <span class="key"><CanvasMarker kind="pinned" /></span>
          <span class="key"><CanvasMarker kind="integration" /></span>
          <span class="key"><CanvasMarker kind="inherited" /></span>
          <span class="key marker-pressed"></span>
          <span class="module"><ClaimOutline label="Cyberpunk 2077" /></span>
        </div>
        <MarkerLegend />
        <div class="row"><KeyFace label="72×72" /><DialCard name="Blender" sub="Work · by app" /></div>
      </section>

      <InspectorSection title="Inspector section">
        {#snippet action()}<IconButton icon={Plus} size="sm" label="Add" />{/snippet}
        <span class="t-small muted">Padding 14×16, gap 10, hairline below.</span>
      </InspectorSection>
    </div>
  {/each}
</div>
<ToastHost />

<style>
  .gallery {
    display: flex;
    height: 100%;
    overflow-x: auto;
  }
  .col {
    flex: 1 0 440px;
    height: 100%;
    overflow-y: auto;
    padding: 24px;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-content: start;
    gap: 14px;
    font: var(--type-body-weight) var(--type-body-size) / var(--type-body-lh) var(--type-body-family);
    color: var(--color-text-1);
    background: var(--color-bg);
    border-right: 1px solid var(--color-border);
  }
  section {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-width: 0;
    gap: 10px;
    padding: 16px;
    background: var(--color-surface);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-hairline);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .wrap {
    flex-wrap: wrap;
  }
  .row > :global(.ui-field) {
    flex: 1;
  }
  .list {
    display: grid;
    gap: 1px;
  }
  .rail {
    padding: 6px;
    background: var(--color-rail);
  }
  .cards > :global(*) {
    flex: 1;
  }
  .canvas {
    background-color: var(--color-canvas);
    background-image: radial-gradient(var(--color-canvas-dot) 1px, transparent 1px);
    background-size: 20px 20px;
  }
  .markers {
    gap: 22px;
    padding: 20px 10px;
  }
  .key {
    position: relative;
    width: 56px;
    height: 56px;
    border-radius: var(--hub-key-radius);
    background: var(--hub-key);
    box-shadow: var(--hub-key-frame);
  }
  .key.marker-pressed {
    box-shadow: 0 0 0 3px var(--color-pressed), 0 0 18px var(--color-pressed);
  }
  .module {
    position: relative;
    width: 70px;
    height: 56px;
    border-radius: var(--hub-chassis-radius);
    background: var(--hub-chassis);
    box-shadow: var(--hub-chassis-shadow);
  }
</style>

<!-- Settings › Appearance: theme cards (bundled and user themes), follow system, density, motion. -->
<script lang="ts">
  import { FolderOpen, RefreshCw, TriangleAlert } from 'lucide-svelte';
  import { loadUserThemes, theme, themeStyle } from '../../lib/theme/theme.svelte';
  import Button from '../ui/Button.svelte';
  import Card from '../ui/Card.svelte';
  import Chip from '../ui/Chip.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import Toggle from '../ui/Toggle.svelte';
  import { openPath, run, saveAppearance } from './actions';
  import Page from './Page.svelte';
  import Row from './Row.svelte';

  const followSystem = $derived(theme.settings.theme === 'system');
  /** The theme shown as chosen: the one in use, or while following the system, the one for the current mode. */
  const current = $derived(theme.active);
  const density = $derived(theme.settings.density ?? ((theme.active && document.documentElement.dataset.density) as 'compact' | 'comfortable' | undefined) ?? 'compact');

  function pick(id: string) {
    const t = theme.list.find((x) => x.id === id);
    // While following the system, a theme that follows it too only replaces its Dark or Light slot;
    // an opt-in theme (Signal) switches following off and stays on.
    if (followSystem && t?.followsSystem) return saveAppearance(t.appearance === 'light' ? { themeLight: id } : { themeDark: id }, 'theme change');
    saveAppearance({ theme: id }, 'theme change');
  }

  async function openFolder() {
    const dir = await run<string>('themes_dir', undefined, 'Could not find the themes folder');
    if (dir) await openPath(dir);
  }

  /** Themes that opt in to following the system are offered as Dark/Light; others stay until changed. */
  const followHint = $derived(
    followSystem ? `Using ${theme.list.find((t) => t.id === theme.settings.themeDark)?.name ?? 'Dark'} or ${theme.list.find((t) => t.id === theme.settings.themeLight)?.name ?? 'Light'}` : 'Your chosen theme stays on until you change it',
  );
</script>

<Page title="Appearance" about="Stored in your config file, like everything else.">
  <section>
    <SectionLabel as="h3">Theme</SectionLabel>
    <div class="themes" role="group" aria-label="Theme">
      {#each theme.list as t (t.id)}
        <Card selected={current === t.id} onclick={() => pick(t.id)} padding={10} label="Theme {t.name}">
          <div class="preview" style={themeStyle(t.id)}>
            <span class="rail"></span>
            <span class="panel"><span class="line"></span><span class="line short"></span><span class="accent"></span></span>
          </div>
          <div class="meta">
            <b>{t.name}</b>
            {#if t.source === 'user'}<Chip variant="outline" icon={null}>{t.file ?? 'user'}</Chip>
            {:else if !t.followsSystem}<Chip variant="outline" icon={null}>Opt-in</Chip>{/if}
          </div>
          {#if t.description}<span class="t-small muted">{t.description}</span>{/if}
        </Card>
      {/each}
    </div>
    <Row label="Follow system" help={followHint}>
      <Toggle title="Follow system" checked={followSystem} onchange={(on) => saveAppearance({ theme: on ? 'system' : current }, 'theme change')} />
    </Row>
  </section>

  <section>
    <SectionLabel as="h3">Your themes</SectionLabel>
    <p class="t-small muted">Drop a <code>.json</code> theme file into the themes folder. It can inherit from Dark, Light or Signal and change only what it needs (see docs/THEMES.md).</p>
    <div class="buttons">
      <Button icon={FolderOpen} onclick={openFolder}>Open themes folder</Button>
      <Button variant="ghost" icon={RefreshCw} onclick={() => loadUserThemes()}>Reload themes</Button>
    </div>
    {#each theme.problems as p (p.file + p.message)}
      <p class="problem" role="alert"><TriangleAlert size={14} /><span><b>{p.file}</b> was not loaded: {p.message}</span></p>
    {/each}
    {#if !theme.problems.length && !theme.list.some((t) => t.source === 'user')}
      <p class="t-small faint">No user themes found.</p>
    {/if}
  </section>

  <section>
    <SectionLabel as="h3">Density</SectionLabel>
    <Row label="Density" help="Row height and spacing">
      <Segmented
        label="Density"
        value={density}
        onchange={(v) => saveAppearance({ density: v }, 'density change')}
        options={[
          { value: 'compact', label: 'Compact 13 px' },
          { value: 'comfortable', label: 'Comfortable 14 px' },
        ]} />
    </Row>
  </section>

  <section>
    <SectionLabel as="h3">Motion</SectionLabel>
    <Row label="Reduce motion" help="Follows the system setting by default">
      <Segmented
        label="Reduce motion"
        value={theme.settings.reduceMotion}
        onchange={(v) => saveAppearance({ reduceMotion: v }, 'motion change')}
        options={[
          { value: 'system', label: 'System' },
          { value: 'on', label: 'On' },
          { value: 'off', label: 'Off' },
        ]} />
    </Row>
  </section>
</Page>

<style>
  section {
    display: grid;
    gap: var(--space-3);
  }
  p {
    margin: 0;
  }
  .themes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--space-3);
  }
  .preview {
    display: flex;
    height: 84px;
    overflow: hidden;
    background: var(--color-bg);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-hairline);
  }
  .preview .rail {
    width: 18px;
    background: var(--color-rail);
    border-right: var(--panel-border-width) solid var(--color-border);
  }
  .preview .panel {
    flex: 1;
    margin: 10px;
    padding: 8px;
    display: grid;
    align-content: start;
    gap: 6px;
    background: var(--color-surface);
    border-radius: var(--radius-sm);
  }
  .line {
    height: 6px;
    width: 80%;
    border-radius: var(--radius-pill);
    background: var(--color-text-2);
  }
  .short {
    width: 50%;
    background: var(--color-text-3);
  }
  .accent {
    width: 36px;
    height: 12px;
    border-radius: var(--radius-control);
    background: var(--color-accent-fill);
  }
  .meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .buttons {
    display: flex;
    gap: var(--space-2);
  }
  .problem {
    display: flex;
    gap: var(--space-2);
    align-items: flex-start;
    color: var(--color-warning);
  }
  .problem span {
    color: var(--color-text-2);
  }
</style>

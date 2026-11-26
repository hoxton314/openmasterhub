<!--
  WP6 owns this file. Wave 0 stand-in: a minimal Appearance section so themes, density and motion
  can be switched (saved on this computer only; WP6 adds `[app]` in config.toml and the rest of S28).
-->
<script lang="ts">
  import { TriangleAlert } from 'lucide-svelte';
  import { chosenTheme, setThemeSettings, theme, themeStyle } from '../../lib/theme/theme.svelte';
  import Card from '../ui/Card.svelte';
  import Chip from '../ui/Chip.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import Toggle from '../ui/Toggle.svelte';

  const followSystem = $derived(theme.settings.theme === 'system');
  function pick(id: string) {
    if (!followSystem) return setThemeSettings({ theme: id });
    const t = theme.list.find((x) => x.id === id);
    setThemeSettings(t?.appearance === 'light' ? { themeLight: id } : { themeDark: id });
  }
</script>

<div class="settings">
  <h1 class="t-title">Settings</h1>
  <section>
    <SectionLabel as="h2">Appearance</SectionLabel>
    <Toggle label="Follow the system's light and dark" checked={followSystem} onchange={(on) => setThemeSettings({ theme: on ? 'system' : chosenTheme() })} />
    <div class="themes">
      {#each theme.list as t (t.id)}
        <Card selected={theme.active === t.id} onclick={() => pick(t.id)} padding={10} label="Theme {t.name}">
          <div class="preview" style={themeStyle(t.id)}>
            <span class="rail"></span>
            <span class="panel"><span class="line"></span><span class="line short"></span><span class="accent"></span></span>
          </div>
          <div class="meta">
            <b>{t.name}</b>
            {#if t.source === 'user'}<Chip variant="outline" icon={null}>{t.file}</Chip>{/if}
          </div>
          {#if t.description}<span class="desc">{t.description}</span>{/if}
        </Card>
      {/each}
    </div>
    {#each theme.problems as p}
      <p class="problem" role="alert"><TriangleAlert size={14} /><span><b>{p.file}</b> was not loaded: {p.message}</span></p>
    {/each}
    <div class="row">
      <span>Density</span>
      <Segmented label="Density" value={theme.settings.density ?? 'theme'} onchange={(v) => setThemeSettings({ density: v === 'theme' ? undefined : v })}
        options={[{ value: 'theme', label: 'Theme default' }, { value: 'compact', label: 'Compact' }, { value: 'comfortable', label: 'Comfortable' }]} />
    </div>
    <div class="row">
      <span>Reduce motion</span>
      <Segmented label="Reduce motion" value={theme.settings.reduceMotion} onchange={(v) => setThemeSettings({ reduceMotion: v })}
        options={[{ value: 'system', label: 'System' }, { value: 'on', label: 'On' }, { value: 'off', label: 'Off' }]} />
    </div>
  </section>
</div>

<style>
  .settings {
    flex: 1;
    overflow-y: auto;
    padding: 32px 40px;
    display: grid;
    align-content: start;
    gap: 20px;
    max-width: 960px;
  }
  section {
    display: grid;
    gap: 14px;
  }
  .themes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
  }
  .preview {
    display: flex;
    height: 84px;
    overflow: hidden;
    background: var(--color-bg);
    border-radius: var(--radius-md);
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
    gap: 8px;
  }
  .desc {
    font-size: var(--type-small-size);
    color: var(--color-text-2);
  }
  .problem {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    color: var(--color-warning);
    font-size: var(--type-small-size);
  }
  .problem span {
    color: var(--color-text-2);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .row > span {
    width: 120px;
    color: var(--color-text-2);
  }
</style>

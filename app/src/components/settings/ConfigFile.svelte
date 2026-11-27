<!-- Settings › Config file: where it lives, reload, back up, restore, export and import. -->
<script lang="ts">
  import { Archive, Download, FileText, RefreshCw, Upload } from 'lucide-svelte';
  import { hub, reload, toast } from '../../lib/hub.svelte';
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import StatusDot from '../ui/StatusDot.svelte';
  import { openPath, run } from './actions';
  import Page from './Page.svelte';
  import Row from './Row.svelte';

  async function backUp() {
    const path = await run<string>('backup_config', undefined, 'Backup failed');
    if (path !== undefined) toast({ text: path ? `Backed up to ${path}` : 'Backed up', kind: 'success' });
  }
  function restore() {
    ui.confirm({
      title: 'Restore a backup?',
      body: 'Pick a backup file. Your current config is backed up first, then replaced.',
      ok: 'Choose a backup…',
      danger: false,
      done: async () => {
        await run('restore_config', undefined, 'Restore failed');
        await reload();
      },
    });
  }
  async function exportAll() {
    const path = await run<string>('export_toml', undefined, 'Export failed');
    if (path) toast({ text: `Exported to ${path}`, kind: 'success' });
  }
  async function importFile() {
    if ((await run('import_toml', undefined, 'Import failed')) !== undefined) await reload();
  }
</script>

<Page title="Config file" about="Everything you set up lives in one plain text file. No account, nothing leaves this PC.">
  <section>
    <SectionLabel as="h3">Location</SectionLabel>
    <div class="path">
      <FileText size={16} />
      <code class="t-mono">{hub.configPath || '~/.config/openmasterhub/config.toml'}</code>
    </div>
    <p class="status t-small muted"><StatusDot status="success" halo={false} size={6} /> Watched · reloads on save</p>
    <div class="buttons">
      <Button icon={FileText} onclick={() => openPath(hub.configPath)} disabled={!hub.configPath}>Open in editor</Button>
      <Button icon={RefreshCw} onclick={() => reload().then(() => toast('Config reloaded'))}>Reload</Button>
    </div>
  </section>
  <section>
    <SectionLabel as="h3">Backups</SectionLabel>
    <Row label="Back up now" help="A dated copy next to the config file">
      <Button icon={Archive} onclick={backUp}>Back up now</Button>
      <Button variant="ghost" onclick={restore}>Restore…</Button>
    </Row>
    <Row label="Move to another computer" help="Export every setup and profile as one .toml, or import one">
      <Button icon={Download} onclick={exportAll}>Export all</Button>
      <Button icon={Upload} onclick={importFile}>Import</Button>
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
  .path {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    color: var(--color-text-2);
    background: var(--color-sunken);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-well);
    word-break: break-all;
  }
  .status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .buttons {
    display: flex;
    gap: var(--space-2);
  }
</style>

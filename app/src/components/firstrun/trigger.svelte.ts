// Opens the first-run flow once per session when a loaded config has no `app.first_run_done`.
// Imported for its side effect by FirstRun.svelte, which App always imports, so App needs no code.

import { hub } from '../../lib/hub.svelte';
import { ui } from '../../lib/ui.svelte';

let shown = false;

$effect.root(() => {
  $effect(() => {
    if (shown || !hub.online || !hub.loaded) return;
    if (hub.config.app?.first_run_done) return;
    shown = true;
    ui.firstRun = true;
  });
});

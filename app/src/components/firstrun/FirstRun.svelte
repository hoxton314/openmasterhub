<!-- WP6 owns this file. Mounted by App while `ui.firstRun` is true. -->
<script lang="ts">
  import { ui } from '../../lib/ui.svelte';
  import Dialog from '../ui/Dialog.svelte';
</script>

<Dialog title="Welcome" subtitle="First run (S29) arrives with WP6." onclose={() => (ui.firstRun = false)} />

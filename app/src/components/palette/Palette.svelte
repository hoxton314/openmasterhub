<!-- WP6 owns this file. Mounted by App while `ui.palette` is true (Ctrl K); closes by clearing it. -->
<script lang="ts">
  import { ui } from '../../lib/ui.svelte';
  import Dialog from '../ui/Dialog.svelte';
</script>

<Dialog title="Search" subtitle="The command palette (S07) arrives with WP6." onclose={() => (ui.palette = false)} />

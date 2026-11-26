<!-- WP4 owns this file. Mounted by App while `ui.newProfile` is set; closes by clearing it. -->
<script lang="ts">
  import { ui } from '../../lib/ui.svelte';
  import Dialog from '../ui/Dialog.svelte';
</script>

<Dialog title="New profile" subtitle="The template gallery (S19) arrives with WP4." onclose={() => (ui.newProfile = null)} />

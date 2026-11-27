<!-- Overlays the library opens from anywhere (mounted once by App). A game picked elsewhere (the
  legacy sidebar sets `games.open`) opens its setup plan in the Library. -->
<script lang="ts">
  import { games } from '../../lib/games.svelte';
  import { ui } from '../../lib/ui.svelte';

  $effect(() => {
    const g = games.open;
    if (!g) return;
    games.open = null;
    ui.go('library', { view: 'integration', id: g.game });
  });
</script>

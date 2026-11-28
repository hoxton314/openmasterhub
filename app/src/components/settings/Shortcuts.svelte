<!-- Settings › Keyboard shortcuts: the shortcuts of this app (the hub's own keys live in the editor). -->
<script lang="ts">
  import KeyCombo from '../ui/KeyCombo.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import Page from './Page.svelte';
  import Row from './Row.svelte';
  import { SHORTCUT_GROUPS } from './shortcuts';
</script>

<Page title="Keyboard shortcuts" about="For this app. Shortcuts that switch setups are set per setup, in Setups.">
  {#each SHORTCUT_GROUPS as g (g.title)}
    <section>
      <SectionLabel as="h3">{g.title}</SectionLabel>
      {#each g.items as s (s.combo + s.label)}
        <Row label={s.label}><KeyCombo combo={s.combo} /></Row>
      {/each}
    </section>
  {/each}
</Page>

<style>
  section {
    display: grid;
  }
</style>

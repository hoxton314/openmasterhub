<!-- Settings › Feedback: what shows when the profile or setup switches (S08). Saved to [app.feedback]. -->
<script lang="ts">
  import { hasCapability, hub } from '../../lib/hub.svelte';
  import Banner from '../ui/Banner.svelte';
  import SectionLabel from '../ui/SectionLabel.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import Toggle from '../ui/Toggle.svelte';
  import { saveApp } from './actions';
  import Page from './Page.svelte';
  import Row from './Row.svelte';

  const fb = $derived(hub.config.app?.feedback ?? {});
  const on = (v: boolean | undefined) => v !== false;
  const overlay = $derived(hub.config.app?.overlay === 'all' || hub.config.app?.overlay === 'off' ? hub.config.app.overlay : 'switches');
  const set = (patch: Partial<NonNullable<typeof fb>>) => saveApp({ feedback: { ...fb, ...patch } }, 'feedback change');
</script>

<Page title="Feedback" about="What you see when the profile or setup changes. A small toast near the tray, never stealing focus.">
  <section>
    <SectionLabel as="h3">When the profile or setup switches</SectionLabel>
    <Row label="Toast" help="Shown for 1.5 s; setup changes carry an Undo for 5 s">
      <Toggle title="Toast" checked={on(fb.toast)} onchange={(v) => set({ toast: v })} />
    </Row>
    <Row label="Toast for" help="Profiles that follow the focused window change all the time, so by default only your own switches show">
      <Segmented
        label="Toast for"
        value={overlay}
        disabled={!on(fb.toast)}
        onchange={(v) => saveApp({ overlay: v }, 'toast change')}
        options={[
          { value: 'switches', label: 'My switches' },
          { value: 'all', label: 'Every change' },
          { value: 'off', label: 'Off' },
        ]} />
    </Row>
    <Row label="Card on the dial" help="The dial draws the matching card">
      <Toggle title="Card on the dial" checked={on(fb.dial)} onchange={(v) => set({ dial: v })} />
    </Row>
    <Row label="Blink the Base LED" help="Twice, in the setup colour">
      <Toggle title="Blink the Base LED" checked={on(fb.led)} onchange={(v) => set({ led: v })} />
    </Row>
    <Row label="Also in fullscreen games" help="Off by default, so games are never interrupted">
      <Toggle title="Also in fullscreen games" checked={fb.in_fullscreen === true} onchange={(v) => set({ in_fullscreen: v })} />
    </Row>
  </section>
  {#if !hasCapability('blink_on_setup') || !hasCapability('setups')}
    <Banner kind="info">The running service does not show the dial card or blink the LED yet. Your choices are saved for when it does.</Banner>
  {/if}
</Page>

<style>
  section {
    display: grid;
  }
</style>

<!--
  Shown while the editor looks at a profile that is not live (S04): changes apply when it becomes
  active. "Show on hub for 10 s" puts it on the hub briefly; "Follow live" returns to the live profile.
-->
<script lang="ts" module>
  /** A temporary show on the hub: the profile to go back to and seconds left. */
  export const showing = $state({ back: '', left: 0 });
  let timer: ReturnType<typeof setInterval> | undefined;
</script>

<script lang="ts">
  import { Eye, Play } from 'lucide-svelte';
  import { invoke } from '../../lib/backend';
  import { editor, hub, isEditingLive, setFollowLive, toast } from '../../lib/hub.svelte';
  import Banner from '../ui/Banner.svelte';
  import Button from '../ui/Button.svelte';

  const SECONDS = 10;

  async function set(name: string) {
    hub.state.profile = name;
    try {
      await invoke('set_profile', { name });
    } catch (e) {
      toast(String(e));
    }
  }

  async function showOnHub() {
    if (showing.left > 0) return;
    showing.back = hub.state.profile;
    showing.left = SECONDS;
    await set(editor.profile);
    timer = setInterval(async () => {
      if (--showing.left > 0) return;
      clearInterval(timer);
      await set(showing.back);
    }, 1000);
  }

  async function backNow() {
    clearInterval(timer);
    showing.left = 0;
    await set(showing.back);
  }

  const name = $derived(editor.profile);
  const liveName = $derived(showing.left > 0 ? showing.back : hub.state.profile);
</script>

{#if showing.left > 0}
  <Banner kind="accent" icon={Eye}>
    <strong>{name} is on the hub</strong> for {showing.left} s, then {showing.back} returns.
    {#snippet actions()}<Button size="sm" onclick={backNow}>Back to {showing.back}</Button>{/snippet}
  </Banner>
{:else if !isEditingLive()}
  <Banner kind="warning" icon={Eye}>
    <strong>You are editing {name}.</strong> {liveName} is live on the hub, so changes here apply when {name} becomes active.
    {#snippet actions()}
      <Button size="sm" icon={Play} onclick={showOnHub}>Show on hub for {SECONDS} s</Button>
      <Button size="sm" variant="ghost" onclick={() => setFollowLive(true)}>Follow live</Button>
    {/snippet}
  </Banner>
{/if}

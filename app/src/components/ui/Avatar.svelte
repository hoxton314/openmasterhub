<!-- Profile letter tile. Colour: the profile's own `color`, else one of the theme's avatar tokens. -->
<script lang="ts">
  import { avatarColor } from '../../lib/model/setups';

  let { name, color, size = 20 }: { name: string; color?: string; size?: number } = $props();
  const bg = $derived(avatarColor({ name, color }));
  const letter = $derived([...name.trim()][0]?.toUpperCase() ?? '?');
</script>

<span class="avatar" aria-hidden="true" style="--avatar-bg:{bg};--s:{size}px">{letter}</span>

<style>
  .avatar {
    display: inline-grid;
    place-items: center;
    flex: none;
    width: var(--s);
    height: var(--s);
    font: 700 calc(var(--s) * 0.5) / 1 var(--type-body-family);
    color: var(--color-on-avatar);
    background: var(--avatar-bg);
    border-radius: var(--radius-avatar);
  }
</style>

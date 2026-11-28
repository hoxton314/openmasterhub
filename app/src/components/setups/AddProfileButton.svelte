<!-- "Add profile": a new one (the gallery) or one of the existing profiles. -->
<script lang="ts">
  import { Plus } from 'lucide-svelte';
  import { hub } from '../../lib/hub.svelte';
  import type { EffectiveSetup } from '../../lib/model';
  import { ui } from '../../lib/ui.svelte';
  import Button from '../ui/Button.svelte';
  import Menu, { type MenuEntry } from '../ui/Menu.svelte';
  import Popover from '../ui/Popover.svelte';
  import { addProfileToSetup } from './ops';

  let { setup, onrule }: { setup: EffectiveSetup; /** Called for an added profile with no app rule (omit in manual setups). */ onrule?: (profile: string) => void } = $props();

  const addable = $derived((hub.config.profile ?? []).filter((p) => !setup.profiles.includes(p.name) && p.name !== setup.fallback));

  function items(): MenuEntry[] {
    const out: MenuEntry[] = [{ label: 'New profile…', icon: Plus, onselect: () => ui.openNewProfile({ setup: setup.name, position: setup.profiles.length }) }];
    if (addable.length) {
      out.push({ separator: true }, { heading: 'Add an existing profile' });
      for (const p of addable) {
        out.push({
          label: p.name,
          hint: onrule && !p.apps?.length ? 'No app rule yet' : undefined,
          onselect: () => {
            addProfileToSetup(setup.name, p.name);
            if (onrule && !p.apps?.length) onrule(p.name);
          },
        });
      }
    }
    return out;
  }
</script>

<Popover placement="bottom-end" role="menu" label="Add profile">
  {#snippet trigger({ toggle, attrs })}<Button icon={Plus} {...attrs} onclick={toggle}>Add profile</Button>{/snippet}
  {#snippet children({ close })}<Menu items={items()} onclose={close} label="Add profile" />{/snippet}
</Popover>

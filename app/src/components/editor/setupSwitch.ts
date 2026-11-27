// Switching which setup the editor shows (not which is live), shared by the sidebar and topbar.

import { editProfile, editor, hub, setFollowLive, setSetup } from '../../lib/hub.svelte';
import { activeSetup, effectiveSetups, setupByName } from '../../lib/model';
import type { SetupMode } from '../../lib/types';
import { ui } from '../../lib/ui.svelte';
import type { MenuEntry } from '../ui/Menu.svelte';
import { Layers, Settings2 } from 'lucide-svelte';

export const MODE_TEXT: Record<SetupMode, string> = {
  follow: 'Follow the focused app',
  manual: 'Choose by hand',
  hybrid: 'Follow, with a pin',
};

/** The setup the service runs. */
export const liveSetupName = () => hub.state.setup ?? activeSetup(hub.config).name;

/** The setup the editor shows. */
export const shownSetup = () => setupByName(hub.config, editor.setup) ?? activeSetup(hub.config, { setup: liveSetupName() });

/** Show a setup's profiles in the editor; the live setup resumes following the hub. */
export function editSetup(name: string) {
  const s = setupByName(hub.config, name);
  if (!s) return;
  if (name === liveSetupName()) {
    setFollowLive(true);
    return;
  }
  const first = s.profiles[0] ?? s.fallback;
  if (first) editProfile(first);
  editor.followLive = false;
  editor.setup = name;
}

export function setupMenu(onclose?: () => void): MenuEntry[] {
  const live = liveSetupName();
  const shown = shownSetup().name;
  const items: MenuEntry[] = effectiveSetups(hub.config).map((s) => ({
    label: s.name,
    icon: Layers,
    checked: s.name === shown,
    hint: s.name === live ? 'active' : undefined,
    onselect: () => editSetup(s.name),
  }));
  items.push({ separator: true });
  if (shown !== live) items.push({ label: `Make ${shown} active`, onselect: () => setSetup(shown) });
  items.push({ label: 'Manage setups…', icon: Settings2, onselect: () => (onclose?.(), ui.go('setups')) });
  return items;
}

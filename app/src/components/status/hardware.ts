// What is wrong with the hardware, if anything (S06). `HardwareState` shows it; the editor area can
// mount `HardwareState` in place of its canvas while `hardwareIssue()` is not undefined.

import { invoke } from '../../lib/backend';
import { hub } from '../../lib/hub.svelte';

export type HardwareIssue = 'no-modules' | 'replug' | 'vendor-app';

export interface VendorApp {
  name: string;
  pid: number;
}

/** The vendor MasterHUB app, when it runs next to masterhubd (`null` when not, or unknown). */
export async function vendorApp(): Promise<VendorApp | null> {
  return (await invoke<VendorApp | null>('vendor_app').catch(() => null)) ?? null;
}

/** The first problem to show, vendor app first (it causes the other two). */
export function hardwareIssue(vendor: VendorApp | null = null): HardwareIssue | undefined {
  if (!hub.online) return undefined;
  if (vendor) return 'vendor-app';
  if (!hub.modules.length) return 'no-modules';
  if (hub.modules.some((m) => m.needs_replug)) return 'replug';
  return undefined;
}

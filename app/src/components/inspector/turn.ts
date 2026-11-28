// The service turns screen images to read upright on the desk; the inspector turns the live
// picture back to show it as the screen sees it.

import { hub } from '../../lib/hub.svelte';
import type { ModuleKind } from '../../lib/types';

export function faceTurn(kind: ModuleKind): number {
  return hub.config.upright === false ? 0 : ((hub.modules.find((m) => m.kind === kind)?.placement?.rotation ?? 0) + (hub.state.rotation ?? 0)) % 360;
}

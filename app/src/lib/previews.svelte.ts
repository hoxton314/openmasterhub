// Key and dial pictures for profiles that are not live: rendered from the config through the
// service's `preview_face` (the same renderer as on the hub, placeholder data for widgets) and
// cached by face. Live profiles show the hub's own pictures (`hub.displays`).

import { invoke } from './backend';
import type { Face } from './types';

/** `JSON(face)` → data URL. */
export const previews = $state<Record<string, string>>({});
const pending = new Set<string>();

/** Only the fields the renderer reads (a key's actions are not part of its picture). */
export function faceOf(f: Face | undefined): Face | undefined {
  if (!f) return undefined;
  const { label, image, background, text_color, widget, fit, label_over_image } = f;
  const face: Face = { label, image, background, text_color, widget, fit, label_over_image };
  for (const k of Object.keys(face) as (keyof Face)[]) if (face[k] === undefined || face[k] === '') delete face[k];
  return Object.keys(face).length ? face : undefined;
}

/**
 * The picture for a face: the cached one, or undefined while it renders (the first read starts
 * it; reading it again in a reactive context updates when it arrives).
 */
export function previewOf(f: Face | undefined): string | undefined {
  const face = faceOf(f);
  if (!face) return undefined;
  const key = JSON.stringify(face);
  const hit = previews[key];
  if (hit !== undefined || pending.has(key)) return hit || undefined;
  pending.add(key);
  invoke<string>('preview_face', { face })
    .then((src) => (previews[key] = src))
    .catch(() => (previews[key] = ''))
    .finally(() => pending.delete(key));
  return undefined;
}

// Mock commands for WP6 · service, hardware and first-run states. Commands here win over the base ones.
// URL switches (dev only), combine freely:
//   ?service=offline|starting|crashed   the service is not reachable (S05); Start/Restart brings it up
//   ?modules=none                        no modules connected (S06)
//   ?replug                              the keypad needs a replug (S06)
//   ?vendor                              the vendor MasterHUB app is running (S06)
//   ?firstrun                            the config has no `app.first_run_done` (S29)

import type { Snapshot } from '../../types';
import type { MockContext, MockHandlers } from '../mock';

const params = typeof location === 'undefined' ? new URLSearchParams() : new URLSearchParams(location.search);
let service = params.get('service');
let vendor = params.has('vendor');

const LOG = [
  '15:31:02 INFO  hid: keypad •••• 3F2A opened',
  '15:31:02 INFO  config: loaded 3 setups, 15 profiles',
  '15:31:07 ERROR audio: PipeWire connection lost',
  '15:31:07 ERROR panicked at crates/masterhubd/src/audio/linux.rs:212',
  'service exited with code 101',
];

const own: MockHandlers = {
  service_status: () => (service === 'crashed' ? { state: 'crashed', reason: 'an audio error' } : service === 'starting' ? { state: 'starting' } : { state: 'stopped' }),
  service_log: () => (service === 'starting' ? LOG.slice(0, 2) : service ? LOG : LOG.slice(0, 2)),
  service_log_dir: () => '/home/you/.local/state/openmasterhub',
  service_autostart: () => true,
  set_service_autostart: () => undefined,
  start_service: (_a, { emit, snapshot }) => bringUp(emit, snapshot),
  restart_service: (_a, { emit, snapshot }) => bringUp(emit, snapshot),
  vendor_app: () => (vendor ? { name: 'MasterHUB.exe', pid: 8812 } : null),
  close_vendor_app: () => {
    vendor = false;
  },
  system_checks: () => [
    { id: 'service', title: 'masterhubd service', detail: 'started with your session', status: 'success', label: 'Running' },
    { id: 'udev', title: 'USB access', detail: 'udev rule 70-masterhub.rules', status: 'success', label: 'Allowed' },
    { id: 'focus', title: 'Focus tracking', detail: 'GNOME needs the Window Calls extension. Hyprland, KDE and Windows work out of the box.', status: 'warning', label: 'Limited', help: 'How to' },
    { id: 'windows', title: 'Temperatures', detail: 'Need LibreHardwareMonitor running. SmartScreen may warn once; choose More info › Run anyway.', status: 'off', label: 'Windows only' },
  ],
  system_info: () => ({ version: '0.9.0' }),
  mcp_program: () => 'masterhubd',
  identify: () => undefined,
  open_path: (a) => console.info('[mock] open_path', a?.path),
  backup_config: () => '/home/you/.config/openmasterhub/backups/config-2026-11-26.toml',
  restore_config: () => undefined,
};

/** Applies the URL switches to the shared snapshot before the app asks for it. */
export function init({ snapshot }: MockContext) {
  if (service) snapshot.online = false;
  if (params.get('modules') === 'none') snapshot.modules = [];
  if (params.has('replug')) for (const m of snapshot.modules) if (m.kind === 'keypad') m.needs_replug = true;
  if (params.has('firstrun') && snapshot.config?.app) delete snapshot.config.app.first_run_done;
}

/** Each `snapshot` request after the first is a reload ("Look again" / "I replugged it"): a replugged module is fine afterwards. */
let snapshots = 0;
export function onSnapshot({ snapshot }: MockContext) {
  snapshots++;
  if (snapshots > 1) for (const m of snapshot.modules) delete m.needs_replug;
}

function bringUp(emit: (event: string, payload: unknown) => void, snapshot: Snapshot) {
  if (!service) return;
  snapshot.online = false;
  service = 'starting';
  setTimeout(() => {
    service = null;
    snapshot.online = true;
    emit('hub:online', true);
  }, 1800);
}

export const handlers = own;

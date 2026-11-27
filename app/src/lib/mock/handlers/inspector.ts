// Mock commands for WP2 · inspector. Commands here win over the base ones in ../mock.ts.
//   run_command_test { cmd, value? } -> { code, ms, output }   (WP9 implements the real one)
//   identify         { serial }                                 (service `Identify`; answered by handlers/system.ts)
// Dev helpers (console / tests): `invoke('dev_fader', { index, value })` moves a fader.

import type { MockHandlers } from '../mock';

export const handlers: MockHandlers = {
  run_command_test: async (args) => {
    const cmd = String(args?.cmd ?? '');
    await new Promise((r) => setTimeout(r, 350));
    if (/^\s*(false|exit\s+[1-9])/.test(cmd)) return { code: 1, ms: 12, output: 'command failed' };
    const out = /\becho\b/.test(cmd) ? cmd.replace(/^.*\becho\s+/, '').replace(/\{value\}/g, String(args?.value ?? '')) : '';
    return { code: 0, ms: 84, output: out };
  },
  dev_fader: (args, { snapshot, emit }) => {
    const fader = snapshot.modules.find((m) => m.kind === 'fader');
    if (fader) emit('hub:input', { serial: fader.serial, control: 'fader', index: Number(args?.index ?? 0), event: 'move', value: Number(args?.value ?? 50) });
  },
};

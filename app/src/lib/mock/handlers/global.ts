// Mock commands for WP6 · global and settings. Commands here win over the base ones in ../mock.ts.
// The global area needs none beyond the base ones (config edits go through `save_config`, audio
// apps through `audio_info`); service, hardware and first-run mocks are in system.ts.

import type { MockHandlers } from '../mock';

export const handlers: MockHandlers = {};

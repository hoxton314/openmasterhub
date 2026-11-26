// Mock commands for WP4 · profiles and templates. Commands here win over the base ones in ../mock.ts.
//
// Contracts the Tauri side (WP7, files.rs) provides:
//   import_toml()            → Profile[] | null   file picker, then the profiles of the chosen TOML (null = cancelled)
//   export_toml({ profile }) → string | null      save dialog, then the path written (null = cancelled)
//   template_save({ template }) → string          writes a user template into `<config dir>/templates/`, returns its path

import type { MockHandlers } from '../mock';

export const handlers: MockHandlers = {
  import_toml: () => [
    {
      name: 'Imported profile',
      apps: ['imported-app'],
      about: 'From a .toml file',
      page: [{ name: 'Main', key: [{ index: 0, label: 'Hello', press: { keys: 'ctrl+h' } }, { index: 1, label: 'World', press: { keys: 'ctrl+w' } }] }],
    },
  ],
  export_toml: (args) => `/home/you/Documents/${String((args?.profile as { name?: string })?.name ?? 'profile').replace(/\s+/g, '-').toLowerCase()}.toml`,
  template_save: (args) => `/home/you/.config/openmasterhub/templates/${String((args?.template as { id?: string })?.id ?? 'template')}.json`,
};

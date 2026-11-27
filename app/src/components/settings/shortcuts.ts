// The app's own keyboard shortcuts, for Settings › Keyboard shortcuts. Keys use the service's
// key syntax (`ctrl+k`), which `KeyCombo` renders as caps.

export interface Shortcut {
  combo: string;
  label: string;
}

export const SHORTCUT_GROUPS: { title: string; items: Shortcut[] }[] = [
  {
    title: 'Everywhere',
    items: [
      { combo: 'ctrl+k', label: 'Search profiles, controls, actions and settings' },
      { combo: 'ctrl+z', label: 'Undo the last change' },
      { combo: 'ctrl+shift+z', label: 'Redo' },
      { combo: 'esc', label: 'Close a dialog or the search, or clear the selection' },
    ],
  },
  {
    title: 'Search',
    items: [
      { combo: 'up', label: 'Move up the results' },
      { combo: 'down', label: 'Move down the results' },
      { combo: 'enter', label: 'Open the highlighted result' },
    ],
  },
  {
    title: 'Editor',
    items: [
      { combo: 'left', label: 'Move to the control on the left (arrow keys)' },
      { combo: 'i', label: 'In the browser preview: simulate a key press' },
    ],
  },
];

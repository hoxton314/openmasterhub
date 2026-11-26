// Key combos in the service's syntax ("ctrl+shift+t", "playpause"; names from
// crates/masterhubd/src/keyboard): recording from KeyboardEvents and pretty names for caps.

const CODE: Record<string, string> = {
  Space: 'space', Enter: 'enter', Tab: 'tab', Backspace: 'backspace', Delete: 'delete', Insert: 'insert',
  Home: 'home', End: 'end', PageUp: 'pageup', PageDown: 'pagedown', ArrowUp: 'up', ArrowDown: 'down',
  ArrowLeft: 'left', ArrowRight: 'right', Minus: 'minus', Equal: 'equal', BracketLeft: 'leftbrace',
  BracketRight: 'rightbrace', Backslash: 'backslash', Semicolon: 'semicolon', Quote: 'apostrophe',
  Backquote: 'grave', Comma: 'comma', Period: 'dot', Slash: 'slash', CapsLock: 'capslock',
  PrintScreen: 'sysrq', ScrollLock: 'scrolllock', Pause: 'pause', ContextMenu: 'menu',
  NumpadEnter: 'kpenter', NumpadAdd: 'kpplus', NumpadSubtract: 'kpminus', NumpadMultiply: 'kpasterisk',
  NumpadDivide: 'kpslash', NumpadDecimal: 'kpdot',
  MediaPlayPause: 'playpause', MediaTrackNext: 'nextsong', MediaTrackPrevious: 'previoussong', MediaStop: 'stopcd',
  AudioVolumeMute: 'mute', AudioVolumeUp: 'volumeup', AudioVolumeDown: 'volumedown',
};

const MODIFIERS: Record<string, string> = { Control: 'ctrl', Shift: 'shift', Alt: 'alt', AltGraph: 'altgr', Meta: 'super', OS: 'super' };

/** The non-modifier key of an event in service syntax, or undefined for a lone modifier. */
export function keyName(e: KeyboardEvent): string | undefined {
  if (e.key in MODIFIERS) return undefined;
  const c = e.code;
  if (/^Key[A-Z]$/.test(c)) return c.slice(3).toLowerCase();
  if (/^Digit\d$/.test(c)) return c.slice(5);
  if (/^F\d{1,2}$/.test(c)) return c.toLowerCase();
  if (/^Numpad\d$/.test(c)) return `kp${c.slice(6)}`;
  return CODE[c] ?? (CODE[e.key] || undefined);
}

/** Modifiers held in an event, in canonical order. */
export function modifiers(e: KeyboardEvent | { ctrlKey: boolean; shiftKey: boolean; altKey: boolean; metaKey: boolean }): string[] {
  return [e.ctrlKey && 'ctrl', e.shiftKey && 'shift', e.altKey && 'alt', e.metaKey && 'super'].filter(Boolean) as string[];
}

/** The full combo of a key event ("ctrl+shift+t"), or undefined while only modifiers are down. */
export function comboOf(e: KeyboardEvent): string | undefined {
  const k = keyName(e);
  return k ? [...modifiers(e), k].join('+') : undefined;
}

const PRETTY: Record<string, string> = {
  ctrl: 'Ctrl', control: 'Ctrl', shift: 'Shift', alt: 'Alt', altgr: 'AltGr', super: 'Super', meta: 'Super', win: 'Win', cmd: 'Cmd',
  leftctrl: 'Ctrl', rightctrl: 'Right Ctrl', leftshift: 'Shift', rightshift: 'Right Shift', leftalt: 'Alt', rightalt: 'AltGr', leftmeta: 'Super',
  space: 'Space', enter: 'Enter', return: 'Enter', tab: 'Tab', backspace: 'Backspace', delete: 'Del', del: 'Del', insert: 'Ins', esc: 'Esc', escape: 'Esc',
  home: 'Home', end: 'End', pageup: 'PgUp', pagedown: 'PgDn', up: '↑', down: '↓', left: '←', right: '→',
  minus: '-', equal: '=', leftbrace: '[', rightbrace: ']', backslash: '\\', semicolon: ';', apostrophe: "'", grave: '`', comma: ',', dot: '.', slash: '/',
  kpplus: 'Numpad +', kpminus: 'Numpad -', kpasterisk: 'Numpad *', kpslash: 'Numpad /', kpdot: 'Numpad .', kpenter: 'Numpad Enter',
  playpause: 'Play/Pause', nextsong: 'Next track', previoussong: 'Previous track', stopcd: 'Stop', mute: 'Mute', volumeup: 'Volume up', volumedown: 'Volume down',
  sysrq: 'PrtSc', print: 'PrtSc',
};

/** "ctrl+shift+t" → ["Ctrl", "Shift", "T"] (a trailing "@700" hold is dropped). */
export function prettyKeys(combo: string): string[] {
  return combo
    .split('@')[0]
    .split('+')
    .map((k) => k.trim().toLowerCase())
    .filter(Boolean)
    .map((k) => PRETTY[k] ?? (/^kp\d$/.test(k) ? `Numpad ${k.slice(2)}` : k.length === 1 ? k.toUpperCase() : k[0].toUpperCase() + k.slice(1)));
}

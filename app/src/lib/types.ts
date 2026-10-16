// Mirrors of the Rust types in masterhub-core (serde JSON shapes).

export type Color = string; // "#rrggbb"

export type Action =
  | { cmd: string }
  | { keys: string }
  | { page: string }
  | { folder: string }
  | { back: boolean }
  | { profile: string }
  | { volume: string } // "+2", "-2", "40" or "{value}"
  | { mute: Mute }
  | { session: number }; // slot of a Claude Code session

export type Mute = 'toggle' | 'on' | 'off';

export type Widget =
  | { kind: 'clock'; format?: string }
  | { kind: 'cpu_usage' }
  | { kind: 'cpu_temp' }
  | { kind: 'ram_usage' }
  | { kind: 'gpu_usage' }
  | { kind: 'gpu_temp' }
  | { kind: 'volume' }
  | { kind: 'now_playing' }
  | { kind: 'claude_session'; slot?: number };

export interface Face {
  label?: string;
  image?: string;
  background?: Color;
  text_color?: Color;
  widget?: Widget;
}

export interface KeyConfig extends Face {
  index: number;
  press?: Action;
  release?: Action;
}

export interface Page {
  name: string;
  key?: KeyConfig[];
}

export interface DialConfig extends Face {
  cw?: Action;
  ccw?: Action;
  press?: Action;
  release?: Action;
}

export interface KnobConfig {
  index: number;
  cw?: Action;
  ccw?: Action;
  press?: Action;
  release?: Action;
}

export interface FaderConfig {
  index: number;
  change?: Action;
}

export type Side = 'left' | 'right';

export interface RollerConfig {
  side: Side;
  away?: Action;
  toward?: Action;
}

export interface Leds {
  base?: Color;
  fader?: Color | Color[];
  knob?: Color | Color[];
  brightness?: number;
}

/** Base LED colours for tool status (Claude Code sessions). */
export interface StatusLeds {
  enabled?: boolean;
  attention?: Color;
  ready?: Color;
  working?: Color;
}

export interface Profile {
  name: string;
  apps?: string[];
  leds?: Leds;
  page?: Page[];
  folder?: Page[];
  dial?: DialConfig;
  fader?: FaderConfig[];
  knob?: KnobConfig[];
  roller?: RollerConfig[];
}

export type Orientation = 'auto' | 'portrait' | 'landscape' | 'portrait_flipped' | 'landscape_flipped';

/** A key shown on every page of every profile except those in `except`. */
export interface PinnedKey extends KeyConfig {
  except?: string[];
}

export interface Config {
  font?: string;
  backlight: number;
  leds?: Leds;
  status?: StatusLeds;
  orientation?: Orientation;
  upright?: boolean;
  pinned?: PinnedKey[];
  profile?: Profile[];
}

export interface Placement {
  col: number;
  row: number;
  cols: number;
  rows: number;
  rotation: number;
}

export type ModuleKind = 'base' | 'keypad' | 'fader' | 'knob' | 'roller' | 'dial';

export interface ModuleState {
  kind: ModuleKind;
  serial: string;
  socket?: string;
  placement?: Placement;
  firmware?: string;
}

export interface HubState {
  profile: string;
  page: string;
  in_folder: boolean;
  horizontal: boolean;
  rotation: number;
  active_app?: string;
}

export interface InputMsg {
  serial: string;
  control: 'key' | 'fader' | 'knob' | 'roller' | 'dial';
  index: number;
  event: string;
  value?: number;
}

export interface Display {
  serial: string;
  target: number;
  src: string;
}

export interface Snapshot {
  online: boolean;
  config?: Config;
  config_path?: string;
  modules: ModuleState[];
  state?: HubState;
  displays: Display[];
}

export type Selection =
  | { kind: 'none' }
  | { kind: 'module'; serial: string }
  | { kind: 'key'; index: number }
  | { kind: 'dial' }
  | { kind: 'knob'; index: number }
  | { kind: 'fader'; index: number }
  | { kind: 'roller'; side: Side };

export const TARGET_DIAL = 255;

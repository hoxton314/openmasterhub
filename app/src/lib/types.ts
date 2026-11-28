// Mirrors of the Rust types in masterhub-core (serde JSON shapes).

export type Color = string; // "#rrggbb"

export type Action =
  | { cmd: string }
  | { keys: string | KeysHold }
  | { page: string }
  | { folder: string }
  | { back: boolean }
  /** A profile name, or "next" | "prev" | "pick" within the active setup (`profile_cycle`). */
  | { profile: string }
  /** Switch setup: a setup name, or "next" | "prev" | "pick" (capability `setup_action`). */
  | { setup: string }
  /** Hybrid setups: pin the live profile or release the pin (capability `pin`). */
  | { pin: 'toggle' | 'release' }
  | { volume: string | ({ level: string } & AudioTarget) } // "+2", "-2", "40" or "{value}"
  | { mute: Mute | ({ state: Mute } & AudioTarget) }
  | { session: number } // slot of a Claude Code session
  | { sessions: 'next' | 'prev' }; // scroll the Claude Code session keys a page

export type Mute = 'toggle' | 'on' | 'off';

/** `keys` as a table: the combo is held for `hold` ms (capability `hold`). */
export interface KeysHold {
  combo: string;
  hold?: number;
}

/** What a volume or mute action controls; none set is the default output. Devices are
 * "default" or a name, apps a program name, groups an `audio_group` name. */
export interface AudioTarget {
  output?: string;
  input?: string;
  app?: string;
  group?: string;
}

export interface AudioGroup {
  name: string;
  apps?: string[];
  steam?: boolean;
}

/** `get_audio`: apps playing audio now and the devices. */
export interface AudioInfo {
  apps: string[];
  outputs: string[];
  inputs: string[];
}

/** `refresh` (seconds) needs capability `widget_refresh`. */
export type Widget = (
  | { kind: 'clock'; format?: string }
  | { kind: 'cpu_usage' }
  | { kind: 'cpu_temp' }
  | { kind: 'ram_usage' }
  | { kind: 'gpu_usage' }
  | { kind: 'gpu_temp' }
  | { kind: 'volume' }
  | { kind: 'now_playing' }
  | { kind: 'claude_session'; slot?: number }
) & { refresh?: number };

export type ImageFit = 'fit' | 'fill' | 'center';

export interface Face {
  label?: string;
  image?: string;
  background?: Color;
  text_color?: Color;
  widget?: Widget;
  /** Image placement (capability `face_fit`). */
  fit?: ImageFit;
  label_over_image?: boolean;
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
  /** Capability `long_press`. */
  long_press?: Action;
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
  /** Knob rings show the volume level of their turn action (capability `knob_volume`). */
  knob_volume?: boolean;
  knob_mute_red?: boolean;
}

/** Base LED colours for tool status (Claude Code sessions). */
export interface StatusLeds {
  enabled?: boolean;
  attention?: Color;
  ready?: Color;
  working?: Color;
  /** Order of the session keys. */
  sort?: 'started' | 'changed' | 'urgent';
  /** Base LED blinks twice on a setup change (default true; capability `blink_on_setup`). */
  blink_on_setup?: boolean;
}

export interface Profile {
  name: string;
  /** The app rule: window classes / exe names with `*` wildcards (shared by every setup). */
  apps?: string[];
  /** Controls this profile does not set come from this profile (capability `base`). */
  base?: string;
  /** Avatar colour (UI only). */
  color?: Color;
  /** Description (templates, cards). */
  about?: string;
  /** Listed under Games; unset = when one of `apps` is a Steam game (`steam_app_*`). */
  game?: boolean;
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
  /** Hidden in every profile of these setups (capability `except_setups`). */
  except_setups?: string[];
}

export type SetupMode = 'follow' | 'manual' | 'hybrid';
export type ReleasePolicy = 'same_trigger' | 'timeout' | 'next_app';

/** `[[setup]]`: profiles in priority (follow) or cycle (manual) order. */
export interface Setup {
  name: string;
  mode?: SetupMode;
  profiles?: string[];
  /** follow/hybrid: used when no rule matches; never in `profiles`. */
  fallback?: string;
  color?: Color;
  /** Global shortcut that switches to this setup. */
  shortcut?: string;
  show_on_dial?: boolean;
  // manual
  wrap?: boolean;
  next?: string;
  prev?: string;
  // hybrid
  pin?: string;
  release?: ReleasePolicy;
  /** Minutes, for `release = "timeout"`. */
  release_after?: number;
}

export type Density = 'compact' | 'comfortable';

export interface Feedback {
  toast?: boolean;
  dial?: boolean;
  led?: boolean;
  in_fullscreen?: boolean;
}

/** `[app]`: GUI-only settings, ignored by the service. */
export interface AppSettings {
  /** A theme id, or "system" (follow the OS between theme_dark and theme_light). */
  theme?: string;
  theme_dark?: string;
  theme_light?: string;
  density?: Density;
  reduce_motion?: 'system' | 'on' | 'off';
  first_run_done?: boolean;
  feedback?: Feedback;
  /** Which switches get the toast: your own (default), every change, or none. */
  overlay?: 'switches' | 'all' | 'off';
  /** `masterhubd mcp` (the MCP server for AI agents) runs only when true. */
  mcp?: boolean;
}

export interface Config {
  app?: AppSettings;
  font?: string;
  backlight: number;
  leds?: Leds;
  status?: StatusLeds;
  orientation?: Orientation;
  upright?: boolean;
  pinned?: PinnedKey[];
  setup?: Setup[];
  profile?: Profile[];
  audio_group?: AudioGroup[];
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
  /** The module ignores software mode until replugged (capability `needs_replug`). */
  needs_replug?: boolean;
}

export interface HubState {
  profile: string;
  page: string;
  in_folder: boolean;
  horizontal: boolean;
  rotation: number;
  active_app?: string;
  /** The active setup (services with capability `setups`). */
  setup?: string;
  /** Why the live profile is live. */
  reason?: Reason;
  /** Hybrid setups: the pinned profile. */
  pinned?: PinState;
}

/** Serialised with `#[serde(tag = "kind", rename_all = "snake_case")]`. */
export type Reason =
  | { kind: 'rule'; pattern: string; index: number }
  | { kind: 'fallback' }
  | { kind: 'manual' }
  | { kind: 'pin' }
  | { kind: 'action' };

export interface PinState {
  profile: string;
  /** Unix seconds. */
  since?: number;
}

/**
 * What the running service implements beyond the 0.8 baseline (`GetCapabilities`). The UI hides
 * or disables controls whose capability is missing. Unknown strings are allowed.
 */
export type Capability =
  | 'setups'
  | 'base'
  | 'hold'
  | 'long_press'
  | 'setup_action'
  | 'profile_cycle'
  | 'pin'
  | 'except_setups'
  | 'blink_on_setup'
  | 'knob_volume'
  | 'face_fit'
  | 'widget_refresh'
  | 'identify'
  | 'needs_replug'
  | 'system_info'
  | 'preview_profile'
  | 'release_claim'
  | 'claim_details';

/** Controls a live-surface client or integration claims. */
export interface Controls {
  keys?: number[];
  dial?: boolean;
  knobs?: number[];
  /** 0 = left, 1 = right. */
  rollers?: number[];
  faders?: number[];
  leds?: string[];
}

/** A claim on the hub (`hub:claim` events / `ClaimState`). */
export interface ClaimInfo {
  claim: number;
  name: string;
  active: boolean;
  owns: Controls;
  key_turn?: number;
  dial_turn?: number;
  // capability `claim_details`
  priority?: number;
  apps?: string[];
  pid?: number;
  program?: string;
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
  /** Missing from older apps/services: treated as empty. */
  capabilities?: string[];
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

/** A game mode `masterhub-games` knows (docs/GAME_MODES.md). */
export interface GameStatus {
  game: string;
  title: string;
  about: string;
  /** Steam app id: a profile following `steam_app_<id>` is the same game. */
  steam: number | null;
  enabled: boolean;
  /** The configured or detected install folder. */
  path: string | null;
  /** Its look's assets were extracted from the user's install (or it needs none). */
  assets: boolean;
}

/** One thing setting up a game does, shown for approval. */
export type GameStep =
  | { kind: 'cet_bridge'; path: string }
  | { kind: 'deadlock_cfg'; path: string }
  | { kind: 'deadlock_icons'; from: string; to: string }
  | { kind: 'wow_addon'; path: string }
  | { kind: 'enable' };

export interface GamePlan {
  game: string;
  title: string;
  path: string | null;
  /** The folder was found through Steam. */
  detected: boolean;
  steps: GameStep[];
  notes: string[];
  error: string | null;
}

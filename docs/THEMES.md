# Themes

The app ships three themes: **Dark**, **Light** and **Signal**. Each is a JSON file, and you can
add your own the same way. The hub drawing stays dark hardware in every theme; a theme changes
the app around it.

## Where theme files live

| | Linux | Windows |
|---|---|---|
| Your themes | `~/.config/openmasterhub/themes/*.json` | `%APPDATA%\openmasterhub\themes\*.json` |
| Bundled themes | `app/src/themes/{dark,light,signal}.json` in this repository | |

Your themes show up in **Settings › Appearance**. A file that can't be used is listed there with
the reason, and is never applied. A theme with the same `id` as a bundled one replaces it.

## A minimal theme

Start from an existing theme with `inherits` and change only what you want:

```json
{
  "$schema": "https://raw.githubusercontent.com/hoxton314/openmasterhub/main/app/src/themes/theme.schema.json",
  "schemaVersion": 1,
  "id": "orchid",
  "name": "Orchid",
  "author": "You",
  "version": "1.0.0",
  "description": "Dark with a pink accent.",
  "appearance": "dark",
  "inherits": "dark",
  "tokens": {
    "color": {
      "accent": "#F07FD0",
      "accentFill": "#C2459F"
    }
  }
}
```

The `$schema` line gives editors such as VS Code completion and checking for every token.
Copy `app/src/themes/theme.schema.json` next to your file and use `"./theme.schema.json"` to
work offline.

## Fields

| Field | Required | Meaning |
|---|---|---|
| `schemaVersion` | yes | Always `1`. |
| `id` | yes | 2–32 characters of `a-z`, `0-9` and `-`. |
| `name` | yes | Shown in Settings. |
| `appearance` | yes | `dark` or `light`: sets the colour scheme of scrollbars and form controls, and which "follow the system" slot the theme fits. |
| `inherits` | no | The id of a theme to start from (`dark`, `light`, `signal` or one of yours). Tokens merge child over parent, at most 4 levels deep. |
| `followsSystem` | no | `false` keeps the theme out of the automatic light/dark switch (Signal does this). Default `true`. |
| `density` | no | `compact` (default) or `comfortable`: the density your sizes are written for. The Density setting adjusts from there. |
| `author`, `version`, `description` | no | Shown in Settings. `version` looks like `1.0.0`. |
| `tokens` | yes | The values, in the groups below. Without `inherits` every token must be set. |

## Token groups

`dark.json` is the complete reference: every token with its value. In short:

| Group | Values | Examples |
|---|---|---|
| `color` | colours | `bg`, `surface`, `raised`, `sunken`, `border`, `border2`, `text1`–`text3`, `accent` (text and rings), `accentFill` (buttons, toggles), `onAccent`, `focus`, `success`, `warning`, `danger`, `selection`, `canvas`, `canvasDot`, `overlay`, `claim`, `rail`, `dialog`. `accentSoft`, `successSoft`, `warningSoft` and `dangerSoft` default to the base colour at 12% opacity. |
| `font` | font family lists | `ui`, `heading`, `mono` |
| `type` | text roles | `display`, `title`, `heading`, `section`, `body`, `small`, `label`, `mono`, `tree`, `chip`, `micro`; each has `family` (`ui`/`heading`/`mono`), `size` (px), `lineHeight`, `weight`, `case` (`none`/`uppercase`), `tracking` (em) and optionally `color` (a colour token name) |
| `radius` | px | `xs`…`xl`, `pill`, and per component: `control`, `segment`, `kbd`, `row`, `sidebarRow`, `chip`, `avatar`, `rail`, `card`, `panel`, `toast`, `popover`, `dialog`, `well` |
| `shadow` | box-shadows | `flat`, `hairline`, `popover`, `dialog`, `segment`, `kbd` |
| `focus` | | `width`, `offset` (px), `color` |
| `space` | px | `1`–`8` (4 px steps up to 40) |
| `size` | px | `rail`, `sidebar`, `inspector`, `topbar`, `control`, `controlSm`, `segment`, `chip`, `rowSidebar`, `rowTree`, `avatar`, `toggleWidth`, `toggleHeight`, `kbd`, `iconButton`, `dialogWidth`, `dialogBorder`, … |
| `motion` | duration + easing | `fast`, `panel`, `toastIn`, `toastOut`, `keyFlash`, `appliedPulse` |
| `style` | switches | `headingCase`, `selectedRow` (`tint`/`invert`), `panelBorderWidth`, `buttonWeight`, `secondaryButton` (`raised`/`surface`/`outline`), `dangerButton` (`outline`/`fill`), `segmentSelected` (`raised`/`fill`), `toggleOff` (`fill`/`outline`), `kbd` (`raised`/`inverse`), `statusChip` (`tint`/`outline`), `activeItem` (`tint`/`fill`), `fieldBorder` (`hairline`/`strong`) |
| `avatar` | colours | `1`–`7`: profile letter tiles (`7` is the fallback grey) |
| `hub` | colours, shadows, px | the device drawing. Bundled themes keep these identical: the hub is the same hardware in every theme. |

Signal shows how far the switches go: square corners (`radius` all `0`), Archivo caps headings
(`font.heading` + `type.*.case`), inverted selected rows and outlined secondary buttons.

## Values

- **Colours:** `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, `hsl()`, `hsla()` or `transparent`.
- **Numbers** are pixels, except `lineHeight`, `weight` and `tracking` (em).
- **References:** `"{group.key}"` uses another token's value after merging, e.g.
  `"rail": "{color.sunken}"` or `"hairline": "0 0 0 1px {color.border}"`.
- **Fonts** must be installed on your computer or bundled with the app (`Inter Variable`,
  `JetBrains Mono Variable`, `Archivo`). Nothing is loaded from the network.

For safety, theme values can't contain `url(`, `var(`, `;`, braces outside references,
backslashes, `@` or `expression`: a theme can recolour the app but can't load anything or inject
styles.

## How it works

`app/src/lib/theme/engine.ts` validates a theme, merges its `inherits` chain, resolves
references, applies the density setting and writes every token as a CSS custom property on
`<html>` (`color.text1` → `--color-text-1`, `type.body.size` → `--type-body-size`,
`radius.control` → `--radius-control`, `hub.key` → `--hub-key`, …), plus a few derived component
variables (`--row-selected-bg`, `--button-secondary-border`, …) from the `style` switches.
Components use only these variables, so switching themes is instant.

To try a theme while developing the app, run `cd app && npm run dev` and open
`http://localhost:5174/?gallery`, which shows every component in Dark, Light and Signal
(`?gallery=dark,orchid` picks the columns). `npm test` checks the bundled themes against the
schema.

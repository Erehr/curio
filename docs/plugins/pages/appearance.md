# Appearance

Curio injects live theme tokens into every plugin page and provides an optional stylesheet (`/curio/ui/curio.css`) and overlay scrollbar helper (`/curio/ui/overlay-scroll.js`) so plain HTML matches Curio's native UI and active theme.

## /curio/ui/curio.js

Include `/curio/ui/curio.js` in every plugin page:

```html
<script src="/curio/ui/curio.js"></script>
```

Even without `curio.css`, `curio.js` performs five setup tasks:

- Sets active theme tokens on `:root` as CSS custom properties (grouped below).
- Sets `color-scheme: dark | light` and toggles `.app-dark`, `.fx-see-through` (Mica/Acrylic) and `.fx-inactive` on `<html>`.
- Keeps `document.documentElement.lang` synced and dispatches `curio-theme` and `curio-language`.
- Sets `data-curio-wheel` on `<html>` to what the wheel does over a picture in Curio's viewer (**Settings › Controls**): `navigate`, `zoom` or `scroll`, so a viewer page can do the same.
- Leaves root `font-size` and scrollbars untouched when you bring your own framework or CSS.

Key CSS custom properties injected on `:root`:

| Category | Variables |
|---|---|
| Surfaces & backgrounds | `--bg`, `--panel`, `--surface`, `--surface-2`, `--surface-3`, `--grid-item`, `--viewer-bg` |
| Borders | `--border`, `--border-subtle`, `--border-strong`, `--float-border`, `--input-border`, `--selection-border` |
| Text | `--text`, `--text-muted`, `--text-faint`, `--grid-caption` |
| Accent & controls | `--accent`, `--accent-text`, `--accent-hover`, `--accent-subtle`, `--accent-border`, `--selection`, `--hover`, `--active`, `--button`, `--button-hover`, `--input`, `--switch-track`, `--switch-track-hover`, `--scrollbar` |
| Status & ratings | `--info`, `--success`, `--warning`, `--danger` (plus `-{subtle,border}` variants for each), `--star` |
| Scale, fonts & materials | `--r`, `--round`, `--g`, `--row`, `--ui-font`, `--mono-font`, `--font-size`, `--fx-panel`, `--fx-side`, `--plugin-*` |

## /curio/ui/curio.css

Link `/curio/ui/curio.css` to opt into Curio's base styles, root `rem` font scaling (`--font-size`), the `.curio-kit` class and overlay scrollbars:

```html
<link rel="stylesheet" href="/curio/ui/curio.css">
<script src="/curio/ui/curio.js"></script>
```

## UI classes

`curio.css` styles semantic HTML elements (`h1`–`h3`, `p`, `a`, `<button>`, `<input>`, `<select>`, `<textarea>`, `<table>`, `<hr>`) and provides utility and layout classes matching Curio's own windows:

| Class | Role |
|---|---|
| `.btn`, `.btn.primary`, `.btn.text`, `.btn.small`, `.icon-btn` | Primary accent, secondary, quiet text, compact and icon buttons |
| `<label class="check">` | Styled checkbox row |
| `.card`, `.field`, `.row`, `.grow`, `.muted`, `.faint`, `.kbd` | Surface cards, inputs, flex rows, subtle text and keycaps |
| `.chip`, `.chip.chip-link` | Tag or value chips (clickable chips match Inspector filter chips) |
| `.curio-page`, `.curio-page-head`, `.curio-page-main`, `.curio-page-side`, `.curio-page-body`, `.curio-page-foot` | Window or tab page layout with header, sidebar, scrolling body and action footer |
| `.curio-section`, `.curio-row`, `.curio-toolbar`, `.curio-sidebar` | Section cards, labeled rows, toolbars and side panels |

## Inspector styles

Additional classes match Curio's Inspector widgets:

- **`.curio-props`**: Key/value metadata rows aligned with Curio's Properties widget.
- **`.curio-progress`**: Compact progress bar (used by Reader's Book and Series widgets).
- **`.curio-select-btn`**: Dropdown trigger button paired with [`ui.choose`](overlays.md#uichoose) so menus render outside the iframe bounds.
- **`.curio-thumb`**: Framed thumbnail tile matching Curio's cover corners.

## Theme tokens

- **Overlay scrollbars**: When `curio.css` is linked and the user has **Overlay scrollbars** enabled, `curio.js` loads `/curio/ui/overlay-scroll.js` and adds `.overlay-scroll` so scrollable areas use Curio's zero-width fade-in scrollbars.
- **Translucent materials**: Use `var(--panel)`, `var(--surface)`, `var(--surface-2)` and `var(--surface-3)` instead of hardcoded hex colors so `.fx-see-through` and `.fx-inactive` apply as expected.
- **Corner radius**: Multiply base radii by `var(--r)` (for example `calc(6px * var(--r))`, where `--r` scales to `0`, `0.5` or `1`) and use `var(--round)` for pills and dots (`0px` when the user chooses square corners).

## curio-theme

Whenever the user switches themes or edits theme colors, `curio.js` updates `:root` variables first and then dispatches a `CustomEvent` on `window`:

```javascript
window.addEventListener('curio-theme', (e) => {
  const { name, dark, tokens } = e.detail
  updateCanvasColors(dark, tokens.bg, tokens.accent)
})
```

## curio-language

Whenever the display language changes in **Settings › General**, `curio.js` updates `document.documentElement.lang` and dispatches:

```javascript
window.addEventListener('curio-language', (e) => {
  loadStrings(e.detail.language)
})
```

## Localization

- **Startup locale**: Check `document.documentElement.lang` (or `(await curio.app.info()).language`) on startup to select your page's active locale.
- **Manifest translations**: Translate manifest strings (`name`, `description`, widget/action/filter/tab/window titles and field labels) by shipping `locales/<lang>.json` beside `plugin.json`. Curio falls back to `en` and then the manifest's literal strings.
- **Host pickers**: Host-rendered pickers (`ui.pickTags`, `ui.pickFolders`, `ui.confirm`, `ui.menu`) follow Curio's active language.

## theme_variables

Plugins can register custom CSS tokens under `theme_variables` in `plugin.json`:

```json
{
  "theme_variables": {
    "accent-soft": "rgba(59, 145, 96, 0.15)",
    "grid-line": "#444444"
  }
}
```

- **Names**: Keys are exposed as **`--plugin-<name>`** (`.` and `_` become `-`, restricted to ASCII letters, digits and `-`, up to 64 characters).
- **Sanitization**: Values accept only letters, digits, spaces and `# % . , + - * / ( )` with balanced parentheses, length up to 200 characters and **no `url(`**. Disallowed values are ignored and logged, and `--safe-mode` ignores plugin theme variables.

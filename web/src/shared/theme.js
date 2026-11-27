// Design tokens → CSS custom properties. Every colour in the app (our components and
// OpenVue's) is derived from these tokens, so a theme file can restyle everything.
// Themes are plain CSS (`:root { --bg: …; }`), importable/exportable; user CSS is injected
// after them and is ignored entirely in safe mode.

export const TOKENS = [
  ['bg', 'Window background'],
  ['panel', 'Sidebars'],
  ['surface', 'Menus, popovers, dialogs'],
  ['surface-2', 'Raised areas'],
  ['surface-3', 'Elevated headers and nested cards'],
  ['border', 'Lines and separators'],
  ['text', 'Primary text'],
  ['text-muted', 'Secondary text'],
  ['text-faint', 'Muted text'],
  ['accent', 'Accent colour'],
  ['info', 'Information notices'],
  ['success', 'Confirmations'],
  ['warning', 'Warnings and offline storage'],
  ['danger', 'Errors and destructive actions'],
  ['star', 'Rating stars'],
]

const hexToRgb = (h) => {
  h = h.replace('#', '')
  if (h.length === 3) h = h.split('').map((c) => c + c).join('')
  return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)]
}
const rgbToHex = (r, g, b) => '#' + [r, g, b].map((v) => Math.round(Math.max(0, Math.min(255, v))).toString(16).padStart(2, '0')).join('')
export const mix = (a, b, t) => {
  const A = hexToRgb(a)
  const B = hexToRgb(b)
  return rgbToHex(A[0] + (B[0] - A[0]) * t, A[1] + (B[1] - A[1]) * t, A[2] + (B[2] - A[2]) * t)
}
export const alpha = (a, t) => {
  const [r, g, b] = hexToRgb(a)
  return `rgba(${r}, ${g}, ${b}, ${t})`
}

/** The translucent colour that, laid over `base`, gives exactly `colour`: the least opaque one that can, so a layer tints what is behind it instead of covering it. */
export function overlayOf(colour, base) {
  const c = hexToRgb(toHex(colour))
  const b = hexToRgb(toHex(base))
  const need = c.map((v, i) => (v > b[i] ? (v - b[i]) / (255 - b[i]) : v < b[i] ? (b[i] - v) / b[i] : 0))
  const a = Math.max(...need)
  if (a < 0.004) return 'transparent'
  const o = b.map((v, i) => Math.round(v + (c[i] - v) / a))
  return `rgba(${o.join(', ')}, ${a.toFixed(3)})`
}

// Layers drawn over a panel, and the token each is drawn over, for see-through windows.
const LAYERS = [['surface-2', 'surface'], ['surface-3', 'surface'], ['hover', 'panel'], ['active', 'panel'], ['button', 'surface-2'], ['button-hover', 'surface-2'], ['input', 'panel'], ['border', 'panel'], ['grid-item', 'bg']]

/** OpenVue's switch tracks as layers, for see-through windows: the preset's toggleswitch tokens read them. */
function trackVars(theme) {
  const t = theme.tokens
  const card = t['surface-2'] || t.surface
  const track = mix(toHex(card), toHex(t.text), theme.dark ? 0.14 : 0.18)
  const trackHover = mix(toHex(card), toHex(t.text), theme.dark ? 0.24 : 0.28)
  return { 'switch-track': overlayOf(track, card), 'switch-track-hover': overlayOf(trackHover, card) }
}

/** Dark or light text, whichever can be read on this colour. */
const luminance = (hex) => {
  const [r, g, b] = hexToRgb(hex).map((v) => v / 255).map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4))
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}
/** White on a colour unless it would fall under 3:1 (WCAG's floor for buttons and large text), dark otherwise. */
export function readableOn(colour) {
  return 1.05 / (luminance(toHex(colour)) + 0.05) >= 3 ? '#ffffff' : '#0b1114'
}

/** The tokens worked out from a theme's own colours: hovers, selections, fields, buttons. */
export function derived(t, dark) {
  const surface2 = toHex(t['surface-2'] || t.surface)
  const surface3 = t['surface-3'] ? toHex(t['surface-3']) : mix(surface2, toHex(t.text), dark ? 0.06 : 0.05)
  const info = t.info ? toHex(t.info) : dark ? '#5b9cf6' : '#2563eb'
  const sFill = dark ? 0.14 : 0.11
  const sBorder = dark ? 0.38 : 0.32
  return {
    'surface-3': surface3,
    info,
    'accent-text': readableOn(t.accent),
    'accent-hover': mix(toHex(t.accent), dark ? '#ffffff' : '#000000', dark ? 0.12 : 0.1),
    'accent-subtle': alpha(toHex(t.accent), dark ? 0.16 : 0.13),
    'accent-border': alpha(toHex(t.accent), dark ? 0.42 : 0.36),
    'border-subtle': alpha(toHex(t.text), dark ? 0.08 : 0.09),
    'border-strong': alpha(toHex(t.text), dark ? 0.17 : 0.18),
    'info-subtle': alpha(info, sFill),
    'info-border': alpha(info, sBorder),
    'success-subtle': alpha(toHex(t.success), sFill),
    'success-border': alpha(toHex(t.success), sBorder),
    'warning-subtle': alpha(toHex(t.warning), sFill),
    'warning-border': alpha(toHex(t.warning), sBorder),
    'danger-subtle': alpha(toHex(t.danger), sFill),
    'danger-border': alpha(toHex(t.danger), sBorder),
    'switch-track': mix(surface2, toHex(t.text), dark ? 0.14 : 0.18),
    'switch-track-hover': mix(surface2, toHex(t.text), dark ? 0.24 : 0.28),
    selection: alpha(toHex(t.accent), dark ? 0.34 : 0.24),
    'selection-border': t.accent,
    hover: mix(toHex(t.panel), toHex(t.text), dark ? 0.07 : 0.06),
    active: mix(toHex(t.panel), toHex(t.text), dark ? 0.13 : 0.11),
    button: mix(surface2, toHex(t.text), dark ? 0.1 : 0.06),
    'button-hover': mix(surface2, toHex(t.text), dark ? 0.17 : 0.11),
    // fields use --surface so they sit lighter than dark sidebars/panels and darker than raised --surface-2 cards, and white in light themes
    input: t.surface,
    'input-border': alpha(toHex(t.text), dark ? 0.14 : 0.16),
    // the edge of a menu, popover, list, dialog or message: Curio draws no shadows, so the edge alone sets a layer off what is under it
    'float-border': mix(toHex(t.surface), toHex(t.text), 0.18),
    scrollbar: mix(toHex(t.bg), toHex(t.text), dark ? 0.24 : 0.3),
    'grid-item': mix(toHex(t.bg), toHex(t.text), 0.05),
    'grid-caption': t['text-muted'],
  }
}

function scheme(name, dark, c, family = name) {
  const { bg, panel, surface, surface2, surface3, border, text, muted, faint, accent, info, danger, warning, success, star } = c
  return { name, family, dark, tokens: { bg, panel, surface, 'surface-2': surface2, 'surface-3': surface3, border, text, 'text-muted': muted, 'text-faint': faint, accent, info, danger, warning, success, star } }
}

// Every stock theme comes in a dark and a light variant under one family name; text, text-muted and text-faint meet WCAG AA contrast across bg, panel, surface and surface-2.
export const BUILTIN = {
  Dark: scheme('Dark', true, { bg: '#141518', panel: '#191a1d', surface: '#222327', surface2: '#2a2b30', surface3: '#32343a', border: '#2b2c31', text: '#eceef0', muted: '#a7a9ae', faint: '#90939b', accent: '#3b9160', info: '#5b9cf6', danger: '#ff6b6b', warning: '#f5a524', success: '#3ecf8e', star: '#ffc83d' }, 'Default'),
  Light: scheme('Light', false, { bg: '#f6f9fa', panel: '#ebf0f2', surface: '#ffffff', surface2: '#f1f5f6', surface3: '#e4ecef', border: '#d5dee2', text: '#1e345d', muted: '#475b70', faint: '#586b7f', accent: '#3b9160', info: '#2563eb', danger: '#d64545', warning: '#b86b00', success: '#24875d', star: '#d99a00' }, 'Default'),
  // nordtheme.com: Polar Night nord0-3, Snow Storm nord4-6, Frost nord7-10, Aurora nord11-15
  Nord: scheme('Nord', true, { bg: '#262b36', panel: '#2b303b', surface: '#2e3440', surface2: '#3b4252', surface3: '#434c5e', border: '#3b4252', text: '#eceff4', muted: '#d8dee9', faint: '#a7b2c6', accent: '#88c0d0', info: '#81a1c1', danger: '#bf616a', warning: '#d08770', success: '#a3be8c', star: '#ebcb8b' }),
  'Nord Light': scheme('Nord Light', false, { bg: '#eceff4', panel: '#e2e7f0', surface: '#f5f7fa', surface2: '#e5e9f0', surface3: '#d8dee9', border: '#cfd6e3', text: '#2e3440', muted: '#434c5e', faint: '#525d73', accent: '#5e81ac', info: '#4c749e', danger: '#bf616a', warning: '#b8684d', success: '#63834f', star: '#c0892b' }, 'Nord'),
  // catppuccin/palette: Mocha and Latte
  Catppuccin: scheme('Catppuccin', true, { bg: '#1e1e2e', panel: '#181825', surface: '#262738', surface2: '#313244', surface3: '#3c3e52', border: '#45475a', text: '#cdd6f4', muted: '#bac2de', faint: '#9ba2bd', accent: '#cba6f7', info: '#89b4fa', danger: '#f38ba8', warning: '#fab387', success: '#a6e3a1', star: '#f9e2af' }),
  'Catppuccin Latte': scheme('Catppuccin Latte', false, { bg: '#eff1f5', panel: '#e6e9ef', surface: '#f7f8fc', surface2: '#e2e5ee', surface3: '#d5d9e4', border: '#ccd0da', text: '#4c4f69', muted: '#565973', faint: '#5e627d', accent: '#8839ef', info: '#1e66f5', danger: '#d20f39', warning: '#d95607', success: '#3a9326', star: '#df8e1d' }, 'Catppuccin'),
  // folke/tokyonight.nvim: Night and Day
  'Tokyo Night': scheme('Tokyo Night', true, { bg: '#1a1b26', panel: '#16161e', surface: '#24283b', surface2: '#2f3549', surface3: '#394058', border: '#292e42', text: '#c0caf5', muted: '#a9b1d6', faint: '#959ec7', accent: '#7aa2f7', info: '#7dcfff', danger: '#f7768e', warning: '#ff9e64', success: '#9ece6a', star: '#e0af68' }),
  'Tokyo Night Day': scheme('Tokyo Night Day', false, { bg: '#e1e2e7', panel: '#d3d8e6', surface: '#edf0f5', surface2: '#d9dde9', surface3: '#cdd2e2', border: '#c4c8da', text: '#2b4b99', muted: '#3c5496', faint: '#44588f', accent: '#2e7de9', info: '#007197', danger: '#d9235a', warning: '#b15c00', success: '#4d6b2e', star: '#8c6c3e' }, 'Tokyo Night'),
  // morhetz/gruvbox: dark0/dark0_hard with bright accents, light0/light0_soft with muted ones
  Gruvbox: scheme('Gruvbox', true, { bg: '#282828', panel: '#1d2021', surface: '#32302f', surface2: '#3c3836', surface3: '#504945', border: '#504945', text: '#ebdbb2', muted: '#d5c4a1', faint: '#b2a38a', accent: '#fe8019', info: '#83a598', danger: '#fb4934', warning: '#fabd2f', success: '#b8bb26', star: '#fabd2f' }),
  'Gruvbox Light': scheme('Gruvbox Light', false, { bg: '#fbf1c7', panel: '#f2e5bc', surface: '#f9f5d7', surface2: '#ebdbb2', surface3: '#e0cfa4', border: '#d5c4a1', text: '#3c3836', muted: '#564c45', faint: '#635850', accent: '#af3a03', info: '#076678', danger: '#9d0006', warning: '#b57614', success: '#6f6b0c', star: '#c78b18' }, 'Gruvbox'),
  // sainnhe/everforest: dark medium and muted botanical sage-linen light
  Everforest: scheme('Everforest', true, { bg: '#2d353b', panel: '#232a2e', surface: '#343f44', surface2: '#3d484d', surface3: '#475258', border: '#475258', text: '#d3c6aa', muted: '#b3b5a4', faint: '#aeb5a4', accent: '#a7c080', info: '#7fbbb3', danger: '#e67e80', warning: '#e69875', success: '#83c092', star: '#dbbc7f' }),
  'Everforest Light': scheme('Everforest Light', false, { bg: '#f2efe0', panel: '#e6e5d3', surface: '#f8f6eb', surface2: '#e9e8d7', surface3: '#dddecb', border: '#ced0bc', text: '#425058', muted: '#516065', faint: '#596869', accent: '#778a00', info: '#3a94c5', danger: '#d94846', warning: '#d96b1c', success: '#2d946d', star: '#c88d00' }, 'Everforest'),
  // altercation/solarized: base03-base3 and the eight accents
  Solarized: scheme('Solarized', true, { bg: '#002b36', panel: '#03303c', surface: '#073642', surface2: '#0d4250', surface3: '#154f5e', border: '#144c5a', text: '#b5c2c2', muted: '#9aa8a8', faint: '#97abac', accent: '#268bd2', info: '#2aa198', danger: '#dc322f', warning: '#cb4b16', success: '#859900', star: '#b58900' }),
  'Solarized Light': scheme('Solarized Light', false, { bg: '#fdf6e3', panel: '#eee8d5', surface: '#fffbf0', surface2: '#f3ecd9', surface3: '#e6dfc9', border: '#dcd4be', text: '#455a61', muted: '#536970', faint: '#52686f', accent: '#268bd2', info: '#2aa198', danger: '#dc322f', warning: '#cb4b16', success: '#738500', star: '#b58900' }, 'Solarized'),
}

/** A theme's name as its tile shows it: the family of a stock theme, the name of one of the person's own. */
export const themeLabel = (name, theme) => theme?.family || name

/** The colour a colour picker can show for a token, which may hold transparency. */
export function toHex(v) {
  const t = String(v || '').trim()
  if (/^#[0-9a-f]{6}$/i.test(t)) return t
  if (/^#[0-9a-f]{3}$/i.test(t)) return rgbToHex(...hexToRgb(t))
  const m = t.match(/rgba?\(\s*(\d+)[\s,]+(\d+)[\s,]+(\d+)/i)
  return m ? rgbToHex(Number(m[1]), Number(m[2]), Number(m[3])) : '#000000'
}

export function toCss(theme) {
  let s = `/* theme: ${theme.name} */\n:root {\n  --mode: ${theme.dark ? 'dark' : 'light'};\n`
  for (const [k, d] of TOKENS) if (theme.tokens[k]) s += `  --${k}: ${theme.tokens[k]}; /* ${d} */\n`
  const worked = Object.keys(derived(theme.tokens, !!theme.dark))
  for (const [k, v] of Object.entries(theme.tokens)) if (!TOKENS.some(([t]) => t === k) && !worked.includes(k)) s += `  --${k}: ${v};\n`
  return s + '}\n'
}

export function fromCss(css) {
  const isDark = !/--mode:\s*light/.test(css)
  const t = structuredClone(isDark ? BUILTIN.Dark : BUILTIN.Light)
  const nameMatch = css.match(/\/\*\s*theme:\s*([^*]+)\*\//)
  t.name = nameMatch ? nameMatch[1].trim() : 'Imported'
  const body = css.replace(/\/\*[\s\S]*?\*\//g, '')
  for (const m of body.matchAll(/--([a-z0-9-]+)\s*:\s*([^;}]+)/gi)) {
    if (m[1] === 'mode') continue
    t.tokens[m[1]] = m[2].trim()
  }
  return t
}

// the app checks plugin variables already; this keeps a bad one from breaking out of the rule
const PLUGIN_VAR = /^plugin-[a-z0-9-]{1,64}$/
const pluginVars = (vars) => Object.fromEntries(Object.entries(vars || {}).filter(([k, v]) => PLUGIN_VAR.test(k) && typeof v === 'string' && !/[;{}<>\\"'@]/.test(v)))

export function applyTheme(theme, { userCss = '', safeMode = false, extraVars = {}, seeThrough = false } = {}) {
  const tokens = { ...(safeMode ? {} : pluginVars(extraVars)), ...derived(theme.tokens, !!theme.dark), ...theme.tokens, ...(seeThrough ? trackVars(theme) : {}) }
  // every layer is worked out from its base's solid colour, so the bases are converted last
  if (seeThrough) for (const [k, base] of LAYERS) if (tokens[k] && theme.tokens[base]) tokens[k] = overlayOf(theme.tokens[k] ?? tokens[k], theme.tokens[base])
  let vars = ':root {\n'
  for (const [k, v] of Object.entries(tokens)) vars += `  --${k}: ${v};\n`
  // an open item sits on the grid's own background, see-through windows included
  vars += '  --viewer-bg: var(--bg);\n'
  vars += '}\n'
  document.getElementById('theme-vars').textContent = vars
  document.documentElement.classList.toggle('app-dark', !!theme.dark)
  document.documentElement.style.colorScheme = theme.dark ? 'dark' : 'light'
  document.getElementById('user-css').textContent = safeMode ? '' : userCss || ''
  return { ...tokens, 'viewer-bg': tokens.bg }
}

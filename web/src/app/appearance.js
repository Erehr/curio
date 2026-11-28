// The theme and look applied to the window: tokens, corners, spacing, fonts and see-through panels.
import { call } from '../platform/ipc.js'
import { fromCss, BUILTIN, applyTheme } from '../shared/theme.js'
import { applyWindowLook } from '../platform/windowFrame.js'
import { remPx } from '../shared/rem.js'
import { S, themeShown } from './state.js'

const SCALES = ['r', 'round', 'g', 'row', 'ui-font', 'mono-font', 'fx-panel', 'fx-side']

export function currentTheme() {
  const dark = S.ui.themeMode === 'dark' || (S.ui.themeMode === 'system' && S.systemDark)
  const name = dark ? S.ui.darkTheme : S.ui.lightTheme
  return S.ui.customThemes[name] && !S.startup.safeMode ? fromCss(S.ui.customThemes[name]) : structuredClone(BUILTIN[name] || BUILTIN[dark ? 'Dark' : 'Light'])
}

let themeFileCss = ''
/** A theme file kept up to date by another program: read now, and again on coming back to the window. */
export async function loadThemeFile() {
  const css = S.ui.themeFile ? await call('read_theme_file').catch(() => '') : ''
  if (css === themeFileCss) return
  themeFileCss = css
  refreshTheme()
}

const CORNERS = { none: 0, small: 0.5 }
// what is otherwise fully round (switches, radio buttons, handles, dots) is square at None and only just rounded at Small
const ROUND = { none: '0px', small: '4px' }
const SPACING = { compact: 0.75, expanded: 1.25 }
const ROW_SCALE = { compact: 0.9, expanded: 1.1 }
/** How much the chosen spacing scales the gaps of the window. */
export const spacingScale = () => SPACING[S.ui.spacing] ?? 1

// the size every rem in the window is measured from: list and menu text as macOS draws it, a pixel above Windows' 12px menu font
const BASE_FONT_PX = 13

const SYSTEM_FONT = "system-ui, 'Segoe UI', 'Noto Sans', 'DejaVu Sans'"

/** A material shows behind the window's panels. */
export const seeThrough = () => S.ui.windowMaterial !== 'off' && (S.startup.materials || []).includes(S.ui.windowMaterial)

export function refreshTheme() {
  const t = currentTheme()
  const tokens = applyTheme(t, { userCss: `${S.ui.userCss || ''}\n${themeFileCss}`, safeMode: !!S.startup.safeMode, extraVars: S.contrib.themeVariables || {}, seeThrough: seeThrough() })
  const root = document.documentElement
  root.style.fontSize = `${BASE_FONT_PX}px`
  remPx.value = BASE_FONT_PX
  root.style.setProperty('--r', CORNERS[S.ui.corners] ?? 1)
  root.style.setProperty('--round', ROUND[S.ui.corners] ?? '999px')
  root.style.setProperty('--g', spacingScale())
  root.style.setProperty('--row', ROW_SCALE[S.ui.spacing] ?? 1)
  const chosen = String(S.ui.font || '').replace(/["\\;{}]/g, '').trim()
  root.style.setProperty('--ui-font', `${chosen && !S.ui.fontSystem ? `"${chosen}", ` : ''}${SYSTEM_FONT}`)
  document.documentElement.classList.toggle('no-side-counts', S.ui.sidebarCounts === false)
  document.documentElement.classList.toggle('overlay-scroll', S.ui.overlayScrollbars !== false)
  // following the system leaves the window's light or dark to it (null); a mode chosen here is kept on it, through the system's own changes
  applyWindowLook(S.ui, { dark: S.ui.themeMode === 'system' ? null : !!t.dark, materials: S.startup.materials || [] })
  const css = getComputedStyle(root)
  for (const k of SCALES) tokens[k] = root.style.getPropertyValue('--' + k).trim() || css.getPropertyValue('--' + k).trim()
  tokens['font-size'] = `${BASE_FONT_PX}px`
  themeShown.theme = { name: t.name, dark: !!t.dark, tokens }
  themeShown.seeThrough = seeThrough()
  S.themeVer++
  if (!S.itemWindow && !S.prefsWindow && !S.pluginWindow && !S.toolWindow) call('theme_shown', { theme: themeShown.theme }).catch(() => {})
}

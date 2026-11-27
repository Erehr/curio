// Settings (`S.ui`): saved, passed between the windows, kept per library where asked, and opened.
import { toRaw } from 'vue'
import { call, errorText } from '../platform/ipc.js'
import { BUILTIN } from '../shared/theme.js'
import { isBuiltinLayout, migrateWorkspaces, renameLayouts, LAYOUT_NAMES_V, LAYOUT_V, defaultArrangement, DEFAULT_FOR, defaultGrid } from '../layout/layouts.js'
import { DEFAULT_KEYS } from './shortcuts.js'
import { S, DEFAULT_UI } from './state.js'
import { toast } from './notify.js'
import { refreshTheme } from './appearance.js'
import { refreshSidebarSoon } from '../sidebar/sidebar.js'

export function openPrefs(tab = null) {
  if (S.prefsWindow) {
    if (tab) S.dialogs.prefsTab = tab
    S.dialogs.prefs = true
    return
  }
  flushUi()
  call('open_prefs_window', { tab: tab || S.dialogs.prefsTab }).catch((e) => toast(errorText(e), 'error'))
}

// ------------------------------------------------------------------ settings + theme

const clone = (v) => JSON.parse(JSON.stringify(v))
const isObj = (v) => !!v && typeof v === 'object' && !Array.isArray(v)
// keys in one order, so two copies of a setting compare by what they hold
const sortedKeys = (v) => (Array.isArray(v) ? v.map(sortedKeys) : v && typeof v === 'object' ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, sortedKeys(v[k])])) : v)
const sameValue = (a, b) => JSON.stringify(sortedKeys(a)) === JSON.stringify(sortedKeys(b))
// nested objects patched per key so unknown entries or values from newer builds stay untouched
const PATCH_OBJECTS = new Set(['keys', 'layouts', 'layoutFor', 'grid', 'picker', 'textView', 'notifySoundEvents', 'searchScope'])
function patchObject(stored, base, current, depth = 0, recurseChildren = false) {
  const out = isObj(stored) ? clone(stored) : {}
  const keys = new Set([...Object.keys(base || {}), ...Object.keys(current || {})])
  for (const k of keys) {
    if (k in current) {
      if (base && k in base && sameValue(current[k], base[k])) continue
      const recurse = depth === 0 ? PATCH_OBJECTS.has(k) : recurseChildren
      if (recurse && isObj(out[k]) && isObj(base?.[k]) && isObj(current[k])) {
        out[k] = patchObject(out[k], base[k], current[k], depth + 1, k === 'layouts' || k === 'grid')
      } else {
        out[k] = clone(current[k])
      }
    } else {
      delete out[k]
    }
  }
  return out
}

// Settings are shared by this program's windows: each change is passed on right away, and kept on
// disk a moment later. Stored copies stay separate from the coerced live `S.ui` so unknown keys and
// values from newer builds survive a save unless explicitly replaced.
export const WINDOW_ID = Math.random().toString(36).slice(2)
const SYNC_MS = 30
const SAVE_MS = 600
let saveTimer = null
let syncTimer = null
let storedUi = null
let usedBase = null
let storedLibLayout = null
let usedLibBase = null

const liveSharedOf = (ui) => {
  const { sharedLayout, ...rest } = ui
  return sharedLayout ? { ...rest, ...sharedLayout } : rest
}
const buildStoredUi = (ui) => {
  const live = liveSharedOf(ui)
  return storedUi && usedBase ? patchObject(storedUi, usedBase, live) : clone(live)
}
const buildStoredLib = (ui) => {
  const live = libraryUi(ui)
  return storedLibLayout && usedLibBase ? patchObject(storedLibLayout, usedLibBase, live) : clone(live)
}

export function loadUi(raw) {
  storedUi = isObj(raw) ? clone(raw) : null
  const merged = mergeUi(raw || {})
  usedBase = clone(liveSharedOf(merged))
  return merged
}

export function receiveUi(json) {
  const parsed = JSON.parse(json)
  const ui = isObj(parsed?.ui) ? parsed.ui : parsed
  const merged = mergeUi(ui)
  if (isObj(parsed?.stored)) {
    storedUi = clone(parsed.stored)
    usedBase = clone(liveSharedOf(merged))
  }
  if (isObj(parsed?.libStored)) {
    storedLibLayout = clone(parsed.libStored)
    usedLibBase = libraryUi(merged)
  }
  return merged
}

export function sendUi() {
  syncTimer = null
  const raw = toRaw(S.ui)
  const payload = { ui: raw, stored: buildStoredUi(raw), libStored: raw.sharedLayout && S.libOpen ? buildStoredLib(raw) : null }
  call('ui_sync', { from: WINDOW_ID, json: JSON.stringify(payload) }).catch(() => {})
}
export function saveUi() {
  clearTimeout(saveTimer)
  saveTimer = setTimeout(writeUi, SAVE_MS)
  if (!syncTimer) syncTimer = setTimeout(sendUi, SYNC_MS)
}

// what Settings › Layout can keep for each library (library_json `layout`); while it does, S.ui.sharedLayout holds every library's own
const LIBRARY_UI = ['layouts', 'layoutFor', 'grid', 'pinnedFilters']
const libraryUiJson = (ui) => JSON.stringify(Object.fromEntries(LIBRARY_UI.map((k) => [k, ui[k]])))
const libraryUi = (ui) => JSON.parse(libraryUiJson(ui))
let libraryUiSaved = ''

/** The settings file gets the shared layout, and the library its own, while each library keeps one. */
function writeUi() {
  clearTimeout(saveTimer)
  saveTimer = null
  const raw = toRaw(S.ui)
  const nextStored = buildStoredUi(raw)
  storedUi = nextStored
  usedBase = clone(liveSharedOf(raw))
  const jobs = [call('ui_settings_save', { json: JSON.stringify(nextStored) })]
  if (raw.sharedLayout && S.libOpen) {
    const nextLib = buildStoredLib(raw)
    const json = JSON.stringify(nextLib)
    if (json !== libraryUiSaved) {
      libraryUiSaved = json
      storedLibLayout = nextLib
      usedLibBase = libraryUi(raw)
      jobs.push(call('library_json', { name: 'layout', set: nextLib }))
    }
  }
  return Promise.all(jobs.map((p) => p.catch(() => {})))
}

/** These settings with the open library's own layout in the shared one's place, while each library keeps one; one it hasn't got yet starts as the shared one. */
export async function withLibraryUi(ui) {
  if (!ui.layoutPerLibrary || !S.libOpen) {
    storedLibLayout = null
    usedLibBase = null
    return ui
  }
  const sharedLayout = ui.sharedLayout || libraryUi(ui)
  const own = await call('library_json', { name: 'layout', set: null }).catch(() => null)
  const out = mergeUi({ ...ui, ...libraryUi(sharedLayout), ...(own || {}), sharedLayout })
  storedLibLayout = isObj(own) ? clone(own) : null
  usedLibBase = libraryUi(out)
  libraryUiSaved = own ? JSON.stringify(own) : ''
  return out
}

// the shape of this window's own main window, never another's
const OWN_UI = ['aspect']

/** The settings another library's window saved since this program last read or wrote them, taken when this window comes to the front: only those that changed, so its next save doesn't put the old ones back and changes made here stay. */
export async function pickUpUi() {
  if (!S.ready || saveTimer) return
  const got = await call('ui_settings_changed').catch(() => null)
  if (!got) return
  let file, base
  try {
    file = JSON.parse(got.text)
    base = got.base ? JSON.parse(got.base) : {}
  } catch {
    return
  }
  const changed = [...new Set([...Object.keys(file), ...Object.keys(base)])].filter((k) => !OWN_UI.includes(k) && !sameValue(file[k], base[k]))
  if (!changed.length) return
  const next = { ...toRaw(S.ui) }
  // the switch for a layout in each library, turned on or off there: the file holds the layout every library shares
  const flipped = changed.includes('layoutPerLibrary') && !!file.layoutPerLibrary !== !!next.layoutPerLibrary
  if (flipped && file.layoutPerLibrary) next.sharedLayout = libraryUi(file)
  else if (flipped) {
    delete next.sharedLayout
    for (const k of LIBRARY_UI) next[k] = file[k]
  }
  for (const k of changed) {
    if (LIBRARY_UI.includes(k) && next.sharedLayout) next.sharedLayout = { ...next.sharedLayout, [k]: file[k] }
    else if (k in file) next[k] = file[k]
    else delete next[k]
  }
  const ui = flipped && file.layoutPerLibrary ? await withLibraryUi(mergeUi(next)) : mergeUi(next)
  // a change made here meanwhile wins
  if (saveTimer) return
  const own = Object.fromEntries(OWN_UI.filter((k) => storedUi && k in storedUi).map((k) => [k, storedUi[k]]))
  storedUi = { ...clone(file), ...own }
  usedBase = clone(liveSharedOf(ui))
  S.ui = ui
  sendUi()
  refreshTheme()
  refreshSidebarSoon(250)
}

/** Settings › Layout: a layout for each library, or one for all. What is on show stays, as this library's own or as everyone's. */
export function setLayoutPerLibrary(on) {
  if (on && S.libOpen) S.ui.sharedLayout = libraryUi(S.ui)
  else delete S.ui.sharedLayout
  S.ui.layoutPerLibrary = on
  saveUi()
}
// the settings as another window last sent them, so passing them on again can be skipped
let receivedUi = ''
export const uiFromElsewhere = () => JSON.stringify(S.ui) === receivedUi
/** The settings on show came from another window. */
export function markReceived() {
  receivedUi = JSON.stringify(S.ui)
}

/** Every setting back to its default; themes the person made and the recently used lists stay. */
export function resetSettings() {
  const own = Object.fromEntries(Object.entries(S.ui.layouts).filter(([n]) => !isBuiltinLayout(n)))
  const keep = { customThemes: S.ui.customThemes, recents: S.ui.recents, aspect: S.ui.aspect, layouts: own }
  S.ui = { ...mergeUi({}), ...keep }
  saveUi()
  refreshTheme()
}
/** Saves the settings now: before another window reads them, or another library is opened here. */
export function flushUi() {
  return writeUi()
}
export function mergeUi(ui) {
  const def = DEFAULT_UI()
  const out = { ...def, ...ui, picker: { ...def.picker, ...(ui.picker || {}) }, textView: { ...def.textView, ...(ui.textView || {}) }, notifySoundEvents: { ...def.notifySoundEvents, ...(ui.notifySoundEvents || {}) } }
  // themes that no longer come with the program
  const known = (n) => BUILTIN[n] || out.customThemes?.[n]
  if (!known(out.darkTheme)) out.darkTheme = 'Dark'
  if (!known(out.lightTheme)) out.lightTheme = 'Light'
  if (!['seek', 'volume', 'navigate'].includes(out.videoWheel)) out.videoWheel = def.videoWheel
  // shortcuts of actions this version doesn't have; plugins' stay, for a plugin that is off or not yet loaded
  out.keys = Object.fromEntries(Object.entries(out.keys || {}).filter(([id]) => id in DEFAULT_KEYS || id.startsWith('plugin:')))
  // workspaces of 0.8.11 and before: an arrangement and the view together, with a vertical copy
  if (ui.workspaces && !ui.layouts) Object.assign(out, migrateWorkspaces(ui.workspaces, ui.workspace))
  // layouts saved before names were versioned (0.8.12 to 0.8.14) are of version 1
  Object.assign(out, renameLayouts(out.layouts, out.layoutFor, ui.layouts ? ui.layoutNamesV || 1 : LAYOUT_NAMES_V))
  out.layoutNamesV = Math.max(ui.layoutNamesV || 0, LAYOUT_NAMES_V)
  delete out.workspaces
  delete out.workspace
  out.layouts = Object.fromEntries(Object.entries(out.layouts || {}).filter(([, l]) => l?.zonesV === LAYOUT_V).map(([n, l]) => [n, knownModes({ ...defaultArrangement(), ...l })]))
  out.layoutFor = { ...DEFAULT_FOR, ...out.layoutFor }
  for (const slot of ['horizontal', 'vertical']) if (!out.layouts[out.layoutFor[slot]] && !isBuiltinLayout(out.layoutFor[slot])) out.layoutFor[slot] = DEFAULT_FOR[slot]
  out.grid = { ...defaultGrid(), ...out.grid }
  out.grid.group = { ...defaultGrid().group, ...(out.grid.group || {}) }
  if (!VIEWS.includes(out.grid.view)) out.grid.view = defaultGrid().view
  return out
}

const VIEWS = ['grid', 'justified', 'masonry', 'list', 'spectrum']
const DOCK_LAYOUTS = ['column', 'grid', 'masonry', 'row']
/** A mode this version doesn't have (a renamed one) would leave the grid empty. */
function knownModes(l) {
  if (!DOCK_LAYOUTS.includes(l.dockLayout)) l.dockLayout = defaultArrangement().dockLayout
  return l
}

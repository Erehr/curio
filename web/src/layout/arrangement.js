// Which layout the window shows: its screen, the zones of that screen's arrangement, and the folder view's own.
import { computed, toRaw } from 'vue'
import { isBuiltinLayout, DEFAULT_FOR, LAYOUT_V, BUILTIN_LAYOUTS, defaultArrangement, isArrangeKey } from './layouts.js'
import { screenKey, PIN_BOTTOM, pluginToolbar, screenDefaults, QUICK, PIN_TOP, defaultZones } from './widgets.js'
import { kindOf } from '../formats/index.js'
import { S } from '../app/state.js'
import { viewerFile } from '../viewer/session.js'
import { saveUi } from '../app/preferences.js'

/** The layout chosen for a window shape, stored the first time it is used so everyday changes have somewhere to go. */
function arrangement(aspect) {
  const slot = aspect === 'vertical' ? 'vertical' : 'horizontal'
  let name = S.ui.layoutFor[slot]
  if (!S.ui.layouts[name] && !isBuiltinLayout(name)) name = S.ui.layoutFor[slot] = DEFAULT_FOR[slot]
  if (S.ui.layouts[name]?.zonesV !== LAYOUT_V) S.ui.layouts[name] = isBuiltinLayout(name) ? BUILTIN_LAYOUTS[name]() : defaultArrangement()
  return S.ui.layouts[name]
}

/** The panel settings of one window shape. */
export const panelsFor = (aspect) => arrangement(aspect)

/** Which cards show their format badge: all, unsupported or none (once a plain on or off). */
/** Whether cards show their format on a corner. */
export const badgeShown = (l) => l.extBadge !== false && l.extBadge !== 'none'

// the view settings a folder can have of its own
const FOLDER_VIEW_KEYS = ['view', 'size', 'sort', 'group']
const FOLDER_VIEW_SET = new Set(FOLDER_VIEW_KEYS)
const folderParent = computed(() => new Map(S.sidebar.folders.map((f) => [f.id, f.parent_id ?? null])))
const ownViews = () => S.ui.folderViews?.[S.startup.current || '']

/** The folder whose own view settings apply to `id`: itself, or its nearest parent that has some. */
function viewFolder(id) {
  const own = ownViews()
  if (!own) return null
  for (let n = 0; id != null && n < 64; n++, id = folderParent.value.get(id)) if (own[id]) return id
  return null
}
const shownFolder = () => (S.scope.kind === 'folder' && !S.scope.ids && S.scope.path == null ? S.scope.id : null)

/** Whether the folder shown has a layout of its own, and whose it follows once that goes: a parent folder's name, or null for All's. */
export function folderViewState() {
  const id = shownFolder()
  if (id == null || !ownViews()?.[id]) return null
  const from = viewFolder(folderParent.value.get(id))
  return { inherits: from == null ? null : S.sidebar.folders.find((f) => f.id === from)?.name || null }
}

/** The folder shown follows its parent's layout again, or All's. */
export function resetFolderView() {
  const id = shownFolder()
  const own = ownViews()
  if (id == null || !own?.[id]) return
  delete own[id]
  saveUi()
}

// what L() reads and writes: the arrangement keys go to the layout of the shape on show, a folder's own view settings to that folder, the rest to the view every layout shares
const routed = new WeakMap()
export const L = () => {
  const g = S.ui.grid
  let r = routed.get(g)
  if (!r) {
    r = new Proxy(g, {
      get: (t, k, rec) => {
        if (isArrangeKey(k)) return arrangement(currentAspect())[k]
        if (FOLDER_VIEW_SET.has(k)) {
          const id = shownFolder()
          const from = id == null ? null : viewFolder(id)
          if (from != null && ownViews()[from][k] !== undefined) return ownViews()[from][k]
        }
        return Reflect.get(t, k, rec)
      },
      set: (t, k, v) => {
        if (isArrangeKey(k)) arrangement(currentAspect())[k] = v
        else if (FOLDER_VIEW_SET.has(k) && shownFolder() != null && S.startup.current) {
          // a folder's first change gives it all of them, as it shows them now, so a parent's later changes no longer reach it
          const id = shownFolder()
          const lib = (S.ui.folderViews ||= {})[S.startup.current] ||= {}
          if (!lib[id]) lib[id] = Object.fromEntries(FOLDER_VIEW_KEYS.map((key) => [key, structuredClone(toRaw(r[key]))]))
          lib[id][k] = v
        } else t[k] = v
        return true
      },
    })
    routed.set(g, r)
  }
  return r
}

/** The shape being shown: the window's own, unless the edit mode picked the other one. */
export const currentAspect = () => S.editAspect || (S.prefsWindow ? S.ui.aspect : S.windowAspect)

export const mainZones = () => arrangement(currentAspect()).zones

/** The screen of an item's kind. */
export function screenOfItem(it) {
  // a viewer plugin's page gets its plugin's screen for these files
  const v = it && S.contrib.viewers?.find((x) => x.exts.includes(String(it.ext).toLowerCase()))
  if (v) return screenKey(v)
  const k = kindOf(it)
  return k === 'video' ? 'video' : k === 'image' || k === 'none' ? 'image' : 'other'
}

/** What kind of thing is on show. */
export function liveScreen() {
  if (S.viewer) {
    // the item may come after the viewer opened, as one opened through the API before the grid loaded it
    void S.itemsVer
    return screenOfItem(S.items.get(viewerFile()) || S.items.get(S.viewer.id))
  }
  if (S.view === 'tags') return 'tags'
  return S.scope.kind === 'folder' || S.scope.kind === 'folders' ? 'folder' : 'main'
}
export const currentScreen = () => (S.customize && S.editScreen) || liveScreen()

function screenStore() {
  return (arrangement(currentAspect()).screens ||= {})
}

/** What a screen starts as: a Format plugin's with its declared toolbar and the Inspector of other files. */
function defaultsOf(screen) {
  const v = screen.includes('/') && S.contrib.viewers?.find((x) => screenKey(x) === screen)
  if (v) {
    const base = zonesOf('other')
    // the widgets the screen asks for go below Curio's, unless they are there already
    const add = (v.screen.inspector || []).filter((id) => !base.inspector.includes(id))
    const end = base.inspector.indexOf(PIN_BOTTOM)
    const at = end < 0 ? base.inspector.length : end
    return { ...base, toolbar: pluginToolbar(v, S.contrib), inspector: [...base.inspector.slice(0, at), ...add, ...base.inspector.slice(at)] }
  }
  return screenDefaults(screen.includes('/') ? 'other' : screen, mainZones())
}

const isRowId = (k) => k === 'row' || k.startsWith('row@')
const libRowsOf = (main) => new Set((main.library || []).filter(isRowId))

// the Library sidebar (and any rows inside it) is shared across every screen, so editing it in Folder or Image never diverges from Main
function withSharedLibrary(main, screenZones) {
  const out = { ...screenZones, library: main.library }
  for (const id of libRowsOf(main)) out[id] = main[id] || []
  return out
}

function screenOnly(main, zones) {
  const skip = new Set(['library', ...libRowsOf(main)])
  return Object.fromEntries(Object.entries(zones).filter(([k]) => !skip.has(k)))
}

function zonesOf(screen) {
  const main = mainZones()
  if (screen === 'main') return main
  return withSharedLibrary(main, screenStore()[screen] || defaultsOf(screen))
}

/** The screens of the selected items' kinds, in the grid; none when nothing is selected or an item is open. */
function selectionScreens() {
  if (S.viewer || S.view === 'tags' || !S.selected.length) return []
  void S.itemsVer
  const kinds = new Set()
  for (const id of S.selected) {
    const it = S.items.get(id)
    if (it) kinds.add(screenOfItem(it))
  }
  return kinds.size ? [...kinds] : ['image']
}

/** The widgets to show now. Selected items get the Inspector of their kind's screen; items of several kinds, the widgets those screens share. */
/** Tab plugins' pages not placed anywhere sit with the library's own views, after the last of them. */
function withTabs(zones) {
  const placed = new Set(Object.values(zones).flat())
  const tabs = (S.contrib.tabs || []).map((t) => `tab:${t.plugin}/${t.id}`).filter((k) => !placed.has(k))
  if (!tabs.length || !zones.library) return zones
  const lib = zones.library
  const at = Math.max(...QUICK.map((q) => lib.indexOf(q)))
  const i = at >= 0 ? at + 1 : Math.max(0, lib.indexOf(PIN_TOP) + 1)
  return { ...zones, library: [...lib.slice(0, i), ...tabs, ...lib.slice(i)] }
}

export function activeZones() {
  const zones = withTabs(zonesOf(currentScreen()))
  if (S.customize) return zonesOf(currentScreen())
  const kinds = selectionScreens()
  if (!kinds.length) return zones
  const lists = kinds.map((k) => zonesOf(k).inspector)
  return { ...zones, inspector: lists[0].filter((id) => lists.every((l) => l.includes(id))) }
}

/** The same, ready to be changed: a screen gets its own copy for toolbar, filterbar and Inspector, while the Library sidebar always edits `main`. */
export function editZones() {
  const screen = currentScreen()
  const main = mainZones()
  if (screen === 'main') return main
  const store = screenStore()
  if (!store[screen]) store[screen] = screenOnly(main, JSON.parse(JSON.stringify(defaultsOf(screen))))
  const sc = store[screen]
  delete sc.library
  const inMain = (k) => k === 'library' || (isRowId(k) && ((main.library || []).includes(k) || (k in main && !(k in sc))))
  const owner = (k) => (inMain(k) ? main : sc)
  return new Proxy(sc, {
    ownKeys: () => [...new Set([...Object.keys(sc).filter((k) => !inMain(k)), 'library', ...libRowsOf(main)])],
    getOwnPropertyDescriptor: (_, k) => Object.getOwnPropertyDescriptor(owner(k), k),
    has: (_, k) => k in owner(k),
    get: (_, k) => owner(k)[k],
    set: (_, k, v) => {
      owner(k)[k] = v
      return true
    },
    deleteProperty: (_, k) => {
      delete owner(k)[k]
      return true
    },
  })
}

export function setEditAspect(a) {
  S.editAspect = a === S.windowAspect ? null : a
}

export function dockSide() {
  return L().inspectorDock
}

export function resetZones() {
  const screen = currentScreen()
  if (screen !== 'main') delete screenStore()[screen]
  else arrangement(currentAspect()).zones = defaultZones()
  saveUi()
}

/** Keep track of the window's shape. */
export function watchWindowShape() {
  const check = () => {
    const a = window.innerWidth >= window.innerHeight ? 'horizontal' : 'vertical'
    if (a !== S.windowAspect) S.windowAspect = a
    if (!S.prefsWindow && S.ui.aspect !== a) {
      S.ui.aspect = a
      saveUi()
    }
  }
  check()
  window.addEventListener('resize', check)
}

/** Expanded → compact (icons only) → hidden → expanded. */
export function cycleLibrary() {
  const l = L()
  if (!l.library) {
    l.library = true
    l.libraryCompact = false
  } else if (!l.libraryCompact) l.libraryCompact = true
  else {
    l.library = false
    l.libraryCompact = false
  }
  saveUi()
}

/** Expanded → icons only → hidden → expanded, as the Library; docked under the grid it has no icons-only form. */
export function cycleInspector() {
  const l = L()
  if (!l.inspector) {
    l.inspector = true
    l.inspectorCompact = false
  } else if (!l.inspectorCompact && dockSide() !== 'bottom') l.inspectorCompact = true
  else {
    l.inspector = false
    l.inspectorCompact = false
  }
  saveUi()
}

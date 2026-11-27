// Layouts: how a window is arranged (panels and widget zones), kept apart from what it shows (view, sort, labels), which every layout shares.
import { defaultZones } from './widgets.js'
import { t } from '../shared/i18n.js'

// Saved layouts of another version start again from the default.
// 9: the Inspector of selected items comes from the screen of their kind
export const LAYOUT_V = 9

export const PANEL_KEYS = ['library', 'inspector', 'libraryWidth', 'inspectorWidth', 'inspectorHeight', 'libraryCompact', 'libraryDock', 'inspectorDock', 'inspectorCompact', 'merged', 'mergeMode', 'splitRatio', 'filterbar', 'dockLayout', 'dockColumns', 'dockItemWidth']
/** What a layout holds; every other key of `L()` is the shared view. */
export const ARRANGE_KEYS = [...PANEL_KEYS, 'zones', 'screens', 'zonesV']
const ARRANGE_SET = new Set(ARRANGE_KEYS)
export const isArrangeKey = (k) => ARRANGE_SET.has(k)

export function defaultPanels() {
  return {
    library: true, inspector: true, libraryWidth: 300, inspectorWidth: 250, inspectorHeight: 320,
    libraryCompact: false, libraryDock: 'left', inspectorDock: 'right', inspectorCompact: false, filterbar: true,
    // merged: the Inspector as a tab of the Library, or as its lower half
    merged: false, mergeMode: 'tabs', splitRatio: 0.5,
    // how the Inspector arranges its widgets when docked under the grid
    dockLayout: 'column', dockColumns: 3, dockItemWidth: 300,
  }
}

export function defaultArrangement() {
  return { ...defaultPanels(), zones: defaultZones(), screens: {}, zonesV: LAYOUT_V }
}

export function defaultGrid() {
  return {
    view: 'masonry', size: 200, gap: 8, showName: false, showInfo: false, showExt: false,
    extBadge: true, showNote: false, showSubfolders: true, subfolders: false,
    sort: { key: 'imported', field: '', desc: true },
    group: { key: 'none', field: '', desc: false },
    sections: { smart: true, folders: true, tags: false, locations: true, panels: true },
    // the columns of the list view, after the name
    listCols: [{ id: 'tags', w: 150 }, { id: 'dimensions', w: 96 }, { id: 'rating', w: 78 }, { id: 'ext', w: 66 }, { id: 'size', w: 72 }, { id: 'added', w: 96 }],
    widgetState: {},
    thumbBg: 'auto', grayscale: false,
  }
}

// Vertical is what a vertical window got before layouts: the Default with the Inspector under the grid
export const BUILTIN_LAYOUTS = {
  Default: () => defaultArrangement(),
  Vertical: () => ({ ...defaultArrangement(), inspectorDock: 'bottom' }),
  Focus: () => ({ ...defaultArrangement(), libraryCompact: true, inspector: false }),
  Minimal: () => ({ ...defaultArrangement(), library: false, inspector: false, filterbar: false }),
  Compact: () => ({ ...defaultArrangement(), libraryCompact: true, inspectorWidth: 220 }),
  Merged: () => ({ ...defaultArrangement(), merged: true }),
  Split: () => ({ ...defaultArrangement(), merged: true, mergeMode: 'split' }),
}
export const DEFAULT_FOR = { horizontal: 'Default', vertical: 'Vertical' }
/** The built-ins' earlier names: those of 0.8.12 and 0.8.13, and Focus until 0.8.14, which is Minimal now. */
export const RENAMED_LAYOUTS = { 'Portrait monitor': 'Vertical', 'Compact panels': 'Compact', 'Merged sidebar': 'Merged', 'Split sidebar': 'Split' }
const RENAMED_BEFORE_V2 = { ...RENAMED_LAYOUTS, Focus: 'Minimal' }
/** Bumped when a built-in's name goes to another built-in, so stored layouts are renamed once. */
export const LAYOUT_NAMES_V = 2
export const isBuiltinLayout = (n) => Object.hasOwn(BUILTIN_LAYOUTS, n)
/** A layout's name as shown: a built-in's is kept by its English key and shown in the window's language. */
export const layoutLabel = (n) => (isBuiltinLayout(n) ? t(`layout.builtin.${n}`) : n)

const SIZE_KEYS = new Set(['libraryWidth', 'inspectorWidth', 'inspectorHeight', 'splitRatio'])
const SHAPE_KEYS = PANEL_KEYS.filter((k) => !SIZE_KEYS.has(k))
const pick = (o, keys) => Object.fromEntries(keys.filter((k) => o[k] !== undefined).map((k) => [k, o[k]]))
const json = (x) => JSON.stringify(x ?? null)
// sizes drift as panels are dragged, so two layouts differing only in them are the same layout
const shapeOf = (l) => json(pick({ ...defaultPanels(), ...l }, SHAPE_KEYS))
export const sameShape = (a, b) => shapeOf(a) === shapeOf(b)

/** The same panels, zones and screens; widths are what dragging changes, so they count too. */
export function sameArrangement(a, b) {
  return ARRANGE_KEYS.filter((k) => k !== 'zonesV').every((k) => json(k === 'screens' ? a[k] || {} : a[k]) === json(k === 'screens' ? b[k] || {} : b[k]))
}

/** A built-in layout the user changed, which its card can restore. */
export const changedBuiltin = (name, stored) => isBuiltinLayout(name) && !!stored && !sameArrangement(stored, BUILTIN_LAYOUTS[name]())

/** The arrangement of an old workspace, with this version's defaults for what it lacks. */
const arrangementOf = (w) => ({ ...defaultArrangement(), ...pick(w, ARRANGE_KEYS), screens: w.screens || {}, zonesV: LAYOUT_V })

function freeLayoutName(base, taken) {
  let name = base
  for (let n = 2; isBuiltinLayout(name) || taken[name]; n++) name = `${base} ${n}`
  return name
}

/**
 * Workspaces (one horizontal arrangement, its vertical copy, and the view) become layouts, the two chosen for the shapes, and the active workspace's view.
 * A workspace shaped as a built-in or as one already kept becomes that layout; the active workspace's sizes and widgets are kept in it.
 * Returns `{ layouts, layoutFor, grid }`.
 */
export function migrateWorkspaces(workspaces, active) {
  const layouts = {}
  const layoutFor = { ...DEFAULT_FOR }
  let grid = defaultGrid()
  const match = (a) => Object.keys(BUILTIN_LAYOUTS).find((n) => sameShape(BUILTIN_LAYOUTS[n](), a)) || Object.keys(layouts).find((n) => !isBuiltinLayout(n) && sameShape(layouts[n], a))
  const keep = (base, a, chosen) => {
    const m = match(a)
    if (!m) {
      const n = freeLayoutName(base, layouts)
      layouts[n] = a
      return n
    }
    if (chosen && !(isBuiltinLayout(m) && sameArrangement(a, BUILTIN_LAYOUTS[m]()))) layouts[m] = a
    return m
  }
  // the active workspace first, so the layouts it is folded into keep its sizes and widgets
  const entries = Object.entries(workspaces || {}).sort(([a], [b]) => (a === active ? -1 : b === active ? 1 : 0))
  for (const [name, w] of entries) {
    // a workspace of an older zones version would have started again from the default
    if (!w || w.zonesV !== LAYOUT_V) continue
    const chosen = name === active
    const h = arrangementOf(w)
    const hName = keep(name, h, chosen)
    // the vertical arrangement was made on its own as the horizontal panels with the Inspector under the grid, and zones as a copy; only panels set apart from that make a layout of its own
    const autoVertical = { ...pick(w, PANEL_KEYS), inspectorDock: 'bottom' }
    const vertical = { ...h, ...pick({ ...autoVertical, ...w.vertical }, PANEL_KEYS), zones: w.zonesVertical || h.zones, screens: w.screensVertical || {}, zonesV: LAYOUT_V }
    const changed = !sameShape(vertical, autoVertical)
    const vName = changed || sameShape(vertical, BUILTIN_LAYOUTS.Vertical()) ? keep(`${hName} vertical`, vertical, chosen) : DEFAULT_FOR.vertical
    if (chosen) {
      layoutFor.horizontal = hName
      layoutFor.vertical = vName
      grid = { ...grid, ...Object.fromEntries(Object.entries(w).filter(([k]) => !isArrangeKey(k) && !['vertical', 'zonesVertical', 'screensVertical'].includes(k))) }
    }
  }
  return { layouts, layoutFor, grid }
}

/** Stored layouts and choices under the built-ins' earlier names take their new ones; `namesV` is the names version they were saved with. */
export function renameLayouts(layouts, layoutFor, namesV = LAYOUT_NAMES_V) {
  const map = namesV < 2 ? RENAMED_BEFORE_V2 : RENAMED_LAYOUTS
  const out = {}
  for (const [n, l] of Object.entries(layouts || {})) out[map[n] && !layouts[map[n]] ? map[n] : n] = l
  const fr = Object.fromEntries(Object.entries(layoutFor || {}).map(([k, n]) => [k, map[n] || n]))
  return { layouts: out, layoutFor: fr }
}

/** The layout of that name: the stored one, else a built-in's factory, else null. */
export function layoutOf(layouts, name) {
  return layouts?.[name] || (isBuiltinLayout(name) ? BUILTIN_LAYOUTS[name]() : null)
}

/** Names in the order the cards show: built-ins, then own layouts by name. */
export function layoutNames(layouts) {
  const own = Object.keys(layouts || {}).filter((n) => !isBuiltinLayout(n)).sort((a, b) => a.localeCompare(b))
  return [...Object.keys(BUILTIN_LAYOUTS), ...own]
}

/** A name for a copy that no layout has. */
export const copyName = (base, layouts) => freeLayoutName(t('layout.copyName', { name: layoutLabel(base) }), layouts || {})

// the window a card draws, in the units its widths are kept in
const WINDOW = { horizontal: [1440, 900], vertical: [900, 1440] }
// thin parts are drawn thicker than they are, so they still read in a small picture
const RAIL = 90
const TOOLBAR = 90
const FILTERBAR = 60

/**
 * The boxes of a layout's picture, in 0–100 of the window's width and height: `toolbar`, `filterbar`, `grid`, `library`, `inspector`, `rail` (icons only), `edge` (a hidden sidebar shown from the edge) and `tab` marks of an Inspector merged as a tab; merged as the lower half, it is an `inspector` box under the Library's.
 */
export function thumbRects(l, aspect = 'horizontal', { edgeReveal = false } = {}) {
  const [W, H] = WINDOW[aspect] || WINDOW.horizontal
  const px = (v) => (v / W) * 100
  const py = (v) => (v / H) * 100
  const out = []
  let left = 0
  let right = 100
  let bottom = 100
  const libW = l.libraryCompact ? RAIL : l.libraryWidth || 300
  const merged = !!l.merged
  const inspSide = l.inspectorDock === 'bottom' ? 'bottom' : l.inspectorDock === 'left' ? 'left' : 'right'
  const side = (kind, dock, w) => {
    const width = px(w)
    if (dock === 'right') {
      out.push({ kind, x: right - width, y: 0, w: width, h: 100 })
      right -= width
    } else {
      out.push({ kind, x: left, y: 0, w: width, h: 100 })
      left += width
    }
  }
  const edge = (dock) => edgeReveal && out.push({ kind: 'edge', x: dock === 'right' ? 98 : 0, y: 0, w: 2, h: 100 })
  if (l.library) {
    side(l.libraryCompact ? 'rail' : 'library', l.libraryDock === 'right' ? 'right' : 'left', libW)
    const lib = out[out.length - 1]
    if (merged && !l.libraryCompact) {
      const ratio = l.splitRatio ?? 0.5
      if (l.mergeMode === 'split') {
        lib.h = 100 * ratio
        out.push({ kind: 'inspector', x: lib.x, y: lib.h, w: lib.w, h: 100 - lib.h })
      } else {
        out.push({ kind: 'tab', active: true, x: lib.x + lib.w * 0.12, y: 4, w: lib.w * 0.34, h: 6 })
        out.push({ kind: 'tab', x: lib.x + lib.w * 0.54, y: 4, w: lib.w * 0.34, h: 6 })
      }
    }
  } else edge(l.libraryDock === 'right' ? 'right' : 'left')
  if (l.inspector && !merged && inspSide !== 'bottom') side(l.inspectorCompact ? 'rail' : 'inspector', inspSide, l.inspectorCompact ? RAIL : l.inspectorWidth || 250)
  else if (!l.inspector && !merged && inspSide !== 'bottom') edge(inspSide)
  out.push({ kind: 'toolbar', x: left, y: 0, w: right - left, h: py(TOOLBAR) })
  let top = py(TOOLBAR)
  if (l.filterbar) {
    out.push({ kind: 'filterbar', x: left, y: top, w: right - left, h: py(FILTERBAR) })
    top += py(FILTERBAR)
  }
  if (l.inspector && !merged && inspSide === 'bottom') {
    const h = py(l.inspectorHeight || 320)
    out.push({ kind: 'inspector', x: left, y: 100 - h, w: right - left, h })
    bottom -= h
  }
  out.push({ kind: 'grid', x: left, y: top, w: right - left, h: bottom - top })
  return out
}

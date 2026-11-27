// The window is made of widgets placed in spaces, like the toolbars of a customisable browser. A layout
// says which widgets sit in which space, in order; anything not listed isn't shown. The same widget can
// be placed several times: copies get a number after the name ("gap@2").

// A layout of its own for each kind of thing on show.
import { t } from '../shared/i18n.js'

// key and icon; the names are `layout.screens.<key>` in the catalogs
const SCREENS = [['main', 'layout-grid'], ['folder', 'folder'], ['tags', 'bookmark'], ['image', 'photo'], ['video', 'player-play'], ['other', 'file']]

/** A Format plugin's screen for the files its viewer page shows (`<plugin>/<id>`). */
export const screenKey = (v) => `${v.plugin}/${v.screen.id}`

/** Curio's screens with Format plugins' own before Other files, which takes whatever none of them shows: key, name and icon. */
export function allScreens(contrib) {
  const named = ([k, icon]) => [k, t(`layout.screens.${k}`), icon]
  const out = SCREENS.filter(([k]) => k !== 'other').map(named)
  for (const v of contrib?.viewers || []) if (!out.some(([k]) => k === screenKey(v))) out.push([screenKey(v), v.screen.title, v.screen.icon || 'file'])
  return [...out, named(SCREENS.find(([k]) => k === 'other'))]
}

// the spaces of the window; their names are `layout.zones.<id>` in the catalogs
export const ZONES = new Set(['toolbar', 'filterbar', 'library', 'inspector'])

// group: where the edit window lists it.
// scroll: long content that shares a scrolling area when it sits in a sidebar.
// panel: needs the width of a sidebar; in a toolbar or a sidebar of icons it is its icon, opening it in a popup.
// headless: shown without a heading of its own, so never folded.
// needs: `fontColumn` for one listed only while a plugin gives the list a Font sample column.
// form (see formOf): `launcher` for a button wherever it is; the rest of the toolbar's controls are `inline`.
// label and hint (`hint: true` where there is one) are `layout.parts.<id>` in the catalogs, filled in by widgetInfo.
export const WIDGETS = {
  // nothing but room
  gap: { icon: 'space', group: 'layout', hint: true },
  spacer: { icon: 'arrows-horizontal', group: 'layout', hint: true },
  separator: { icon: 'separator', group: 'layout', hint: true },
  row: { icon: 'layout-navbar', group: 'layout', isRow: true, hint: true },
  // every sidebar has both, once: what lies between them scrolls, what lies outside stays in place
  'pin.top': { icon: 'pin', fixed: true },
  'pin.bottom': { icon: 'pin', fixed: true },
  // single buttons, to arrange in any row or toolbar
  'b.menu': { icon: 'menu-2', group: 'buttons' },
  'b.add': { icon: 'plus', group: 'buttons' },
  'b.libswitch': { icon: 'arrows-exchange', group: 'buttons' },
  'b.prefs': { icon: 'settings', group: 'buttons' },
  'b.edit': { icon: 'layout-board', group: 'buttons' },
  'b.library': { icon: 'layout-sidebar', group: 'buttons', hint: true },
  'b.inspector': { icon: 'layout-sidebar-right', group: 'buttons', hint: true },
  'b.ontop': { icon: 'pin', group: 'buttons', hint: true },
  // Curio's tools, each in a window of its own, for the selected items
  'b.duplicates': { icon: 'copy-check', group: 'buttons' },
  'b.rename': { icon: 'cursor-text', group: 'buttons' },
  'b.compare': { icon: 'layout-columns', group: 'buttons' },
  'm.quick': { icon: 'inbox', group: 'buttons', hint: true },
  'm.favorites': { icon: 'pin', group: 'buttons' },
  'm.smart': { icon: 'layout-grid', group: 'buttons' },
  'm.folders': { icon: 'folder', group: 'buttons' },
  'm.tags': { icon: 'hash', group: 'buttons' },
  'm.locations': { icon: 'device-desktop', group: 'buttons' },
  'm.libraries': { icon: 'books', group: 'buttons' },
  // finding your way around
  header: { icon: 'menu-2', form: 'own' },
  libswitcher: { icon: 'books', group: 'browse' },
  libcard: { icon: 'id-badge-2', group: 'browse', hint: true },
  'q.all': { icon: 'inbox', group: 'browse', scroll: true },
  'q.uncategorized': { icon: 'folder-question', group: 'browse', scroll: true },
  'q.untagged': { icon: 'bookmark-question', group: 'browse', scroll: true },
  'q.recent': { icon: 'clock', group: 'browse', scroll: true },
  'q.random': { icon: 'arrows-shuffle', group: 'browse', scroll: true },
  'q.missing': { icon: 'alert-triangle', group: 'browse', scroll: true, hint: true },
  'q.tags': { icon: 'bookmark', group: 'browse', scroll: true },
  'q.trash': { icon: 'trash', group: 'browse', scroll: true },
  favorites: { icon: 'pin', group: 'browse', scroll: true },
  smart: { icon: 'layout-grid', group: 'browse', scroll: true },
  folders: { icon: 'folder', group: 'browse', scroll: true },
  tags: { icon: 'hash', group: 'browse', scroll: true },
  locations: { icon: 'device-desktop', group: 'browse', scroll: true },
  libraries: { icon: 'books', group: 'browse', scroll: true },
  panels: { icon: 'puzzle', group: 'browse', scroll: true },
  libfilter: { icon: 'filter', group: 'browse' },
  // toolbar controls
  librarytoggle: { icon: 'layout-sidebar', group: 'toolbar', form: 'launcher' },
  nav: { icon: 'arrows-left-right', group: 'toolbar' },
  title: { icon: 'letter-case', group: 'toolbar' },
  shuffle: { icon: 'refresh', group: 'toolbar', hint: true, form: 'launcher' },
  breadcrumb: { icon: 'chevrons-right', group: 'toolbar', hint: true },
  tagview: { icon: 'layout-list', group: 'toolbar' },
  tagsort: { icon: 'arrows-sort', group: 'toolbar', form: 'launcher' },
  zoom: { icon: 'zoom-in', group: 'toolbar' },
  plugins: { icon: 'puzzle-2', group: 'toolbar', form: 'launcher' },
  actions: { icon: 'bolt', group: 'toolbar', form: 'launcher' },
  layout: { icon: 'layout', group: 'toolbar', form: 'launcher' },
  filtertoggle: { icon: 'adjustments', group: 'toolbar', form: 'launcher' },
  search: { icon: 'search', group: 'toolbar' },
  fontsample: { icon: 'typography', group: 'toolbar', hint: true, needs: 'fontColumn' },
  filters: { icon: 'filter-cog', group: 'toolbar' },
  inspectortoggle: { icon: 'layout-sidebar-right', group: 'toolbar', form: 'launcher' },
  // an opened item
  'v.back': { icon: 'arrow-left', group: 'viewer', form: 'launcher' },
  'v.count': { icon: 'list-numbers', group: 'viewer' },
  'v.group': { icon: 'files', group: 'viewer', hint: true },
  'v.zoom': { icon: 'zoom-in', group: 'viewer' },
  'v.reveal': { icon: 'folder-open', group: 'viewer', form: 'launcher' },
  'v.open': { icon: 'external-link', group: 'viewer', form: 'launcher' },
  'v.flip': { icon: 'flip-vertical', group: 'viewer', form: 'launcher' },
  'v.rotate': { icon: 'rotate-clockwise-2', group: 'viewer', form: 'launcher' },
  'v.actual': { icon: 'circle-dashed-number-1', group: 'viewer', form: 'launcher' },
  'v.fit': { icon: 'arrows-maximize', group: 'viewer', form: 'launcher' },
  'v.strip': { icon: 'layout-bottombar', group: 'viewer', form: 'launcher' },
  'v.live': { icon: 'live-photo', group: 'viewer', form: 'launcher' },
  'v.pair': { icon: 'switch-horizontal', group: 'viewer', form: 'launcher' },
  't.mode': { icon: 'markdown', group: 'viewer', hint: true },
  't.wrap': { icon: 'text-wrap', group: 'viewer', form: 'launcher' },
  't.lines': { icon: 'list-numbers', group: 'viewer', form: 'launcher' },
  // about what is selected: shown only while it applies
  // `some`: one item or several, each acting on all of them
  'w.preview': { icon: 'photo', group: 'item', scroll: true, panel: true, when: 'some', headless: true },
  'w.palette': { icon: 'palette', group: 'item', scroll: true, panel: true, when: 'one', headless: true },
  'w.name': { icon: 'cursor-text', group: 'item', scroll: true, panel: true, when: 'one', headless: true },
  'w.annotation': { icon: 'notes', group: 'item', scroll: true, panel: true, when: 'some', headless: true },
  'w.url': { icon: 'link', group: 'item', scroll: true, panel: true, when: 'some', headless: true },
  'w.tags': { icon: 'bookmark', group: 'item', scroll: true, panel: true, when: 'some' },
  'w.folders': { icon: 'folder', group: 'item', scroll: true, panel: true, when: 'some' },
  'w.properties': { icon: 'list-details', group: 'item', scroll: true, panel: true, when: 'some' },
  'w.rating': { icon: 'star', group: 'item', scroll: true, panel: true, when: 'some', headless: true },
  'w.dimensions': { icon: 'dimensions', group: 'property', scroll: true, panel: true, when: 'one', headless: true },
  'w.duration': { icon: 'clock', group: 'property', scroll: true, panel: true, when: 'one', headless: true },
  'w.pages': { icon: 'book-2', group: 'property', scroll: true, panel: true, when: 'one', headless: true },
  'w.size': { icon: 'file', group: 'property', scroll: true, panel: true, when: 'some', headless: true },
  'w.type': { icon: 'file-code', group: 'property', scroll: true, panel: true, when: 'one', headless: true },
  'w.added': { icon: 'calendar-plus', group: 'property', scroll: true, panel: true, when: 'one', headless: true },
  'w.created': { icon: 'calendar', group: 'property', scroll: true, panel: true, when: 'one', headless: true },
  'w.modified': { icon: 'calendar-event', group: 'property', scroll: true, panel: true, when: 'one', headless: true },
  'w.export': { icon: 'upload', group: 'item', scroll: true, panel: true, when: 'some', headless: true },
  'w.file': { icon: 'folder-search', group: 'item', scroll: true, panel: true, when: 'one' },
  'w.parts': { icon: 'files', group: 'item', scroll: true, panel: true, when: 'one' },
  'w.batch': { icon: 'stack-2', group: 'item', scroll: true, panel: true, when: 'many', headless: true },
  't.best': { icon: 'star', group: 'tags', scroll: true, panel: true, headless: true },
  'w.summary': { icon: 'info-circle', group: 'item', scroll: true, panel: true, when: 'none', headless: true },
  // `view`: shown only while the items are in that view (Layout in the toolbar)
  'w.spectrum': { icon: 'color-filter', group: 'browse', scroll: true, panel: true, when: 'none', headless: true, view: 'spectrum', hint: true },
}

// the edit window's sections, in order; their names are `layout.groups.<id>` in the catalogs
export const GROUPS = ['layout', 'buttons', 'actions', 'toolbar', 'viewer', 'browse', 'item', 'tags', 'property', 'field']

/** A Tab plugin's view and widget name. */
export const tabKey = (t) => `tab:${t.plugin}/${t.id}`

/** A Window plugin's window as a button. */
export const winKey = (w) => `win:${w.plugin}/${w.id}`

/** A viewer page's control as a widget of the toolbar. */
export const toolKey = (t) => `tool:${t.plugin}/${t.id}`

/** "gap@2" is a copy of "gap". */
export const baseOf = (id) => id.split('@')[0]

/** Plugin widgets and fields arrive at run time. */
export function widgetInfo(id, contrib = null, fields = null, actions = null) {
  const b = baseOf(id)
  const p = WIDGETS[b]
  if (p) return { ...p, label: t(`layout.parts.${b}.label`), hint: p.hint ? t(`layout.parts.${b}.hint`) : undefined }
  if (b.startsWith('action:')) {
    const a = actions?.find((x) => x.id === b.slice(7))
    return a ? { label: a.name, icon: a.icon || 'bolt', group: 'actions', action: a, form: 'launcher' } : null
  }
  const w = contrib?.widgets?.find((x) => x.id === b)
  // a page widget says when it shows: one item (the default), several, one or more (`some`), or always (`any`)
  if (w?.page) return { label: w.title, icon: w.icon || 'puzzle', group: 'field', scroll: true, panel: true, headless: !!w.headless, when: w.selection === 'any' ? null : ['many', 'some'].includes(w.selection) ? w.selection : 'one', plugin: w, page: true }
  if (w) return { label: w.title, icon: w.icon || 'puzzle', group: 'field', scroll: true, panel: true, headless: !!w.headless, when: 'one', plugin: w }
  if (b.startsWith('tool:')) {
    const t = contrib?.viewerTools?.find((x) => toolKey(x) === b)
    return t ? { label: t.title, icon: t.icon || 'puzzle', group: 'viewer', form: t.kind === 'button' || !t.kind ? 'launcher' : 'inline', tool: t } : null
  }
  if (b.startsWith('tab:')) {
    const t = contrib?.tabs?.find((x) => tabKey(x) === b)
    return t ? { label: t.title, icon: t.icon || 'puzzle', group: 'browse', scroll: true, tab: t } : null
  }
  if (b.startsWith('win:')) {
    const w = contrib?.windows?.find((x) => winKey(x) === b)
    return w ? { label: w.title, icon: w.icon || 'app-window', group: 'buttons', form: 'launcher', win: w } : null
  }
  if (b.startsWith('field:')) {
    const f = fields?.find((x) => x.key === b.slice(6))
    return { label: f?.label || b.slice(6), icon: 'forms', group: 'field', scroll: true, panel: true, when: 'one', field: f }
  }
  return null
}

const HEADER_ROW = ['b.menu', 'spacer@2', 'b.add', 'b.libswitch', 'b.library']
// what the Inspector shows for selected items, on the screens of each kind of item (image, video, other)
const ITEM_DEFAULT = ['w.preview', 'w.palette', 'w.name', 'w.annotation', 'w.url', 'w.rating', 'w.tags', 'w.folders', 'w.parts', 'w.properties', 'exif.camera', 'ai.generation', 'ai.model', 'curio.fonts.info', 'curio.models.info', 'audio.tags', 'audio.info', 'video.info', 'w.batch']
// the screens items show on, whose Inspector is the one a selection of them gets
export const ITEM_SCREENS = ['image', 'video', 'other']
// and Format plugins' screens, which are those of items too
export const isItemScreen = (screen) => ITEM_SCREENS.includes(screen) || screen.includes('/')
export const PANELS = ['library', 'inspector']
// the views every library has, each a widget of its own
export const QUICK = ['q.all', 'q.uncategorized', 'q.untagged', 'q.recent', 'q.random', 'q.missing', 'q.tags', 'q.trash']
export const PIN_TOP = 'pin.top'
export const PIN_BOTTOM = 'pin.bottom'

export function defaultZones(sections = { smart: true, folders: true, tags: false, locations: true, panels: true }) {
  const lists = ['favorites', 'smart', 'folders', 'tags', 'locations', 'libraries', 'panels'].filter((k) => sections[k] || (sections[k] === undefined && k === 'favorites'))
  return {
    toolbar: ['librarytoggle', 'nav', 'title', 'shuffle', 'spacer', 'plugins', 'actions', 'layout', 'filtertoggle', 'search', 'inspectortoggle'],
    filterbar: ['filters'],
    library: ['row', 'libswitcher', PIN_TOP, ...QUICK, ...lists, 'spacer@3', PIN_BOTTOM, 'libfilter'],
    // nothing selected: what is in view (the library, a folder or several)
    inspector: [PIN_TOP, 'w.summary', 'w.spectrum', PIN_BOTTOM],
    row: [...HEADER_ROW],
  }
}

const VIEW_BAR_START = ['v.back', 'v.count', 'v.group']
const VIEW_BAR_END = ['v.strip', 'nav']
const SCREEN_TOPS = {
  tags: ['librarytoggle', 'nav', 'title', 'spacer', 'tagview', 'tagsort', 'search'],
  image: [...VIEW_BAR_START, 'v.zoom', 'spacer', 'v.live', 'v.pair', 'v.reveal', 'v.open', 'v.flip', 'v.rotate', 'v.actual', 'v.fit', ...VIEW_BAR_END],
  video: [...VIEW_BAR_START, 'spacer', 'v.reveal', 'v.open', 'v.flip', 'v.rotate', ...VIEW_BAR_END],
  other: [...VIEW_BAR_START, 'v.zoom', 'spacer', 't.mode', 't.wrap', 't.lines', 'v.reveal', 'v.open', 'v.actual', ...VIEW_BAR_END],
}

/** A Format plugin's screen's toolbar as it declares it: its own tools by their ids, Curio's widgets by their names where no tool takes them. */
export function pluginToolbar(v, contrib) {
  const tools = (contrib?.viewerTools || []).filter((t) => t.plugin === v.plugin)
  const ids = v.screen.toolbar?.length ? v.screen.toolbar : ['v.back', 'v.count', 'v.group', 'breadcrumb', 'spacer', ...tools.map((t) => t.id), 'v.strip', 'nav']
  return ids.map((id) => (tools.some((t) => t.id === id) ? toolKey({ plugin: v.plugin, id }) : WIDGETS[baseOf(id)] ? id : null)).filter(Boolean)
}

/** What a screen shows until it is given a layout of its own: the main one, with its own toolbar. */
export function screenDefaults(screen, main) {
  if (!SCREEN_TOPS[screen]) return main
  return { ...main, toolbar: SCREEN_TOPS[screen], filterbar: [], inspector: screen === 'tags' ? [PIN_TOP, 't.best', PIN_BOTTOM] : [PIN_TOP, ...ITEM_DEFAULT, PIN_BOTTOM] }
}

/** Whether a widget belongs on a screen: widgets for selected items on the items' screens, the rest where nothing is selected. */
export function fitsScreen(p, screen) {
  const items = isItemScreen(screen)
  if (p.id === 't.best') return screen === 'tags'
  // a viewer page's controls on its own plugin's screens
  if (p.tool) return screen.startsWith(`${p.tool.plugin}/`)
  if (p.when === 'none') return !items
  if (p.when) return items
  return true
}

/** The pinned top always comes before the pinned bottom. */
export function orderPinMarkers(list) {
  const t = list.indexOf(PIN_TOP)
  const b = list.indexOf(PIN_BOTTOM)
  if (t >= 0 && b >= 0 && b < t) {
    list[t] = PIN_BOTTOM
    list[b] = PIN_TOP
  }
}

export function zoneOf(zones, id) {
  return Object.keys(zones).find((z) => zones[z].includes(id)) || null
}

/** Does a widget of this kind appear anywhere (under any number)? */
export const isPlaced = (zones, base) => Object.keys(zones).some((z) => zones[z].some((id) => baseOf(id) === base))

/** A name not used yet, for one more copy of a widget. */
export function newInstance(zones, base) {
  const used = new Set(Object.keys(zones).flatMap((z) => zones[z]))
  if (!used.has(base)) return base
  for (let n = 2; ; n++) if (!used.has(`${base}@${n}`)) return `${base}@${n}`
}

/** Move a widget to a position in a space (from anywhere, or new). */
export function placePart(zones, id, zone, index) {
  const from = zoneOf(zones, id)
  let at = index
  if (from) {
    const i = zones[from].indexOf(id)
    zones[from].splice(i, 1)
    if (from === zone && i < at) at--
  }
  zones[zone].splice(Math.max(0, Math.min(at, zones[zone].length)), 0, id)
}

export function hidePart(zones, id) {
  const z = zoneOf(zones, id)
  if (z) zones[z].splice(zones[z].indexOf(id), 1)
  // a row takes what it holds along
  if (zones[id] && !ZONES.has(id)) delete zones[id]
}

/** Where a widget may go: rows only in sidebars, the pinned edges only in their own sidebar. A widget too wide for a toolbar goes there as its icon. */
export const fitsZone = (info, zone) => {
  if (info?.fixed) return PANELS.includes(zone)
  if (info?.isRow) return baseOf(zone) !== 'row' && zone !== 'toolbar' && zone !== 'filterbar'
  return true
}

/**
 * How a widget shows where there isn't a sidebar's width for it:
 * `panel` turns into its icon, opening the widget in a popup; `inline` stays itself in a toolbar (and turns into its icon only in a sidebar of icons);
 * `launcher` is a button everywhere; `own` draws its own small form (the library's lists as menus, its filter); `layout` is room and lines.
 */
export function formOf(info) {
  if (!info) return null
  if (info.form) return info.form
  if (info.panel) return 'panel'
  if (info.group === 'layout' || info.fixed) return 'layout'
  if (info.group === 'buttons') return 'launcher'
  if (info.group === 'browse' || info.tab) return 'own'
  return 'inline'
}

<script setup>
import { t } from '../shared/i18n.js'
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { listedRoot, sideClick, sideSelected, sideTargets, deleteSide, startNewFolder, finishNewFolder, newFolderListed, rootIcon, LOOK_COLORS, colorName, sideScoped, folderInScope, rootDirsMap, loadRootDirs } from './sidebar.js'
import { tagColor } from '../tags/tags.js'
import { smartOpen, rootOpen, folderOpen, S } from '../app/state.js'
import { openPrefs, saveUi, flushUi } from '../app/preferences.js'
import { L, cycleLibrary } from '../layout/arrangement.js'
import { setScope, filterByTag } from '../library/navigation.js'
import { edit, isFavorite, toggleFavorite, reorderFavorites } from '../library/edits.js'
import { run, toast } from '../app/notify.js'
import { openLibrary, importPaths, refreshLibraries, relocateRoot, addPaths } from '../library/lifecycle.js'
import { afterOpen } from '../app/boot.js'
import { requestSearch } from '../library/results.js'
import { toggleOnTop } from '../app/windowChrome.js'
import { call, errorText } from '../platform/ipc.js'
import { beginDrag, finalIndex } from '../app/dragDrop.js'
import { fmtSize } from '../library/itemSelectors.js'
import { fuzzy } from '../shared/text.js'
import AppIcon from '../shared/AppIcon.vue'
import { rem } from '../shared/rem.js'
import { WIDGETS } from '../layout/widgets.js'
import AppSearchField from '../shared/AppSearchField.vue'
import AppHighlight from '../shared/AppHighlight.vue'
import AppMenu from '../shared/AppMenu.vue'
import Popover from 'openvue/popover'
import InputText from 'openvue/inputtext'
import BrowseWidgetsSideScope from './BrowseWidgetsSideScope.vue'
import BrowseWidgetsFolderNode from './BrowseWidgetsFolderNode.vue'
import { shortcutLabel, withKey } from '../app/shortcuts.js'
import { openTool } from '../tools/tools.js'

// Everything for finding your way around a library: lists in a sidebar, or an icon with a menu
// when the part sits in a toolbar or in the compact sidebar.
const props = defineProps({ id: String, mode: { type: String, default: 'panel' } })
const lay = computed(() => L())
// the menu forms always show as an icon with a menu, wherever they are put
const listId = computed(() => props.id.replace(/^m\./, ''))
const isMenuForm = computed(() => props.id.startsWith('m.'))
const isList = computed(() => props.mode === 'panel' && !isMenuForm.value)
const rail = computed(() => props.mode === 'rail')
const q = computed(() => S.sideFilter)
const LISTS = ['favorites', 'libraries', 'quick', 'smart', 'folders', 'tags', 'locations', 'panels']
const fmt = (n) => (n == null ? '' : Number(n).toLocaleString())
// a list's count, left out when there is nothing to count
const cnt = (n) => (n ? fmt(n) : '')

const menu = ref()
const mainMenu = ref()
const addMenu = ref()
const libMenu = ref()
const rowMenu = ref()
const addLibMenu = ref()
const libraries = computed(() => S.libraries)
const renaming = ref(null)
const ctx = ref(null)
const newName = ref('')

onMounted(() => {
  if (['libswitcher', 'libcard', 'header', 'libraries', 'b.libswitch', 'm.libraries'].includes(props.id)) refreshLibraries()
})

// ---------------------------------------------------------------- quick items
const quick = computed(() => {
  const c = S.sidebar.counts || {}
  return [
    { kind: 'all', label: t('sidebar.quick.all'), icon: 'inbox', count: c.all },
    { kind: 'favorites', label: t('sidebar.quick.favorites'), icon: 'heart', count: c.favorites },
    { kind: 'uncategorized', label: t('sidebar.quick.uncategorized'), icon: 'folder-question', count: c.uncategorized },
    { kind: 'untagged', label: t('sidebar.quick.untagged'), icon: 'bookmark-question', count: c.untagged },
    { kind: 'recent', label: t('sidebar.quick.recent'), icon: 'clock', count: c.recent },
    { kind: 'random', label: t('sidebar.quick.random'), icon: 'arrows-shuffle' },
    { kind: 'missing', label: t('sidebar.quick.missing'), icon: 'alert-triangle', count: c.missing, warn: true, hidden: !(c.missing > 0) },
    { kind: 'tags', label: t('sidebar.quick.tags'), icon: 'bookmark', count: c.tags },
    { kind: 'trash', label: t('sidebar.quick.trash'), icon: 'trash', count: c.trash, drop: 'trash' },
  ]
})
// this widget, when it is one of the views above
const trashItems = () => [{ label: t('sidebar.trash.empty'), icon: 'trash-x', command: () => (S.dialogs.confirm = { title: t('sidebar.trash.title'), body: t('sidebar.trash.body'), ok: () => run('item.purge', { all: true }) }) }]
// a Tab plugin's page is a view like All tags
const tabItem = (t) => ({ kind: `tab:${t.plugin}/${t.id}`, label: t.title, icon: t.icon || 'puzzle', tab: true })
const qItem = computed(() => {
  if (props.id.startsWith('tab:')) {
    const t = S.contrib.tabs?.find((x) => `tab:${x.plugin}/${x.id}` === props.id)
    return t ? tabItem(t) : null
  }
  return props.id.startsWith('q.') ? quick.value.find((x) => x.kind === props.id.slice(2)) : null
})
function clickQuick(item) {
  S.sideSel = { kind: null, ids: [], anchor: null }
  if (item.kind === 'tags' || item.tab) {
    S.view = item.kind
    S.viewer = null
  } else setScope(item.kind)
}
const isActive = (item) => (item.kind === 'tags' || item.tab ? S.view === item.kind : S.view === 'grid' && S.scope.kind === item.kind)

// ---------------------------------------------------------------- menus
const libName = computed(() => (S.startup.current || '').split(/[\\/]/).pop()?.replace(/\.curio$/, '') || t('sidebar.library'))
function newWindow(path = null) {
  ;(path ? call('library.open', { library: path }) : call('new_window', { path })).catch((e) => toast(errorText(e), 'error'))
}
function forgetLibrary(l) {
  call('library.forget', { library: l.path })
    .then(async () => {
      toast(t('sidebar.forgotten', { name: l.name }))
      if (l.current) await afterOpen()
      else refreshLibraries()
    })
    .catch((e) => toast(errorText(e), 'error'))
}
const forgetItem = (l) => ({ label: t('sidebar.menu.removeFromList'), icon: 'playlist-x', visible: !l.openElsewhere, command: () => forgetLibrary(l) })
const renameItem = (l) => ({ label: t('sidebar.menu.rename'), icon: 'cursor-text', visible: !l.current && !l.openElsewhere && l.exists, command: () => (S.dialogs.libName = { mode: 'rename', path: l.path, name: l.name }) })
const askNewLibrary = () => (S.dialogs.libName = { mode: 'new', name: t('sidebar.newLibraryName') })
// a row switches to its library, its button gives the library's own options and dragging it reorders them, as its row in the Library does; the switcher and the Libraries menu are the same list
const refreshLibMenus = () => {
  libMenu.value?.refresh()
  menu.value?.refresh()
}
const libItems = () => {
  refreshLibraries()
  return [
    ...libraries.value.map((l, i) => ({ label: l.name, icon: l.icon || 'books', color: l.color, current: l.current, command: () => libClick(l), more: libOptions(l), drop: 'lib:' + i, drag: (e, row) => dragLib(e, l, i, refreshLibMenus, row) })),
    { separator: true },
    ...libAddItems(),
  ]
}
// imports land in the folder shown, a location is added beside the library's folders
const addPick = async (kind, copy) => {
  const p = await call('pick', { kind })
  if (p) copy ? addPaths(p, true) : importPaths(p, false, null)
}
const addItems = () => [
  { label: t('sidebar.menu.importFolder'), icon: 'folder-share', command: () => addPick('folder', true) },
  { label: t('sidebar.menu.addLocation'), icon: 'folder-plus', command: () => addPick('folder', false) },
  { label: t('sidebar.menu.importFiles'), icon: 'copy', command: () => addPick('files', true) },
  { label: t('sidebar.menu.importLinks'), icon: 'world-download', command: () => (S.dialogs.url = true) },
  { separator: true },
  { label: t('sidebar.menu.importEagle'), icon: 'file-import', command: () => run('import_dot_library', { copy: true }) },
  { label: t('sidebar.menu.addEagle'), icon: 'file-import', command: () => run('import_dot_library', { copy: false }) },
]
function exportSel(kind) {
  run('export', { kind, ids: S.selected.length ? [...S.selected] : Array.from(S.results.ids) })
}
const mainItems = () => [
  { label: t('sidebar.menu.settings'), icon: 'settings', shortcut: shortcutLabel(S.ui.keys, 'app.settings'), command: () => openPrefs() },
  { label: t('sidebar.menu.editLayout'), icon: 'layout-board', command: () => (S.customize = true) },
  { label: t('sidebar.menu.fields'), icon: 'forms', command: () => { openPrefs('fields') } },
  { label: t('sidebar.menu.plugins'), icon: 'puzzle', command: () => { openPrefs('plugins') } },
  { label: t('sidebar.menu.duplicates'), icon: 'copy-check', command: () => openTool('duplicates') },
  { separator: true },
  { label: t('sidebar.menu.newWindow'), icon: 'app-window', command: () => newWindow() },
  { label: t('sidebar.menu.onTop'), icon: 'pin', checked: S.onTop, command: toggleOnTop },
  { separator: true },
  { label: t('sidebar.menu.exportJson'), icon: 'file-export', command: () => exportSel('json') },
  { label: t('sidebar.menu.exportCsv'), icon: 'table-export', command: () => exportSel('csv') },
  { label: t('sidebar.menu.exportSidecars'), icon: 'files', command: () => exportSel('sidecar-folder') },
  { separator: true },
  { label: t('sidebar.menu.quit'), icon: 'power', shortcut: shortcutLabel(S.ui.keys, 'app.quit'), command: () => call('quit_app').catch((e) => toast(errorText(e), 'error')) },
]

// a panel's filter is one of the filters, shown on the filter bar, not words in the search box
function usePanelFilter(f) {
  S.filters.preset = { label: f.label, query: f.query }
  requestSearch()
}

// ---------------------------------------------------------------- smart folders
// the same indents as the folder tree: the arrow sits in the row's padding, left of the icon
const TREE_PAD = 1.12
const TREE_INDENT = 1.12
const CARET = 0.98
// whole px, so the rows' icons line up with each other
const treePx = (n) => Math.round(rem(n)) + 'px'
const smartKids = (parent) => S.sidebar.smart.filter((s) => (s.parent_id ?? null) === parent)
// a tree in the order they were dragged to; a filter shows the matches as one flat list
const smart = computed(() => {
  if (q.value) return S.sidebar.smart.filter((s) => fuzzy(s.name, q.value)).map((sf) => ({ sf, depth: 0, kids: false }))
  const out = []
  const walk = (parent, depth) => {
    for (const sf of smartKids(parent)) {
      const kids = smartKids(sf.id).length > 0
      out.push({ sf, depth, kids })
      if (kids && smartOpen[sf.id]) walk(sf.id, depth + 1)
    }
  }
  walk(null, 0)
  return out
})
const smartOrder = computed(() => smart.value.map((r) => r.sf.id))
const smartBucketsMap = reactive({})
async function loadSmartBuckets(id) {
  smartBucketsMap[id] = (await call('smart_buckets', { id }).catch(() => null)) || []
}
function toggleSmart(sf) {
  smartOpen[sf.id] = !smartOpen[sf.id]
  if (smartOpen[sf.id] && sf.group_by) loadSmartBuckets(sf.id)
}
const bucketLabel = (sf, b) => (sf.group_by === 'rating' ? (Number(b.value) ? `${b.value} ★` : t('filters.value.none')) : sf.group_by === 'kind' ? t(`filters.kindNames.${b.value}`) : b.label)
const bucketIcon = (sf) => (sf.group_by?.startsWith('year:') || sf.group_by?.startsWith('month:') ? 'calendar' : sf.group_by === 'rating' ? 'star' : sf.group_by === 'tag' ? 'bookmark' : 'folder')
watch(
  () => (isList.value && listId.value === 'smart' ? S.sidebar.smart.filter((s) => s.group_by && smartOpen[s.id]).map((s) => `${s.id}:${s.group_by}:${s.count}:${s.query}`).join() : ''),
  (key) => {
    if (!key) return
    for (const s of S.sidebar.smart) if (s.group_by && smartOpen[s.id]) loadSmartBuckets(s.id)
  },
  { immediate: true },
)
const newSmart = (parent = null) => (S.dialogs.smart = { id: null, name: t('filters.smart.newName'), query: '', parent })
function dragSmart(e, sf) {
  beginDrag(e, {
    drop: (target, zone) => {
      if (target === 'smart-root') return edit.smart('place', { id: sf.id, parent: null, before: null })
      const tid = Number(target.split(':')[1])
      if (zone === 'inside') {
        smartOpen[tid] = true
        return edit.smart('place', { id: sf.id, parent: tid, before: null })
      }
      const t = S.sidebar.smart.find((x) => x.id === tid)
      const sibs = smartKids(t?.parent_id ?? null).filter((x) => x.id !== sf.id)
      const at = sibs.findIndex((x) => x.id === tid)
      edit.smart('place', { id: sf.id, parent: t?.parent_id ?? null, before: zone === 'before' ? tid : sibs[at + 1]?.id ?? null })
    },
  }, sf.name, { zones: true, accept: (t) => t === 'smart-root' || (t.startsWith('smart:') && Number(t.split(':')[1]) !== sf.id) })
}
function smartRowItems(sf) {
  const ids = sideTargets('smart', sf.id)
  const many = ids.length > 1
  return [
    { label: t('sidebar.menu.editRules'), icon: 'adjustments', visible: !many, command: () => (S.dialogs.smart = { ...sf }) },
    { label: t('sidebar.menu.newSubfolder'), icon: 'folder-plus', visible: !many, command: () => { smartOpen[sf.id] = true; newSmart(sf.id) } },
    { label: isFavorite('smart', sf.id) ? t('sidebar.menu.unpin') : t('sidebar.menu.pin'), icon: isFavorite('smart', sf.id) ? 'pinned-off' : 'pin', visible: !many, command: () => toggleFavorite('smart', sf.id) },
    { label: t('sidebar.menu.rename'), icon: 'cursor-text', visible: !many, command: () => (renaming.value = { id: sf.id, name: sf.name }) },
    { label: t('sidebar.menu.changeIcon'), icon: 'icons', visible: !many, command: () => (S.dialogs.icon = { kind: 'smart', id: sf.id, current: sf.icon }) },
    { separator: true },
    { label: many ? t('sidebar.menu.deleteSmarts', { count: ids.length }) : smartKids(sf.id).length ? t('sidebar.menu.deleteWithSubfolders') : t('sidebar.menu.delete'), icon: 'trash', danger: true, shortcut: 'Delete', command: () => deleteSide('smart', ids) },
  ]
}
async function finishRename() {
  const r = renaming.value
  renaming.value = null
  if (!r || !r.name.trim()) return
  const sf = S.sidebar.smart.find((s) => s.id === r.id)
  await edit.smart('update', { id: r.id, name: r.name.trim(), query: sf?.query || '' })
}

// ---------------------------------------------------------------- folders
const topFolders = computed(() => S.sidebar.folders.filter((f) => f.parent_id == null))
function endNewFolder() {
  const name = newName.value
  newName.value = ''
  finishNewFolder(name)
}
function folderMenuItems(parent = null) {
  return S.sidebar.folders.filter((f) => (f.parent_id ?? null) === parent).map((f) => {
    const kids = folderMenuItems(f.id)
    return { label: f.name, icon: f.icon || 'folder', color: f.color, count: cnt(f.count), drop: 'folder:' + f.id, current: S.scope.kind === 'folder' && S.scope.id === f.id, items: kids.length ? kids : undefined, command: () => setScope('folder', f.id) }
  })
}

// ---------------------------------------------------------------- locations
const allRoots = computed(() => S.sidebar.roots.filter(listedRoot))
const roots = computed(() => allRoots.value.filter((r) => fuzzy(r.label, q.value)))
const rootOrder = computed(() => roots.value.map((r) => r.id))
// a location indexed in place lists the folders inside it; refreshed as locations are added or rescanned
function toggleRoot(r) {
  rootOpen[r.id] = !rootOpen[r.id]
  if (rootOpen[r.id]) loadRootDirs(r.id)
}
watch(
  () => (isList.value && listId.value === 'locations' ? allRoots.value.map((r) => `${r.id}:${r.asset_count}:${r.last_scan}`).join() : ''),
  (key) => {
    if (!key) return
    for (const r of allRoots.value) loadRootDirs(r.id)
  },
  { immediate: true },
)
const canOpen = (r) => (rootDirsMap[r.id]?.length ?? 0) > 0
/** The open part of a location's folder tree, as rows. */
function dirRows(r) {
  const list = rootDirsMap[r.id] || []
  const parentOf = (p) => (p.includes('/') ? p.slice(0, p.lastIndexOf('/')) : '')
  const shown = (p) => {
    for (let at = parentOf(p); at; at = parentOf(at)) if (!rootOpen[r.id + ':' + at]) return false
    return true
  }
  return list.filter(([p]) => shown(p)).map(([p, count]) => ({ path: p, count, name: p.slice(p.lastIndexOf('/') + 1), depth: p.split('/').length, kids: list.some(([q]) => q.startsWith(p + '/')) }))
}
const dirActive = (r, d) => S.scope.kind === 'rootdir' && S.scope.id === r.id && S.scope.path === d.path

// the folder or location the view shows, wherever it was opened from (an item's folder in the Inspector, the breadcrumb, back and forward, a subfolder in the grid): its list opens down to it and scrolls its row into view
const REVEAL_TRIES = 20
const REVEAL_WAIT_MS = 50
watch(() => [S.scope.kind, S.scope.id, S.scope.path], async () => {
  if (props.mode !== 'panel') return
  const { kind, id, path } = S.scope
  let row
  if (listId.value === 'folders' && kind === 'folder') {
    const parentOf = (f) => S.sidebar.folders.find((x) => x.id === f)?.parent_id ?? null
    for (let p = parentOf(id); p != null; p = parentOf(p)) folderOpen[p] = true
    row = `[data-drop="folder:${id}"]`
  } else if (listId.value === 'locations' && (kind === 'root' || kind === 'rootdir')) {
    if (kind === 'rootdir' && path) {
      rootOpen[id] = true
      const parts = path.split('/')
      for (let i = 1; i < parts.length; i++) rootOpen[id + ':' + parts.slice(0, i).join('/')] = true
      loadRootDirs(id)
    }
    row = kind === 'root' ? `[data-drop="root:${id}"]` : `[data-dir="${CSS.escape(id + ':' + path)}"]`
  } else return
  // the opened rows are drawn first, and a location's folders come from disk
  for (let i = 0; i < REVEAL_TRIES; i++) {
    await new Promise((r) => setTimeout(r, REVEAL_WAIT_MS))
    const els = document.querySelectorAll(row)
    if (!els.length) continue
    for (const el of els) intoView(el)
    return
  }
}, { immediate: true })
/** Scrolls the list holding `el` just enough to show it, and nothing else on the page. */
function intoView(el) {
  let box = el.parentElement
  while (box && !/(auto|scroll)/.test(getComputedStyle(box).overflowY)) box = box.parentElement
  if (!box) return
  const r = el.getBoundingClientRect()
  const b = box.getBoundingClientRect()
  if (r.top < b.top) box.scrollTop -= b.top - r.top
  else if (r.bottom > b.bottom) box.scrollTop += r.bottom - b.bottom
}
const dirItems = (r, d) => [{ label: t('sidebar.menu.copyIn'), icon: 'copy-plus', disabled: !d.count, command: () => run('transfer', { action: 'copy_in', ids: [], root: r.id, path: d.path }) }]
function openDir(r, d) {
  S.sideSel = { kind: null, ids: [], anchor: null }
  setScope('rootdir', r.id, null, d.path)
}
function dragRoot(e, r) {
  beginDrag(e, {
    drop: (target, zone) => {
      const tid = Number(target.split(':')[1])
      const list = allRoots.value.filter((x) => x.id !== r.id)
      const at = list.findIndex((x) => x.id === tid)
      run('root_op', { op: 'place', id: r.id, flag: null, before: zone === 'before' ? tid : list[at + 1]?.id ?? null })
    },
  }, r.label, { sort: true, accept: (t) => t.startsWith('root:') && Number(t.split(':')[1]) !== r.id })
}
function rootRowItems(r) {
  const ids = sideTargets('root', r.id)
  if (ids.length > 1) return [{ label: t('sidebar.menu.removeLocations', { count: ids.length }), icon: 'trash', danger: true, shortcut: 'Delete', command: () => deleteSide('root', ids) }]
  return [
    { label: t('sidebar.menu.rescan'), icon: 'refresh', command: () => run('location.rescan', { id: r.id }) },
    { label: t('sidebar.menu.reveal'), icon: 'folder-open', command: () => run('root_op', { op: 'reveal', id: r.id, flag: null }) },
    { label: t('sidebar.menu.relocate'), icon: 'route', command: () => relocateRoot(r.id) },
    { label: t('sidebar.menu.copyIn'), icon: 'copy-plus', disabled: !r.asset_count, command: () => run('transfer', { action: 'copy_in', ids: [], root: r.id }) },
    { label: isFavorite('root', r.id) ? t('sidebar.menu.unpin') : t('sidebar.menu.pin'), icon: isFavorite('root', r.id) ? 'pinned-off' : 'pin', command: () => toggleFavorite('root', r.id) },
    { label: r.watch ? t('sidebar.menu.unwatch') : t('sidebar.menu.watch'), icon: r.watch ? 'eye-off' : 'eye', command: () => run('root_op', { op: 'watch', id: r.id, flag: !r.watch }) },
    {
      label: t('sidebar.menu.filesHere'), icon: r.access === 'read_only' ? 'lock' : r.access === 'organize' ? 'folders' : 'pencil',
      items: [['read_only', 'lock'], ['full', 'pencil'], ['organize', 'folders']].map(([v, icon]) => ({ label: t(`sidebar.access.${v}`), icon, current: r.access === v, command: () => run('location.setAccess', { id: r.id, access: v }) })),
    },
    { separator: true },
    { label: t('sidebar.menu.icon'), icon: 'mood-smile', command: () => (S.dialogs.icon = { kind: 'root', id: r.id, current: r.icon }) },
    { dots: LOOK_COLORS.map(([n, c]) => ({ label: colorName(n), color: c, selected: (r.color || null) === c, command: () => edit.locationLook(r.id, { color: c }) })) },
    { separator: true },
    { label: t('sidebar.menu.removeFromLibrary'), icon: 'trash', danger: true, command: () => deleteSide('root', ids) },
  ]
}

// ---------------------------------------------------------------- tags
const tags = computed(() => [...S.sidebar.tags].filter((t) => t.count > 0 && fuzzy(t.name, q.value)).sort((a, b) => b.count - a.count).slice(0, 12))

// ---------------------------------------------------------------- favorites
const favorites = computed(() =>
  (S.sidebar.favorites || [])
    .map((f) => {
      if (f.kind === 'folder') {
        const x = S.sidebar.folders.find((v) => v.id === f.id)
        return x && { ...f, name: x.name, icon: x.icon || 'folder', color: x.color, count: x.count, drop: 'folder:' + x.id }
      }
      if (f.kind === 'smart') {
        const x = S.sidebar.smart.find((v) => v.id === f.id)
        return x && { ...f, name: x.name, icon: x.icon || 'layout-grid' }
      }
      if (f.kind === 'root') {
        const x = S.sidebar.roots.find((v) => v.id === f.id && listedRoot(v))
        return x && { ...f, name: x.label, icon: rootIcon(x), color: x.color, count: x.asset_count }
      }
      const x = S.sidebar.tags.find((v) => v.id === f.id)
      return x && { ...f, name: x.name, icon: 'hash', count: x.count, drop: 'tag:' + x.id }
    })
    .filter(Boolean),
)
const favActive = (f) => S.view === 'grid' && S.scope.kind === f.kind && S.scope.id === f.id
function openFav(f) {
  S.sideSel = { kind: null, ids: [], anchor: null }
  setScope(f.kind, f.id)
}
function favMenu(e, f) {
  openRow(e, [{ label: t('sidebar.menu.unpin'), icon: 'pinned-off', command: () => toggleFavorite(f.kind, f.id) }])
}
function dragFav(e, f, i) {
  beginDrag(e, {
    drop: (target, zone) => {
      const list = favorites.value.map(({ kind, id }) => ({ kind, id }))
      const [moved] = list.splice(i, 1)
      list.splice(finalIndex(i, target, zone), 0, moved)
      reorderFavorites(list)
    },
  }, f.name, { sort: true, lift: e.currentTarget.parentElement, accept: (t) => t.startsWith('fav:') && Number(t.split(':')[1]) !== i })
}

// ---------------------------------------------------------------- libraries
async function openFolderLibrary() {
  await flushUi()
  if (await run('open_library_dialog')) location.reload()
}
function dragLib(e, l, i, placed, row) {
  beginDrag(e, {
    drop: (target, zone) => {
      const to = Number(target.split(':')[1])
      const before = zone === 'before' ? libraries.value[to]?.path : libraries.value[to + 1]?.path
      if (before === l.path) return
      call('library.place', { library: l.path, before: before ?? null }).then(refreshLibraries).then(placed).catch((err) => toast(errorText(err), 'error'))
    },
  }, l.name, { sort: true, lift: row, accept: (t) => t.startsWith('lib:') && Number(t.split(':')[1]) !== i })
}
function libClick(l) {
  if (l.current) return
  if (l.openElsewhere) newWindow(l.path)
  else openLibrary(l.path).then(refreshLibraries)
}
const setLibLook = (l, what, value) => call('library.setLook', { library: l.path, [what]: value ?? '' }).then(refreshLibraries).catch((e) => toast(errorText(e), 'error'))
const libOptions = (l) => [
  { label: t('sidebar.menu.openNewWindow'), icon: 'app-window', disabled: !l.exists, visible: !l.current && !l.openElsewhere, command: () => newWindow(l.path) },
  { separator: true },
  renameItem(l),
  { label: t('sidebar.menu.icon'), icon: 'mood-smile', command: () => (S.dialogs.icon = { kind: 'library', path: l.path, current: l.icon }) },
  { dots: LOOK_COLORS.map(([n, c]) => ({ label: colorName(n), color: c, selected: (l.color || null) === c, command: () => setLibLook(l, 'color', c) })) },
  { separator: true },
  forgetItem(l),
]
function libMenuFor(e, l) {
  openRow(e, libOptions(l))
}
// the open library's own look, for the switcher and the card
const curLib = computed(() => libraries.value.find((l) => l.current) || {})
const libAddItems = () => [
  { label: t('sidebar.menu.newLibrary'), icon: 'library-plus', command: askNewLibrary },
  { label: t('sidebar.menu.openLibraryFolder'), icon: 'folder-open', command: openFolderLibrary },
]

// F2 with a smart folder selected
watch(() => S.renameSide, (r) => {
  if (r?.kind === 'smart' && props.id === 'smart') {
    const sf = S.sidebar.smart.find((x) => x.id === r.id)
    if (sf) renaming.value = { id: sf.id, name: sf.name }
    S.renameSide = null
  }
})

// ---------------------------------------------------------------- right-click and rail menus
const rowItems = ref([])
function openRow(e, items) {
  rowItems.value = items
  rowMenu.value.show(e)
}
const barItems = () => {
  const list = menuEntries()
  // in the compact sidebar the menu is all there is of the section, so it names it, as the section's heading would
  return rail.value && label.value && menuCount.value != null ? [{ heading: true, label: t('sidebar.heading', { name: label.value, n: menuCount.value }) }, ...list] : list
}
const menuEntries = () => {
  switch (listId.value) {
    case 'favorites': return favorites.value.map((f) => ({ label: f.name, icon: f.icon, color: f.color, count: f.count != null ? fmt(f.count) : undefined, drop: f.drop, current: favActive(f), command: () => openFav(f) }))
    case 'libraries': return libItems()
    case 'quick': return quick.value.filter((x) => !x.hidden).map((x) => ({ label: x.label, icon: x.icon, count: cnt(x.count), drop: x.drop, current: isActive(x), command: () => clickQuick(x) }))
    case 'smart': {
      const tree = (parent) => smartKids(parent).map((s) => {
        const kids = tree(s.id)
        return { label: s.name, icon: s.icon || 'layout-grid', count: cnt(s.count), current: S.scope.kind === 'smart' && S.scope.id === s.id, items: kids.length ? kids : undefined, command: () => setScope('smart', s.id) }
      })
      return [...tree(null), { separator: true }, { label: t('sidebar.menu.newSmart'), icon: 'plus', command: () => newSmart() }]
    }
    case 'folders': return [...folderMenuItems(), { separator: true }, { label: t('sidebar.menu.newFolder'), icon: 'plus', shortcut: shortcutLabel(S.ui.keys, 'folder.new'), command: () => startNewFolder() }]
    case 'tags': return tags.value.map((t) => ({ label: t.name, icon: 'hash', count: cnt(t.count), drop: 'tag:' + t.id, command: () => setScope('tag', t.id) }))
    case 'locations': return [...allRoots.value.map((r) => ({ label: r.label, icon: rootIcon(r), color: r.color, count: cnt(r.asset_count), current: S.scope.kind === 'root' && S.scope.id === r.id, command: () => setScope('root', r.id) })), { separator: true }, { label: t('sidebar.menu.addLocation'), icon: 'plus', command: () => addPick('folder', false) }]
    case 'panels': return S.contrib.panels.flatMap((p) => p.filters.map((f) => ({ label: f.label, icon: 'filter', command: () => usePanelFilter(f) })))
    case 'libswitcher': case 'libcard': return libItems()
    default: return []
  }
}
// what a section's menu would list; one with nothing to list has no icon in the compact sidebar
const menuCount = computed(() => {
  switch (listId.value) {
    case 'favorites': return favorites.value.length
    case 'quick': return quick.value.filter((x) => !x.hidden).length
    case 'smart': return S.sidebar.smart.length
    case 'folders': return S.sidebar.folders.length
    case 'tags': return tags.value.length
    case 'locations': return allRoots.value.length
    case 'panels': return S.contrib.panels.reduce((n, x) => n + x.filters.length, 0)
    case 'libraries': return libraries.value.length
    default: return null
  }
})
const emptyMenu = computed(() => ['favorites', 'tags', 'panels'].includes(listId.value) && menuCount.value === 0)
const label = computed(() => (LISTS.includes(listId.value) ? t(`sidebar.lists.${listId.value}`) : ['library', 'libcard'].includes(listId.value) ? libName.value : undefined))
// every widget has an icon of its own, which is all the compact sidebar shows of it
const icon = computed(() => ({ favorites: 'pin', libraries: 'books', quick: 'inbox', smart: 'layout-grid', folders: 'folder', tags: 'hash', locations: 'device-desktop', panels: 'puzzle', library: 'books', libcard: 'books', libswitcher: 'books' })[listId.value] || WIDGETS[props.id]?.icon || 'point')
// ---------------------------------------------------------------- a section's heading folds it
const sectionState = computed(() => {
  const l = L()
  l.widgetState ||= {}
  return (l.widgetState['list.' + props.id] ||= {})
})
const closed = computed(() => !!sectionState.value.collapsed && !q.value && !sideScoped())
// the filter's options, as the search's scope: only empty folders, only folders of some colours
const sideScopePop = ref()
// in the compact sidebar the options open inside its filter's popover, which another popover would close
const railScope = ref(false)
const clearSideScope = () => Object.assign(S.sideScope, { empty: null, colors: [] })
function toggleSection() {
  sectionState.value.collapsed = !sectionState.value.collapsed
  saveUi()
}
// the compact sidebar has no lists to narrow, so its filter lists what it finds above the box
const filterPop = ref()
// beside its button like the rail's hover menus, its bottom level with the button's, growing upwards as results come
function placeRailFilter() {
  const el = filterPop.value?.container
  const btn = document.getElementById('side-filter-btn')
  if (!el || !btn) return
  const r = btn.getBoundingClientRect()
  // on the right of the window it opens to the button's left
  const left = r.left > innerWidth / 2 ? Math.max(4, r.left - RAIL_POP_GAP - el.offsetWidth) : r.right + RAIL_POP_GAP
  el.style.insetInlineStart = el.style.left = left + window.scrollX + 'px'
  el.style.top = Math.max(4, r.bottom - el.offsetHeight) + window.scrollY + 'px'
}
const RAIL_POP_GAP = 6
// the popover aligns itself again whenever its content changes size: that alignment is this one
watch(filterPop, (p) => p && (p.alignOverlay = placeRailFilter))
// what the sidebar would show, as one list: with the options set, only the folders they let through
const railResults = computed(() => {
  const t = S.sideFilter.trim()
  const scoped = sideScoped()
  if (!t && !scoped) return []
  const hit = (name) => !t || fuzzy(name, t)
  const out = []
  if (!scoped) for (const s of S.sidebar.smart) if (hit(s.name)) out.push({ key: 's' + s.id, icon: s.icon || 'layout-grid', label: s.name, count: s.count, go: () => setScope('smart', s.id) })
  for (const f of S.sidebar.folders) if (hit(f.name) && folderInScope(f)) out.push({ key: 'f' + f.id, icon: f.icon || 'folder', color: f.color, label: f.name, count: f.count, go: () => setScope('folder', f.id) })
  if (!scoped) for (const g of S.sidebar.tags) if (g.count > 0 && hit(g.name)) out.push({ key: 't' + g.id, icon: 'bookmark', color: tagColor(g), label: g.name, count: g.count, go: () => setScope('tag', g.id) })
  if (!scoped) for (const r of allRoots.value) if (hit(r.label)) out.push({ key: 'r' + r.id, icon: rootIcon(r), color: r.color, label: r.label, count: r.asset_count, go: () => setScope('root', r.id) })
  return out.slice(0, 60)
})
function pickRail(r) {
  if (!r) return
  r.go()
  filterPop.value.hide()
}
// beside the compact sidebar a menu opens on hover or on a click, as Settings › Layout says
const railHover = () => rail.value && S.ui.railOpen !== 'click'
function railEnter(e) {
  if (railHover()) menu.value.hoverAt(e.currentTarget)
}
function railLeave() {
  if (railHover()) menu.value.leave()
}
function railClick(e) {
  if (!rail.value) menu.value.toggle(e)
  else if (!railHover()) menu.value.isOpen() ? menu.value.hide() : menu.value.hoverAt(e.currentTarget, true)
}
</script>

<template>
  <template v-if="id === 'b.menu'">
    <button class="icon-btn" v-tooltip.bottom="$t('sidebar.tips.menu')" @click="mainMenu.toggle($event)"><AppIcon name="menu-2" :size="18" /></button>
    <AppMenu ref="mainMenu" :model="mainItems" />
  </template>
  <template v-else-if="id === 'b.add'">
    <button class="icon-btn" v-tooltip.bottom="$t('sidebar.tips.add')" @click="addMenu.toggle($event)"><AppIcon name="plus" :size="18" /></button>
    <AppMenu ref="addMenu" :model="addItems" />
  </template>
  <template v-else-if="id === 'b.libswitch'">
    <button class="icon-btn" v-tooltip.bottom="withKey($t('sidebar.tips.switchLibrary'), 'library.choose')" @click="libMenu.toggle($event)"><AppIcon name="arrows-exchange" :size="18" /></button>
    <AppMenu ref="libMenu" :model="libItems" />
  </template>
  <button v-else-if="id === 'b.library'" class="icon-btn" v-tooltip.bottom="lay.libraryCompact ? $t('sidebar.tips.hideLibrary') : $t('sidebar.tips.compact')" @click="cycleLibrary"><AppIcon name="layout-sidebar" :size="18" /></button>

  <!-- header: main menu, add, switch library, sidebar state -->
  <div v-else-if="id === 'header'" class="p-header" :class="mode" data-window-drag>
    <button class="icon-btn" v-tooltip.bottom="$t('sidebar.tips.menu')" @click="mainMenu.toggle($event)"><AppIcon name="menu-2" :size="18" /></button>
    <template v-if="!rail">
      <div class="grow" />
      <button class="icon-btn" v-tooltip.bottom="$t('sidebar.tips.add')" @click="addMenu.toggle($event)"><AppIcon name="plus" :size="18" /></button>
      <button class="icon-btn" v-tooltip.bottom="withKey($t('sidebar.tips.switchLibrary'), 'library.choose')" @click="libMenu.toggle($event)"><AppIcon name="arrows-exchange" :size="18" /></button>
    </template>
    <button class="icon-btn" v-tooltip.bottom="lay.libraryCompact ? $t('sidebar.tips.hideLibrary') : $t('sidebar.tips.compact')" @click="cycleLibrary"><AppIcon name="layout-sidebar" :size="18" /></button>
    <AppMenu ref="mainMenu" :model="mainItems" />
    <AppMenu ref="addMenu" :model="addItems" />
    <AppMenu ref="libMenu" :model="libItems" />
  </div>

  <!-- icon with a menu: in a toolbar on click, in the compact sidebar on hover or on click -->
  <template v-else-if="!isList && !qItem && id !== 'libfilter'">
    <template v-if="!(rail && emptyMenu && !S.customize)">
      <button class="icon-btn rail-btn" :class="{ wide: mode === 'bar' && !isMenuForm }" :aria-label="label" v-tooltip.bottom="isMenuForm && !rail ? label : undefined" @click="railClick" @pointerenter="railEnter" @pointerleave="railLeave">
        <AppIcon :name="icon" :size="18" /><span v-if="mode === 'bar' && !isMenuForm">{{ label }}</span>
      </button>
      <AppMenu ref="menu" :model="barItems" />
    </template>
  </template>

  <template v-else-if="qItem">
    <template v-if="!qItem.hidden || S.customize">
      <button v-if="rail" class="icon-btn rail-btn" :class="{ on: isActive(qItem), warn: qItem.warn }" :data-drop="qItem.drop || undefined" v-tooltip.bottom="qItem.label" @click="clickQuick(qItem)"><AppIcon :name="qItem.icon" :size="18" /></button>
      <div v-else class="item" :class="{ active: isActive(qItem), warn: qItem.warn, dim: qItem.hidden }" :data-drop="qItem.drop || undefined" @click="clickQuick(qItem)" @contextmenu.prevent="qItem.kind === 'trash' && openRow($event, trashItems())">
        <AppIcon :name="qItem.icon" :size="18" />
        <span class="name">{{ qItem.label }}</span>
        <span v-if="qItem.count != null" class="count">{{ cnt(qItem.count) }}</span>
      </div>
    </template>
  </template>

  <template v-else-if="id === 'libfilter'">
    <div class="bottom" :class="{ bar: mode === 'bar' }">
      <template v-if="rail">
        <button id="side-filter-btn" class="icon-btn rail-btn" :class="{ on: sideScoped() }" v-tooltip.right="$t('sidebar.filter.tip')" @click="filterPop.toggle($event)"><AppIcon name="filter" :size="18" /></button>
        <!-- the same field and options as the open sidebar's, with what they find listed above it -->
        <Popover ref="filterPop" class="rail-filter" @show="placeRailFilter" @hide="(S.sideFilter = ''), (railScope = false)">
          <div v-if="S.sideFilter || sideScoped()" class="rail-results">
            <div v-for="r in railResults" :key="r.key" class="item" @click="pickRail(r)"><AppIcon :name="r.icon" :size="16" :style="{ color: r.color || undefined }" /><span class="name"><AppHighlight :text="r.label" :q="S.sideFilter" /></span><span class="count">{{ cnt(r.count) }}</span></div>
            <div v-if="!railResults.length" class="faint small none">{{ $t('sidebar.filter.none') }}</div>
          </div>
          <BrowseWidgetsSideScope v-if="railScope" class="rail-scope" />
          <AppSearchField id="side-filter" v-focus v-model="S.sideFilter" icon="filter" menu :menuOn="sideScoped()" :placeholder="$t('sidebar.filter.placeholder')" class="filter-box" @menu="railScope = !railScope" @menu-clear="clearSideScope" @keydown.enter="pickRail(railResults[0])" @keydown.esc="filterPop.hide()" />
        </Popover>
      </template>
      <template v-else>
        <AppSearchField id="side-filter" v-model="S.sideFilter" icon="filter" menu :menuOn="sideScoped()" :placeholder="$t('sidebar.filter.placeholder')" class="filter-box" :class="{ typed: !!S.sideFilter }" @menu="sideScopePop.toggle($event)" @menu-clear="clearSideScope" @keydown.esc="S.sideFilter = ''" />
        <Popover ref="sideScopePop"><BrowseWidgetsSideScope /></Popover>
      </template>
    </div>
  </template>

  <button v-else-if="id === 'libswitcher'" id="lib-switch" class="lib-switch" @click="libMenu.toggle($event)">
    <span class="lib-icon" :style="{ background: curLib.color || undefined }"><AppIcon :name="curLib.icon || 'books'" :size="14" /></span>
    <span class="ellipsis">{{ libName }}</span>
    <AppIcon name="selector" :size="14" class="faint" />
    <AppMenu ref="libMenu" :model="libItems" />
  </button>

  <button v-else-if="id === 'libcard'" class="lib-card" @click="libMenu.toggle($event)">
    <span class="lib-tile" :style="{ background: curLib.color || undefined }"><AppIcon :name="curLib.icon || 'books'" :size="20" /></span>
    <span class="lib-text"><span class="lib-name ellipsis">{{ libName }}</span><span class="lib-sum ellipsis">{{ $t('sidebar.libCard', { count: S.sidebar.counts?.all ?? 0, n: fmt(S.sidebar.counts?.all ?? 0), size: fmtSize(S.sidebar.counts?.size ?? 0) }) }}</span></span>
    <AppIcon name="selector" :size="14" class="faint" />
    <AppMenu ref="libMenu" :model="libItems" />
  </button>


  <template v-else-if="id === 'favorites'">
    <template v-if="favorites.length">
      <div class="head row"><span class="sec-toggle" @click="toggleSection"><span class="section-title">{{ $t('sidebar.sections.favorites', { n: favorites.length }) }}</span><AppIcon :name="closed ? 'caret-right-filled' : 'caret-down-filled'" :size="11" class="sec-caret" /></span><span class="grow" /></div>
      <div v-for="(f, i) in closed ? [] : favorites" :key="f.kind + f.id" class="fav-wrap" :data-drop="'fav:' + i">
        <div class="item" :class="{ active: favActive(f) }" :data-drop="f.drop || undefined" @click="openFav(f)" @pointerdown="dragFav($event, f, i)" @contextmenu.prevent="favMenu($event, f)">
          <span class="caret-space" />
          <AppIcon :name="f.icon" :size="18" :style="{ color: f.color || undefined }" />
          <span class="name"><AppHighlight :text="f.name" :q="q" /></span>
          <span v-if="f.count != null" class="count">{{ cnt(f.count) }}</span>
        </div>
      </div>
    </template>
  </template>

  <template v-else-if="id === 'libraries'">
    <div class="head row"><span class="sec-toggle" @click="toggleSection"><span class="section-title">{{ $t('sidebar.sections.libraries', { n: libraries.length }) }}</span><AppIcon :name="closed ? 'caret-right-filled' : 'caret-down-filled'" :size="11" class="sec-caret" /></span><span class="grow" />
      <button class="icon-btn mini" v-tooltip="$t('sidebar.tips.addLibrary')" @click="addLibMenu.toggle($event)"><AppIcon name="plus" :size="14" /></button></div>
    <AppMenu ref="addLibMenu" :model="libAddItems" />
    <div v-for="(l, i) in closed ? [] : libraries" :key="l.path" class="item" :class="{ active: l.current, dim: !l.exists }" :data-drop="'lib:' + i" v-tooltip.bottom="{ value: String(l.path), showDelay: 600 }" @click="l.exists && libClick(l)" @pointerdown="dragLib($event, l, i)" @contextmenu.prevent="libMenuFor($event, l)">
      <span class="caret-space" />
      <AppIcon :name="l.icon || 'books'" :size="18" :style="{ color: l.color || undefined }" />
      <span class="name">{{ l.name }}</span>
      <AppIcon v-if="l.openElsewhere" name="app-window" :size="14" class="faint" v-tooltip="$t('sidebar.tips.openElsewhere')" />
    </div>
  </template>

  <template v-else-if="id === 'smart'">
    <div class="head row" data-drop="smart-root"><span class="sec-toggle" @click="toggleSection"><span class="section-title">{{ $t('sidebar.sections.smart', { n: S.sidebar.smart.length }) }}</span><AppIcon :name="closed ? 'caret-right-filled' : 'caret-down-filled'" :size="11" class="sec-caret" /></span><span class="grow" />
      <button class="icon-btn mini" v-tooltip="$t('sidebar.tips.newSmart')" @click="newSmart()"><AppIcon name="plus" :size="14" /></button></div>
    <template v-for="{ sf, depth, kids } in closed ? [] : smart" :key="sf.id">
      <div class="item tree" :data-depth="depth" :class="{ active: sideSelected('smart', sf.id) }" :style="{ paddingLeft: treePx(TREE_PAD + depth * TREE_INDENT) }" :data-drop="'smart:' + sf.id"
           @click="sideClick($event, 'smart', sf.id, smartOrder)" @dblclick="(kids || sf.group_by) && toggleSmart(sf)" @pointerdown="dragSmart($event, sf)" @contextmenu.prevent="openRow($event, smartRowItems(sf))">
        <span v-if="kids || sf.group_by" class="caret" :style="{ left: treePx(TREE_PAD - CARET + depth * TREE_INDENT) }" @click.stop="toggleSmart(sf)" @pointerdown.stop><AppIcon :name="smartOpen[sf.id] ? 'chevron-down' : 'chevron-right'" :size="12" /></span>
        <AppIcon :name="sf.icon || 'layout-grid'" :size="18" />
        <InputText v-if="renaming && renaming.id === sf.id" v-focus v-model="renaming.name" class="field inline" @keydown.enter="finishRename" @keydown.esc="renaming = null" @blur="finishRename" @click.stop />
        <span v-else class="name"><AppHighlight :text="sf.name" :q="q" /></span>
        <span class="count">{{ cnt(sf.count) }}</span>
      </div>
      <template v-if="sf.group_by && smartOpen[sf.id]">
        <div v-for="b in (smartBucketsMap[sf.id] || []).filter((x) => !q || fuzzy(bucketLabel(sf, x), q))" :key="sf.id + ':' + b.value" class="item tree" :data-depth="depth + 1" :class="{ active: S.view === 'grid' && S.scope.kind === 'smartBucket' && S.scope.id === sf.id && S.scope.path === b.value }" :style="{ paddingLeft: treePx(TREE_PAD + (depth + 1) * TREE_INDENT) }" @click="setScope('smartBucket', sf.id, null, b.value)">
          <AppIcon :name="bucketIcon(sf)" :size="16" />
          <span class="name"><AppHighlight :text="bucketLabel(sf, b)" :q="q" /></span>
          <span class="count">{{ cnt(b.count) }}</span>
        </div>
      </template>
    </template>
  </template>

  <template v-else-if="id === 'folders'">
    <div class="head row" data-drop="folders-root"><span class="sec-toggle" @click="toggleSection"><span class="section-title">{{ $t('sidebar.sections.folders', { n: S.sidebar.folders.length }) }}</span><AppIcon :name="closed ? 'caret-right-filled' : 'caret-down-filled'" :size="11" class="sec-caret" /></span><span class="grow" />
      <button class="icon-btn mini" v-tooltip="withKey($t('sidebar.tips.newFolder'), 'folder.new')" @click="startNewFolder()"><AppIcon name="plus" :size="14" /></button></div>
    <BrowseWidgetsFolderNode v-for="f in closed ? [] : topFolders" :key="f.id" :folder="f" :depth="0" :filter="q" :tail="[]" />
    <div v-if="S.newFolder && S.newFolder.parent == null && !newFolderListed()" class="item">
      <span class="caret-space" /><AppIcon name="folder" :size="18" />
      <InputText v-if="S.newFolder.name == null" v-focus v-model="newName" class="field inline" :placeholder="$t('sidebar.folderName')" @keydown.enter="endNewFolder" @keydown.esc="newName = ''; S.newFolder = null" @blur="endNewFolder" />
      <span v-else class="name">{{ S.newFolder.name }}</span>
    </div>
  </template>

  <template v-else-if="id === 'tags'">
    <template v-if="S.sidebar.tags.length">
      <div class="head row"><span class="sec-toggle" @click="toggleSection"><span class="section-title">{{ $t('sidebar.sections.tags') }}</span><AppIcon :name="closed ? 'caret-right-filled' : 'caret-down-filled'" :size="11" class="sec-caret" /></span><span class="grow" /></div>
      <div v-for="t in closed ? [] : tags" :key="t.id" class="item" :class="{ active: S.scope.kind === 'tag' && S.scope.id === t.id }" :data-drop="'tag:' + t.id" @click="($event.ctrlKey || $event.metaKey) ? filterByTag(t.name, true) : setScope('tag', t.id)">
        <span class="caret-space" /><AppIcon name="bookmark" :size="16" :style="{ color: tagColor(t) || undefined }" /><span class="name"><AppHighlight :text="t.name" :q="q" /></span><span class="count">{{ cnt(t.count) }}</span>
      </div>
    </template>
  </template>

  <template v-else-if="id === 'locations'">
    <div class="head row"><span class="sec-toggle" @click="toggleSection"><span class="section-title">{{ $t('sidebar.sections.locations', { n: roots.length }) }}</span><AppIcon :name="closed ? 'caret-right-filled' : 'caret-down-filled'" :size="11" class="sec-caret" /></span><span class="grow" />
      <button class="icon-btn mini" v-tooltip="$t('sidebar.tips.addLocation')" @click="addPick('folder', false)"><AppIcon name="plus" :size="14" /></button></div>
    <template v-for="r in closed ? [] : roots" :key="r.id">
    <div class="item" :class="{ active: sideSelected('root', r.id), warn: !r.online }" :style="{ paddingLeft: treePx(TREE_PAD) }" :data-drop="'root:' + r.id"
         v-tooltip.bottom="{ value: r.path + (S.scanning[r.id] ? '\n' + $t('sidebar.scanning') : r.online ? '' : '\n' + $t('sidebar.offline')), showDelay: 500 }"
         @click="sideClick($event, 'root', r.id, rootOrder)" @dblclick="!q && canOpen(r) && toggleRoot(r)" @pointerdown="!q && dragRoot($event, r)" @contextmenu.prevent="openRow($event, rootRowItems(r))">
      <span v-if="!q && canOpen(r)" class="caret" :style="{ left: treePx(TREE_PAD - CARET) }" @click.stop="toggleRoot(r)" @pointerdown.stop><AppIcon :name="rootOpen[r.id] ? 'chevron-down' : 'chevron-right'" :size="12" /></span>
      <AppIcon :name="rootIcon(r)" :size="18" :class="{ spinning: S.scanning[r.id] }" :style="{ color: r.online ? r.color || undefined : undefined }" />
      <span class="name"><AppHighlight :text="r.label" :q="q" /></span>
      <span class="count">{{ cnt(r.asset_count) }}</span>
    </div>
    <template v-if="!q && rootOpen[r.id]">
      <div v-for="d in dirRows(r)" :key="d.path" class="item" data-drag-with :data-dir="r.id + ':' + d.path" :class="{ active: dirActive(r, d) }" :style="{ paddingLeft: treePx(TREE_PAD + d.depth * TREE_INDENT) }" @click="openDir(r, d)" @dblclick="d.kids && (rootOpen[r.id + ':' + d.path] = !rootOpen[r.id + ':' + d.path])" @contextmenu.prevent="openRow($event, dirItems(r, d))">
        <span v-if="d.kids" class="caret" :style="{ left: treePx(TREE_PAD - CARET + d.depth * TREE_INDENT) }" @click.stop="rootOpen[r.id + ':' + d.path] = !rootOpen[r.id + ':' + d.path]"><AppIcon :name="rootOpen[r.id + ':' + d.path] ? 'chevron-down' : 'chevron-right'" :size="12" /></span>
        <AppIcon name="folder" :size="18" />
        <span class="name">{{ d.name }}</span>
        <span class="count">{{ cnt(d.count) }}</span>
      </div>
    </template>
    </template>
  </template>

  <template v-else-if="id === 'panels'">
    <template v-for="p in S.contrib.panels" :key="`${p.plugin}/${p.id}`">
      <div class="head row"><span class="section-title grow">{{ p.title }}</span></div>
      <div v-if="p.text" class="faint small pad">{{ p.text }}</div>
      <div v-for="f in p.filters" :key="f.label" class="item" @click="usePanelFilter(f)"><span class="caret-space" /><AppIcon name="filter" /><span class="name">{{ f.label }}</span></div>
    </template>
  </template>

  <AppMenu ref="rowMenu" :model="() => rowItems" />
</template>

<style scoped>
.p-header { display: flex; align-items: center; gap: calc(0.28rem * var(--g)); height: round(3.36rem, 1px); padding: 0 calc(0.7rem * var(--g)); flex: none; }
.p-header.rail { flex-direction: column; height: auto; padding: calc(0.7rem * var(--g)) 0 calc(0.42rem * var(--g)); }
.p-header.bar { height: auto; padding: 0; }
.rail-btn { flex: none; align-self: center; width: round(2.38rem, 1px); height: round(2.38rem, 1px); margin: 1px auto; display: flex; }
.rail-btn.wide { width: auto; padding: 0 calc(0.56rem * var(--g)); margin: 0; }
.rail-btn.warn { color: var(--warning); }
.lib-switch { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); margin: 0.14rem calc(0.84rem * var(--g)) calc(0.56rem * var(--g)); padding: calc(0.28rem * var(--g)) calc(0.42rem * var(--g)); border-radius: calc(6px * var(--r)); border: 0; background: transparent; color: var(--text); font: inherit; font-weight: 500; font-size: 1rem; cursor: pointer; max-width: calc(100% - 1.68rem); flex: none; }
.lib-switch:hover { background: var(--hover); }
.lib-card { display: flex; align-items: center; gap: calc(0.7rem * var(--g)); margin: 0.14rem calc(0.7rem * var(--g)) calc(0.56rem * var(--g)); padding: calc(0.42rem * var(--g)) calc(0.56rem * var(--g)); border-radius: calc(8px * var(--r)); border: 0; background: transparent; color: var(--text); font: inherit; text-align: left; cursor: pointer; flex: none; min-width: 0; }
.lib-card:hover { background: var(--hover); }
.lib-tile { flex: none; display: inline-flex; align-items: center; justify-content: center; width: round(2.38rem, 1px); height: round(2.38rem, 1px); border-radius: calc(8px * var(--r)); background: var(--accent); color: var(--accent-text); }
.lib-text { flex: 1; min-width: 0; display: flex; flex-direction: column; line-height: 1.3; }
.lib-name { font-weight: 600; font-size: 1rem; }
.lib-sum { font-size: 0.85rem; color: var(--text-faint); }
.lib-icon { display: inline-flex; align-items: center; justify-content: center; width: round(1.54rem, 1px); height: round(1.54rem, 1px); border-radius: calc(6px * var(--r)); background: var(--accent); color: var(--accent-text); }
.item { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); height: round(2.1rem * var(--row), 1px); padding: 0 calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); color: var(--text); cursor: default; font-size: 1rem; }
.item .ico { color: var(--text-muted); }
.item:hover { background: var(--hover); }
.item.active { background: var(--active); }
.item.dim { opacity: .5; }
.item.warn .ico, .item.warn .name { color: var(--warning); }
.item .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.caret-space { display: none; }
.item { position: relative; }
.caret { position: absolute; top: 0; bottom: 0; width: round(0.98rem, 1px); display: inline-flex; align-items: center; justify-content: center; color: var(--text-faint); cursor: pointer; }
.head { margin: calc(1.12rem * var(--g)) 0 calc(0.28rem * var(--g)); padding: 0 0 0 0.14rem; height: round(1.68rem, 1px); }
.sec-toggle { display: inline-flex; align-items: center; gap: calc(0.35rem * var(--g)); height: round(1.68rem, 1px); padding: 0 calc(0.42rem * var(--g)); border-radius: calc(6px * var(--r)); cursor: default; }
.sec-toggle { white-space: nowrap; min-width: 0; }
.sec-toggle:hover { background: var(--hover); }
.sec-toggle:hover .section-title { color: var(--text); }
.sec-caret { visibility: hidden; color: var(--text-muted); }
.sec-toggle:hover .sec-caret { visibility: visible; }
.icon-btn.mini { min-width: round(1.54rem, 1px); height: round(1.54rem, 1px); opacity: 0.8; }
.field.inline { padding: 0.14rem calc(0.42rem * var(--g)); height: round(1.68rem, 1px); }
.bottom { padding: calc(0.56rem * var(--g)) calc(0.7rem * var(--g)) calc(0.84rem * var(--g)); flex: none; }
/* in a row of widgets it takes the room left over, and gives it back when the sidebar narrows */
.bottom.bar { flex: 1 1 0; min-width: 0; padding: 0; }
.filter-box { flex: none; }
.rail .bottom { position: relative; display: flex; justify-content: center; padding: calc(0.42rem * var(--g)) 0; }
.rail .bottom.bar { flex: none; }
.rail-filter { width: round(19.58rem, 1px); }
.rail-scope { width: auto; margin: 0 calc(0.28rem * var(--g)) calc(0.7rem * var(--g)); padding-bottom: calc(0.7rem * var(--g)); border-bottom: 1px solid var(--border); }
.rail-results { max-height: min(29.37rem, 55vh); overflow-y: auto; margin-bottom: calc(0.42rem * var(--g)); }
.rail-results .none { padding: calc(0.42rem * var(--g)) calc(0.56rem * var(--g)); }
.small { font-size: 0.85rem; }
.pad { padding: 0 calc(0.56rem * var(--g)) calc(0.28rem * var(--g)); }
</style>

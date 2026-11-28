// The sidebar's lists and what selecting in them does.
import { reactive } from 'vue'
import { call, errorText } from '../platform/ipc.js'
import { setFormatsOff, setPagedExts } from '../formats/index.js'
import { t } from '../shared/i18n.js'
import { S, folderOpen, rootOpen } from '../app/state.js'
import { toast, run, playNotifySound } from '../app/notify.js'
import { selectOnly } from '../library/selection.js'
import { invalidateItems, searched, indexOf } from '../library/results.js'
import { setScope, resetFilters } from '../library/navigation.js'
import { edit } from '../library/edits.js'
import { L } from '../layout/arrangement.js'
import { localizeContrib, localizeFields } from '../plugins/pluginText.js'

/** Settings is a window of its own. */
/** Locations the user added: files linked one by one get a hidden location per folder, the library's own store is reached through its folders. */
/** The colours a folder, location or library can be given. */
/** A folder the sidebar filter's options let through; its name is matched separately. */
export function folderInScope(f) {
  const o = S.sideScope
  // empty: true keeps only empty folders, false leaves them out
  return (o.empty == null || (f.count === 0) === o.empty) && (!o.colors.length || o.colors.includes(f.color || null))
}
// Cached subfolder lists per location id (`[[path, count], ...]`), shared by the sidebar and the grid's Subfolders section.
export const rootDirsMap = reactive({})
const rootDirsStamp = new Map()
const rootDirsPending = new Map()
const rootStamp = (id) => {
  const r = S.sidebar.roots.find((x) => x.id === id)
  return r ? `${r.asset_count}:${r.last_scan}` : ''
}
// counts up when the library changes, so a list asked for in the one before is dropped
let rootDirsGen = 0
export function clearRootDirs() {
  rootDirsGen++
  for (const k of Object.keys(rootDirsMap)) delete rootDirsMap[k]
  rootDirsStamp.clear()
  rootDirsPending.clear()
}
export async function loadRootDirs(id, force = false) {
  const stamp = rootStamp(id)
  if (!force && rootDirsMap[id] && rootDirsStamp.get(id) === stamp) return rootDirsMap[id]
  if (rootDirsPending.has(id)) return rootDirsPending.get(id)
  const gen = rootDirsGen
  const p = call('root_dirs', { id })
    .then((list) => {
      if (gen !== rootDirsGen) return list
      rootDirsMap[id] = list
      rootDirsStamp.set(id, stamp)
      return list
    })
    .catch(() => rootDirsMap[id] || [])
    .finally(() => rootDirsPending.delete(id))
  rootDirsPending.set(id, p)
  return p
}
/** Direct child directories inside a location (at root when `base` is empty), sorted by name. */
export function directRootDirs(rootId, base = '') {
  const list = rootDirsMap[rootId] || []
  const prefix = base ? base.replace(/^\/+|\/+$/g, '') + '/' : ''
  const out = []
  for (const [p, count] of list) {
    if (prefix) {
      if (!p.startsWith(prefix)) continue
      const rest = p.slice(prefix.length)
      if (!rest || rest.includes('/')) continue
      out.push({ path: p, name: rest, count })
    } else if (p && !p.includes('/')) {
      out.push({ path: p, name: p, count })
    }
  }
  return out
}
/** Whether any of these folders has folders inside it, so Include subfolders changes what shows. */
export const hasSubfolders = (ids) => S.sidebar.folders.some((f) => f.parent_id != null && ids.includes(f.parent_id))
/** The view shows a folder, several folders, or a location directory with subfolders inside. */
export const subfoldersApply = () => {
  if (S.scope.kind === 'folder') return hasSubfolders([S.scope.id])
  if (S.scope.kind === 'folders') return hasSubfolders(S.scope.ids || [])
  if (S.scope.kind === 'root') return directRootDirs(S.scope.id, '').length > 0
  if (S.scope.kind === 'rootdir') return directRootDirs(S.scope.id, S.scope.path || '').length > 0
  return false
}
export const sideScoped = () => S.sideScope.empty != null || S.sideScope.colors.length > 0
// each look colour's id and value; its name is `sidebar.colors.<id>` in the catalogs
export const LOOK_COLORS = [['none', null], ['accent', 'var(--accent)'], ['red', '#e5686b'], ['orange', '#ef9a3d'], ['yellow', '#e9c944'], ['green', '#4fc57c'], ['teal', '#38bdc9'], ['blue', '#4a90e2'], ['purple', '#a98bea'], ['pink', '#e784b4']]
export const colorName = (id) => t(`sidebar.colors.${id}`)
const DRIVE_ICONS = { network: 'cloud-network', removable: 'usb', read_only_media: 'disc' }
/** A location's own icon, else one for the kind of drive it is on; offline always shows as such, and a scan under way as a spinner. */
export const rootIcon = (r) => (S.scanning[r.id] ? 'loader-2' : !r.online ? 'plug-connected-x' : r.icon || DRIVE_ICONS[r.hint] || 'device-desktop')

export function listedRoot(r) {
  return r.listed
}

// the answers as Rust gives them, so plugins' text can be shown again in another language
let rawFields = null
let rawContrib = null

// each refresh counts up, so an answer overtaken by a later one, or from the library before a switch, is dropped
let sidebarSeq = 0
export async function refreshSidebar() {
  if (!S.libOpen) return
  const seq = ++sidebarSeq
  try {
    const before = rootAccessKey()
    const sidebar = await call('sidebar')
    if (seq !== sidebarSeq || sidebar.library !== S.startup.current) return
    rawFields = sidebar.fields
    S.sidebar = { ...sidebar, fields: localizeFields(rawFields) }
    const ids = new Set(sidebar.roots.map((r) => String(r.id)))
    for (const k of Object.keys(rootDirsMap)) {
      if (!ids.has(k)) {
        delete rootDirsMap[k]
        rootDirsStamp.delete(Number(k))
      } else if (rootDirsStamp.get(Number(k)) !== rootStamp(Number(k))) {
        loadRootDirs(Number(k), true)
      }
    }
    // cached items carry their location's read-only flag
    if (before !== null && before !== rootAccessKey()) invalidateItems([...S.items.keys()])
    const contrib = await call('contributions')
    if (seq !== sidebarSeq) return
    rawContrib = contrib
    S.contrib = localizeContrib(rawContrib)
    setFormatsOff(S.contrib.disabledExts)
    setPagedExts(S.contrib.pageThumbExts)
  } catch (e) {
    toast(errorText(e), 'error')
  }
}

/** Plugins' text in the language the window shows now. */
export function retranslatePlugins() {
  if (rawFields && S.sidebar) S.sidebar.fields = localizeFields(rawFields)
  if (rawContrib) S.contrib = localizeContrib(rawContrib)
}

function rootAccessKey() {
  return S.sidebar?.roots ? S.sidebar.roots.map((r) => `${r.id}:${r.access}`).join() : null
}

/** Open or close every folder (or the given ones): closed if any were open. */
export function setAllFoldersOpen(ids = null) {
  const list = ids || S.sidebar.folders.map((f) => f.id)
  const open = !list.some((id) => folderOpen[id])
  for (const id of list) folderOpen[id] = open
}

// ------------------------------------------------------------------ sidebar selection

/** Folders in the order they appear, for Shift + click. */
export function visibleFolderOrder() {
  const out = []
  const walk = (parent) => {
    for (const f of S.sidebar.folders) {
      if ((f.parent_id ?? null) !== parent) continue
      out.push(f.id)
      if (folderOpen[f.id]) walk(f.id)
    }
  }
  walk(null)
  return out
}

function showSideSelection() {
  const { kind, ids } = S.sideSel
  if (!ids.length) return setScope('all')
  if (ids.length === 1 || kind === 'smart') return setScope(kind, ids[ids.length - 1])
  setScope(kind === 'folder' ? 'folders' : 'roots', 0, [...ids])
}

/**
 * A click on a folder, smart folder or location:
 * Ctrl adds the current one and the clicked one, or removes the clicked one unless it is the last;
 * Shift adds everything from the current one to the clicked one to what is already selected.
 * Ctrl or Shift selections show a fresh view, without the search or filters.
 */
export function sideClick(e, kind, id, order) {
  S.sideFocus = true
  const sel = S.sideSel
  const current = sel.kind === kind ? sel.anchor : S.scope.kind === kind ? S.scope.id : null
  const base = sel.kind === kind ? sel.ids : S.scope.kind === kind ? [S.scope.id] : []
  const fresh = () => {
    S.search = ''
    resetFilters()
  }
  if (e.ctrlKey || e.metaKey) {
    if (base.includes(id)) {
      if (base.length < 2) return
      S.sideSel = { kind, ids: base.filter((x) => x !== id), anchor: current }
    } else {
      S.sideSel = { kind, ids: [...base, id], anchor: id }
    }
    fresh()
    return showSideSelection()
  }
  if (e.shiftKey && order && current != null) {
    const a = order.indexOf(current)
    const b = order.indexOf(id)
    if (a >= 0 && b >= 0) {
      const range = order.slice(Math.min(a, b), Math.max(a, b) + 1)
      S.sideSel = { kind, ids: [...base, ...range.filter((x) => !base.includes(x))], anchor: current }
      fresh()
      return showSideSelection()
    }
  }
  S.sideSel = { kind, ids: [id], anchor: id }
  setScope(kind, id)
}

export function sideSelected(kind, id) {
  const s = S.sideSel
  if (s.kind === kind && s.ids.length) return s.ids.includes(id)
  return S.view === 'grid' && S.scope.kind === kind && S.scope.id === id
}

/** The ids a right-click on this row should act on. */
export function sideTargets(kind, id) {
  const s = S.sideSel
  if (s.kind === kind && s.ids.includes(id) && s.ids.length > 1) return [...s.ids]
  S.sideSel = { kind, ids: [id], anchor: id }
  return [id]
}

/** Delete the selected folders, smart folders or locations (asks first). */
export function deleteSide(kind, ids) {
  const n = ids.length
  const nameOf = (id) => (kind === 'folder' ? S.sidebar.folders : kind === 'smart' ? S.sidebar.smart : S.sidebar.roots).find((x) => x.id === id)
  const one = n === 1 ? nameOf(ids[0])?.name || nameOf(ids[0])?.label : null
  const leave = () => {
    S.sideSel = { kind: null, ids: [], anchor: null }
    if ((S.scope.kind === kind && ids.includes(S.scope.id)) || S.scope.ids?.some((i) => ids.includes(i))) setScope('all')
  }
  if (kind === 'folder') {
    // nothing in it and no subfolders: gone at once, as there is nothing to lose
    if (ids.every((id) => nameOf(id)?.count === 0 && !S.sidebar.folders.some((f) => f.parent_id === id))) {
      ;(async () => {
        for (const id of ids) await edit.folder('delete', { id })
        leave()
      })()
      return
    }
    S.dialogs.confirm = {
      title: one ? t('sidebar.delete.folder', { name: one }) : t('sidebar.delete.folders', { count: n }),
      body: t('sidebar.delete.folderBody'),
      okLabel: t('sidebar.delete.ok'),
      ok: async () => {
        for (const id of ids) await edit.folder('delete', { id })
        leave()
      },
    }
  } else if (kind === 'smart') {
    S.dialogs.confirm = {
      title: one ? t('sidebar.delete.smart', { name: one }) : t('sidebar.delete.smarts', { count: n }),
      body: t('sidebar.delete.smartBody'),
      okLabel: t('sidebar.delete.ok'),
      ok: async () => {
        for (const id of ids) await edit.smart('delete', { id })
        leave()
        playNotifySound('deleteFolder')
      },
    }
  } else {
    S.dialogs.confirm = {
      title: one ? t('sidebar.delete.location', { name: one }) : t('sidebar.delete.locations', { count: n }),
      body: t('sidebar.delete.locationBody'),
      okLabel: t('sidebar.delete.remove'),
      ok: async () => {
        for (const id of ids) await run('location.remove', { id })
        leave()
      },
    }
  }
}

/** Move folders under `parent` (null = the top), before `before` when given. A folder of the same name already there asks first: merge into it, keep both (the moved one is numbered) or cancel. */
export async function moveFolders(ids, parent, before) {
  for (const id of ids) {
    const f = S.sidebar.folders.find((x) => x.id === id)
    if (!f) continue
    const name = f.name.toLowerCase()
    const clash = (f.parent_id ?? null) !== parent && S.sidebar.folders.find((x) => x.id !== id && (x.parent_id ?? null) === parent && x.name.toLowerCase() === name)
    const how = clash
      ? await new Promise((resolve) => {
          S.dialogs.choice = {
            title: t('sidebar.clash.title', { name: clash.name }),
            body: t('sidebar.clash.body', { name: f.name }),
            choices: [
              { label: t('sidebar.clash.merge'), value: 'merge', severity: 'primary' },
              { label: t('sidebar.clash.keep'), value: 'keep', severity: 'secondary' },
            ],
            pick: resolve,
            cancel: () => resolve(null),
          }
        })
      : 'keep'
    if (how === 'merge') await edit.folder('merge', { id, parent: clash.id })
    else if (how && before !== undefined) await edit.folder('place', { id, parent, before })
    else if (how) await edit.folder('move', { id, parent })
  }
}

/** Shows a place (`{ kind: 'folder' | 'root' | 'rootdir' | 'all', id, path }`) with the item selected in it, scrolled into view. */
export async function showItemIn(id, at) {
  if (at.kind === 'folder') {
    for (let p = S.sidebar.folders.find((f) => f.id === at.id)?.parent_id; p != null; p = S.sidebar.folders.find((f) => f.id === p)?.parent_id) folderOpen[p] = true
    setScope('folder', at.id)
  } else if (at.kind === 'rootdir') {
    // the location's tree opens down to the folder
    rootOpen[at.id] = true
    const parts = at.path.split('/')
    for (let i = 1; i < parts.length; i++) rootOpen[at.id + ':' + parts.slice(0, i).join('/')] = true
    setScope('rootdir', at.id, null, at.path)
  } else if (at.kind === 'root') setScope('root', at.id)
  else setScope('all')
  await searched()
  const i = indexOf(id)
  if (i == null) return toast(t('sidebar.hiddenByFilters'), 'info')
  selectOnly(id, i)
  S.revealAt = i
}

/** Show the name box for a new folder. */
export function startNewFolder(parent = null) {
  const l = L()
  if (!l.library || l.libraryCompact) {
    l.library = true
    l.libraryCompact = false
  }
  if (parent != null) folderOpen[parent] = true
  S.newFolder = { parent, name: null }
}

const NEW_FOLDER_SHOWN_MS = 3000
/** The name box was left: the folder is made, and its name stays where the box was until the sidebar lists it. */
export async function finishNewFolder(name) {
  const t = S.newFolder
  name = (name || '').trim()
  if (!t || t.name != null) return
  if (!name) {
    S.newFolder = null
    return
  }
  S.newFolder = { parent: t.parent, name }
  const done = () => S.newFolder?.name === name && (S.newFolder = null)
  try {
    await edit.folder('create', { name, parent: t.parent })
  } catch {
    done()
  }
  setTimeout(done, NEW_FOLDER_SHOWN_MS)
}
/** Whether the folder just named is listed yet. */
export const newFolderListed = () => !!S.newFolder?.name && S.sidebar.folders.some((f) => (f.parent_id ?? null) === S.newFolder.parent && f.name === S.newFolder.name)
let sidebarTimer = null
/** Refreshes the sidebar `ms` after the first of the changes asking for it: a steady stream of them, as a scan sends, refreshes it every `ms` rather than never. */
export function refreshSidebarSoon(ms) {
  if (sidebarTimer) return
  sidebarTimer = setTimeout(() => {
    sidebarTimer = null
    refreshSidebar()
  }, ms)
}

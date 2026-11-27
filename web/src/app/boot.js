// A window's start, the switch to another library, and what it picks up again when it takes the focus.
import { markRaw, nextTick, toRaw } from 'vue'
import { getCurrentWindow, getCurrentWebview } from '../platform/windowFrame.js'
import { t } from '../shared/i18n.js'
import { call, on, saidText } from '../platform/ipc.js'
import { setMediaBase } from '../platform/fileUrls.js'
import { loadActions } from '../actions/actions.js'
import { serveWindowApi } from './windowRequests.js'
import { S, folderOpen } from './state.js'
import { toast } from './notify.js'
import { clearSelection, loadDetail, selectOnly } from '../library/selection.js'
import { loadSavedFilters } from '../filters/query.js'
import { resetResults, requestSearch, searchSoon, invalidateItems } from '../library/results.js'
import { edit } from '../library/edits.js'
import { watchWindowShape, L } from '../layout/arrangement.js'
import { loadThemeFile, refreshTheme } from './appearance.js'
import { pickUpUi, openPrefs, withLibraryUi, sendUi, loadUi, receiveUi, WINDOW_ID, markReceived } from './preferences.js'
import { refreshSidebar, refreshSidebarSoon, clearRootDirs } from '../sidebar/sidebar.js'
import { refreshLibraries, openLibrary, saveLinks, addPaths, pollStatus } from '../library/lifecycle.js'

window.addEventListener('focus', async () => {
  await pickUpUi()
  if (S.ui.themeFile) loadThemeFile()
  // shortcuts of Actions may have been changed in Settings
  if (S.libOpen && !S.prefsWindow) loadActions()
})

/** The library opened with problems (damaged database, rejected restore): say so, offer the backups. */
function offerRestore(problems) {
  if (!problems?.length || S.prefsWindow) return
  S.dialogs.confirm = {
    title: t('app.restore.title'),
    body: t('app.restore.body', { problems: problems.join(' ') }),
    ok: () => openPrefs('backup'),
  }
}

function showWarnings(warnings) {
  if (!warnings?.length || S.prefsWindow) return
  toast(warnings.join(' '), 'error')
}

let openingPath = null
let openingPromise = null
export function afterOpen(target = null) {
  if (target && openingPath === target && openingPromise) return openingPromise
  openingPath = target
  const p = runAfterOpen().finally(() => {
    if (openingPromise === p) {
      openingPromise = null
      openingPath = null
    }
  })
  openingPromise = p
  return p
}

async function runAfterOpen() {
  refreshLibraries()
  const st = await call('startup')
  setMediaBase(st.mediaBase)
  S.startup = { ...S.startup, ...st, current: st.current }
  S.libOpen = !!st.current
  offerRestore(st.problems)
  showWarnings(st.warnings)
  resetResults()
  clearRootDirs()
  clearSelection()
  S.scanning = {}
  S.scope = { kind: 'all', id: 0 }
  S.sideSel = { kind: null, ids: [], anchor: null }
  if (!S.libOpen) {
    S.viewer = null
    S.view = 'grid'
    S.sidebar = { counts: {}, folders: [], tags: [], tagGroups: [], smart: [], roots: [], fields: [], history: [], favorites: [] }
    return
  }
  // this library's own layout, which the app's other windows take too
  if (S.ui.layoutPerLibrary && !S.prefsWindow) {
    S.ui = await withLibraryUi(toRaw(S.ui))
    sendUi()
  }
  await refreshSidebar()
  await loadSavedFilters()
  await loadActions()
  refreshTheme()
  requestSearch()
}

// ------------------------------------------------------------------ boot

export async function boot() {
  const qs = new URLSearchParams(location.search)
  const plugin = qs.get('plugin')
  const tool = qs.get('tool')
  const prefsTab = qs.get('prefs')
  const only = qs.get('items')
  const secondary = !!(plugin || tool || prefsTab || only)
  const [st, saved, toolInfo, pluginInfo] = await Promise.all([
    call('startup'),
    call('ui_settings_load').catch(() => null),
    tool ? call('tool_window_info').catch(() => null) : Promise.resolve(null),
    plugin ? call('plugin_window_info').catch(() => null) : Promise.resolve(null),
  ])
  setMediaBase(st.mediaBase)
  S.startup = st
  S.libOpen = !!st.current
  if (plugin) S.pluginWindow = pluginInfo ? { plugin: pluginInfo.plugin, page: pluginInfo.page, title: pluginInfo.title, run: pluginInfo.run } : { plugin, page: 'index.html', title: plugin, run: {} }
  if (tool) S.toolWindow = { tool, run: toolInfo?.run || {}, seq: toolInfo ? 1 : 0, drop: null, over: null }
  if (prefsTab) {
    S.prefsWindow = true
    S.dialogs.prefsTab = prefsTab
    S.dialogs.prefs = true
  }
  offerRestore(st.problems)
  showWarnings(st.warnings)
  try {
    if (saved) S.ui = loadUi(JSON.parse(saved))
  } catch {
    /* defaults */
  }
  S.ui = await withLibraryUi(toRaw(S.ui))
  S.systemDark = window.matchMedia('(prefers-color-scheme: dark)').matches
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
    S.systemDark = e.matches
    refreshTheme()
  })
  refreshTheme()
  watchWindowShape()
  // another window of this program changed a setting
  on('ui-sync', (m) => {
    if (m.from === WINDOW_ID) return
    S.ui = receiveUi(m.json)
    markReceived()
    refreshTheme()
    // a plugin may have been switched
    refreshSidebarSoon(250)
  })
  on('prefs-tab', (tab) => {
    if (S.prefsWindow) S.dialogs.prefsTab = tab
  })
  // what this or another window of the app changed that every window shows
  on('event', (e) => {
    if (e.name === 'library.listChanged') refreshLibraries()
  })
  on('library-switched', (path) => {
    if (S.startup.current !== path) afterOpen(path)
  })
  on('lib', (ev) => {
    if (ev.kind === 'changed') {
      if (ev.ids?.length) invalidateItems(ev.ids)
      if (ev.ids?.includes(S.selected[S.selected.length - 1])) loadDetail()
      if (ev.structure) {
        refreshSidebarSoon(250)
      }
      // what's shown follows every change; a sort set to random would deal again, so it waits
      if (ev.requery && L().sort.key !== 'random') searchSoon(150)
    } else if (ev.kind === 'thumbs') {
      invalidateItems(ev.ids)
    } else if (ev.kind === 'scan') {
      if (ev.done) delete S.scanning[ev.location]
      else S.scanning[ev.location] = true
    } else if (ev.text) {
      toast(saidText(ev.said, ev.text), ev.kind === 'error' ? 'error' : 'info', undefined, null, ev.kind === 'error' ? 'error' : 'import')
    }
  })
  // files dragged in from outside: show where they would land, and open a folder held over
  let lastFolderEl = null
  let expandTimer = null
  const folderAt = (p) => {
    if (!p?.position) return null
    const r = window.devicePixelRatio || 1
    const el = document.elementFromPoint(p.position.x / r, p.position.y / r)
    return el?.closest?.('[data-drop^="folder:"]') || null
  }
  const markFolder = (el) => {
    if (el === lastFolderEl) return
    lastFolderEl?.classList.remove('drop-hover')
    lastFolderEl = el
    el?.classList.add('drop-hover')
    clearTimeout(expandTimer)
    if (el) {
      const id = Number(el.dataset.drop.split(':')[1])
      expandTimer = setTimeout(() => (folderOpen[id] = true), 600)
    }
    const here = S.scope.kind === 'folder' && S.view === 'grid' ? S.scope.id : null
    const id = el ? Number(el.dataset.drop.split(':')[1]) : here
    S.fileDrag = { ...S.fileDrag, folder: id != null ? S.sidebar.folders.find((f) => f.id === id)?.name || null : null }
  }
  const WEB_ADDRESS = /^https?:\/\//i
  const isLibrary = (paths) => paths?.length === 1 && /\.curio[\\/]?$/i.test(paths[0])
  const endFileDrag = () => {
    clearTimeout(expandTimer)
    lastFolderEl?.classList.remove('drop-hover')
    lastFolderEl = null
    S.fileDrag = null
  }
  const enter = (p) => {
    lastFolderEl = null
    S.fileDrag = { folder: null, library: isLibrary(p?.paths) }
    if (S.pluginWindow || S.toolWindow) {
      // the window comes to the front, so what is dropped on it is seen landing there
      getCurrentWindow().setFocus().catch(() => {})
      return
    }
    const el = folderAt(p)
    if (el) markFolder(el)
    else {
      const here = S.scope.kind === 'folder' && S.view === 'grid' ? S.scope.id : null
      S.fileDrag.folder = here != null ? S.sidebar.folders.find((f) => f.id === here)?.name || null : null
    }
  }
  const drop = (p) => {
    const el = folderAt(p) || lastFolderEl
    endFileDrag()
    // what a browser drops as an address: Linux hands it on among the paths, Windows apart from them
    const urls = [...(p?.urls || []), ...(p?.paths || []).filter((x) => WEB_ADDRESS.test(x))]
    const paths = (p?.paths || []).filter((x) => !WEB_ADDRESS.test(x))
    if (!paths.length && !urls.length) return
    // a single library dropped on the window opens rather than being imported
    if (isLibrary(paths)) return openLibrary(paths[0])
    if (S.pluginWindow) {
      if (!paths.length) return toast(t('app.dropLinksOnLibrary'), 'warn')
      return import('../plugins/bridge.js').then((b) => b.dropFilesOnWindow(paths, p.position))
    }
    if (S.toolWindow) {
      if (!paths.length) return toast(t('app.dropLinksOnLibrary'), 'warn')
      return import('../tools/tools.js').then((x) => x.dropFilesOnTool(paths, p.position))
    }
    const folder = el ? Number(el.dataset.drop.split(':')[1]) : S.scope.kind === 'folder' && S.view === 'grid' ? S.scope.id : null
    if (urls.length) saveLinks(urls, folder)
    if (paths.length) p.made ? addPaths(paths, true, folder) : dropOwnOrAdd(paths, folder)
  }
  // items of this library dragged out and back, over a window standing over this one, are not added again: on a folder they join it
  const dropOwnOrAdd = async (paths, folder) => {
    const known = await call('items_at_paths', { paths }).catch(() => ({ ids: [], rest: paths }))
    if (known.ids.length && folder != null) edit.folder('add', { id: folder, ids: known.ids })
    // copied in, a folder with all its subfolders: locations are made only with Add location
    if (known.rest.length) addPaths(known.rest, true, folder)
  }
  const overPage = (p) => (S.toolWindow ? import('../tools/tools.js').then((x) => x.dragOverTool(p?.position)) : S.pluginWindow && import('../plugins/bridge.js').then((b) => b.dragOverWindow(p?.position)))
  const fileDrag = (p) => {
    if (p.type === 'enter') (enter(p), overPage(p))
    else if (p.type === 'over') S.pluginWindow || S.toolWindow ? overPage(p) : markFolder(folderAt(p))
    else if (p.type === 'leave') (endFileDrag(), overPage(null))
    else if (p.type === 'drop') (drop(p), overPage(null))
  }
  // this window's own drops only: an app-wide listener would also hear those on the app's other windows
  getCurrentWebview().onDragDropEvent(({ payload: p }) => fileDrag(p)).catch(() => {})
  // where Curio's own drop target takes Tauri's place, it tells what it was given as drop-in
  getCurrentWebview().listen('drop-in', ({ payload: p }) => fileDrag(p)).then(() => call('take_drops')).catch(() => {})
  pollStatus()
  if (only && S.libOpen) {
    const ids = only.split(',').map(Number).filter(Boolean)
    S.itemWindow = true
    S.results = markRaw({ ids: Uint32Array.from(ids), aspects: new Float32Array(ids.length).fill(1) })
    S.resultsVer++
    refreshSidebar().catch(() => {})
    selectOnly(ids[0], 0)
    S.viewer = { index: 0, id: ids[0] }
  } else if (S.libOpen && (S.toolWindow || S.pluginWindow)) {
    refreshSidebar().catch(() => {})
  } else if (S.libOpen && S.prefsWindow) {
    refreshSidebar().catch(() => {})
  } else if (S.libOpen) {
    await refreshSidebar()
    await loadSavedFilters()
    await loadActions()
    refreshTheme()
    requestSearch()
  }
  serveWindowApi().catch(() => {})
  S.ready = true
  if (secondary) nextTick(() => requestAnimationFrame(() => requestAnimationFrame(() => call('reveal_window').catch(() => {}))))
}

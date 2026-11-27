// The API's window operations (src/core/api/operations/mod.rs, `in_window`): the main window does what a call asks and answers it, and tells what it shows for the API's events.
import { watch } from 'vue'
import { getCurrentWindow } from '../platform/windowFrame.js'
import { call, on } from '../platform/ipc.js'
import { openPluginPage } from '../plugins/pluginActions.js'
import { S, emptyFilters } from './state.js'
import { afterOpen } from './boot.js'
import { saveUi, flushUi } from './preferences.js'
import { filterByField, filterByTag, scopeTitle, setScope } from '../library/navigation.js'
import { fullQuery } from '../filters/query.js'
import { indexOf, requestSearch, searched } from '../library/results.js'
import { loadDetail, selectOnly } from '../library/selection.js'
import { toast } from './notify.js'
import { viewerFile, itemFiles, loadViewerFiles } from '../viewer/session.js'

class Refused extends Error {
  constructor(code, message) {
    super(message)
    this.code = code
  }
}

const SHOW = { smart: 'smartFolder', root: 'location', rootdir: 'locationFolder', roots: 'locations' }

/** What the window shows, as view.get answers and view.changed tells. */
function viewState() {
  const sc = S.scope
  return { show: SHOW[sc.kind] || sc.kind, id: sc.ids || sc.id || null, path: sc.path ?? null, name: scopeTitle(), search: S.search, query: fullQuery(), count: S.results.ids.length, view: S.view, sample: { text: S.ui.fontSample?.text || '' } }
}

/** Where the pointer was last, for ui.menu. */
export const pointer = { x: 0, y: 0 }

/** Asks the person (app/ApiAsk.vue); resolves with the answer, or null when cancelled. */
function ask(call, what) {
  if (S.apiAsk) throw new Refused('busy', 'The window is already asking something')
  return new Promise((resolve) => (S.apiAsk = { ...what, call, resolve }))
}

/** Asks the person over this window, as a page's question is asked: a tool's own confirmation or folder pick. */
export const askHere = (what) => ask(null, what)

// back from a Tab page, items it selected that the grid doesn't show are let go, so no key acts on what can't be seen
watch(() => S.view, (v, was) => {
  if (!was?.startsWith('tab:') || v.startsWith('tab:')) return
  const kept = S.selected.filter((id) => indexOf(id) != null)
  if (kept.length === S.selected.length) return
  S.selected = kept
  loadDetail()
})

const OPS = {
  'library.switch': async ({ path }) => {
    await flushUi()
    await call('open_library', { path })
    await afterOpen()
    return {}
  },
  // a Tab plugin's page is what the window shows, so the items it lists can be selected; the grid's are those in its results
  'selection.set': ({ ids }) => {
    const shown = S.view.startsWith('tab:') ? ids : ids.filter((id) => indexOf(id) != null)
    const last = shown.length ? indexOf(shown[shown.length - 1]) : null
    S.selected = shown
    S.anchor = last
    S.focus = last
    loadDetail()
    return { selected: shown.length, notShown: ids.length - shown.length }
  },
  'view.get': () => viewState(),
  'view.show': async ({ kind, id }) => {
    setScope(kind, id)
    await searched()
    return viewState()
  },
  'view.setSample': ({ text }) => {
    S.ui.fontSample = { text: text ?? S.ui.fontSample?.text ?? '' }
    saveUi()
    return viewState()
  },
  'view.setSearch': async ({ text }) => {
    S.search = text
    requestSearch()
    await searched()
    if (S.queryError) throw new Refused('bad_request', S.queryError)
    return viewState()
  },
  'view.showWith': async ({ tag, field, value }) => {
    if (tag != null) filterByTag(tag)
    else filterByField(field, value)
    await searched()
    return viewState()
  },
  'view.setFilters': async (f) => {
    Object.assign(S.filters, emptyFilters(), f)
    requestSearch()
    await searched()
    return viewState()
  },
  // `file`: one of the files grouped into the item, shown in its place when the viewer steps through it (loadViewerFiles)
  'viewer.open': async ({ id, file }) => {
    let i = indexOf(id)
    if (i == null) {
      setScope('all')
      await searched()
      i = indexOf(id)
    }
    if (i == null) throw new Refused('not_found', 'The item is in the trash or hidden by a locked filter')
    const [row] = await call('grid_items', { ids: [id] }).catch(() => [])
    if (row && row.status !== 'Ok') throw new Refused('failed', row.status === 'Offline' ? "The item's storage is offline" : "The item's file is missing")
    S.view = 'grid'
    selectOnly(id, i)
    S.viewer = { index: i, id }
    // the files are read again, as the file may have been grouped in just now
    if (file != null) {
      await loadViewerFiles(id, row?.parts)
      if (S.viewer?.id === id && itemFiles().includes(file)) S.viewer.file = file
    }
    return {}
  },
  'viewer.close': () => {
    const open = !!S.viewer
    S.viewer = null
    return { closed: open }
  },
  'viewer.current': () => ({ id: S.viewer?.id ?? null, file: viewerFile() }),
  'ui.notify': ({ text, kind }) => {
    toast(text, kind)
    return {}
  },
  'ui.confirm': (a, id) => ask(id, { kind: 'confirm', ...a }).then((ok) => ({ ok: !!ok })),
  'ui.prompt': (a, id) => ask(id, { kind: 'prompt', ...a }).then((value) => ({ value })),
  'ui.pickTags': (a, id) => ask(id, { kind: 'tags', ...a }).then((tags) => ({ tags })),
  'ui.pickFolders': (a, id) => ask(id, { kind: 'folders', ...a }).then((folders) => ({ folders })),
  'ui.pickField': (a, id) => ask(id, { kind: 'field', ...a }).then((key) => ({ key })),
  'ui.pickColor': (a, id) => ask(id, { kind: 'color', ...a }).then((color) => ({ color })),
  'ui.menu': (a, id) => ask(id, { kind: 'menu', ...a }).then((choice) => ({ id: choice })),
  // not an operation of its own: a browser extension asking for a token (core/api/pair.rs); the answer is the libraries allowed, none for all
  'pair.ask': (a, id) => ask(id, { kind: 'pair', ...a }).then((libraries) => (libraries ? { ok: true, libraries } : { ok: false })),
  // not an operation of its own: an address a plugin takes over opens its page (core/api/operations/mod.rs, hand_over)
  'plugin.open': async ({ plugin, opens, run }) => {
    await openPluginPage(plugin, opens, run || {})
    return { success: true }
  },
}

/** Answers the API's calls for this window: the main window all of them, a Window plugin's window the questions its page asks. */
export async function serveWindowApi() {
  if (S.itemWindow) return
  const me = getCurrentWindow().label
  pointer.x = innerWidth / 2
  pointer.y = innerHeight / 3
  window.addEventListener('pointermove', (e) => ((pointer.x = e.clientX), (pointer.y = e.clientY)), { passive: true })
  await on('api-window', async ({ id, op, args, to }) => {
    if ((to || 'main') !== me) return
    const reply = (r) => call('api_window_reply', { id, ...r }).catch(() => {})
    try {
      const f = OPS[op]
      if (!f) throw new Refused('unknown_operation', `The window can't do ${op}`)
      reply({ ok: (await f(args || {}, id)) ?? {} })
    } catch (e) {
      reply({ error: { code: e?.code || 'failed', message: String(e?.message ?? e) } })
    }
  })
  // the caller stopped waiting
  await on('api-window-cancel', (id) => {
    const a = S.apiAsk
    if (a?.call === id) {
      S.apiAsk = null
      a.resolve(null)
    }
  })
  // a plugin window, a tool's and Settings answer only what is asked of them
  if (S.pluginWindow || S.prefsWindow || S.toolWindow) return
  call('api_window_ready').catch(() => {})
  // what is shown, once its search is done; a change to the items alone isn't one
  let told = null
  let timer = null
  // a search that finds the same items leaves resultsVer as it is, so its end counts too
  watch([() => S.resultsVer, () => S.searching, () => S.view, () => S.startup.current, () => JSON.stringify(S.ui.fontSample)], () => {
    clearTimeout(timer)
    timer = setTimeout(() => {
      if (S.searching || !S.libOpen) return
      const v = viewState()
      const key = JSON.stringify([S.startup.current, v.show, v.id, v.path, v.search, v.query, v.view, v.sample])
      if (key === told) return
      told = key
      call('window_shown', { what: 'view', value: v }).catch(() => {})
    }, 200)
  })
  watch([() => S.viewer?.id ?? null, viewerFile], ([id, file]) => call('window_shown', { what: 'viewer', value: { id, file } }).catch(() => {}))
}

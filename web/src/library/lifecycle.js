// Opening libraries and items, importing and exporting, and the status of work in the background.
import { call, errorText } from '../platform/ipc.js'
import { S } from '../app/state.js'
import { t } from '../shared/i18n.js'
import { toast, run } from '../app/notify.js'
import { remember } from './edits.js'
import { flushUi } from '../app/preferences.js'
import { afterOpen } from '../app/boot.js'

export function openItem(id, how = 'open') {
  return call('open_item', { id, how }).catch((e) => toast(errorText(e), 'error'))
}

/** New files go to the given folder, or to the folder being looked at. */
/** Paste what the clipboard holds: files, a picture, a link or a path. In a folder, it lands there. */
export function pasteClipboard() {
  const folder = S.scope.kind === 'folder' && S.view === 'grid' ? S.scope.id : null
  call('import_clipboard', { folder })
    .then((r) => {
      if (r === 'image') toast(t('library.imagePasted'), 'info', undefined, null, 'import')
      else if (/^https?:\/\//.test(r.trim())) saveLinks([r.trim()], folder)
      else if (r.startsWith('files\n')) addPaths(r.split('\n').slice(1), true, folder)
      else if (r.trim()) addPaths([r.trim()], true, folder)
      else toast(t('library.nothingToPaste'), 'error')
    })
    .catch((err) => toast(errorText(err), 'error'))
}

/** Copies of the items' files in a folder the person picks. */
export async function exportFiles(ids) {
  const dir = await call('pick', { kind: 'folder', title: t('library.exportTo') })
  if (!dir) return
  const r = await run('item.export', { ids, folder: dir[0] })
  toast(t('library.exported', { count: r.count }))
}

/** A location's folder was moved or its drive letter changed: the person shows where it is now. */
export async function relocateRoot(id) {
  const dir = await call('pick', { kind: 'folder', title: t('library.locateFolder') })
  if (dir) await run('location.relocate', { id, path: dir[0] })
}

/** How many items a query finds; throws the parser's message when it doesn't parse. */
export async function countOf(query) {
  const r = await call('query.validate', { query })
  if (!r.valid) throw r.error
  return r.count
}

/** Downloads, or links for web pages; the window says "Saved …" once each is in. */
export function saveLinks(urls, folder) {
  toast(t('library.savingLinks', { count: urls.length }), 'info', undefined, null, 'import')
  return call('item.add.fromUrls', { urls, folders: folder != null ? [folder] : [] })
    .then((r) => {
      // addresses a plugin takes over went to it
      if (r?.handed?.length) toast(t('library.handed', { count: r.handed.length }), 'info', undefined, null, 'import')
      for (const e of r?.errors || []) toast(errorText(e), 'error')
      return r
    })
    .catch((e) => toast(errorText(e), 'error'))
}

export async function addPaths(paths, copy, folder = null) {
  if (!paths?.length) return
  if (!S.libOpen) await startFirstLibrary()
  const target = folder ?? (S.scope.kind === 'folder' && S.view === 'grid' ? S.scope.id : null)
  await importPaths(paths, copy, target)
}

/** Import into `folder` (null = the library's top level). Copies onto names already taken there ask first: keep both, replace or skip. */
export async function importPaths(paths, copy, folder) {
  const go = async (clash) => {
    await run('item.add.fromPaths', { paths, copy, folders: folder != null ? [folder] : [], onClash: clash ?? null })
    if (folder != null) remember('folders', folder)
    pollStatus()
  }
  const c = await call('import_clashes', { paths, copy, folder }).catch(() => ({ count: 0 }))
  if (!c.count) return go('keep')
  const some = c.names.join(', ') + (c.count > c.names.length ? ', …' : '')
  S.dialogs.choice = {
    title: t('library.clash.title', { count: c.count }),
    body: folder != null ? t('library.clash.inFolder', { names: some, name: S.sidebar.folders.find((f) => f.id === folder)?.name || t('app.drops.theFolder') }) : t('library.clash.inLibrary', { names: some }),
    choices: [
      { label: t('library.clash.keep'), value: 'keep', severity: 'primary' },
      { label: t('library.clash.replace'), value: 'replace', severity: 'danger' },
      { label: t('library.clash.skip'), value: 'skip', severity: 'secondary' },
    ],
    pick: go,
  }
}

// How often to ask how background work is going: quickly while an import has a progress bar to fill.
const POLL_MS = 1000
const POLL_IMPORT_MS = 200
let pollTimer = null
export async function pollStatus() {
  clearTimeout(pollTimer)
  try {
    if (S.libOpen) S.status = await call('status')
  } catch {
    /* ignore */
  }
  pollTimer = setTimeout(pollStatus, S.status?.import || S.status?.jobs?.length ? POLL_IMPORT_MS : POLL_MS)
}

/** What a first drop on the Welcome page lands in: a new library in Curio's own folder. */
async function startFirstLibrary() {
  await flushUi()
  await run('new_library', { name: t('library.firstName'), choose: false })
  await afterOpen()
}

/** Read this computer's library list again; any window changing it says so with `library.listChanged`. */
export async function refreshLibraries() {
  S.libraries = await call('library_list').catch(() => S.libraries)
}

/** Open a library here; if another window has it, that window comes forward instead. */
export async function openLibrary(path) {
  try {
    await flushUi()
    await call('open_library', { path })
    await afterOpen(path)
  } catch (e) {
    toast(errorText(e), 'info')
  }
}

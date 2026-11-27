// What the viewer shows, and stepping through items and through the files of a grouped item.
import { call } from '../platform/ipc.js'
import { S } from '../app/state.js'
import { L } from '../layout/arrangement.js'
import { spectrumView } from '../display/spectrumView.js'
import { selectOnly } from '../library/selection.js'
import { ensureItems } from '../library/results.js'
import { openAsFilter, showMadeWith, unreadable, fileGone } from '../library/navigation.js'

/** Spectrum's row-by-row result index order when opened from Spectrum, else null (use S.results directly). */
export function viewerOrder() {
  if (!S.itemWindow && L().view === 'spectrum' && spectrumView.order?.length) return spectrumView.order
  return null
}

/** Results arrays (ids and aspects) in the viewer's active order. */
export function viewerResults() {
  return viewerOrder() && spectrumView.results ? spectrumView.results : S.results
}

/** Position and total length of the open item in the viewer's active order. */
export function viewerPosition() {
  if (!S.viewer) return { pos: -1, length: 0 }
  const ord = viewerOrder()
  if (ord) {
    const pos = spectrumView.posOf?.[S.viewer.index] ?? -1
    if (pos >= 0) return { pos, length: ord.length }
  }
  return { pos: S.viewer.index, length: S.results.ids.length }
}

/** Result index in S.results.ids at position `pos` of the viewer's active order. */
export function viewerResultAt(pos) {
  const ord = viewerOrder()
  if (ord) return pos >= 0 && pos < ord.length ? ord[pos] : -1
  return pos >= 0 && pos < S.results.ids.length ? pos : -1
}

/** Asset id at position `pos` of the viewer's active order. */
export function viewerIdAt(pos) {
  const idx = viewerResultAt(pos)
  return idx >= 0 ? S.results.ids[idx] : null
}

/** Opens the viewer on an item, unless its file can't be read; a model has no viewer, so opening one shows the pictures made with it. */
export function openViewer(index, id) {
  const asFilter = openAsFilter(S.items.get(id))
  if (asFilter) {
    showMadeWith(id, asFilter.fields)
    return true
  }
  if (unreadable(id)) return false
  S.viewer = { index, id }
  return true
}

/** Another item while one is open: a step of one passes over items whose files can't be read, a jump onto one only says so. */
export function stepViewer(d) {
  if (!S.viewer || !d) return
  const { pos, length } = viewerPosition()
  if (pos < 0) return
  let p = pos + d
  if (Math.abs(d) === 1) while (p >= 0 && p < length && fileGone(viewerIdAt(p))) p += d
  if (p < 0 || p >= length) return
  const idx = viewerResultAt(p)
  const id = S.results.ids[idx]
  if (unreadable(id)) return
  S.viewer = { index: idx, id }
  selectOnly(id, idx)
}

/** The file the viewer shows: one grouped into the open item, or the item's own. */
export const viewerFile = () => (S.viewer ? (S.viewer.file ?? S.viewer.id) : null)

/** The files of the open item the viewer steps through; none unless files were grouped into it by hand. */
export const itemFiles = () => (S.viewer && S.viewerFiles.item === S.viewer.id ? S.viewerFiles.ids : [])

const byName = new Intl.Collator('en', { numeric: true, sensitivity: 'base' })
/** The item's own file first, then those grouped into it by hand, by name. Files a rule grouped (a Live Photo's clip, a RAW, sidecars) belong to their file and have tools of their own. */
export async function loadViewerFiles(id, parts) {
  if (!parts) {
    S.viewerFiles = { item: id, ids: [] }
    if (S.viewer?.id === id && S.viewer.file != null) S.viewer.file = null
    return
  }
  const d = await call('asset_detail', { id }).catch(() => null)
  if (S.viewer?.id !== id) return
  const own = (d?.parts || []).filter((p) => p.rule === 'user').sort((a, b) => byName.compare(a.name, b.name))
  const ids = own.length ? [id, ...own.map((p) => p.id)] : []
  ensureItems(ids)
  S.viewerFiles = { item: id, ids }
  if (S.viewer.file != null && !ids.includes(S.viewer.file)) S.viewer.file = null
}

/** Another file of the open item, as stepViewer moves between items. Answers whether the item has files to step through, so its keys are taken at either end too. */
export function stepFile(d) {
  const ids = itemFiles()
  if (!ids.length) return false
  let n = Math.max(0, ids.indexOf(viewerFile())) + d
  if (Math.abs(d) === 1) while (n >= 0 && n < ids.length && fileGone(ids[n])) n += d
  if (d && n >= 0 && n < ids.length && !unreadable(ids[n])) S.viewer.file = ids[n] === S.viewer.id ? null : ids[n]
  return true
}

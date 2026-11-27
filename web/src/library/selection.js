// Which items are selected, and the detail the Inspector shows for them.
import { call } from '../platform/ipc.js'
import { S } from '../app/state.js'

// ------------------------------------------------------------------ selection + detail

export function selectOnly(id, idx) {
  S.selected = [id]
  S.anchor = idx
  S.focus = idx
  loadDetail()
}

export function toggleSelect(id, idx) {
  const i = S.selected.indexOf(id)
  if (i >= 0) S.selected.splice(i, 1)
  else S.selected.push(id)
  S.anchor = idx
  S.focus = idx
  loadDetail()
}

export function selectRange(to, layout = null) {
  const from = S.anchor ?? to
  const [a, b] = from <= to ? [from, to] : [to, from]
  let ids
  if (layout?.sections?.some((s) => s.collapsed)) {
    ids = []
    for (let i = a; i <= b; i++) if (layout.h[i] > 0) ids.push(S.results.ids[i])
  } else {
    ids = Array.from(S.results.ids.subarray(a, b + 1))
  }
  const target = S.results.ids[to]
  S.selected = [...ids.filter((x) => x !== target), target]
  S.focus = to
  loadDetail()
}

/** Select every item belonging to one section span `[start, start + count)`. */
export function selectSpan(start, count) {
  if (!count) return
  S.selected = Array.from(S.results.ids.subarray(start, start + count))
  S.anchor = start
  S.focus = start + count - 1
  loadDetail()
}

export function selectAll() {
  S.selected = Array.from(S.results.ids)
  loadDetail()
}

// through loadDetail, so a detail still on its way for the items that were selected stays out
export function clearSelection() {
  S.selected = []
  loadDetail()
}

let selTimer = null
let summarySeq = 0
function selectionChanged() {
  clearTimeout(selTimer)
  // counted now, not when the timer runs: a summary asked for meanwhile answers for the selection before
  const seq = ++summarySeq
  selTimer = setTimeout(async () => {
    const ids = S.selected.slice(0, 200000)
    call('set_selection', { ids }).catch(() => {})
    if (ids.length > 1) {
      try {
        const s = await call('selection_summary', { ids })
        if (seq === summarySeq) S.summary = s
      } catch {
        /* ignore */
      }
    } else S.summary = null
  }, 120)
}

/** Totals for the current folder / smart folder when nothing is selected. */
export async function scopeSummary() {
  const ids = Array.from(S.results.ids.subarray(0, 200000))
  return call('selection_summary', { ids })
}

let detailSeq = 0
export async function loadDetail() {
  selectionChanged()
  const id = S.selected[S.selected.length - 1]
  const seq = ++detailSeq
  if (id == null || S.selected.length !== 1) {
    S.detail = null
    return
  }
  try {
    const d = await call('asset_detail', { id })
    if (seq === detailSeq) (S.detail = d), readOffFields(id, seq)
  } catch {
    if (seq === detailSeq) S.detail = null
  }
}
// long enough that stepping through items doesn't read each file passed
const READ_FIELDS_MS = 250
/** The fields switched off are read from the item's file when shown, not stored. */
async function readOffFields(id, seq) {
  if (!S.sidebar.fields.some((f) => f.indexed === false && f.readers?.length && f.visible)) return
  await new Promise((r) => setTimeout(r, READ_FIELDS_MS))
  if (seq !== detailSeq) return
  const r = await call('item.readFields', { id }).catch(() => null)
  if (!r || seq !== detailSeq || !S.detail) return
  const idOf = new Map(S.sidebar.fields.map((f) => [f.key, f.id]))
  const row = (fid, x, idx) => ({ field_id: fid, idx, text: typeof x === 'string' ? x : null, num: typeof x === 'number' ? x : typeof x === 'boolean' ? +x : null })
  S.detail.read = Object.entries(r.fields).filter(([k]) => idOf.has(k)).flatMap(([k, v]) => [v].flat(Infinity).map((x, i) => row(idOf.get(k), x, i)))
}

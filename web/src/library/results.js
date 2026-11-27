// The search and its results: the ids in order, the rows fetched for what shows, and the Random view's order.
import { markRaw } from 'vue'
import { call, errorText, search as apiSearch } from '../platform/ipc.js'
import { arrangeLike } from './shuffle.js'
import { touch, trimOldest } from './rowCache.js'
import { S } from '../app/state.js'
import { loadDetail } from './selection.js'
import { fullQuery, plainWords } from '../filters/query.js'
import { L } from '../layout/arrangement.js'

let searchSeq = 0
let searchTimer = null
let searchQueued = false
// a change asked for a search while one was under way: it runs once that one is shown
let refreshAfter = false
export function requestSearch(delay = 0) {
  clearTimeout(searchTimer)
  searchQueued = true
  searchTimer = setTimeout(doSearch, delay)
}

/** The request for what the view finds, unsorted: for counts and facets over it. `text` leaves a filter out when given. */
export const viewRequest = (text = fullQuery()) => ({ scope: S.scope.kind, scope_id: S.scope.id || 0, scope_ids: S.scope.ids || [], scope_path: S.scope.path || '', history: '', text, sort_key: 'imported', sort_field: '', descending: false, subfolders: !!L().subfolders })

/** A search `delay` after the first of the changes asking for one, where `requestSearch` waits for them to stop: changes arriving steadily, as a scan sends them, still show. */
export function searchSoon(delay) {
  if (searchQueued) return
  // one started now would set aside the answer under way, and on a large library a steady stream of changes would never let one show
  if (S.searching) refreshAfter = true
  else requestSearch(delay)
}

/** Resolves once the search asked for last is shown. */
export async function searched(limit = 30000) {
  const t0 = Date.now()
  while ((searchQueued || S.searching) && Date.now() - t0 < limit) await new Promise((r) => setTimeout(r, 20))
}

// The Random view keeps its order until asked to reshuffle
let shuffleOrder = null
let reshuffleNext = false
function keepRandomOrder(r, scope) {
  if (scope !== 'random') return r
  if (reshuffleNext || !shuffleOrder) {
    reshuffleNext = false
    shuffleOrder = Array.from(r.ids)
    return r
  }
  if (r.groups?.length) {
    const ids = new Uint32Array(r.ids.length)
    const aspects = new Float32Array(r.aspects.length)
    const order = []
    for (const g of r.groups) {
      const end = g.start + g.count
      const a = arrangeLike(shuffleOrder, r.ids.subarray(g.start, end), r.aspects.subarray(g.start, end))
      ids.set(a.ids, g.start)
      aspects.set(a.aspects, g.start)
      for (const id of a.order) order.push(id)
    }
    shuffleOrder = order
    return { ids, aspects, notes: r.notes, groups: r.groups }
  }
  const a = arrangeLike(shuffleOrder, r.ids, r.aspects)
  shuffleOrder = a.order
  return { ids: a.ids, aspects: a.aspects, notes: r.notes, groups: r.groups }
}
export function reshuffle() {
  reshuffleNext = true
  requestSearch()
}
/** Another library: the Random view starts a new order. */
function forgetRandomOrder() {
  shuffleOrder = null
}

/** Local calendar boundary timestamps (epoch ms) for date grouping: [tomorrow, today, yesterday, thisWeek, lastWeek, thisMonth, lastMonth, thisYear]. */
function localDateBounds() {
  const now = new Date()
  const y = now.getFullYear()
  const m = now.getMonth()
  const d = now.getDate()
  // Monday-start week
  const dow = (now.getDay() + 6) % 7
  return {
    date_bounds: [
      new Date(y, m, d + 1).getTime(),
      new Date(y, m, d).getTime(),
      new Date(y, m, d - 1).getTime(),
      new Date(y, m, d - dow).getTime(),
      new Date(y, m, d - dow - 7).getTime(),
      new Date(y, m, 1).getTime(),
      new Date(y, m - 1, 1).getTime(),
      new Date(y, 0, 1).getTime(),
    ],
    tz_offset_ms: -now.getTimezoneOffset() * 60000,
  }
}

const sameList = (a, b) => a.length === b.length && a.every((v, i) => v === b[i])
const sameNotes = (a = new Set(), b = new Set()) => a.size === b.size && [...a].every((id) => b.has(id))
const sameGroups = (a, b) => (!a && !b) || (!!a && !!b && a.length === b.length && a.every((g, i) => g.key === b[i].key && g.start === b[i].start && g.count === b[i].count))
const sameResults = (a, b) => !!a?.ids && sameNotes(a.notes, b.notes) && sameGroups(a.groups, b.groups) && sameList(a.ids, b.ids) && sameList(a.aspects, b.aspects)

async function doSearch() {
  searchQueued = false
  if (!S.libOpen || S.itemWindow || S.prefsWindow) return
  const seq = ++searchSeq
  const lay = L()
  const grp = lay.view === 'spectrum' ? { key: 'none', field: '', desc: false } : lay.group || { key: 'none', field: '', desc: false }
  const needDates = grp.key === 'imported' || grp.key === 'modified' || grp.key === 'created' || grp.key === 'field'
  const dates = needDates ? localDateBounds() : { date_bounds: [], tz_offset_ms: 0 }
  const req = {
    scope: S.scope.kind,
    scope_id: S.scope.id || 0,
    scope_ids: S.scope.ids || [],
    scope_path: S.scope.path || '',
    history: plainWords(S.search) ? S.search.trim() : '',
    text: fullQuery(),
    sort_key: lay.sort.key,
    sort_field: lay.sort.field || '',
    descending: !!lay.sort.desc,
    subfolders: !!lay.subfolders,
    notes: !!lay.showNote && lay.view !== 'list',
    group_by: grp.key || 'none',
    group_field: grp.field || '',
    group_desc: !!grp.desc,
    date_bounds: dates.date_bounds,
    tz_offset_ms: dates.tz_offset_ms,
  }
  S.searching = true
  const t0 = performance.now()
  try {
    const r = keepRandomOrder(await apiSearch(req), req.scope)
    if (seq !== searchSeq) return
    // what a search lists is there, whatever an earlier request for it found
    gone.clear()
    S.resultsMs = Math.round(performance.now() - t0)
    S.queryError = null
    // an edit that left the list as it was (a folder renamed, moved or recoloured) redraws nothing
    if (sameResults(S.results, r)) return
    S.results = markRaw(r)
    S.resultsVer++
    // the selection keeps what is still listed: one pass that stops once all are found, no set of every result
    if (S.selected.length) {
      const want = new Set(S.selected)
      const found = new Set()
      for (const id of r.ids) if (want.has(id) && found.add(id).size === want.size) break
      if (found.size !== want.size) {
        S.selected = S.selected.filter((id) => found.has(id))
        loadDetail()
      }
    }
    S.focus = S.selected.length ? indexOf(S.selected[S.selected.length - 1]) : null
    if (S.viewer) {
      const i = indexOf(S.viewer.id)
      if (i == null) S.viewer = null
      else S.viewer.index = i
    }
  } catch (e) {
    if (seq === searchSeq) S.queryError = errorText(e)
  } finally {
    if (seq === searchSeq) {
      S.searching = false
      if (refreshAfter) {
        refreshAfter = false
        requestSearch()
      }
    }
  }
}

function placeOf(id) {
  const i = shownAt.get(id)
  return i != null && S.results.ids[i] === id ? i : indexOf(id)
}

export function indexOf(id) {
  const ids = S.results.ids
  if (S.focus != null && ids[S.focus] === id) return S.focus
  const i = ids.indexOf(id)
  return i < 0 ? null : i
}

// ------------------------------------------------------------------ items (only visible ones are fetched)

// past ROWS_MAX rows the ones used longest ago go, down to ROWS_KEEP: those on show were asked for last and stay
const ROWS_MAX = 30000
const ROWS_KEEP = 20000
const pending = new Set()
// rows on show that changed: they stay on screen until their new rows replace them, so cards and the Inspector don't blank out
const stale = new Set()
// asked for and not there (removed since the search): asked again only after a change names them
const gone = new Set()
let fetchTimer = null
// where in the results an item was last asked for from, so its row's shape is checked there rather than looked for through every result
const shownAt = new Map()
/** Rows for these items, asked for once each; `at`, their places in the results, where the caller knows them. */
export function ensureItems(ids, at = null) {
  let any = false
  if (at) {
    if (shownAt.size > ROWS_MAX) shownAt.clear()
    ids.forEach((id, k) => shownAt.set(id, at[k]))
  }
  for (const id of ids) {
    // one id that isn't a number fails the whole request, and every row asked for with it
    if (!Number.isInteger(id)) continue
    touch(S.items, id)
    if ((!S.items.has(id) || stale.has(id)) && !pending.has(id) && !gone.has(id)) {
      pending.add(id)
      any = true
    }
  }
  if (any && !fetchTimer) fetchTimer = setTimeout(flushItems, 0)
}

// a failed fetch is asked for again a few times; without that its cards stay empty until something else redraws them
const RETRY_MS = 300
const RETRIES = 5
const GIVE_UP_RETRY_MS = 3000
const tries = new Map()

async function flushItems() {
  fetchTimer = null
  const ids = [...pending]
  pending.clear()
  if (!ids.length) return
  // rows for a library that has since been closed must not land in the next one's cache
  const map = S.items
  try {
    const rows = await call('grid_items', { ids })
    if (S.items !== map) return
    for (const id of ids) {
      tries.delete(id)
      stale.delete(id)
    }
    let reshaped = false
    for (const r of rows) {
      S.items.delete(r.id)
      S.items.set(r.id, r)
      // a book, comic, PDF or link gets its shape with its thumbnail, after the results that hold it were laid out
      const asp = r.aspect > 0 ? r.aspect : r.width > 0 && r.height > 0 ? r.width / r.height : 0
      const i = asp > 0 ? placeOf(r.id) : null
      if (i != null && Math.abs(S.results.aspects[i] - asp) > 0.01) {
        S.results.aspects[i] = asp
        reshaped = true
      }
    }
    if (reshaped) S.resultsVer++
    // without this a removed item's card would ask again on every redraw
    if (rows.length < ids.length) {
      const got = new Set(rows.map((r) => r.id))
      for (const id of ids) {
        if (got.has(id)) continue
        gone.add(id)
        S.items.delete(id)
      }
    }
    if (S.items.size > ROWS_MAX) trimOldest(S.items, ROWS_KEEP, new Set(S.selected))
    S.itemsVer++
    const want = rows.filter((r) => r.thumb_state === 0 || r.thumb_state === 3).map((r) => r.id)
    if (want.length) call('prioritize_thumbs', { ids: want }).catch(() => {})
  } catch (e) {
    if (S.items !== map) return
    const again = ids.filter((id) => (tries.get(id) || 0) < RETRIES)
    for (const id of again) {
      tries.set(id, (tries.get(id) || 0) + 1)
      pending.add(id)
    }
    if (again.length && !fetchTimer) fetchTimer = setTimeout(flushItems, RETRY_MS)
    else if (!again.length) {
      console.warn('grid_items failed', e)
      // cards still empty ask again a little later, rather than waiting for a click to redraw them
      setTimeout(() => S.items === map && S.itemsVer++, GIVE_UP_RETRY_MS)
    }
  }
}

// past this many rows a change drops them rather than asking for all of them again at once
const REFRESH_IN_PLACE = 2000

/** Rows that changed: asked for again, the old ones staying on show meanwhile. */
export function invalidateItems(ids) {
  const inPlace = ids.length <= REFRESH_IN_PLACE
  for (const id of ids) {
    gone.delete(id)
    S.cells.delete(id)
    for (const q of cellRequests) if (q.ids.has(id)) q.dropped.add(id)
    if (inPlace && S.items.has(id)) stale.add(id)
    else S.items.delete(id)
  }
  if (inPlace) ensureItems(ids.filter((id) => stale.has(id)))
  S.itemsVer++
}

// Everything else a list-view column can show, fetched only for rows on screen.
const pendingCells = new Set()
let cellsTimer = null
// requests on their way: each answer lands only in the cache it was asked for, and not for cells dropped since it was asked
const cellRequests = new Set()
export function ensureCells(ids) {
  let any = false
  for (const id of ids) {
    if (Number.isInteger(id) && !S.cells.has(id) && !pendingCells.has(id)) {
      pendingCells.add(id)
      any = true
    }
  }
  if (any && !cellsTimer) {
    cellsTimer = setTimeout(async () => {
      cellsTimer = null
      const list = [...pendingCells]
      pendingCells.clear()
      const q = { map: S.cells, ids: new Set(list), dropped: new Set() }
      cellRequests.add(q)
      try {
        const r = await call('list_cells', { ids: list })
        if (S.cells !== q.map) return
        for (const k of Object.keys(r)) if (!q.dropped.has(Number(k))) S.cells.set(Number(k), r[k])
        S.cellsVer++
      } catch {
        /* library switching */
      } finally {
        cellRequests.delete(q)
      }
    }, 0)
  }
}

/** Another library opened in this window: what was fetched or queued for the last one is dropped, and answers still on their way find their caches gone. */
export function resetResults() {
  forgetRandomOrder()
  clearTimeout(fetchTimer)
  fetchTimer = null
  pending.clear()
  tries.clear()
  stale.clear()
  gone.clear()
  shownAt.clear()
  S.items = markRaw(new Map())
  S.itemsVer++
  clearTimeout(cellsTimer)
  cellsTimer = null
  pendingCells.clear()
  S.cells = markRaw(new Map())
}

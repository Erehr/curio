// Where the window is: the scope, filters set from an item, and going back and forward.
import { markRaw } from 'vue'
import { call } from '../platform/ipc.js'
import { S, emptyFilters } from '../app/state.js'
import { t } from '../shared/i18n.js'
import { toast } from '../app/notify.js'
import { clearSelection, selectOnly } from './selection.js'
import { filterKindOf } from '../filters/query.js'
import { requestSearch } from './results.js'

// Clicking a value on an item shows the whole library through the matching filter chip, where it can be seen, widened or taken off; holding Ctrl/Cmd adds to the active filters instead of replacing them.
function showAllWith(set, add = false) {
  if (add) {
    S.view = 'grid'
    S.viewer = null
  } else {
    setScope('all')
    if (!S.filterLock) {
      resetFilters()
      S.search = ''
    }
    S.viewer = null
  }
  set(S.filters)
  requestSearch()
}
export function filterByTag(name, add = false) {
  showAllWith((f) => {
    if (!add) return Object.assign(f, { tags: [name], tagsNot: [], untagged: null })
    if (f.tagsNot.includes(name)) f.tagsNot.splice(f.tagsNot.indexOf(name), 1)
    if (f.untagged === true) f.untagged = null
    if (!f.tags.includes(name)) {
      if (f.tags.length === 1 && f.tagRule === 'OR') f.tagRule = 'AND'
      f.tags.push(name)
    }
  }, add)
}
export function filterByFolder(name, add = false) {
  showAllWith((f) => {
    if (!add) return Object.assign(f, { folders: [name], foldersNot: [] })
    if (f.foldersNot.includes(name)) f.foldersNot.splice(f.foldersNot.indexOf(name), 1)
    if (!f.folders.includes(name)) f.folders.push(name)
  }, add)
}
/** Show All through the field's filter, set the way that filter asks: the value itself, a range of just it, or the text it contains. */
export function filterByField(key, value, add = false) {
  const field = S.sidebar.fields.find((x) => x.key === key)
  const kind = field ? filterKindOf(field) : 'list'
  showAllWith((f) => {
    const prev = add ? f.fieldPick[key] : null
    const pick = prev ? { values: [...(prev.values || [])], not: [...(prev.not || [])], contains: prev.contains || '', min: prev.min ?? null, max: prev.max ?? null, has: prev.has ?? null, ...(prev.ranges ? { ranges: { ...prev.ranges } } : {}) } : { values: [], not: [], contains: '', min: null, max: null, has: null }
    if (kind === 'range' && !Number.isNaN(Number(value))) {
      const num = Number(value)
      if (prev && (pick.min != null || pick.max != null)) {
        pick.min = pick.min != null ? Math.min(pick.min, num) : num
        pick.max = pick.max != null ? Math.max(pick.max, num) : num
      } else pick.min = pick.max = num
    } else if (kind === 'text') {
      const term = `"${String(value).replace(/"/g, '')}"`
      if (pick.contains) {
        if (!pick.contains.includes(term)) pick.contains = `${pick.contains}, ${term}`
      } else pick.contains = term
    } else if (kind === 'has') {
      pick.has = true
    } else {
      if (pick.not.includes(value)) pick.not.splice(pick.not.indexOf(value), 1)
      if (!pick.values.includes(value)) pick.values.push(value)
    }
    f.fieldPick[key] = pick
  }, add)
}

/** Clear the filters; the lock goes with them. */
export function resetFilters() {
  Object.assign(S.filters, emptyFilters())
  S.filterLock = false
}

// ------------------------------------------------------------------ navigation

export function setScope(kind, id = 0, ids = null, path = null) {
  if (S.scope.kind === kind && S.scope.id === id && S.scope.path === (path ?? undefined) && S.view === 'grid' && !ids && !S.scope.ids) return
  S.back.push({ ...S.scope })
  S.fwd = []
  S.scope = ids ? { kind, id, ids } : path != null ? { kind, id, path } : { kind, id }
  // rows selected in the sidebar stay lit only while they are what the view shows
  const sel = S.sideSel
  if (sel.kind && !(ids ? ids.every((i) => sel.ids.includes(i)) : sel.kind === kind && sel.ids.includes(id))) S.sideSel = { kind: null, ids: [], anchor: null }
  // a locked filter comes along; otherwise each view starts clean
  if (!S.filterLock) {
    resetFilters()
    S.search = ''
  }
  S.view = 'grid'
  S.viewer = null
  S.scrollTo = 0
  requestSearch()
}

/** Take items off the screen at once; the selection moves to whatever took their place. */
export function removeFromView(ids) {
  const gone = new Set(ids)
  const r = S.results
  const keep = []
  let first = null
  for (let i = 0; i < r.ids.length; i++) {
    if (gone.has(r.ids[i])) {
      if (first == null) first = i
    } else keep.push(i)
  }
  let groups = null
  if (r.groups) {
    groups = []
    let start = 0
    for (const g of r.groups) {
      let count = 0
      const end = g.start + g.count
      for (let i = g.start; i < end; i++) if (!gone.has(r.ids[i])) count++
      if (count > 0) {
        groups.push({ ...g, start, count })
        start += count
      }
    }
  }
  S.results = markRaw({ ids: Uint32Array.from(keep, (i) => r.ids[i]), aspects: Float32Array.from(keep, (i) => r.aspects[i]), notes: r.notes, groups })
  S.resultsVer++
  const next = keep.length ? Math.min(first ?? 0, keep.length - 1) : null
  if (next == null) {
    S.viewer = null
    return clearSelection()
  }
  const id = S.results.ids[next]
  selectOnly(id, next)
  if (S.viewer) S.viewer = { index: next, id }
}

export const fileGone = (id) => {
  const it = S.items.get(id)
  return !!it && it.status !== 'Ok'
}

/** Says so when an item's file can't be read, as a viewer opened on it would show nothing. */
export function unreadable(id) {
  if (!fileGone(id)) return false
  const it = S.items.get(id)
  const offline = it.status === 'Offline'
  toast(offline ? t('library.offline', { name: it.title }) : t('library.missing', { name: it.title }), 'warn')
  return true
}
/** A plugin's format that opens as a filter (`open.show_with`), for this item's extension. */
export const openAsFilter = (it) => it && S.contrib.opensAsFilter?.find((x) => x.exts.includes(String(it.ext).toLowerCase()))
/** All, through the first of these fields set to this file's name as other items record it, the one more items record it in winning. */
export async function showMadeWith(id, fields) {
  const d = fields?.length && (await call('asset_detail', { id }).catch(() => null))
  if (!d) return
  const stem = d.asset.name.replace(/\.[^.]+$/, '')
  const counts = await Promise.all(fields.map((k) => call('item.count', { query: `${k} = ${JSON.stringify(stem)}` }).then((r) => r.count, () => 0)))
  filterByField(fields[counts.indexOf(Math.max(...counts))], stem)
}

// a view of several folders or locations has them selected in the sidebar
const sideSelOf = (scope) => (scope.ids ? { kind: scope.kind === 'folders' ? 'folder' : 'root', ids: [...scope.ids], anchor: scope.ids[scope.ids.length - 1] } : { kind: null, ids: [], anchor: null })

export function goBack() {
  if (S.viewer) {
    S.viewer = null
    return
  }
  const s = S.back.pop()
  if (s) {
    S.fwd.push(S.scope)
    S.scope = s
    S.sideSel = sideSelOf(s)
    S.view = 'grid'
    S.viewer = null
    requestSearch()
  }
}

export function goForward() {
  const s = S.fwd.pop()
  if (s) {
    S.back.push(S.scope)
    S.scope = s
    S.sideSel = sideSelOf(s)
    S.view = 'grid'
    S.viewer = null
    requestSearch()
  }
}

export function scopeTitle() {
  const { kind, id } = S.scope
  if (S.view === 'tags') return S.tagTitle || t('library.allTags', { count: S.sidebar.tags.length })
  if (S.view.startsWith('tab:')) return S.contrib.tabs?.find((t) => `tab:${t.plugin}/${t.id}` === S.view)?.title || t('library.plugin')
  if (['all', 'favorites', 'uncategorized', 'untagged', 'recent', 'random', 'trash', 'missing'].includes(kind)) return t(`library.views.${kind}`)
  const find = (list) => list.find((x) => x.id === id)
  if (kind === 'folder') return find(S.sidebar.folders)?.name || t('library.views.folder')
  if (kind === 'smart') return find(S.sidebar.smart)?.name || t('library.views.smart')
  if (kind === 'smartBucket') {
    const sf = find(S.sidebar.smart)
    const v = S.scope.path || ''
    const label = sf?.group_by === 'rating' ? (Number(v) ? `${v} ★` : t('filters.value.none')) : sf?.group_by === 'kind' ? t(`filters.kindNames.${v}`) : sf?.group_by === 'ext' ? v.toUpperCase() : v
    return sf ? `${sf.name} › ${label}` : label
  }
  if (kind === 'tag') return '# ' + (find(S.sidebar.tags)?.name || t('library.views.tag'))
  if (kind === 'root') return find(S.sidebar.roots)?.label || t('library.views.root')
  if (kind === 'rootdir') return S.scope.path?.split('/').pop() || t('library.views.folder')
  if (kind === 'folders') return t('library.views.folders', { count: S.scope.ids?.length || 0 })
  if (kind === 'roots') return t('library.views.roots', { count: S.scope.ids?.length || 0 })
  return t('library.views.all')
}

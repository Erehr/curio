// The query the filters, the search field and the scope make, in the query language (docs/api/query/syntax.md).
import { call } from '../platform/ipc.js'
import { t } from '../shared/i18n.js'
import { S, COLOR_ACCURACY } from '../app/state.js'
import { run } from '../app/notify.js'
import { wordClause, wordsOf, pairedField, withRange } from './smartRules.js'

// ------------------------------------------------------------------ queries

const RATIOS = { '4:3': 4 / 3, '16:9': 16 / 9 }
const ratioCond = (r, t = S.startup.ratioTolerance) => `(aspect >= ${(r - t).toFixed(4)} AND aspect <= ${(r + t).toFixed(4)})`
const anyOf = (list) => (list.length > 1 ? `(${list.join(' OR ')})` : list[0])

const quoted = (list) => list.map((t) => JSON.stringify(String(t))).join(', ')

// An extension row can mean the whole extension ("webp") or only its still ("webp:s") or moving ("webp:a") files.
function extCond(v) {
  const [e, part] = String(v).split(':')
  const x = `ext = ${JSON.stringify(e)}`
  return part === 'a' ? `(${x} AND animated = 1)` : part === 's' ? `(${x} AND animated = 0)` : x
}

/** A date filter's conditions: within some days, or from one day to another, the last day counting whole. */
function dated(p, f, k) {
  if (f[k + 'Within']) p.push(`${k} WITHIN ${f[k + 'Within']}d`)
  if (f[k + 'From']) p.push(`${k} >= ${f[k + 'From']}`)
  if (f[k + 'To']) {
    const d = new Date(`${f[k + 'To']}T00:00:00`)
    d.setDate(d.getDate() + 1)
    p.push(`${k} < ${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`)
  }
}

/** The filters as a query. `except` leaves one out, for the counts in that filter's own menu. */
export function filterQuery(except = null) {
  const f = S.filters
  const p = []
  if (except !== 'types') {
    const ptypeExts = (k) => (S.contrib.formatTypes || []).find((x) => x.label === k.slice(6))?.exts || []
    const plainIn = f.kinds.filter((k) => !k.startsWith('ptype:'))
    const ptypeIn = f.kinds.filter((k) => k.startsWith('ptype:')).map((k) => ptypeExts(k)).filter((es) => es.length).map((es) => `ext IN (${quoted(es)})`)
    if (plainIn.length || ptypeIn.length) {
      const parts = [...(plainIn.length ? [`kind IN (${plainIn.join(', ')})`] : []), ...ptypeIn]
      p.push(anyOf(parts))
    }
    const plainOut = f.kindsNot.filter((k) => !k.startsWith('ptype:'))
    if (plainOut.length) p.push(`kind NOT IN (${plainOut.join(', ')})`)
    for (const k of f.kindsNot.filter((x) => x.startsWith('ptype:'))) {
      const es = ptypeExts(k)
      if (es.length) p.push(`ext NOT IN (${quoted(es)})`)
    }
    if (f.animated != null) p.push(`animated = ${f.animated ? 1 : 0}`)
  }
  if (except !== 'ext') {
    if (f.exts.length) p.push(anyOf(f.exts.map(extCond)))
    for (const v of f.extsNot) p.push(`NOT ${extCond(v)}`)
  }
  if (except !== 'rating') {
    if (f.ratings.length) p.push(anyOf(f.ratings.map((r) => `rating = ${r}`)))
    for (const r of f.ratingsNot) p.push(`rating != ${r}`)
    if (f.favorite != null) p.push(`favorite = ${f.favorite ? 1 : 0}`)
    if (f.explicit != null) p.push(`explicit = ${f.explicit ? 1 : 0}`)
  }
  const shapeCond = (s) => {
    if (s === 'custom') return f.ratioW > 0 && f.ratioH > 0 ? ratioCond(f.ratioW / f.ratioH) : null
    return RATIOS[s] ? ratioCond(RATIOS[s]) : `shape = ${s}`
  }
  if (except !== 'shape') {
    const list = f.shapes.map(shapeCond).filter(Boolean)
    if (list.length) p.push(anyOf(list))
    for (const c of f.shapesNot.map(shapeCond).filter(Boolean)) p.push(`NOT ${c}`)
  }
  if (f.color) p.push(f.color === 'grayscale' ? 'color = grayscale' : `color ~ ${f.color}${f.colorAccuracy === COLOR_ACCURACY ? '' : '/' + f.colorAccuracy}`)
  if (f.gray != null) p.push(f.gray ? 'color = grayscale' : 'NOT color = grayscale')
  dated(p, f, 'added')
  const sizeU = f.sizeUnit.toLowerCase()
  if (f.sizeMin > 0) p.push(`size >= ${f.sizeMin}${sizeU}`)
  if (f.sizeMax > 0) p.push(`size <= ${f.sizeMax}${sizeU}`)
  const durU = { sec: 's', min: 'm', hour: 'h' }[f.durUnit]
  if (f.durMin > 0) p.push(`duration >= ${f.durMin}${durU}`)
  if (f.durMax > 0) p.push(`duration <= ${f.durMax}${durU}`)
  if (f.pagesMin > 0) p.push(`pages >= ${f.pagesMin}`)
  if (f.pagesMax > 0) p.push(`pages <= ${f.pagesMax}`)
  // any of them, all of them, or exactly them
  const tagConds = []
  if (f.tags.length) {
    if (f.tagRule === 'AND') tagConds.push(`tags HAS ALL (${quoted(f.tags)})`)
    else if (f.tagRule === 'EQUAL') tagConds.push(`(tags HAS ALL (${quoted(f.tags)}) AND tag_count = ${f.tags.length})`)
    else tagConds.push(`tag IN (${quoted(f.tags)})`)
  }
  // Untagged: chosen finds items without tags, left out those with some
  if (f.untagged === true) tagConds.push('tag_count = 0')
  if (tagConds.length) p.push(anyOf(tagConds))
  if (f.untagged === false) p.push('tag_count > 0')
  if (f.tagsNot.length) p.push(`tags HAS NONE (${quoted(f.tagsNot)})`)
  if (f.folders.length) p.push(`folder IN (${quoted(f.folders)})`)
  if (f.foldersNot.length) p.push(`folder NOT IN (${quoted(f.foldersNot)})`)
  if (f.folderColors.length) p.push(`folder_color IN (${quoted(f.folderColors)})`)
  if (f.folderColorsNot.length) p.push(`folder_color NOT IN (${quoted(f.folderColorsNot)})`)
  dated(p, f, 'modified')
  dated(p, f, 'created')
  if (f.noteHas != null) p.push(`annotation ${f.noteHas ? 'IS NOT EMPTY' : 'IS EMPTY'}`)
  if (keywords(f.note).length) p.push(containsAny('annotation', f.note))
  if (f.urlHas != null) p.push(`url ${f.urlHas ? 'IS NOT EMPTY' : 'IS EMPTY'}`)
  if (keywords(f.url).length) p.push(containsAny('url', f.url))
  if (f.minW > 0) p.push(`width >= ${f.minW}`)
  if (f.maxW > 0) p.push(`width <= ${f.maxW}`)
  if (f.minH > 0) p.push(`height >= ${f.minH}`)
  if (f.maxH > 0) p.push(`height <= ${f.maxH}`)
  for (const c of f.fieldConds || []) {
    if (!c.key) continue
    if (c.op === 'empty') p.push(`${c.key} IS EMPTY`)
    else if (c.op === 'notempty') p.push(`${c.key} IS NOT EMPTY`)
    else if (c.value !== '' && c.value != null) {
      const v = /^-?\d+(\.\d+)?$/.test(String(c.value)) ? c.value : JSON.stringify(String(c.value))
      p.push(`${c.key} ${c.op} ${v}`)
    }
  }
  for (const [key, c] of Object.entries(f.fieldPick || {})) {
    if (!c || except === 'field:' + key) continue
    // each value with the range given beside it for its paired field, as a LoRA with its strength
    const paired = pairedField(key, S.sidebar.fields)?.key
    if (c.values?.length) p.push(anyOf(c.values.map((v) => `${key} = ${fieldLit(v)}` + withRange(paired, c.ranges?.[v]?.min, c.ranges?.[v]?.max))))
    for (const v of c.not || []) p.push(`NOT ${key} = ${fieldLit(v)}`)
    if (keywords(c.contains).length) p.push(containsAny(key, c.contains))
    if (c.min != null && c.min !== '') p.push(`${key} >= ${Number(c.min)}`)
    if (c.max != null && c.max !== '') p.push(`${key} <= ${Number(c.max)}`)
    if (c.has != null) p.push(`${key} ${c.has ? 'IS NOT EMPTY' : 'IS EMPTY'}`)
  }
  if (f.preset) p.push(`(${f.preset.query})`)
  return p.join(' AND ')
}
// commas separate keywords except inside quotes
const keywords = (s) => (String(s || '').match(/"[^"]*"|[^,]+/g) || []).map((t) => t.replace(/"/g, '').trim()).filter(Boolean)
// "red, blue" finds either word
const containsAny = (key, s) => anyOf(keywords(s).map((k) => `${key} CONTAINS ${JSON.stringify(k)}`))
const fieldLit = (v) => (typeof v === 'number' ? String(v) : JSON.stringify(String(v)))

/** How a field's own filter asks, unless Settings says otherwise: numbers by range, words from a list, long text by what it contains. */
export function filterKindOf(field) {
  if (field.filter_kind) return field.filter_kind
  const t = field.type
  if (t === 'integer' || t === 'float') return 'range'
  if (t === 'date' || t === 'datetime' || t === 'json' || t === 'file_ref') return 'has'
  if (field.widget === 'block' || field.widget === 'code') return 'text'
  return 'list'
}

/** A field filter chip is in use. */
export const fieldPickActive = (c) => !!c && (c.values?.length > 0 || c.not?.length > 0 || !!c.contains || (c.min != null && c.min !== '') || (c.max != null && c.max !== '') || c.has != null)

/** Saved filter sets are kept with the library. */
export async function loadSavedFilters() {
  const list = await call('saved_filters', { set: null }).catch(() => [])
  S.savedFilters = list
}
export function storeSavedFilters(list) {
  S.savedFilters = list
  return run('saved_filters', { set: list })
}

// ------------------------------------------------------------------ what plain words search

// the places words are looked for, and their icons; their names are `filters.scopes.<id>` in the catalogs
export const SEARCH_SCOPES = [['name', 'square-letter-t'], ['folder', 'folder'], ['ext', 'file-code'], ['tag', 'bookmark'], ['url', 'link'], ['annotation', 'notes']]
const STRUCTURED = /[=<>~]|\b[\w.]+:|\b(AND|OR|NOT|CONTAINS|IS|IN|HAS|WITHIN|BETWEEN|MATCHES|STARTS|ENDS)\b/

// what a field's owner is called where fields are grouped by it
const OWNERS = { user: 'user', 'builtin.format.image': 'image', 'builtin.format.video': 'video', 'builtin.format.audio': 'audio', 'builtin.format.document': 'document', 'builtin.format.archive': 'archive', 'builtin.format': 'format' }
export const ownerName = (o) => (OWNERS[o] ? t(`filters.owners.${OWNERS[o]}`) : null) || S.contrib.names?.[o] || (o || '').split('.').pop().replace(/[-_]/g, ' ').replace(/^./, (c) => c.toUpperCase())

/** Fields that take part in filters, sorts, columns and search: every field but those switched off in Settings › Fields. */
export const indexedFields = () => S.sidebar.fields.filter((f) => f.indexed !== false)
/** Items can be sorted by it. */
const SORTABLE_TYPES = new Set(['text', 'integer', 'float', 'boolean', 'date', 'datetime', 'single_select', 'url', 'file_ref'])
export const sortsBy = (f) => f.indexed !== false && !f.pair && SORTABLE_TYPES.has(f.type)
/** A field fits a view when its readers apply to every kind of file in it, so each item can have a value; one filled by hand, or by readers of every file, fits any. `exts`: the view's extensions, null while not known. */
export const fitsView = (f, exts) => !exts || !f.exts || [...exts].every((e) => f.exts.includes(e))
/** The fields offered in the search scope: the indexed ones the full-text index holds. */
export const searchableFields = () => indexedFields().filter((f) => ['text', 'single_select', 'multi_select', 'url'].includes(f.type))

/** Words typed to look for, not a written-out query: only these are kept as recent searches. */
export const plainWords = (text) => !!text.trim() && !STRUCTURED.test(text)

/** Plain words looked for in the ticked places. A written-out query is left as it is. */
export function scopedSearch(text) {
  if (!text || STRUCTURED.test(text)) return text
  const on = (k) => S.ui.searchScope?.[k] !== false
  const chosen = SEARCH_SCOPES.filter(([k]) => on(k)).map(([k]) => k)
  const fields = searchableFields().filter((f) => on('field:' + f.key)).map((f) => f.key)
  if (!chosen.length && !fields.length) return text
  const all = chosen.length === SEARCH_SCOPES.length && fields.length === searchableFields().length
  const groups = wordsOf(text).map((raw) => {
    const w = JSON.stringify(raw.replace(/^"|"$/g, ''))
    // everything ticked: the fast full-text index, plus what it doesn't hold
    if (all) return wordClause(raw)
    const parts = [...chosen, ...fields].map((k) => `${k} CONTAINS ${w}`)
    return `(${parts.join(' OR ')})`
  })
  return groups.join(' AND ')
}

export function fullQuery(except = null) {
  const u = scopedSearch(S.search.trim())
  const f = filterQuery(except)
  if (u && f) return `(${u}) AND ${f}`
  return u || f
}

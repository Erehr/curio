// The list view's columns: the name always comes first, the rest are the person's choice, including
// every custom field. `sort` is the sort key clicking the header asks for; a column without one
// can't be sorted.
import { rem } from '../shared/rem.js'
import { t } from '../shared/i18n.js'
import { sizeText } from '../shared/numbers.js'
import { S } from '../app/state.js'

const two = (n) => String(n).padStart(2, '0')
export const fmtSize = (b) => sizeText(b, 1)
export const fmtDate = (ms) => {
  if (!ms) return ''
  const d = new Date(ms)
  return `${d.getFullYear()}/${two(d.getMonth() + 1)}/${two(d.getDate())} ${two(d.getHours())}:${two(d.getMinutes())}`
}
export const fmtDay = (ms) => {
  if (!ms) return ''
  const d = new Date(ms)
  return `${d.getFullYear()}/${two(d.getMonth() + 1)}/${two(d.getDate())}`
}
const fmtDur = (t) => (t ? (t >= 3600 ? `${two(Math.floor(t / 3600))}:` : '') + `${two(Math.floor((t % 3600) / 60))}:${two(Math.floor(t % 60))}` : '')

// a built-in column is named in the language shown when its name is read
const KINDS = new Set(['image', 'video', 'audio', 'document', 'code', 'link', 'archive', 'font', 'model', 'other'])
const kindOfItem = (it) => {
  const ext = String(it.ext || '').toLowerCase()
  const pt = ext && (S.contrib.formatTypes || []).find((x) => x.exts.includes(ext))
  if (pt) return pt.label
  return KINDS.has(it.kind) ? t(`filters.kinds.${it.kind}`) : it.kind
}
const col = (id, rest) => ({ id, get label() { return t(`grid.columns.${id}`) }, ...rest })
export const BUILTIN_COLUMNS = [
  // the thumbnail as a column of its own, to move or size; the name then goes without it
  col('preview', { w: 90, render: 'thumb' }),
  col('tags', { w: 150, render: 'tags' }),
  col('dimensions', { w: 96, sort: 'resolution', mono: true, value: (it) => (it.width && it.height && (it.kind === 'image' || it.kind === 'video') ? `${it.width} × ${it.height}` : '') }),
  col('rating', { w: 78, sort: 'rating', render: 'rating' }),
  col('ext', { w: 66, sort: 'ext', mono: true, value: (it) => it.ext.toUpperCase() }),
  col('size', { w: 72, sort: 'size', mono: true, value: (it) => fmtSize(it.size) }),
  col('added', { w: 96, sort: 'imported', mono: true, cell: (c) => fmtDay(c?.asset?.added_at) }),
  col('modified', { w: 150, sort: 'modified', mono: true, cell: (c) => fmtDate(c?.asset?.mtime) }),
  col('created', { w: 150, sort: 'created', mono: true, cell: (c) => fmtDate(c?.asset?.ctime) }),
  col('duration', { w: 90, sort: 'duration', mono: true, cell: (c) => fmtDur(c?.asset?.duration) }),
  col('colors', { w: 120, render: 'colors' }),
  col('folders', { w: 160, cell: (c) => (c?.folders || []).map((f) => f.name).join(', ') }),
  col('note', { w: 200, cell: (c) => c?.asset?.annotation || '' }),
  col('url', { w: 200, cell: (c) => c?.asset?.source_url || '' }),
  col('kind', { w: 90, value: kindOfItem }),
  col('path', { w: 260, mono: true, cell: (c) => c?.asset?.abs_path || '' }),
]

/** Everything that can be a column right now: Curio's own, every custom field, and plugins' columns drawn by one of the window's renderers. */
export function allColumns(fields, pluginColumns = []) {
  const custom = fields.filter((f) => f.visible !== false && f.indexed !== false && f.type !== 'json').map((f) => ({
    id: 'field:' + f.key,
    label: f.label,
    w: f.type === 'text' ? 180 : 100,
    sort: f.type !== 'multi_select' ? 'field:' + f.key : null,
    mono: f.type !== 'text',
    fieldId: f.id,
    cell: (c) => (c?.values || []).filter((v) => v.field_id === f.id).map((v) => v.text ?? v.num).filter((v) => v != null).join(', '),
  }))
  const plugins = pluginColumns.map((c) => ({ id: `plugin:${c.plugin}/${c.id}`, label: c.label, w: c.width || 300, render: c.render }))
  return [...BUILTIN_COLUMNS, ...custom, ...plugins]
}

/** Column clicked: sort by it, or turn the sort around if it already is the sort. */
export function nextSort(sort, column) {
  const isField = column.sort.startsWith('field:')
  const key = isField ? 'field' : column.sort
  const field = isField ? column.sort.slice(6) : ''
  if (sort.key === key && (sort.field || '') === field) return { key, field, desc: !sort.desc }
  return { key, field, desc: false }
}

export const sortedBy = (sort, column) => {
  if (!column.sort) return false
  const isField = column.sort.startsWith('field:')
  return isField ? sort.key === 'field' && sort.field === column.sort.slice(6) : sort.key === column.sort
}

// The size control sets the rows' height, from room for two lines of text at its smallest (in rem) to a large thumbnail at its largest (in px).
const LIST_ROW_MIN = 3.36
const LIST_ROW_MAX = 260
const SIZE_MIN = 60
const SIZE_MAX = 640
export function listRowHeight(size) {
  const min = rem(LIST_ROW_MIN)
  return Math.round(min + ((Math.min(SIZE_MAX, Math.max(SIZE_MIN, size)) - SIZE_MIN) * (LIST_ROW_MAX - min)) / (SIZE_MAX - SIZE_MIN))
}

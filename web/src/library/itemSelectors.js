// What the widgets about the selected item share.
import { computed } from 'vue'
import { S } from '../app/state.js'
import { sizeText } from '../shared/numbers.js'

export const detail = computed(() => S.detail)
export const asset = computed(() => S.detail?.asset)
export const selectedIds = () => [...S.selected]

// a field switched off shows what its file says now (`S.detail.read`), not the values kept from before
export const fieldValues = computed(() => {
  const off = new Set(S.sidebar.fields.filter((f) => f.indexed === false).map((f) => f.id))
  const m = {}
  for (const v of S.detail?.values || []) if (!off.has(v.field_id)) (m[v.field_id] ||= []).push(v)
  for (const v of S.detail?.read || []) (m[v.field_id] ||= []).push(v)
  return m
})

export const fmtSize = (b) => sizeText(b, 2)
const two = (n) => String(n).padStart(2, '0')
export const fmtDate = (ms) => {
  if (!ms) return '-'
  const d = new Date(ms)
  return `${d.getFullYear()}/${two(d.getMonth() + 1)}/${two(d.getDate())} ${two(d.getHours())}:${two(d.getMinutes())}`
}
export const fmtDur = (t) => (t >= 3600 ? `${two(Math.floor(t / 3600))}:` : '') + `${two(Math.floor((t % 3600) / 60))}:${two(Math.floor(t % 60))}`

/** The page or entry count, which is stored as a field. */
export const pageCount = computed(() => {
  const f = S.sidebar.fields.find((x) => x.key === 'pages')
  const v = f && fieldValues.value[f.id]?.[0]
  return v ? v.num ?? v.text : null
})

/** Fields the person made themselves, shown with the properties. */
export const userFields = computed(() => S.sidebar.fields.filter((f) => f.visible && f.owner === 'user'))

/** A plugin widget's fields for this item: those with a value, and the plugin's own filled by hand (which can be filled in); a field paired with another listed one (each LoRA's strength with its LoRA) goes on that one's rows. */
export function pluginFieldsOf(w) {
  if (!w) return []
  const list = w.fields.map((k) => S.sidebar.fields.find((f) => f.key === k)).filter((f) => f && (fieldValues.value[f.id] || (!f.owner.startsWith('builtin.') && !f.readers?.length)))
  return list.filter((f) => !f.pair || !list.some((g) => g.key === f.pair))
}

/** Pixel dimensions belong to image and video files, never to thumbnails of books, comics, PDFs or links. */
export const hasDimensions = (it) => !!(it?.width > 0 && it?.height > 0 && (it.kind === 'image' || it.kind === 'video'))

/** A widget about the selected item that has nothing to show for it stays out of the way, in a sidebar and as an icon alike. */
export function widgetEmpty(info, type) {
  const a = asset.value
  const w = info?.plugin
  if (w) return (!!w.kinds?.length && !w.kinds.includes(a?.kind)) || (!!w.exts?.length && !w.exts.includes(String(a?.ext).toLowerCase())) || !pluginFieldsOf(w).length
  return (type === 'w.parts' && !a?.parts) || (type === 'w.palette' && !a?.palette?.length) || (type === 'w.dimensions' && !hasDimensions(a)) || (type === 'w.duration' && !a?.duration) || (type === 'w.pages' && !pageCount.value)
}

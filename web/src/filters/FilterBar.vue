<script setup>
import { t } from '../shared/i18n.js'
import { numberText } from '../shared/numbers.js'
import { computed, nextTick, reactive, ref, watch } from 'vue'
import Popover from 'openvue/popover'
import InputText from 'openvue/inputtext'
import Select from 'openvue/select'
import Slider from 'openvue/slider'
import InputNumber from 'openvue/inputnumber'
import Textarea from 'openvue/textarea'
import RadioButton from 'openvue/radiobutton'
import AppListboxTristate from '../shared/AppListboxTristate.vue'
import AppCheckboxTristate from '../shared/AppCheckboxTristate.vue'
import DatePicker from 'openvue/datepicker'
import { S, emptyFilters } from '../app/state.js'
import { saveUi } from '../app/preferences.js'
import { requestSearch, viewRequest } from '../library/results.js'
import { filterQuery, fullQuery, storeSavedFilters, fieldPickActive, filterKindOf, indexedFields, ownerName } from './query.js'
import { openPicker } from '../tags/tags.js'
import { resetFilters } from '../library/navigation.js'
import { withKey } from '../app/shortcuts.js'
import { LOOK_COLORS, colorName } from '../sidebar/sidebar.js'
import { fieldGroups, pairedField } from './smartRules.js'
import { call } from '../platform/ipc.js'
import { beginDrag, finalIndex } from '../app/dragDrop.js'
import AppIcon from '../shared/AppIcon.vue'
import AppMenu from '../shared/AppMenu.vue'
import AppListSearch from '../shared/AppListSearch.vue'
import AppHighlight from '../shared/AppHighlight.vue'
import { fuzzy } from '../shared/text.js'
import ColorPicker from 'openvue/colorpicker'

// All filters the bar can show; pinned ones appear as chips. `pages` shows only when a plugin (such as Reader) defines the field or the filter is active.
const ALL = computed(() => [['color', 'palette'], ['tags', 'bookmark'], ['folder', 'folder'], ['folderColor', 'color-swatch'], ['shape', 'layout-board-split'], ['rating', 'star'], ['types', 'category'], ['ext', 'file-code'], ['dimensions', 'dimensions'], ['duration', 'clock'], ['size', 'file'], ...(S.sidebar.fields.some((x) => x.key === 'pages') || f.pagesMin || f.pagesMax ? [['pages', 'book-2']] : []), ['note', 'notes'], ['url', 'link'], ['added', 'calendar-plus'], ['modified', 'calendar-cog'], ['created', 'calendar'], ['fields', 'forms'], ['presets', 'bookmarks']].map(([id, icon]) => ({ id, icon, label: t(`filters.bar.${id}`) })))
defineProps({ mode: { type: String, default: 'bar' } })
const pop = ref()
const pinPop = ref()
const bar = ref()
const current = ref(null)
const f = S.filters
// fields given a filter of their own in Settings → Fields
const FIELD_ICONS = { integer: 'numbers', float: 'numbers', boolean: 'toggle-left', date: 'calendar', datetime: 'calendar', multi_select: 'tags', single_select: 'list', url: 'link', json: 'braces' }
const asFilter = (x) => ({ id: 'field:' + x.key, label: x.indexed === false ? t('layout.sorts.notIndexed', { name: x.label }) : x.label, icon: FIELD_ICONS[x.type] || 'forms', field: x })
// a field without a filter of its own still gets a chip once pinned, or while a value clicked on an item filters by it
// a field paired with a multi-value field (a LoRA's strength) only makes sense beside that field's values; single-value pairs (number in series) can still filter on their own
const pairedTo = (x) => !!x.pair && S.sidebar.fields.some((g) => g.key === x.pair && g.key !== x.key && g.type === 'multi_select')
// numbers a plugin filters together (a 3D model's width, depth and height): one filter named by their filter_group, with a range for each
const fieldSets = computed(() => {
  const m = new Map()
  for (const x of S.sidebar.fields) {
    if (!x.filter_group || x.indexed === false || filterKindOf(x) !== 'range') continue
    const k = x.owner + '\n' + x.filter_group
    if (!m.has(k)) m.set(k, [])
    m.get(k).push(x)
  }
  return [...m.values()].filter((xs) => xs.length > 1).map((xs) => ({ id: 'fset:' + xs[0].key, label: xs[0].filter_group, icon: 'dimensions', set: xs, main: xs.some((x) => x.own_filter) }))
})
const setOf = (id) => fieldSets.value.find((s) => s.id === id) || null
const setFor = (x) => fieldSets.value.find((s) => s.set.some((y) => y.key === x.key)) || null
// a set stands where its first field would
function fieldItems(fields) {
  const out = []
  for (const x of fields) {
    if (pairedTo(x)) continue
    const s = setFor(x)
    if (!s) out.push(asFilter(x))
    else if (!s.main && !out.includes(s)) out.push(s)
  }
  return out
}
const fieldFilters = computed(() => S.sidebar.fields.filter((x) => !pairedTo(x) && !setFor(x)).filter((x) => (x.own_filter && x.indexed !== false) || S.ui.pinnedFilters.includes('field:' + x.key) || fieldPickActive(f.fieldPick[x.key]) || current.value === 'field:' + x.key).map(asFilter))
// every other indexed field, in groups by where it comes from, folded until opened
const otherFields = computed(() => fieldGroups(S.sidebar.fields, ownerName).map((g) => ({ label: g.label, items: fieldItems(g.items) })).filter((g) => g.items.length))
const mainFilters = computed(() => [...ALL.value.slice(0, -2), ...fieldFilters.value.filter((a) => a.field.own_filter && a.field.indexed !== false), ...fieldSets.value.filter((s) => s.main), ...ALL.value.slice(-2)])
// a pinned field switched off stays where it was pinned, saying so
const allFilters = computed(() => {
  const list = [...mainFilters.value, ...otherFields.value.flatMap((g) => g.items)]
  return [...list, ...fieldFilters.value.filter((a) => !list.some((b) => b.id === a.id))]
})
const openGroup = ref(null)
// More filters, found by name: a search opens every group with a match and shows only those
const pinQ = ref('')
const pinTerm = computed(() => pinQ.value.trim())
const pinMain = computed(() => mainFilters.value.filter((a) => fuzzy(a.label, pinTerm.value)))
const pinGroups = computed(() => otherFields.value.map((g) => ({ ...g, items: pinTerm.value && !fuzzy(g.label, pinTerm.value) ? g.items.filter((a) => fuzzy(a.label, pinTerm.value)) : g.items })).filter((g) => g.items.length))
const pinOpen = (g) => !!pinTerm.value || openGroup.value === g.label
function pinFirst() {
  const a = pinMain.value[0] || (pinTerm.value && pinGroups.value[0]?.items[0])
  if (a) useFilter(a.id)
}
const conditionFields = computed(() => indexedFields())
const pinned = computed(() => {
  const list = S.ui.pinnedFilters.map((id) => allFilters.value.find((a) => a.id === id)).filter(Boolean)
  // a filter set from elsewhere (an item's field, a sidebar panel's preset) shows even when its chip isn't pinned
  for (const a of [...ALL.value, ...fieldFilters.value, ...fieldSets.value]) if (!list.some((b) => b.id === a.id) && active(a.id)) list.push({ ...a, extra: true })
  const once = trying.value && !list.some((b) => b.id === trying.value) && allFilters.value.find((a) => a.id === trying.value)
  if (once) list.push({ ...once, extra: true })
  return list
})
const fieldKey = (id) => (id?.startsWith('field:') ? id.slice(6) : null)
const curField = computed(() => fieldFilters.value.find((a) => a.id === current.value)?.field || null)
const curSet = computed(() => setOf(current.value))
// a set's rows are named by a letter, as Dimensions' are, when the letters tell them apart
function setNames(s) {
  const names = s.set.map((x) => x.label.replace(/\s*\(.*\)$/, ''))
  const letters = names.map((n) => n[0].toUpperCase())
  const short = new Set(letters).size === letters.length ? letters : names
  return Object.fromEntries(s.set.map((x, i) => [x.key, short[i]]))
}
const curNames = computed(() => (curSet.value ? setNames(curSet.value) : {}))
const lettered = computed(() => Object.values(curNames.value).every((n) => n.length === 1))
const curKind = computed(() => (curField.value ? filterKindOf(curField.value) : null))
const pickOf = (key) => (f.fieldPick[key] ||= { values: [], not: [], contains: '', min: null, max: null, has: null, ranges: {} })
const curPick = computed(() => (curField.value ? f.fieldPick[curField.value.key] || null : null))
const curPaired = computed(() => (curField.value && curKind.value === 'list' ? pairedField(curField.value.key, S.sidebar.fields) : null))
const rangeOf = (v) => curPick.value?.ranges?.[v] || {}
// a range typed beside a value picks it
function setRange(v, end, n) {
  const p = curPick.value
  p.ranges ||= {}
  p.ranges[v] = { ...p.ranges[v], [end]: n }
  if (n != null && !p.values.includes(v)) pick(p.values, p.not, v)
}
const fieldFacets = reactive({ values: [], set: 0, total: 0 })
const fmtValue = (field, v) => (field?.type === 'boolean' ? (Number(v) ? t('filters.value.yes') : t('filters.value.no')) : typeof v === 'number' ? numberText(v, { digits: 6, grouping: false }) : String(v))
const fieldRows = computed(() => {
  const field = curField.value
  const p = curPick.value
  if (!field || !p) return []
  const rows = fieldFacets.values.map(([v, n]) => ({ v, n, label: fmtValue(field, v) }))
  for (const v of [...p.values, ...p.not]) if (!rows.some((r) => r.v === v)) rows.push({ v, n: 0, label: fmtValue(field, v) })
  return rows
})
// what the numbers among the results run from and to
const fieldSpan = computed(() => {
  const nums = fieldFacets.values.map(([v]) => v).filter((v) => typeof v === 'number')
  return nums.length ? [Math.min(...nums), Math.max(...nums)] : null
})

const PALETTE = ['#101418', '#F5F7F8', '#97A1A8', '#A9825A', '#F08FB4', '#E8483F', '#F39A45', '#F2CF3D', '#4DBE8C', '#38BDC9', '#3A7BD5', '#7A5BE0']
const pickerHex = computed(() => (f.color && f.color !== 'grayscale' ? f.color : '#38BDC9'))
function setHex(v) {
  const t = v.trim()
  if (!t) f.color = null
  else if (/^#?[0-9a-f]{6}$/i.test(t)) f.color = (t.startsWith('#') ? t : '#' + t).toUpperCase()
}
const KINDS = ['image', 'video', 'audio', 'document', 'code', 'link', 'archive', 'font', 'model', 'other']
const kindLabel = (k) => (k.startsWith('ptype:') ? k.slice(6) : t(`filters.kindNames.${k}`))
const SHAPES = ['landscape', 'portrait', 'square', 'wide', 'tall', '4:3', '16:9']
const shapeName = (v) => (v.includes(':') ? v : t(`filters.shapes.${v}`))
const SIZE_UNITS = ['KB', 'MB', 'GB']
const DURATION_UNITS = ['sec', 'min', 'hour']
// The menus list only options that have items; a ticked one always stays.
const facets = reactive({ rating: {}, favorite: 0, explicit: 0, shape: {}, kinds: {}, exts: {}, animatedExts: {}, animated: 0, folderColors: {} })
const shapeRows = computed(() => SHAPES.filter((v) => facets.shape[v] > 0 || f.shapes.includes(v) || f.shapesNot.includes(v)))
const ratingRows = computed(() => [5, 4, 3, 2, 1].filter((r) => facets.rating[r] > 0 || f.ratings.includes(r) || f.ratingsNot.includes(r)))
const kindRows = computed(() => KINDS.filter((k) => facets.kinds[k] > 0 || f.kinds.includes(k) || f.kindsNot.includes(k)))
const pluginTypeRows = computed(() =>
  (S.contrib.formatTypes || [])
    .map((ft) => ({ value: 'ptype:' + ft.label, label: ft.label, count: ft.exts.reduce((s, e) => s + (facets.exts[e] || 0), 0) }))
    .filter((r) => r.count > 0 || f.kinds.includes(r.value) || f.kindsNot.includes(r.value)),
)
// A format that has both still and moving files gets a row for each.
const extRows = computed(() => {
  const out = []
  for (const [e, n] of Object.entries(facets.exts).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))) {
    const moving = facets.animatedExts[e] || 0
    if (moving > 0 && moving < n) out.push({ v: `${e}:s`, label: e, n: n - moving }, { v: `${e}:a`, label: e, tag: 'animated', n: moving })
    else out.push({ v: e, label: e, tag: moving > 0 && e !== 'gif' ? 'animated' : null, n })
  }
  for (const v of [...f.exts, ...f.extsNot]) {
    if (!out.some((r) => r.v === v)) out.push({ v, label: v.split(':')[0], tag: v.endsWith(':a') ? 'animated' : null, n: 0 })
  }
  return out
})
// the lists as AppListboxTristate takes them
const shapeOpts = computed(() => [...shapeRows.value.map((v) => ({ value: v, label: shapeName(v), count: facets.shape[v] || 0 })), { value: 'custom', label: t('filters.bar.customShape'), only: true }])
const ratingOpts = computed(() => [...ratingRows.value, 0].map((r) => ({ value: r, label: r ? t('filters.bar.stars', { n: r }) : t('filters.value.none'), count: facets.rating[r] || 0 })))
const kindOpts = computed(() => {
  const base = kindRows.value.map((k) => ({ value: k, label: t(`filters.kindNames.${k}`), count: facets.kinds[k] || 0 }))
  const otherIdx = base.findIndex((o) => o.value === 'other')
  return otherIdx >= 0 ? [...base.slice(0, otherIdx), ...pluginTypeRows.value, ...base.slice(otherIdx)] : [...base, ...pluginTypeRows.value]
})
const extOpts = computed(() => extRows.value.map((r) => ({ value: r.v, label: r.label, tag: r.tag, count: r.n })))
const fieldOpts = computed(() => fieldRows.value.map((r) => ({ value: r.v, label: String(r.label), count: r.n })))
const listQ = ref('')
const showPaired = ref(false)
const filteredFieldOpts = computed(() => {
  const q = listQ.value.trim()
  return q ? fieldOpts.value.filter((o) => fuzzy(o.label, q)) : fieldOpts.value
})
// the colours folders have, in the order the colour menus list them, and any other a plugin gave one
const folderColorName = (c) => {
  const id = LOOK_COLORS.find(([, v]) => v === c)?.[0]
  return id ? colorName(id) : c
}
const folderColorOpts = computed(() => {
  const used = new Set([...S.sidebar.folders.map((x) => x.color).filter(Boolean), ...f.folderColors, ...f.folderColorsNot])
  const known = LOOK_COLORS.map(([, c]) => c).filter((c) => c && used.has(c))
  return [...known, ...[...used].filter((c) => !known.includes(c))].map((c) => ({ value: c, label: folderColorName(c), count: facets.folderColors[c.toLowerCase()] || 0 }))
})
// left click picks, right click leaves out; doing it again clears it
function pick(list, not, v) {
  if (list.includes(v)) list.splice(list.indexOf(v), 1)
  else {
    if (not.includes(v)) not.splice(not.indexOf(v), 1)
    list.push(v)
  }
}
const facetReq = (id) => viewRequest(fullQuery(id))
async function loadFacets(id) {
  try {
    Object.assign(facets, await call('filter_facets', { req: facetReq(id) }))
  } catch {
    Object.assign(facets, { rating: {}, favorite: 0, explicit: 0, shape: {}, kinds: {}, exts: {}, animatedExts: {}, animated: 0, folderColors: {} })
  }
}
async function loadFieldFacets(id) {
  Object.assign(fieldFacets, { values: [], set: 0, total: 0 })
  try {
    Object.assign(fieldFacets, await call('field_facets', { req: facetReq(id), key: fieldKey(id) }))
  } catch {}
}
const WITHIN = computed(() => [1, 7, 30, 90, 365].map((d) => [d, t(`filters.within.d${d}`)]))
const OPS = computed(() => [['=', '='], ['!=', '≠'], ['<', '<'], ['<=', '≤'], ['>', '>'], ['>=', '≥'], ['CONTAINS', t('filters.conditions.contains')], ['empty', t('filters.conditions.empty')], ['notempty', t('filters.conditions.notEmpty')]])

// what each filter keeps, so one chip can be emptied by itself
const FILTER_KEYS = {
  color: ['color', 'gray', 'colorAccuracy'], tags: ['tags', 'tagsNot', 'untagged'], folder: ['folders', 'foldersNot'], folderColor: ['folderColors', 'folderColorsNot'], shape: ['shapes', 'shapesNot'], rating: ['ratings', 'ratingsNot', 'favorite', 'explicit'],
  types: ['kinds', 'kindsNot', 'animated'], ext: ['exts', 'extsNot'], dimensions: ['minW', 'maxW', 'minH', 'maxH'], duration: ['durMin', 'durMax'], size: ['sizeMin', 'sizeMax'],
  pages: ['pagesMin', 'pagesMax'], note: ['note', 'noteHas'], url: ['url', 'urlHas'], added: ['addedWithin', 'addedFrom', 'addedTo'], modified: ['modifiedWithin', 'modifiedFrom', 'modifiedTo'], created: ['createdWithin', 'createdFrom', 'createdTo'], fields: ['fieldConds'],
}
function clearOne(id) {
  const s = setOf(id)
  if (s) {
    for (const x of s.set) delete f.fieldPick[x.key]
    return requestSearch()
  }
  if (fieldKey(id)) {
    delete f.fieldPick[fieldKey(id)]
    return requestSearch()
  }
  const blank = emptyFilters()
  for (const k of FILTER_KEYS[id] || []) f[k] = blank[k]
  requestSearch()
}
const active = (id) => {
  if (fieldKey(id)) return fieldPickActive(f.fieldPick[fieldKey(id)])
  if (setOf(id)) return setOf(id).set.some((x) => fieldPickActive(f.fieldPick[x.key]))
  switch (id) {
    case 'color': return !!f.color || f.gray != null
    case 'tags': return f.tags.length + f.tagsNot.length > 0 || f.untagged != null
    case 'folder': return f.folders.length + f.foldersNot.length > 0
    case 'folderColor': return f.folderColors.length + f.folderColorsNot.length > 0
    case 'shape': return f.shapes.length + f.shapesNot.length > 0
    case 'rating': return f.ratings.length + f.ratingsNot.length > 0 || f.favorite != null || f.explicit != null
    case 'types': return f.kinds.length + f.kindsNot.length > 0 || f.animated != null
    case 'ext': return f.exts.length + f.extsNot.length > 0
    case 'dimensions': return !!(f.minW || f.maxW || f.minH || f.maxH)
    case 'duration': return !!(f.durMin || f.durMax)
    case 'size': return !!(f.sizeMin || f.sizeMax)
    case 'pages': return !!(f.pagesMin || f.pagesMax)
    case 'note': return !!f.note.trim() || f.noteHas != null
    case 'url': return !!f.url.trim() || f.urlHas != null
    case 'added': case 'modified': case 'created': return !!(f[id + 'Within'] || f[id + 'From'] || f[id + 'To'])
    case 'fields': return f.fieldConds.length > 0
    case 'presets': return !!f.preset
  }
  return false
}
// What is chosen, right on the chip: "mp4, jpg, -json".
const named = (list, not) => [...list, ...not.map((v) => '-' + v)].join(', ')
const extName = (v) => (v.endsWith(':a') ? t('filters.chips.animatedExt', { ext: v.split(':')[0] }) : v.split(':')[0])
function pickText(field, c) {
  const range = (c.min != null && c.min !== '') || (c.max != null && c.max !== '') ? `${c.min ?? ''}–${c.max ?? ''}` : ''
  const has = c.has == null ? '' : c.has ? t('filters.chips.set') : t('filters.chips.empty')
  return [named(c.values.map((v) => fmtValue(field, v)), (c.not || []).map((v) => fmtValue(field, v))), c.contains, range, has].filter(Boolean).join(', ')
}
function chipText(p) {
  // a field's name leads, since its icon alone doesn't say which field it is
  if (p.field) return fieldPickActive(f.fieldPick[p.field.key]) ? `${p.label}: ` + pickText(p.field, f.fieldPick[p.field.key]) : ''
  if (p.set) {
    const names = setNames(p)
    const parts = p.set.filter((x) => fieldPickActive(f.fieldPick[x.key])).map((x) => `${names[x.key]} ${pickText(x, f.fieldPick[x.key])}`)
    return parts.length ? `${p.label}: ` + parts.join(', ') : ''
  }
  switch (p.id) {
    case 'types': return [named(f.kinds.map(kindLabel), f.kindsNot.map(kindLabel)), f.animated === true ? t('filters.chips.animated') : f.animated === false ? '-' + t('filters.chips.animated') : ''].filter(Boolean).join(', ')
    case 'ext': return named(f.exts.map(extName), f.extsNot.map(extName))
    case 'shape': return named(f.shapes.map((v) => (v === 'custom' ? (f.ratioW && f.ratioH ? `${f.ratioW}:${f.ratioH}` : t('filters.bar.customShape')) : shapeName(v))), f.shapesNot.map(shapeName))
    case 'tags': return [f.untagged === true ? t('filters.chips.untagged') : f.untagged === false ? '-' + t('filters.chips.untagged') : '', named(f.tags, f.tagsNot)].filter(Boolean).join(', ')
    case 'folder': return named(f.folders, f.foldersNot)
    case 'folderColor': return named(f.folderColors.map(folderColorName), f.folderColorsNot.map(folderColorName))
    case 'rating': return [
      named(f.ratings.map((r) => (r ? r + '\u2605' : t('filters.value.none'))), f.ratingsNot.map((r) => (r ? r + '\u2605' : t('filters.value.none')))),
      f.favorite === true ? t('filters.chips.favorite') : f.favorite === false ? '-' + t('filters.chips.favorite') : '',
      f.explicit === true ? t('filters.chips.explicit') : f.explicit === false ? '-' + t('filters.chips.explicit') : '',
    ].filter(Boolean).join(', ')
    case 'added': case 'modified': case 'created': return dateText(p.id)
    case 'presets': return f.preset?.label || ''
  }
  return ''
}
// a date filter: one of the quick spans, or days from one to another (either end may be open)
const ymd = (d) => (d ? `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}` : null)
const dayOf = (s) => (s ? new Date(`${s}T00:00:00`) : null)
const shortDay = (s) => dayOf(s).toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' })
function dateText(k) {
  if (f[k + 'Within']) return WITHIN.value.find(([d]) => d === f[k + 'Within'])?.[1] || t('filters.within.days', { count: f[k + 'Within'] })
  const [a, b] = [f[k + 'From'], f[k + 'To']]
  if (a && b) return a === b ? shortDay(a) : `${shortDay(a)} – ${shortDay(b)}`
  return a ? t('filters.within.from', { day: shortDay(a) }) : b ? t('filters.within.until', { day: shortDay(b) }) : ''
}
function setWithin(k, d) {
  f[k + 'Within'] = f[k + 'Within'] === d ? null : d
  f[k + 'From'] = f[k + 'To'] = null
}
const dateRange = computed({
  get: () => {
    const k = current.value
    const [a, b] = [dayOf(f[k + 'From']), dayOf(f[k + 'To'])]
    return a || b ? [a, b] : null
  },
  set: (v) => {
    const k = current.value
    f[k + 'From'] = ymd(v?.[0])
    f[k + 'To'] = ymd(v?.[1])
    f[k + 'Within'] = null
  },
})
// One menu at a time: clicking another chip moves the menu there, its own chip closes it.
const openId = ref(null)
let openedAt = 0
function open(e, id) {
  const target = e.currentTarget
  // tags and folders use the same popups as adding them to items
  if (id === 'tags' || id === 'folder') {
    pop.value.hide()
    const list = id === 'tags' ? f.tags : f.folders
    const not = id === 'tags' ? f.tagsNot : f.foldersNot
    openPicker(id === 'tags' ? 'tags' : 'folders', { currentTarget: target }, {
      value: [...list],
      excluded: [...not],
      onToggle: (v) => pick(list, not, v),
      onExclude: (v) => pick(not, list, v),
      onClose: () => trying.value === id && (trying.value = null),
      ...(id === 'tags'
        ? { rule: f.tagRule, onRule: (v) => (f.tagRule = v), untagged: f.untagged, onUntagged: (v) => (f.untagged = v) }
        : {}),
    })
    return
  }
  if (openId.value === id) {
    pop.value.hide()
    return
  }
  pop.value.hide()
  current.value = id
  openId.value = id
  openedAt = performance.now()
  listQ.value = ''
  if (['rating', 'shape', 'types', 'ext', 'folderColor'].includes(id)) loadFacets(id)
  if (fieldKey(id)) {
    const p = pickOf(fieldKey(id))
    showPaired.value = Object.values(p.ranges || {}).some((r) => r.min != null || r.max != null)
    loadFieldFacets(id)
  }
  for (const x of setOf(id)?.set || []) pickOf(x.key)
  setTimeout(() => pop.value.show({ currentTarget: target }, target), 0)
}
// a menu with a search field opens with the caret in it
function focusSearch() {
  requestAnimationFrame(() => pop.value?.container?.querySelector('.p-listbox-filter, .list-search input')?.focus())
}
function onHide() {
  if (performance.now() - openedAt < 300) return
  openId.value = null
  trying.value = null
  listQ.value = ''
}
function togglePin(id) {
  const i = S.ui.pinnedFilters.indexOf(id)
  if (i >= 0) S.ui.pinnedFilters.splice(i, 1)
  else S.ui.pinnedFilters.push(id)
  saveUi()
}
// a filter chosen in More filters: one on the bar opens there; any other comes onto the bar for this once, and leaves it when its menu closes with nothing chosen or when it is cleared later
const trying = ref(null)
async function useFilter(id) {
  pinPop.value.hide()
  if (!S.ui.pinnedFilters.includes(id) && !active(id)) trying.value = id
  await nextTick()
  const chip = [...bar.value.querySelectorAll('.fchip')].find((c) => c.dataset.filter === id)
  if (chip) open({ currentTarget: chip }, id)
}
// drag pinned filters to reorder them
function dragPin(e, i) {
  beginDrag(e, {
    drop: (target, zone) => {
      const list = S.ui.pinnedFilters
      const [x] = list.splice(i, 1)
      list.splice(finalIndex(i, target, zone), 0, x)
      saveUi()
    },
  }, pinned.value[i]?.label || '', {
    sort: true,
    accept: (t) => t.startsWith('fpin:'),
    onStart: () => {
      pop.value?.hide()
      openId.value = null
      if (S.picker) S.picker = null
    },
  })
}
const presets = computed(() => S.contrib.filters.map((x) => ({ label: x.label, query: x.query })))

// ---------------------------------------------------------------- at the right end of the bar
const hasFilter = computed(() => !!filterQuery())
const anything = computed(() => hasFilter.value || !!S.search.trim())
const savedMenu = ref()
const filtersNow = () => ({ filters: JSON.parse(JSON.stringify(S.filters)), search: S.search })
const replaceSaved = (x, change) => storeSavedFilters(S.savedFilters.map((y) => (y.id === x.id ? { ...y, ...change } : y)))
function saveCurrent() {
  S.dialogs.name = { title: t('filters.saved.saveTitle'), name: '', done: (name) => storeSavedFilters([...S.savedFilters, { id: Date.now().toString(36), name, ...filtersNow() }]) }
}
function applySaved(x) {
  Object.assign(S.filters, emptyFilters(), JSON.parse(JSON.stringify(x.filters)))
  S.search = x.search || ''
  requestSearch()
}
// as the Actions menu: a row applies its filters, its button renames, updates or removes them
const savedItems = () => [
  ...S.savedFilters.map((x) => ({
    label: x.name,
    icon: 'filter',
    command: () => applySaved(x),
    more: [
      { label: t('filters.saved.rename'), icon: 'cursor-text', command: () => (S.dialogs.name = { title: t('filters.saved.renameTitle'), name: x.name, okLabel: t('filters.saved.rename'), done: (name) => replaceSaved(x, { name }) }) },
      { label: t('filters.saved.update'), icon: 'refresh', disabled: !anything.value, command: () => replaceSaved(x, filtersNow()) },
      { separator: true },
      { label: t('filters.saved.remove'), icon: 'trash', command: () => removeSaved(x) },
    ],
  })),
  { separator: true },
  { label: t('filters.saved.saveCurrent'), icon: 'device-floppy', disabled: !anything.value, command: saveCurrent },
  { label: t('filters.saved.saveSmart'), icon: 'layout-grid-add', disabled: !anything.value, command: saveAsSmart },
]
function saveAsSmart() {
  S.dialogs.smart = { id: null, name: t('filters.smart.newName'), query: fullQuery() }
}
function removeSaved(x) {
  storeSavedFilters(S.savedFilters.filter((y) => y.id !== x.id))
}
watch(() => filterQuery(), () => requestSearch(150))
</script>

<template>
  <div ref="bar" class="filterbar" :class="mode">
    <button v-for="(p, i) in pinned" :key="p.id" class="fchip" :class="{ on: active(p.id), open: openId === p.id || S.picker?.anchor?.dataset?.filter === p.id }" :data-filter="p.id" :data-drop="p.extra ? undefined : 'fpin:' + i" @pointerdown="!p.extra && $event.button === 0 && dragPin($event, i)" @mousedown.middle.prevent @auxclick.prevent="$event.button === 1 && active(p.id) && clearOne(p.id)" @click="open($event, p.id)">
      <span v-if="p.id === 'color'" class="rainbow" :class="{ solid: f.color && f.color !== 'grayscale', gray: f.color === 'grayscale' }" :style="f.color && f.color !== 'grayscale' ? { background: f.color } : {}" />
      <AppIcon v-else :name="p.icon" :size="17" />
      <template v-if="chipText(p)"><span class="chip-picked ellipsis">{{ chipText(p) }}</span></template>
      <template v-else>{{ p.label }}</template>
      <span v-if="active(p.id)" class="chip-clear" :aria-label="$t('filters.tips.clearOne')" @pointerdown.stop @click.stop="clearOne(p.id)"><AppIcon name="x" :size="10" /></span>
    </button>
    <button class="icon-btn" v-tooltip.bottom="$t('filters.tips.more')" @click="pinPop.toggle($event)"><AppIcon name="plus" :size="18" /></button>

    <div v-if="anything || S.savedFilters.length" class="f-right">
      <button class="icon-btn" v-tooltip.bottom="$t('filters.tips.saved')" @click="savedMenu.toggle($event)"><AppIcon name="bookmarks" :size="18" /></button>
      <AppMenu ref="savedMenu" :model="savedItems" />
      <template v-if="anything">
        <button class="icon-btn" :class="{ on: S.filterLock }" v-tooltip.bottom="S.filterLock ? $t('filters.tips.unlock') : $t('filters.tips.lock')" @click="S.filterLock = !S.filterLock"><AppIcon :name="S.filterLock ? 'lock' : 'lock-open'" :size="18" /></button>
        <span class="f-sep" />
        <button class="icon-btn" :disabled="!hasFilter" v-tooltip.bottom="withKey($t('filters.tips.clear'), 'filters.clear')" @click="resetFilters"><AppIcon name="trash" :size="18" /></button>
      </template>
    </div>

    <Popover ref="pinPop" class="list-pop" @hide="pinQ = ''">
      <AppListSearch v-focus v-model="pinQ" @keydown.enter="pinFirst" />
      <div class="pins">
        <div v-for="a in pinMain" :key="a.id" class="pin-row" @click="useFilter(a.id)">
          <AppIcon :name="a.icon" :size="17" /><span class="grow ellipsis"><AppHighlight :text="a.label" :q="pinTerm" /></span>
          <button class="icon-btn mini" @click.stop="togglePin(a.id)" v-tooltip="S.ui.pinnedFilters.includes(a.id) ? $t('filters.tips.unpin') : $t('filters.tips.pin')">
            <AppIcon :name="S.ui.pinnedFilters.includes(a.id) ? 'pin' : 'pinned-off'" />
          </button>
        </div>
        <hr v-if="pinMain.length && pinGroups.length" class="menu-hr" />
        <template v-for="g in pinGroups" :key="g.label">
          <div class="pin-row pin-group" @click="openGroup = openGroup === g.label ? null : g.label">
            <AppIcon :name="pinOpen(g) ? 'chevron-down' : 'chevron-right'" class="faint" /><AppIcon name="forms" :size="17" /><span class="grow ellipsis"><AppHighlight :text="g.label" :q="pinTerm" /></span><span class="faint pin-count">{{ g.items.length }}</span>
          </div>
          <template v-if="pinOpen(g)">
            <div v-for="a in g.items" :key="a.id" class="pin-row pin-sub" @click="useFilter(a.id)">
              <AppIcon :name="a.icon" :size="17" /><span class="grow ellipsis"><AppHighlight :text="a.label" :q="pinTerm" /></span>
              <button class="icon-btn mini" @click.stop="togglePin(a.id)" v-tooltip="S.ui.pinnedFilters.includes(a.id) ? $t('filters.tips.unpin') : $t('filters.tips.pin')">
                <AppIcon :name="S.ui.pinnedFilters.includes(a.id) ? 'pin' : 'pinned-off'" />
              </button>
            </div>
          </template>
        </template>
        <div v-if="!pinMain.length && !pinGroups.length" class="faint pins-none">{{ $t('filters.bar.nothingFound') }}</div>
      </div>
    </Popover>

    <Popover ref="pop" class="fpop" @show="focusSearch" @hide="onHide">
      <div class="panel" :class="['menu-' + current, { fit: curKind === 'range' || !!curSet, paired: !!(curPaired && showPaired) }]">
        <template v-if="current === 'color'">
          <ColorPicker inline :modelValue="pickerHex.slice(1)" @update:modelValue="(v) => (f.color = '#' + v.toUpperCase())" />
          <div class="pal">
            <button class="pal-c none" :class="{ sel: !f.color }" v-tooltip="$t('filters.bar.anyColor')" @click="f.color = null" />
            <button v-for="c in PALETTE" :key="c" class="pal-c" :class="{ sel: f.color === c }" :style="{ background: c }" @click="f.color = c" />
          </div>
          <div class="hex-row">
            <InputText class="hex mono" :modelValue="f.color && f.color !== 'grayscale' ? f.color : ''" :placeholder="'#FF0000'" spellcheck="false" maxlength="7" @change="setHex($event.target.value)" />
            <span class="hex-sw" :style="{ background: pickerHex }" />
          </div>
          <div class="acc" :class="{ off: !f.color || f.color === 'grayscale' }" v-tooltip.bottom="$t('filters.bar.accuracy')"><span class="faint">{{ $t('filters.bar.accuracy') }}</span><Slider v-model="f.colorAccuracy" :min="0" :max="100" :step="5" :disabled="!f.color || f.color === 'grayscale'" class="grow" /><span class="acc-n mono faint">{{ f.colorAccuracy }}</span></div>
          <AppCheckboxTristate v-model="f.gray" :label="$t('filters.bar.grayscale')" />
        </template>
        <template v-else-if="current === 'folderColor'">
          <AppListboxTristate :options="folderColorOpts" :chosen="f.folderColors" :out="f.folderColorsNot" :empty="$t('filters.bar.noFolderColor')" @choose="(v) => pick(f.folderColors, f.folderColorsNot, v)" @leave-out="(v) => pick(f.folderColorsNot, f.folderColors, v)">
            <template #label="{ option }"><span class="fc-row"><span class="swatch-dot" :style="{ background: option.value }" />{{ option.label }}</span></template>
          </AppListboxTristate>
        </template>
        <template v-else-if="current === 'shape'">
          <AppListboxTristate :options="shapeOpts" :chosen="f.shapes" :out="f.shapesNot" @choose="(v) => pick(f.shapes, f.shapesNot, v)" @leave-out="(v) => pick(f.shapesNot, f.shapes, v)" />
          <div v-if="f.shapes.includes('custom')" class="minmax ratio">
            <InputNumber v-model="f.ratioW" class="mm grow" :min="1" :useGrouping="false" :maxFractionDigits="3" placeholder="W" />
            <span class="faint">:</span>
            <InputNumber v-model="f.ratioH" class="mm grow" :min="1" :useGrouping="false" :maxFractionDigits="3" placeholder="H" />
          </div>
        </template>
        <template v-else-if="current === 'rating'">
          <AppListboxTristate :options="ratingOpts" :chosen="f.ratings" :out="f.ratingsNot" @choose="(v) => pick(f.ratings, f.ratingsNot, v)" @leave-out="(v) => pick(f.ratingsNot, f.ratings, v)">
            <template #label="{ option }"><span v-if="option.value" class="stars"><AppIcon v-for="n in 5" :key="n" name="star-filled" :size="16" :class="{ dim: n > option.value }" /></span><template v-else>{{ $t('filters.value.none') }}</template></template>
          </AppListboxTristate>
          <hr class="menu-hr" />
          <AppCheckboxTristate v-model="f.favorite" :label="$t('filters.bar.favorite')" :count="facets.favorite || 0" />
          <AppCheckboxTristate v-model="f.explicit" :label="$t('filters.bar.explicit')" :count="facets.explicit || 0" />
        </template>
        <template v-else-if="current === 'types'">
          <AppListboxTristate :options="kindOpts" :chosen="f.kinds" :out="f.kindsNot" @choose="(v) => pick(f.kinds, f.kindsNot, v)" @leave-out="(v) => pick(f.kindsNot, f.kinds, v)" />
          <hr class="menu-hr" />
          <AppCheckboxTristate v-model="f.animated" :label="$t('filters.bar.animated')" :count="facets.animated || 0" />
        </template>
        <template v-else-if="current === 'ext'">
          <AppListboxTristate :options="extOpts" :chosen="f.exts" :out="f.extsNot" filter @choose="(v) => pick(f.exts, f.extsNot, v)" @leave-out="(v) => pick(f.extsNot, f.exts, v)" />
        </template>
        <template v-else-if="current === 'dimensions'">
          <div v-for="[a, b, l] in [['minW', 'maxW', 'W'], ['minH', 'maxH', 'H']]" :key="l" class="minmax dim">
            <span class="dim-l">{{ l }}</span>
            <InputNumber v-model="f[a]" class="mm" :min="0" :useGrouping="false" :maxFractionDigits="3" :placeholder="$t('filters.value.min')" />
            <span class="faint">-</span>
            <InputNumber v-model="f[b]" class="mm" :min="0" :useGrouping="false" :maxFractionDigits="3" :placeholder="$t('filters.value.max')" />
          </div>
        </template>
        <template v-else-if="current === 'duration'">
          <div class="minmax">
            <InputNumber v-model="f.durMin" class="mm" :min="0" :useGrouping="false" :maxFractionDigits="3" :placeholder="$t('filters.value.min')" />
            <span class="faint">-</span>
            <InputNumber v-model="f.durMax" class="mm" :min="0" :useGrouping="false" :maxFractionDigits="3" :placeholder="$t('filters.value.max')" />
            <Select v-model="f.durUnit" :options="DURATION_UNITS" :optionLabel="(u) => $t(`filters.units.${u}`)" class="unit" />
          </div>
        </template>
        <template v-else-if="current === 'size'">
          <div class="minmax">
            <InputNumber v-model="f.sizeMin" class="mm" :min="0" :useGrouping="false" :maxFractionDigits="3" :placeholder="$t('filters.value.min')" />
            <span class="faint">-</span>
            <InputNumber v-model="f.sizeMax" class="mm" :min="0" :useGrouping="false" :maxFractionDigits="3" :placeholder="$t('filters.value.max')" />
            <Select v-model="f.sizeUnit" :options="SIZE_UNITS" class="unit" />
          </div>
        </template>
        <template v-else-if="current === 'pages'">
          <div class="minmax">
            <InputNumber v-model="f.pagesMin" class="mm" :min="0" :useGrouping="false" :maxFractionDigits="3" :placeholder="$t('filters.value.min')" />
            <span class="faint">-</span>
            <InputNumber v-model="f.pagesMax" class="mm" :min="0" :useGrouping="false" :maxFractionDigits="3" :placeholder="$t('filters.value.max')" />
          </div>
        </template>
        <template v-else-if="current === 'note' || current === 'url'">
          <AppCheckboxTristate v-if="current === 'note'" v-model="f.noteHas" :label="$t('filters.bar.hasNote')" />
          <AppCheckboxTristate v-else v-model="f.urlHas" :label="$t('filters.bar.hasUrl')" />
          <Textarea v-if="current === 'note'" v-model="f.note" class="kw" :placeholder="$t('filters.bar.keywords')" spellcheck="false" />
          <Textarea v-else v-model="f.url" class="kw" :placeholder="$t('filters.bar.keywords')" spellcheck="false" />
        </template>
        <template v-else-if="current === 'added' || current === 'modified' || current === 'created'">
          <DatePicker v-model="dateRange" selectionMode="range" inline :manualInput="false" class="range-cal" />
          <div class="spans">
            <label v-for="[d, l] in WITHIN" :key="d" class="tri-one" @click.prevent="setWithin(current, d)">
              <RadioButton :modelValue="f[current + 'Within']" :value="d" readonly tabindex="-1" class="span-radio" /><span class="grow">{{ l }}</span>
            </label>
          </div>
        </template>
        <template v-else-if="current === 'fields'">
          <div v-for="(c, i) in f.fieldConds" :key="i" class="row cond">
            <Select v-model="c.key" :options="conditionFields" optionLabel="label" optionValue="key" filter autoFilterFocus :placeholder="$t('filters.rules.field')" class="fsel" />
            <Select v-model="c.op" :options="OPS" :optionLabel="(o) => o[1]" :optionValue="(o) => o[0]" class="osel" />
            <InputText v-if="c.op !== 'empty' && c.op !== 'notempty'" v-model="c.value" class="vin" />
            <button class="icon-btn" @click="f.fieldConds.splice(i, 1)"><AppIcon name="minus" /></button>
          </div>
          <button class="icon-btn" @click="f.fieldConds.push({ key: '', op: '=', value: '' })"><AppIcon name="plus" /> {{ $t('filters.bar.addCondition') }}</button>
        </template>
        <div v-else-if="curSet" class="set-rows">
          <template v-for="x in curSet.set" :key="x.key">
            <span :class="{ 'dim-l': lettered }" v-tooltip.left="lettered ? x.label : null">{{ curNames[x.key] }}</span>
            <InputNumber :modelValue="f.fieldPick[x.key]?.min ?? null" class="mm" :useGrouping="false" :maxFractionDigits="3" :placeholder="$t('filters.value.min')" :aria-label="$t('filters.value.from', { name: x.label })" @update:modelValue="(n) => (pickOf(x.key).min = n)" />
            <span class="faint">-</span>
            <InputNumber :modelValue="f.fieldPick[x.key]?.max ?? null" class="mm" :useGrouping="false" :maxFractionDigits="3" :placeholder="$t('filters.value.max')" :aria-label="$t('filters.value.to', { name: x.label })" @update:modelValue="(n) => (pickOf(x.key).max = n)" />
          </template>
        </div>
        <template v-else-if="curField && curPick">
          <template v-if="curKind === 'range'">
            <div class="minmax">
              <InputNumber v-model="curPick.min" class="mm" :useGrouping="false" :maxFractionDigits="3" :placeholder="fieldSpan ? String(fmtValue(curField, fieldSpan[0])) : $t('filters.value.min')" />
              <span class="faint">-</span>
              <InputNumber v-model="curPick.max" class="mm" :useGrouping="false" :maxFractionDigits="3" :placeholder="fieldSpan ? String(fmtValue(curField, fieldSpan[1])) : $t('filters.value.max')" />
            </div>
          </template>
          <template v-else-if="curKind === 'list'">
            <AppListSearch v-model="listQ" class="field-search">
              <button v-if="curPaired" class="icon-btn" :class="{ on: showPaired }" :aria-pressed="showPaired" :aria-label="curPaired.label" v-tooltip.bottom="curPaired.label" @click="showPaired = !showPaired"><AppIcon name="adjustments-horizontal" :size="18" /></button>
            </AppListSearch>
            <AppListboxTristate :options="filteredFieldOpts" :chosen="curPick.values" :out="curPick.not" :empty="$t('filters.bar.nothingHere')" @choose="(v) => pick(curPick.values, curPick.not, v)" @leave-out="(v) => pick(curPick.not, curPick.values, v)">
              <template v-if="curPaired && showPaired" #extra="{ option }">
                <span class="pair-range" @click.stop @mousedown.stop @keydown.stop>
                  <InputNumber :modelValue="rangeOf(option.value).min ?? null" class="pr" :useGrouping="false" :maxFractionDigits="3" :placeholder="$t('filters.value.min')" :aria-label="$t('filters.value.from', { name: curPaired.label })" @update:modelValue="(n) => setRange(option.value, 'min', n)" />
                  <span class="faint">-</span>
                  <InputNumber :modelValue="rangeOf(option.value).max ?? null" class="pr" :useGrouping="false" :maxFractionDigits="3" :placeholder="$t('filters.value.max')" :aria-label="$t('filters.value.to', { name: curPaired.label })" @update:modelValue="(n) => setRange(option.value, 'max', n)" />
                </span>
              </template>
            </AppListboxTristate>
          </template>
          <template v-else>
            <AppCheckboxTristate v-model="curPick.has" :label="$t('filters.bar.hasValue')" />
            <Textarea v-if="curKind === 'text'" v-model="curPick.contains" class="kw" :placeholder="$t('filters.bar.keywords')" spellcheck="false" />
          </template>
        </template>
        <template v-else-if="current === 'presets'">
          <div v-if="!presets.length" class="faint">{{ $t('filters.bar.noPresets') }}</div>
          <div v-for="p in presets" :key="p.label" class="pin-row" :class="{ sel: f.preset?.label === p.label }" @click="f.preset = f.preset?.label === p.label ? null : p">
            <AppIcon name="filter" /> {{ p.label }}</div>
        </template>
      </div>
    </Popover>
  </div>
</template>

<style scoped>
.f-right { margin-left: auto; display: inline-flex; align-items: center; gap: 0.14rem; flex: none; }
.f-sep { width: 1px; height: round(1.26rem, 1px); margin: 0 calc(0.28rem * var(--g)); background: var(--border); }
.chip-picked { max-width: 12.59rem; }
.panel.menu-types, .panel.menu-ext, .panel[class*='menu-field:'] { width: round(16.08rem, 1px); gap: 0; }
.panel[class*='menu-field:'] { width: round(18.18rem, 1px); }
/* room for a range beside each value */
.panel[class*='menu-field:'].paired { width: round(26.57rem, 1px); }
.menu-hr { width: 100%; border: 0; border-top: 1px solid var(--border); margin: calc(0.28rem * var(--g)) 0; }
.none { padding: calc(0.56rem * var(--g)); }
.filterbar { display: flex; align-items: center; gap: 0.14rem; flex-wrap: wrap; flex: 1; min-width: 0; }
.filterbar.panel { padding: calc(0.28rem * var(--g)) calc(0.7rem * var(--g)); flex: none; }
.fchip { display: inline-flex; align-items: center; gap: calc(0.42rem * var(--g)); height: round(2.1rem, 1px); padding: 0 calc(0.49rem * var(--g)); border-radius: calc(6px * var(--r)); border: 0; background: transparent; color: var(--text); font: inherit; font-size: 1rem; cursor: pointer; white-space: nowrap; }
.fchip .ico { color: var(--text-muted); }
.fchip:hover, .fchip.open { background: var(--active); }
.fchip { position: relative; }
.fchip.on { background: var(--active); color: var(--text); }
.fchip.on:hover .chip-clear { display: flex; }
.fchip.on .ico { color: var(--text); }
.rainbow { width: round(1.12rem, 1px); height: round(1.12rem, 1px); border-radius: var(--round); flex: none; background: conic-gradient(#ef4444, #facc15, #22c55e, #14b8a6, #3b82f6, #a855f7, #ec4899, #ef4444); -webkit-mask: radial-gradient(circle, transparent 4.5px, #000 5px); mask: radial-gradient(circle, transparent 4.5px, #000 5px); }
.rainbow.solid { -webkit-mask: none; mask: none; }
.rainbow.gray { -webkit-mask: none; mask: none; background: linear-gradient(135deg, #fff, #333); }
/* as wide as OpenVue's picker, whose parts are fixed in pixels */
.panel.menu-color { width: calc(176px + 0.98rem * var(--g)); gap: calc(0.7rem * var(--g)); padding: calc(0.49rem * var(--g)); }
.menu-color :deep(.p-colorpicker-panel-inline) { width: 176px; height: 150px; position: relative; }
.menu-color :deep(.p-colorpicker-color-selector) { inset-block-start: 0; inset-inline-start: 0; }
.menu-color :deep(.p-colorpicker-hue) { inset-block-start: 0; inset-inline-start: 159px; }
.pal { display: grid; grid-template-columns: repeat(7, 1fr); gap: 0.42rem; }
.pal-c { aspect-ratio: 1; padding: 0; border: 0; border-radius: var(--round); cursor: pointer; outline: 1px solid color-mix(in srgb, var(--text) 16%, transparent); outline-offset: -1px; }
.pal-c.sel { outline: 2px solid var(--accent); outline-offset: 1px; }
.pal-c.none { background: linear-gradient(135deg, transparent 46%, var(--danger) 46% 54%, transparent 54%); }
.pal-c.grayish { background: linear-gradient(135deg, #fff, #333); }
.hex-row { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); }
.hex { flex: 1; min-width: 0; height: round(1.96rem, 1px); padding: 0 calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); border: 1px solid var(--input-border); background: var(--input); color: var(--text); font-size: 0.92rem; outline: 0; }
.hex:focus { border-color: var(--accent); }
.hex-sw { width: round(1.96rem, 1px); height: round(1.96rem, 1px); border-radius: calc(6px * var(--r)); flex: none; }
.acc { display: flex; align-items: center; gap: calc(0.7rem * var(--g)); padding: 0 calc(0.28rem * var(--g)); font-size: 0.92rem; }
.acc.off { opacity: 0.5; }
.acc-n { width: 2.2em; text-align: right; font-size: 0.85rem; }
.spans { display: grid; grid-template-columns: 1fr 1fr; padding-top: calc(0.28rem * var(--g)); border-top: 1px solid var(--border); }
.span-radio { pointer-events: none; }
.panel { width: round(25.17rem, 1px); display: flex; flex-direction: column; gap: calc(0.7rem * var(--g)); }
.pins { width: round(18.18rem, 1px); max-height: 70vh; overflow-y: auto; padding: calc(0.35rem * var(--g)); box-sizing: border-box; }
/* a field sits under its group's name, as the search scope's do */
.pin-row.pin-sub { padding-left: calc(3.85rem * var(--g)); }
.pin-row { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); height: round(2.1rem, 1px); padding: 0 calc(0.42rem * var(--g)); border-radius: calc(6px * var(--r)); cursor: pointer; }
.pin-row .icon-btn.mini { min-width: round(1.68rem, 1px); height: round(1.68rem, 1px); padding: 0; }
.pin-count { padding-right: calc(0.35rem * var(--g)); }
.pins-none { padding: calc(0.42rem * var(--g)); }
.pin-row:hover, .pin-row.sel { background: var(--hover); }
.stars { display: inline-flex; gap: 0.14rem; color: var(--text-muted); }
.stars .dim { opacity: 0.3; }
.panel.menu-rating { width: round(13.99rem, 1px); gap: 0; }
.panel.menu-folderColor { width: round(13.99rem, 1px); gap: 0; }
.fc-row { display: inline-flex; align-items: center; gap: calc(0.56rem * var(--g)); }
.fc-row .swatch-dot { cursor: inherit; }
.panel.menu-shape { width: round(15.52rem, 1px); gap: 0; }
.panel.menu-pages { width: round(12.87rem, 1px); }
.panel.menu-added, .panel.menu-modified, .panel.menu-created { width: round(23.08rem, 1px); gap: calc(0.42rem * var(--g)); }
.range-cal :deep(.p-datepicker-panel) { border: 0; padding: 0; background: transparent; }
/* the month and year in the middle between the menu's top and the line under them, as the menu has less room above than the header below */
.range-cal :deep(.p-datepicker-header) { padding-top: calc(0.14rem * var(--g)); }
.panel.menu-duration, .panel.menu-size { width: round(17.9rem, 1px); }
.panel.menu-dimensions { width: round(15.38rem, 1px); gap: 0; }
.panel.menu-note, .panel.menu-url, .panel[class*='menu-field:'], .panel[class*='menu-fset:'] { gap: 0; }
.panel.menu-note, .panel.menu-url { width: round(18.18rem, 1px); }
.panel.fit { width: max-content; }
.minmax.dim { padding: calc(0.28rem * var(--g)) calc(0.56rem * var(--g)); }
.dim-l { width: round(0.98rem, 1px); text-align: center; font-size: 1.08rem; }
/* a set's rows share their columns, so words as names keep the fields lined up */
.set-rows { display: grid; grid-template-columns: repeat(4, auto); align-items: center; gap: calc(0.56rem * var(--g)); padding: calc(0.28rem * var(--g)) calc(0.56rem * var(--g)); }
.kw { min-height: round(4.48rem, 1px); margin: calc(0.42rem * var(--g)) 0.14rem 0.14rem; padding: calc(0.42rem * var(--g)) calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); border: 1px solid var(--input-border); background: var(--input); color: var(--text); font: inherit; resize: vertical; outline: 0; }
.kw:focus { border-color: var(--accent); }
.kw::placeholder { color: var(--text-faint); }
.minmax { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); padding: calc(0.56rem * var(--g)); }
.minmax.ratio { padding: calc(0.42rem * var(--g)); }
.mm { width: round(4.9rem, 1px); min-width: 0; height: round(1.96rem, 1px); }
.pair-range { display: flex; align-items: center; gap: calc(0.28rem * var(--g)); flex: none; }
.field-search { margin: calc(-0.35rem * var(--g)) calc(-0.35rem * var(--g)) calc(0.28rem * var(--g)); }
.pr { width: round(3.92rem, 1px); min-width: 0; height: round(1.82rem, 1px); }
.pr :deep(.p-inputnumber-input) { width: 100%; height: round(1.82rem, 1px); padding: 0 calc(0.42rem * var(--g)); text-align: center; }
.mm :deep(.p-inputnumber-input) { width: 100%; height: round(1.96rem, 1px); padding: 0 calc(0.56rem * var(--g)); text-align: center; }
.unit { width: round(5.03rem, 1px); height: round(1.96rem, 1px); }
.unit :deep(.p-select-label) { padding: 0 0 0 calc(0.56rem * var(--g)); line-height: round(1.82rem, 1px); font-size: 1rem; }
.unit :deep(.p-select-dropdown) { width: round(1.68rem, 1px); }
.w { width: 100%; }
.cond { gap: calc(0.28rem * var(--g)); }
.fsel { width: round(9.09rem, 1px); }
.osel { width: round(6.99rem, 1px); }
.vin { width: round(6.29rem, 1px); }
</style>

<style>
.fpop .p-popover-content { padding: calc(0.35rem * var(--g)); }
/* a list's search reaches the menu's edges, over the padding */
.fpop .tri-list { --list-search-margin: calc(-0.35rem * var(--g)) calc(-0.35rem * var(--g)) calc(0.28rem * var(--g)); }
</style>

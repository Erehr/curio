<script setup>
import { computed, markRaw, nextTick, onBeforeUnmount, onMounted, reactive, ref, toRaw, watch } from 'vue'
import Button from 'openvue/button'
import Checkbox from 'openvue/checkbox'
import Column from 'openvue/column'
import DataTable from 'openvue/datatable'
import InputGroup from 'openvue/inputgroup'
import InputGroupAddon from 'openvue/inputgroupaddon'
import InputNumber from 'openvue/inputnumber'
import Message from 'openvue/message'
import ProgressBar from 'openvue/progressbar'
import RadioButton from 'openvue/radiobutton'
import Select from 'openvue/select'
import SelectButton from 'openvue/selectbutton'
import Slider from 'openvue/slider'
import ToggleSwitch from 'openvue/toggleswitch'
import { call, errorText } from '../platform/ipc.js'
import { getCurrentWindow } from '../platform/windowFrame.js'
import { previewUrl } from '../platform/fileUrls.js'
import { S } from '../app/state.js'
import { saveUi } from '../app/preferences.js'
import { toast } from '../app/notify.js'
import { askHere } from '../app/windowRequests.js'
import { t } from '../shared/i18n.js'
import { percentText, sizeText } from '../shared/numbers.js'
import { rem } from '../shared/rem.js'
import AppIcon from '../shared/AppIcon.vue'
import AppImageCompare from '../shared/AppImageCompare.vue'
import ToolPage from './ToolPage.vue'
import ToolThumbnail from './ToolThumbnail.vue'
import { asJob, itemsOf, loadRows, rows as gridRows } from './tools.js'
import { WAYS, RANGE, KEEP_RULES, SORTS, complete, findArgs, prepare, keeper, freed, removed, settleArgs, matchOf, likeness, relation, dateText } from './duplicatesEngine.js'

// The duplicate finder: where to look and the ways to compare in the side column, the groups found beside the active one with its members compared, and moving the copies to the trash in the footer.
const title = (way) => t(`tools.duplicates.ways.${way}.title`)
const hint = (way) => t(`tools.duplicates.ways.${way}.hint`)
// a count in a message: shown with its digits grouped, its form chosen by the number
const tc = (key, n, named = {}) => t(key, { ...named, count: n.toLocaleString() }, n)
// a score, rounded down: 99.6% alike isn't identical
const percent = (score) => percentText(Math.floor(score * 100) / 100)
const options = (values, label) => values.map((value) => ({ value, label: label(value) }))
const ROW_H = 4.48
// a group in the simple view: its line and a row of pictures with two lines under each
const TILE_ROW_H = 14.97
// a row's height in px, as the virtual scroller places them
const rowPx = (details) => Math.round(rem(details ? ROW_H : TILE_ROW_H))
const CHUNK = 100
const POLL_MS = 500
const lengthText = (s) => `${Math.floor(s / 60)}:${String(Math.round(s % 60)).padStart(2, '0')}`
const shapeText = (it) => (it?.kind === 'video' || it?.kind === 'audio' ? (it.duration ? lengthText(it.duration) : '') : it?.width ? `${it.width} × ${it.height}` : '')
const placeText = (m) => [(m.item?.folders || []).map((f) => f.name).join(', '), m.store ? '' : t('tools.duplicates.detail.linked')].filter(Boolean).join(' · ')
/** How a member matches the kept one, in words. */
function matchText(g, i) {
  const m = matchOf(g, i)
  return m.is === 'way' ? t(`tools.duplicates.match.ways.${m.way}`, { percent: percent(m.score) }) : t(`tools.duplicates.match.${m.is}`)
}

const s = reactive(complete(S.ui.duplicates || {}))
const open = ref(WAYS.find((w) => w !== 'content' && s.ways[w].on) || null)
const opened = ref({ items: [], action: null })
const view = ref(null)
const folder = ref(null)
const keepFolder = ref(null)
const job = ref(null)
const progress = ref(null)
const result = ref(null)
const groups = ref([])
// the group shown beside the list, by key, so it stays the same one while the list changes
const active = ref(null)
// the groups' order, set when they come and when another order is chosen, so a group never moves while it is worked on
const order = ref([])
const focus = ref(null)
const show = ref('all')
const sort = ref('space')
const mode = ref('side')
const busy = ref(null)
const done = ref(null)
const loading = ref(null)
const tick = ref(0)
const bump = () => tick.value++

// ------------------------------------------------------------ settings, kept with Curio's
watch(s, () => {
  S.ui.duplicates = structuredClone(toRaw(s))
  saveUi()
}, { deep: true })

// what the window was opened for wins over the scope used last time
let wanted = null
// a scope that can't apply now (nothing selected, no folder shown or chosen) falls back to the library
function want() {
  if (wanted) Object.assign(s, wanted)
  const n = opened.value.items.length
  const can = { library: true, selected: n > 0, shown: !view.value || shownUsable.value, folder: !!folder.value }
  if (!can[s.scope]) s.scope = 'library'
}
// several selected: those; one picked from the item menu: its copies anywhere
watch(() => S.toolWindow.seq, () => {
  const run = S.toolWindow.run || {}
  const items = run.items || []
  opened.value = { items, action: run.action || null }
  wanted = items.length > 1 || (items.length === 1 && run.action === 'find') ? { scope: 'selected' } : null
  want()
  refreshView()
}, { immediate: true })
async function refreshView() {
  view.value = await call('view.get').catch(() => null)
  want()
}
const onFocus = () => refreshView()

// ------------------------------------------------------------ the search
const shownUsable = computed(() => view.value?.show === 'folder' && view.value.view === 'grid')
const scopeOptions = computed(() => [
  { value: 'library', label: t('tools.duplicates.scope.library') },
  { value: 'shown', label: t('tools.duplicates.scope.shown'), disabled: !shownUsable.value },
  { value: 'selected', label: opened.value.items.length ? t('tools.duplicates.scope.selectedCount', { n: opened.value.items.length.toLocaleString() }) : t('tools.duplicates.scope.selected'), disabled: !opened.value.items.length },
  { value: 'folder', label: folder.value ? folder.value.name : t('tools.duplicates.scope.folder') },
])
function scopeArgs() {
  if (s.scope === 'selected') return { ids: opened.value.items }
  if (s.scope === 'folder' && folder.value) return { folder: folder.value.id }
  if (s.scope === 'shown' && shownUsable.value) {
    const v = view.value
    const out = v.show === 'folder' ? { folder: v.id } : {}
    if (v.query) out.query = v.query
    return Object.keys(out).length ? out : null
  }
  return null
}
const canFind = computed(() => WAYS.some((w) => s.ways[w].on) && !(s.scope === 'folder' && !folder.value))
async function chooseFolder(title, selected = []) {
  const picked = await askHere({ kind: 'folders', title, selected }).catch(() => null)
  const id = picked?.[0]
  if (id == null) return null
  const f = await call('folder.get', { id }).catch(() => null)
  return { id, name: f?.name || t('tools.duplicates.folder') }
}
async function pickFolder() {
  const f = await chooseFolder(t('tools.duplicates.scope.pick'), folder.value ? [folder.value.id] : [])
  if (!f) return false
  folder.value = f
  return true
}
// one item selected has nothing to be compared with but the library; the choice for several stays as it was
const single = computed(() => s.scope === 'selected' && opened.value.items.length === 1)
const against = computed(() => (single.value ? 'library' : s.against))
async function setScope(v) {
  if (v === 'folder' && !(await pickFolder()) && !folder.value) return
  s.scope = v
}

async function find() {
  if (!canFind.value || job.value) return
  const scope = scopeArgs()
  const args = { ...findArgs(s), ...(scope || {}), ...(scope && against.value === 'library' ? { withLibrary: true } : {}) }
  done.value = null
  result.value = null
  groups.value = []
  progress.value = { done: 0, total: 0, text: t('tools.duplicates.starting') }
  try {
    const r = await call('duplicates.find', args)
    job.value = r.job
    watchJob(r.job)
  } catch (e) {
    progress.value = null
    toast(errorText(e), 'error')
  }
}
// the job's progress, looked at while it runs
let polling = 0
function watchJob(id) {
  clearInterval(polling)
  polling = setInterval(async () => {
    const j = await call('job.get', { job: id }).catch(() => null)
    if (!j || j.id !== job.value) return
    if (j.state === 'running') progress.value = { done: j.done, total: j.total, text: j.text }
    else finished(j)
  }, POLL_MS)
}
async function finished(j) {
  clearInterval(polling)
  job.value = null
  progress.value = null
  if (j.state === 'failed') return toast(j.error || t('tools.duplicates.failed'), 'error')
  if (!j.result) return
  await present(j.result)
}
async function stop() {
  if (job.value) await call('job.cancel', { job: job.value }).catch(() => {})
}

/** The groups with their items, the kept one chosen by the rule. */
async function present(r) {
  const ids = [...new Set(r.groups.flatMap((g) => g.members.map((m) => m.id)))]
  const items = new Map()
  loading.value = { done: 0, total: ids.length }
  for (const it of await itemsOf(ids, (n) => (loading.value = { done: n, total: ids.length }))) items.set(it.id, markRaw(it))
  await loadRows(ids)
  loading.value = null
  const list = r.groups.map((g, n) => prepare(g, items, `g${n}`)).filter((g) => g.members.every((m) => m.item))
  for (const g of list) g.keep = keeper(g, s.keep, keepFolder.value?.id)
  groups.value = list.map(markRaw)
  result.value = r
  reorder()
  active.value = shown.value[0]?.key ?? null
  focus.value = null
  bump()
}

// ------------------------------------------------------------ the groups
function reorder() {
  const by = SORTS.find((x) => x.value === sort.value).order
  order.value = groups.value.slice().sort(by).map((g) => g.key)
}
const shown = computed(() => {
  tick.value
  const byKey = new Map(groups.value.map((g) => [g.key, g]))
  return order.value.map((k) => byKey.get(k)).filter((g) => g && (show.value === 'all' || (show.value === 'identical') === g.identical))
})
const current = computed(() => {
  tick.value
  return shown.value.find((g) => g.key === active.value) || shown.value[0] || null
})
/** After groups went: the one now where the active one was. */
function keepPlace(at) {
  const list = shown.value
  active.value = list[Math.min(at, list.length - 1)]?.key ?? null
}
const totals = computed(() => {
  tick.value
  let count = 0
  let bytes = 0
  let stay = 0
  for (const g of groups.value) {
    if (!g.include) continue
    const f = freed(g)
    count += f.count
    bytes += f.bytes
    stay += f.stay
  }
  return { count, bytes, stay }
})

function setKeep(g, i) {
  g.keep = i
  g.remove.delete(i)
  g.include = true
  g.touched = true
  if (focus.value === i) focus.value = null
  bump()
}
function toggleRemove(g, i) {
  if (g.remove.has(i)) g.remove.delete(i)
  else g.remove.add(i)
  if (!removed(g).length) g.include = false
  bump()
}
function toggleInclude(g, on = !g.include) {
  g.include = on && removed(g).length > 0
  bump()
}
function applyRule(rule, folderId = keepFolder.value?.id) {
  for (const g of groups.value) {
    g.keep = keeper(g, rule, folderId)
    g.remove.delete(g.keep)
  }
  focus.value = null
  bump()
}
async function setRule(v) {
  if (v === 'folder') {
    const f = await chooseFolder(t('tools.duplicates.keep.pick'))
    if (!f) return
    keepFolder.value = f
  }
  s.keep = v
  applyRule(v)
}
const allShownIncluded = computed(() => {
  tick.value
  const list = shown.value
  const on = list.filter((g) => g.include).length
  return { all: list.length > 0 && on === list.length, some: on > 0 && on < list.length }
})
function includeShown(on) {
  for (const g of shown.value) g.include = on && removed(g).length > 0
  bump()
}
/** Drops groups, or members of them, from what is shown; a group left with one member goes. */
function drop(g, members = null) {
  if (members) {
    const keepIds = new Set(g.members.filter((_, i) => !members.includes(i)).map((m) => m.id))
    if (keepIds.size > 1) {
      const left = prepare({ members: g.members.filter((m) => keepIds.has(m.id)).map(({ item, ...m }) => m), links: [] }, new Map(g.members.map((m) => [m.id, m.item])), g.key)
      // the links between the members left stay as they were
      const old = g.members.map((m) => m.id)
      left.members.forEach((a, i) => left.members.forEach((b, j) => {
        if (i < j) {
          const r = relation(g, old.indexOf(a.id), old.indexOf(b.id))
          if (r && r.way !== 'content') left.links.set(`${i}-${j}`, r)
        }
      }))
      left.keep = keeper(left, s.keep, keepFolder.value?.id)
      left.include = g.include
      groups.value = groups.value.map((x) => (x === g ? markRaw(left) : x))
      focus.value = null
      return bump()
    }
  }
  const at = shown.value.indexOf(g)
  groups.value = groups.value.filter((x) => x !== g)
  keepPlace(at)
  focus.value = null
  bump()
}
// skipping takes a group, or one member of it, out of these results only; nothing is changed in the library
const skip = (g, i = null) => drop(g, i == null ? null : [i])

// ------------------------------------------------------------ moving to the trash
async function apply() {
  const list = groups.value.filter((g) => g.include && removed(g).length)
  if (!list.length || busy.value) return
  const n = totals.value.count
  const ok = await askHere({ kind: 'confirm', title: tc('tools.duplicates.confirm.title', n), body: s.combine ? t('tools.duplicates.confirm.combine') : t('tools.duplicates.confirm.plain'), okLabel: t('tools.duplicates.confirm.ok') }).catch(() => false)
  if (!ok) return
  busy.value = { done: 0, total: list.length }
  let removedCount = 0
  const skipped = []
  const settled = new Set()
  try {
    await asJob(tc('tools.duplicates.moving', n), list, CHUNK, async (part) => {
      const r = await call('duplicates.resolve', { groups: part.map(settleArgs), combine: s.combine })
      removedCount += r.removed
      skipped.push(...r.skipped)
      part.forEach((g) => settled.add(g))
    }, (k) => (busy.value = { done: k, total: list.length }))
  } catch (e) {
    toast(errorText(e), 'error')
  } finally {
    busy.value = null
  }
  const left = new Set(skipped.map((x) => x.id))
  const at = Math.max(0, shown.value.indexOf(current.value))
  groups.value = groups.value.filter((g) => !settled.has(g) || g.members.some((m, i) => left.has(m.id) && i !== g.keep && !g.remove.has(i)))
  keepPlace(at)
  done.value = { removed: removedCount, skipped }
  toast(tc('tools.duplicates.moved', removedCount))
  bump()
}

// ------------------------------------------------------------ keys
const close = () => getCurrentWindow().close().catch(() => {})
const typing = (e) => e.target.closest?.('input, textarea, [contenteditable], .p-select, .p-slider')
function onKey(e) {
  if (S.apiAsk || S.picker) return
  if (e.key === 'Escape' && !document.querySelector('.p-select-overlay')) return close()
  if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
    e.preventDefault()
    return groups.value.length ? apply() : find()
  }
  if (typing(e) || !current.value) return
  const g = current.value
  if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    e.preventDefault()
    const list = shown.value
    const at = Math.max(0, Math.min(list.length - 1, list.indexOf(g) + (e.key === 'ArrowDown' ? 1 : -1)))
    active.value = list[at]?.key ?? null
    focus.value = null
    scrollToActive()
  } else if (e.key === ' ' && !e.target.closest?.('button, [role="button"]')) {
    e.preventDefault()
    toggleInclude(g)
  } else if (/^[1-9]$/.test(e.key) && Number(e.key) <= g.members.length) {
    setKeep(g, Number(e.key) - 1)
  }
}
onMounted(() => {
  addEventListener('keydown', onKey)
  addEventListener('focus', onFocus)
})
onBeforeUnmount(() => {
  removeEventListener('keydown', onKey)
  removeEventListener('focus', onFocus)
  clearInterval(polling)
})
const list = ref(null)
// the other view shows the same group where this one did
function toggleDetails() {
  s.details = !s.details
  nextTick(() => setTimeout(scrollToActive, 30))
}
function scrollToActive() {
  const at = shown.value.indexOf(current.value)
  nextTick(() => list.value?.$el?.querySelector('.p-virtualscroller')?.scrollTo?.({ top: Math.max(0, (at - 1) * rowPx(s.details)) }))
}

// ------------------------------------------------------------ the side column
const flip = (w) => (open.value = open.value === w ? null : w)
function headKey(e, w) {
  if ((e.key === 'Enter' || e.key === ' ') && e.target === e.currentTarget) {
    e.preventDefault()
    flip(w)
  }
}
function setThreshold(w, v) {
  s.ways[w].t = v / 100
  s.ways[w].on = true
}

// ------------------------------------------------------------ the body
const problems = computed(() => {
  const r = result.value
  if (!r) return []
  const lines = []
  if (r.stopped) lines.push(t('tools.duplicates.problems.stopped'))
  for (const k of ['missing', 'noThumbnail', 'noFfmpeg', 'plain']) if (r[k]) lines.push(tc(`tools.duplicates.problems.${k}`, r[k]))
  if (r.unreadableCount) {
    const named = r.unreadable.slice(0, 3).map((u) => `${u.name} (${u.error})`).join(', ')
    lines.push(tc('tools.duplicates.problems.unreadable', r.unreadableCount, { list: r.unreadableCount > 3 ? t('tools.duplicates.problems.unreadableMore', { list: named }) : named }))
  }
  return lines
})
const outcome = computed(() => {
  const d = done.value
  if (!d) return null
  const reasons = {}
  for (const x of d.skipped) reasons[x.reason] = (reasons[x.reason] || 0) + 1
  const left = Object.entries(reasons).map(([why, n]) => tc('tools.duplicates.outcome.reason', n, { why: why.charAt(0).toLowerCase() + why.slice(1) }))
  return { moved: tc('tools.duplicates.outcome.moved', d.removed), left: left.length ? t('tools.duplicates.outcome.left', { list: left.join('; ') }) : '' }
})
const showOptions = computed(() => options(['all', 'identical', 'similar'], (v) => t(`tools.duplicates.show.${v}`)))
const sortOptions = computed(() => options(SORTS.map((x) => x.value), (v) => t(`tools.duplicates.sorts.${v}`)))
const keepOptions = computed(() => KEEP_RULES.map((r) => ({ value: r.value, label: r.value === 'folder' && keepFolder.value ? t('tools.duplicates.keep.inFolder', { folder: keepFolder.value.name }) : t(`tools.duplicates.keep.rules.${r.value}`) })))
const modeOptions = computed(() => options(['side', 'slider', 'difference'], (v) => t(`tools.duplicates.modes.${v}`)))
const againstOptions = computed(() => [{ value: 'each', label: t('tools.duplicates.against.each'), disabled: single.value }, { value: 'library', label: t('tools.duplicates.against.library') }])
function setShow(v) {
  show.value = v
  active.value = shown.value[0]?.key ?? null
}
function setSort(v) {
  sort.value = v
  reorder()
  active.value = shown.value[0]?.key ?? null
}
function groupLabel(g) {
  const f = freed(g)
  const what = g.identical ? t('tools.duplicates.group.identical') : t('tools.duplicates.group.alike', { percent: percent(likeness(g)) })
  return [what, tc('tools.duplicates.group.copies', g.members.length), f.bytes ? t('tools.duplicates.group.frees', { size: sizeText(f.bytes) }) : ''].filter(Boolean).join(' · ')
}
// a click on a picture keeps it; the row stays the active one
function keepTile(g, i) {
  if (i !== g.keep) setKeep(g, i)
  active.value = g.key
}
function tilesOf(g) {
  return g.members.map((m, i) => {
    const r = relation(g, i, g.keep)
    return { i, id: m.id, name: m.item?.fileName || m.item?.name || '', kept: i === g.keep, going: g.include && i !== g.keep && !g.remove.has(i), match: i === g.keep ? t('tools.duplicates.keep.label') : !r ? t('tools.duplicates.match.alike') : r.way === 'content' ? t('tools.duplicates.match.identical') : percent(r.score), facts: [sizeText(m.item?.size || 0), shapeText(m.item)].filter(Boolean).join(' · ') }
  })
}
// a row redraws only when its object changes, so each drawing gives every group a new one with what it shows
const groupRows = computed(() => {
  tick.value
  const simple = !s.details
  return shown.value.map((g) => ({ key: g.key, g, include: g.include, keep: g.keep, removable: removed(g).length > 0, name: g.members[g.keep].item?.name || '', text: groupLabel(g), tiles: simple ? tilesOf(g) : null }))
})
const selectedRow = computed(() => groupRows.value.find((r) => r.key === current.value?.key) || null)
function pickRow(r) {
  if (!r) return
  active.value = r.key
  focus.value = null
}

/** The member compared with the kept one: the one clicked, else the least like it. */
function other(g) {
  const f = focus.value
  if (f != null && f !== g.keep && f < g.members.length) return f
  let best = -1
  let low = 2
  g.members.forEach((_, i) => {
    if (i === g.keep) return
    const score = relation(g, i, g.keep)?.score ?? 0
    if (score < low) [best, low] = [i, score]
  })
  return best
}
const PREVIEW = 2048
const previewOf = (uid) => {
  const r = gridRows.get(uid)
  return r ? previewUrl(r.id, PREVIEW, r.thumb_ver) : ''
}
const detail = computed(() => {
  tick.value
  const g = current.value
  if (!g) return null
  const o = other(g)
  const k = g.members[g.keep]
  const b = g.members[o]
  const differs = (i, f) => i !== g.keep && f(g.members[i].item) !== f(k.item)
  const members = g.members.map((m, i) => ({
    i,
    m,
    g,
    key: m.id,
    kept: i === g.keep,
    removing: !g.remove.has(i),
    match: m.file != null && g.members.some((x, j) => j !== i && x.file === m.file) ? t('tools.duplicates.match.sameFile', { match: matchText(g, i) }) : matchText(g, i),
    differs: { size: differs(i, (it) => it?.size), shape: differs(i, shapeText), added: differs(i, (it) => dateText(it?.imported)) },
  }))
  return { g, o, images: [previewOf(k.id), previewOf(b.id)], labels: [t('tools.duplicates.keep.keptNamed', { name: k.item?.name || '' }), b.item?.name || ''], compareKey: `${g.key}-${g.keep}-${o}`, members }
})
async function selectInCurio(g) {
  const r = await call('selection.set', { ids: g.members.map((m) => m.id) }).catch(() => null)
  if (r?.notShown) toast(tc('tools.duplicates.detail.notShown', r.notShown))
}
function focusMember(e) {
  if (e.data.kept) return
  focus.value = e.data.i
  bump()
}

const summary = computed(() => {
  const n = totals.value
  if (!groups.value.length) return ''
  if (!n.count) return t('tools.duplicates.summary.none')
  const parts = [tc('tools.duplicates.summary.toTrash', n.count)]
  if (n.bytes) parts.push(t('tools.duplicates.summary.freed', { size: sizeText(n.bytes) }))
  if (n.stay) parts.push(tc('tools.duplicates.summary.stay', n.stay))
  return parts.join(', ')
})
const fraction = (p) => (p.total ? Math.round((p.done / p.total) * 100) : 0)
</script>

<template>
  <ToolPage class="duplicates-tool">
    <template #side>
      <div class="scope">
        <div class="opt">
          <span class="side-label">{{ $t('tools.duplicates.scope.label') }}</span>
          <Select :model-value="s.scope" :options="scopeOptions" option-label="label" option-value="value" option-disabled="disabled" :aria-label="$t('tools.duplicates.scope.label')" @update:model-value="setScope" />
        </div>
        <Button v-if="s.scope === 'folder'" :label="$t('tools.duplicates.scope.other')" text size="small" severity="secondary" class="add-pair" @click="pickFolder" />
        <div class="opt">
          <span class="side-label">{{ $t('tools.duplicates.against.label') }}</span>
          <SelectButton :model-value="against" :options="againstOptions" option-label="label" option-value="value" option-disabled="disabled" :allow-empty="false" :disabled="s.scope === 'library'" @update:model-value="(v) => (s.against = v)" />
        </div>
      </div>
      <div class="methods">
        <div v-for="w in WAYS" :key="w" class="method" :class="{ open: w !== 'content' && open === w, on: s.ways[w].on, plain: w === 'content' }" :data-way="w">
          <div v-if="w === 'content'" v-tooltip.right="hint(w)" class="method-head" @click="s.ways[w].on = !s.ways[w].on">
            <span class="method-name">{{ title(w) }}</span>
            <span class="method-sum faint" />
            <span @click.stop><ToggleSwitch v-model="s.ways[w].on" :aria-label="$t('tools.duplicates.use', { way: title(w) })" /></span>
          </div>
          <div v-else class="method-head" role="button" tabindex="0" :aria-expanded="open === w" @click="flip(w)" @keydown="headKey($event, w)">
            <span class="method-name">{{ title(w) }}</span>
            <span class="method-sum faint">{{ open === w ? '' : percent(s.ways[w].t) }}</span>
            <span @click.stop><ToggleSwitch v-model="s.ways[w].on" :aria-label="$t('tools.duplicates.use', { way: title(w) })" /></span>
          </div>
          <div v-if="w !== 'content' && open === w" class="method-body">
            <p class="hint">{{ hint(w) }}</p>
            <div class="opt">
              <span class="side-label">{{ $t('tools.duplicates.similarityAt', { percent: percent(s.ways[w].t) }) }}</span>
              <Slider :model-value="Math.round(s.ways[w].t * 100)" :min="Math.round(RANGE[w][0] * 100)" :max="100" :aria-label="$t('tools.duplicates.similarity')" @update:model-value="(v) => setThreshold(w, v)" />
            </div>
            <label v-if="w === 'looks'" class="tool-check"><Checkbox v-model="s.ways.looks.others" binary /><span>{{ $t('tools.duplicates.covers') }}</span></label>
          </div>
        </div>
      </div>
      <div class="side-more">
        <div class="opt">
          <span class="side-label">{{ $t('tools.duplicates.minSize') }}</span>
          <InputGroup>
            <InputNumber :model-value="s.minKb" :min="0" :use-grouping="false" :aria-label="$t('tools.duplicates.minSize')" @update:model-value="(v) => (s.minKb = v ?? 0)" />
            <InputGroupAddon>{{ $t('filters.units.KB') }}</InputGroupAddon>
          </InputGroup>
        </div>
      </div>
      <div class="side-foot">
        <Button :label="job ? $t('tools.duplicates.scanning') : $t('tools.duplicates.find')" :loading="!!job" :disabled="!canFind || !!job" class="find" @click="find" />
      </div>
    </template>

    <div v-if="progress" class="working">
      <span class="working-text">{{ progress.text || $t('tools.duplicates.searching') }}</span>
      <ProgressBar v-if="progress.total" :value="fraction(progress)" :show-value="false" />
      <ProgressBar v-else mode="indeterminate" />
      <Button :label="$t('tools.duplicates.stop')" size="small" severity="secondary" @click="stop" />
    </div>
    <div v-else-if="loading" class="working">
      <span class="working-text">{{ tc('tools.duplicates.loading', loading.total) }}</span>
      <ProgressBar :value="fraction(loading)" :show-value="false" />
    </div>
    <template v-else>
      <div v-if="busy" class="working">
        <span class="working-text">{{ tc('tools.duplicates.movingGroups', busy.total, { done: busy.done.toLocaleString() }) }}</span>
        <ProgressBar :value="fraction(busy)" :show-value="false" />
      </div>
      <Message v-if="outcome" :severity="outcome.left ? 'warn' : 'success'" :closable="false" class="note"><div>{{ outcome.moved }}</div><div v-if="outcome.left">{{ outcome.left }}</div></Message>
      <Message v-if="problems.length" severity="warn" :closable="false" class="note"><div v-for="(l, i) in problems" :key="i">{{ l }}</div></Message>
      <div v-if="!groups.length" class="empty">
        <AppIcon name="copy-check" :size="40" class="faint" />
        <p>{{ result ? $t('tools.duplicates.empty.none') : $t('tools.duplicates.empty.start') }}</p>
        <p v-if="result" class="faint">{{ $t('tools.duplicates.empty.more') }}</p>
      </div>
      <template v-else>
        <div class="results-bar">
          <Checkbox :model-value="allShownIncluded.all" :indeterminate="allShownIncluded.some" binary :aria-label="$t('tools.duplicates.results.includeAll')" @update:model-value="includeShown" />
          <span class="grow faint">{{ tc('tools.duplicates.results.groups', groups.length) }}</span>
          <SelectButton :model-value="show" :options="showOptions" option-label="label" option-value="value" :allow-empty="false" size="small" @update:model-value="setShow" />
          <Select :model-value="sort" :options="sortOptions" option-label="label" option-value="value" :aria-label="$t('tools.duplicates.sorts.label')" class="sort" @update:model-value="setSort" />
          <span class="side-label">{{ $t('tools.duplicates.keep.label') }}</span>
          <Select :model-value="s.keep" :options="keepOptions" option-label="label" option-value="value" :aria-label="$t('tools.duplicates.keep.rule')" class="keep" @update:model-value="setRule" />
          <button v-tooltip.top="$t('tools.duplicates.results.details')" class="icon-btn" :class="{ on: s.details }" :aria-label="$t('tools.duplicates.results.details')" :aria-pressed="String(s.details)" @click="toggleDetails"><AppIcon name="layout-sidebar-right" :size="16" /></button>
        </div>
        <div class="split">
          <div class="groups" :class="{ wide: !s.details }">
            <DataTable :key="s.details ? 'list' : 'tiles'" ref="list" :value="groupRows" data-key="key" selection-mode="single" :selection="selectedRow" :meta-key-selection="false" size="small" class="tool-table group-table" :class="{ simple: !s.details }" striped-rows scrollable scroll-height="flex" :virtual-scroller-options="{ itemSize: rowPx(s.details) }" @update:selection="pickRow">
              <Column header="">
                <template #body="{ data: r }">
                  <div v-if="r.tiles" class="tile-row" :class="{ off: !r.include }">
                    <div class="tile-head">
                      <span @click.stop><Checkbox :model-value="r.include" binary :disabled="!r.removable" :aria-label="$t('tools.duplicates.results.include')" @update:model-value="(v) => toggleInclude(r.g, v)" /></span>
                      <span class="group-sum faint ellipsis">{{ r.text }}</span>
                      <button v-tooltip.top="$t('tools.duplicates.results.skip')" class="icon-btn leave" :aria-label="$t('tools.duplicates.results.skip')" @click.stop="skip(r.g)"><AppIcon name="x" :size="16" /></button>
                    </div>
                    <div class="tiles">
                      <div v-for="tile in r.tiles" :key="tile.id" class="tile" :class="{ kept: tile.kept, going: tile.going }" role="button" tabindex="0" :title="tile.kept ? $t('tools.duplicates.keep.keptNamed', { name: tile.name }) : $t('tools.duplicates.keep.named', { name: tile.name })" :aria-pressed="String(tile.kept)" @click.stop="keepTile(r.g, tile.i)" @keydown.enter.space.prevent.stop="keepTile(r.g, tile.i)">
                        <ToolThumbnail :item="tile.id" :size="120" />
                        <span class="tile-match" :class="{ kept: tile.kept }">{{ tile.match }}</span>
                        <span class="tile-facts faint">{{ tile.facts }}</span>
                      </div>
                    </div>
                  </div>
                  <div v-else class="group-row" :class="{ off: !r.include }">
                    <span @click.stop><Checkbox :model-value="r.include" binary :disabled="!r.removable" :aria-label="$t('tools.duplicates.results.include')" @update:model-value="(v) => toggleInclude(r.g, v)" /></span>
                    <div class="strip">
                      <div v-for="(m, i) in r.g.members.slice(0, 4)" :key="m.id" v-tooltip.top="i === r.keep ? $t('tools.duplicates.keep.kept') : $t('tools.duplicates.keep.this')" class="strip-thumb" :class="{ kept: i === r.keep }" @click.stop="keepTile(r.g, i)"><ToolThumbnail :item="m.id" :size="44" /></div>
                      <span v-if="r.g.members.length > 4" class="more faint">+{{ r.g.members.length - 4 }}</span>
                    </div>
                    <div class="group-text"><span class="group-name ellipsis">{{ r.name }}</span><span class="group-sum faint ellipsis">{{ r.text }}</span></div>
                    <button v-tooltip.top="$t('tools.duplicates.results.skip')" class="icon-btn leave" :aria-label="$t('tools.duplicates.results.skip')" @click.stop="skip(r.g)"><AppIcon name="x" :size="16" /></button>
                  </div>
                </template>
              </Column>
            </DataTable>
          </div>
          <div v-if="s.details && !detail" class="empty faint">{{ $t('tools.duplicates.detail.empty') }}</div>
          <div v-else-if="s.details" class="detail">
            <div class="detail-head">
              <SelectButton v-model="mode" :options="modeOptions" option-label="label" option-value="value" :allow-empty="false" size="small" :aria-label="$t('tools.duplicates.detail.compare')" />
              <span class="grow" />
              <Button :label="$t('tools.duplicates.detail.select')" text size="small" severity="secondary" @click="selectInCurio(detail.g)" />
              <button v-tooltip.top="$t('tools.duplicates.results.skip')" class="icon-btn" :aria-label="$t('tools.duplicates.results.skip')" @click="skip(detail.g)"><AppIcon name="x" :size="16" /></button>
            </div>
            <div class="compare"><AppImageCompare :key="detail.compareKey" :images="detail.images" :mode="mode" toolbar :labels="detail.labels" /></div>
            <DataTable :value="detail.members" data-key="key" size="small" striped-rows class="tool-table members" :table-style="{ tableLayout: 'fixed', width: '100%' }" :row-class="(r) => ({ focused: r.i === detail.o, kept: r.kept })" @row-click="focusMember">
              <Column :header="$t('tools.duplicates.columns.keep')" :style="{ width: '3.36rem' }" :header-style="{ width: '3.36rem' }">
                <template #body="{ data }"><span @click.stop><RadioButton :model-value="data.kept" :value="true" :name="`keep-${data.g.key}`" :aria-label="$t('tools.duplicates.keep.named', { name: data.m.item?.name || '' })" @update:model-value="setKeep(data.g, data.i)" /></span></template>
              </Column>
              <Column header="" :style="{ width: '3.64rem' }" :header-style="{ width: '3.64rem' }">
                <template #body="{ data }"><div v-tooltip.top="data.kept ? $t('tools.duplicates.keep.kept') : $t('tools.duplicates.keep.this')" class="thumb-cell" @click.stop="keepTile(data.g, data.i)"><ToolThumbnail :item="data.m.id" :size="40" /></div></template>
              </Column>
              <Column :header="$t('tools.duplicates.columns.name')">
                <template #body="{ data }"><div class="name-cell"><span class="name">{{ data.m.item?.fileName || data.m.item?.name || '' }}</span><span class="match" :class="{ kept: data.kept }">{{ data.match }}</span><span v-if="placeText(data.m)" class="faint place">{{ placeText(data.m) }}</span></div></template>
              </Column>
              <Column :header="$t('tools.duplicates.columns.size')" :style="{ width: '7.55rem' }" :header-style="{ width: '7.55rem' }">
                <template #body="{ data }"><div class="name-cell"><span :class="{ differs: data.differs.size }">{{ sizeText(data.m.item?.size || 0) }}</span><span :class="{ differs: data.differs.shape }">{{ shapeText(data.m.item) }}</span></div></template>
              </Column>
              <Column :header="$t('tools.duplicates.columns.added')" :style="{ width: '6.43rem' }" :header-style="{ width: '6.43rem' }">
                <template #body="{ data }"><span :class="{ differs: data.differs.added }">{{ dateText(data.m.item?.imported) }}</span></template>
              </Column>
              <Column :header="$t('tools.duplicates.columns.remove')" :style="{ width: '6.71rem' }" :header-style="{ width: '6.71rem' }">
                <template #body="{ data }">
                  <span v-if="!data.kept" class="remove-cell" @click.stop>
                    <Checkbox :model-value="data.removing" binary :aria-label="$t('tools.duplicates.columns.removeNamed', { name: data.m.item?.name || '' })" @update:model-value="toggleRemove(data.g, data.i)" />
                    <button v-tooltip.top="$t('tools.duplicates.results.skip')" class="icon-btn leave" :aria-label="$t('tools.duplicates.results.skip')" @click="skip(data.g, data.i)"><AppIcon name="x" :size="16" /></button>
                  </span>
                </template>
              </Column>
            </DataTable>
          </div>
        </div>
      </template>
    </template>

    <template #foot>
      <label v-tooltip.top="$t('tools.duplicates.foot.combineHint')" class="tool-check combine"><Checkbox v-model="s.combine" binary /><span>{{ $t('tools.duplicates.foot.combine') }}</span></label>
      <span class="grow faint">{{ summary }}</span>
      <Button :label="done ? $t('tools.duplicates.foot.close') : $t('tools.duplicates.foot.cancel')" severity="secondary" @click="close" />
      <Button :label="totals.count ? tc('tools.duplicates.foot.apply', totals.count) : $t('tools.duplicates.foot.applyNone')" :disabled="!totals.count || !!busy || !!job" :loading="!!busy" @click="apply" />
    </template>
  </ToolPage>
</template>

<style>
.duplicates-tool .tool-page-side { width: round(20.98rem, 1px); }
.duplicates-tool .method-sum { text-align: right; }
.duplicates-tool .scope { display: flex; flex-direction: column; gap: calc(0.7rem * var(--g)); padding: calc(0.84rem * var(--g)) calc(0.98rem * var(--g)) calc(0.98rem * var(--g)); border-bottom: 1px solid var(--border); }
.duplicates-tool .scope .p-select { width: 100%; }
.duplicates-tool .scope .p-selectbutton { display: flex; }
.duplicates-tool .scope .p-selectbutton .p-togglebutton { flex: 1; }
.duplicates-tool .method-body { display: flex; flex-direction: column; gap: calc(0.7rem * var(--g)); padding: calc(0.28rem * var(--g)) calc(0.98rem * var(--g)) calc(0.98rem * var(--g)); }
.duplicates-tool .method-body .p-slider { margin: calc(0.42rem * var(--g)) calc(0.42rem * var(--g)) 0; }
.duplicates-tool .side-more { padding: calc(0.84rem * var(--g)) calc(0.98rem * var(--g)); }
.duplicates-tool .side-more .p-inputnumber-input { width: 100%; min-width: 0; }
.duplicates-tool .side-foot { display: flex; align-items: center; gap: 0.56rem; padding: calc(0.7rem * var(--g)) calc(0.56rem * var(--g)) calc(0.98rem * var(--g)) calc(0.98rem * var(--g)); margin-top: auto; }
.duplicates-tool .side-foot .find { flex: 1; }
.duplicates-tool .note { margin: 0.7rem 0.98rem 0; }
.duplicates-tool .empty { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 0.42rem; text-align: center; padding: 1.68rem; }
.duplicates-tool .empty p { margin: 0; }
.duplicates-tool .results-bar { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); padding: calc(0.56rem * var(--g)) calc(0.98rem * var(--g)); border-bottom: 1px solid var(--border); flex: none; }
.duplicates-tool .results-bar .sort { width: round(11.89rem, 1px); }
.duplicates-tool .results-bar .keep { width: round(13.99rem, 1px); }
.duplicates-tool .split { flex: 1; min-height: 0; display: flex; }
.duplicates-tool .groups { width: round(23.78rem, 1px); flex: none; border-right: 1px solid var(--border); min-height: 0; display: flex; flex-direction: column; }
.duplicates-tool .groups .p-datatable { flex: 1; min-height: 0; }
.duplicates-tool .groups.wide { flex: 1; width: auto; border-right: 0; }
.duplicates-tool .results-bar .on { color: var(--accent); }
.duplicates-tool .strip-thumb, .duplicates-tool .thumb-cell { cursor: pointer; }
.duplicates-tool .tile-row { height: round(14.97rem, 1px); box-sizing: border-box; display: flex; flex-direction: column; gap: 0.42rem; padding: calc(0.56rem * var(--g)) calc(0.98rem * var(--g)); }
.duplicates-tool .tile-head { display: flex; align-items: center; gap: 0.7rem; min-width: 0; }
.duplicates-tool .tile-row.off .tiles { opacity: 0.75; }
.duplicates-tool .tiles { display: flex; gap: calc(0.7rem * var(--g)); overflow-x: auto; overflow-y: hidden; flex: 1; min-height: 0; }
.duplicates-tool .tile { box-sizing: border-box; flex: none; width: round(8.39rem, 1px); display: flex; flex-direction: column; gap: 0.14rem; cursor: pointer; }
.duplicates-tool .tile:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; border-radius: calc(4px * var(--r)); }
.duplicates-tool .tile .tool-thumb { width: round(8.39rem, 1px) !important; height: round(8.39rem, 1px) !important; border-radius: calc(6px * var(--r)); }
.duplicates-tool .tile.kept .tool-thumb { outline: 2px solid var(--accent); outline-offset: -2px; }
.duplicates-tool .tile.going .tool-thumb { opacity: 0.55; }
.duplicates-tool .tile:hover .tool-thumb { opacity: 1; }
.duplicates-tool .tile-match { font-size: 0.85rem; }
.duplicates-tool .tile-match.kept { color: var(--accent); font-weight: 500; }
.duplicates-tool .tile-facts { font-size: 0.85rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.duplicates-tool .group-table .p-datatable-thead { display: none; }
.duplicates-tool .group-table td { padding: 0 !important; }
.duplicates-tool .group-row { display: flex; align-items: center; gap: 0.7rem; height: round(4.48rem, 1px); padding: 0 calc(0.84rem * var(--g)); box-sizing: border-box; }
.duplicates-tool .group-row.off .group-name { color: var(--text-muted); }
.duplicates-tool .strip { display: flex; align-items: center; gap: 0.21rem; flex: none; }
.duplicates-tool .strip-thumb { width: round(3.08rem, 1px); height: round(3.08rem, 1px); }
.duplicates-tool .strip-thumb .tool-thumb { width: round(3.08rem, 1px) !important; height: round(3.08rem, 1px) !important; border-radius: calc(4px * var(--r)); }
.duplicates-tool .strip-thumb.kept .tool-thumb { outline: 2px solid var(--accent); outline-offset: -2px; }
.duplicates-tool .more { font-size: 0.85rem; padding: 0 0.14rem; }
.duplicates-tool .group-text { display: flex; flex-direction: column; min-width: 0; flex: 1; gap: 0.14rem; }
.duplicates-tool .group-name { font-weight: 500; }
.duplicates-tool .group-sum { font-size: 0.85rem; }
.duplicates-tool .detail { flex: 1; min-width: 0; display: flex; flex-direction: column; min-height: 0; }
.duplicates-tool .detail-head { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); padding: calc(0.56rem * var(--g)) calc(0.7rem * var(--g)) calc(0.56rem * var(--g)) calc(0.98rem * var(--g)); border-bottom: 1px solid var(--border); }
.duplicates-tool .detail-head .grow { flex: 1; }
.duplicates-tool .compare { flex: 1; min-height: round(11.19rem, 1px); display: flex; flex-direction: column; }
.duplicates-tool .compare .curio-compare { flex: 1; min-height: 0; }
.duplicates-tool .members { flex: none; max-height: 40%; overflow: auto; border-top: 1px solid var(--border); }
.duplicates-tool .members td { vertical-align: middle; }
.duplicates-tool .members tr.focused > td { background: var(--hover); }
.duplicates-tool .members .match.kept { color: var(--accent); }
.duplicates-tool .match { font-size: 0.92rem; }
.duplicates-tool .members .differs { color: var(--warning); }
.duplicates-tool .thumb-cell { width: round(2.8rem, 1px); height: round(2.24rem, 1px); }
.duplicates-tool .thumb-cell .tool-thumb { width: round(2.8rem, 1px) !important; height: round(2.24rem, 1px) !important; border-radius: calc(4px * var(--r)); }
.duplicates-tool .name-cell { display: flex; flex-direction: column; min-width: 0; }
.duplicates-tool .name { overflow-wrap: anywhere; line-height: 1.3; }
.duplicates-tool .place { font-size: 0.85rem; overflow-wrap: anywhere; }
.duplicates-tool .remove-cell { display: flex; align-items: center; gap: 0.42rem; }
.duplicates-tool :is(.members, .group-table) tr:is(:hover, .p-datatable-row-selected) .leave, .duplicates-tool .leave:focus-visible { visibility: visible; }
.duplicates-tool .tile-head .group-sum { flex: 1; }
.duplicates-tool .group-row .leave { margin-left: auto; }
/* the zoom slider's track, its handle reaching 8px past it, starts where the heading's controls do */
.duplicates-tool .detail .curio-compare-tools { padding-left: calc(0.98rem * var(--g) - 0.56rem); padding-right: calc(0.7rem * var(--g)); }
.duplicates-tool .members.p-datatable :is(.p-datatable-thead > tr > th, .p-datatable-tbody > tr > td):first-child { padding-left: calc(0.98rem * var(--g)); }
.duplicates-tool .members.p-datatable :is(.p-datatable-thead > tr > th, .p-datatable-tbody > tr > td):last-child { padding-right: calc(0.7rem * var(--g)); }
.duplicates-tool .combine { white-space: nowrap; }
</style>

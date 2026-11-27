<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, toRaw, watch } from 'vue'
import Button from 'openvue/button'
import Checkbox from 'openvue/checkbox'
import Column from 'openvue/column'
import DataTable from 'openvue/datatable'
import InputGroup from 'openvue/inputgroup'
import InputGroupAddon from 'openvue/inputgroupaddon'
import InputNumber from 'openvue/inputnumber'
import InputText from 'openvue/inputtext'
import Message from 'openvue/message'
import ProgressBar from 'openvue/progressbar'
import Select from 'openvue/select'
import SelectButton from 'openvue/selectbutton'
import ToggleSwitch from 'openvue/toggleswitch'
import { call, errorText } from '../platform/ipc.js'
import { getCurrentWindow } from '../platform/windowFrame.js'
import { S } from '../app/state.js'
import { saveUi } from '../app/preferences.js'
import { toast } from '../app/notify.js'
import { choose } from '../plugins/pageOverlay.js'
import { t } from '../shared/i18n.js'
import { rem } from '../shared/rem.js'
import AppIcon from '../shared/AppIcon.vue'
import ToolPage from './ToolPage.vue'
import ToolThumbnail from './ToolThumbnail.vue'
import { asJob, itemsOf, loadRows } from './tools.js'
import { METHODS, TAGS, complete, plan } from './renameEngine.js'

// Batch rename: methods in the side column, run top to bottom, every item's new name in the table as it is typed, clashes marked before anything changes.
// It changes the names shown in the library; files on disk keep theirs.
const title = (m) => t(`tools.rename.methods.${m}`)
const removeName = (type) => t(`tools.rename.remove.types.${type}`)
const caseName = (how) => t(`tools.rename.case.names.${how}`)
const REMOVE = ['text', 'position', 'chars', 'digits', 'symbols', 'accents', 'brackets']
const ADD = ['start', 'end', 'position', 'before', 'after']
const CASES = ['lower', 'upper', 'title', 'sentence', 'invert']
const opts = (list, label) => list.map((value) => ({ value, label: label(value) }))
// an empty result keeps the name, which shows as unchanged; these can't be used as they are
const STATES = { dup: () => t('tools.rename.states.dup'), long: () => t('tools.rename.states.long') }
// the examples in the fields, written as the methods read them
const NAME_EXAMPLE = '<Name> <Inc Nr:001>'
const COUNTER_EXAMPLE = '<Inc Nr:001>'
const same = (a, b) => JSON.stringify(complete(a)) === JSON.stringify(complete(b))
const CHUNK = 500
const UNDO_MS = 30000
// the table's fixed columns and a name cell's padding, in rem, which the row heights are worked out from
const THUMB_W = 3.92
// room in the new name's cell for the mark of a name that can't be used
const MARK_W = 1.54
const DROP_W = 3.08
const CELL_PAD = 1.68
const ROW_MIN = 3.15
const ROW_PAD = 1.26
const LINE = 1.3

// a count in a message: shown with its digits grouped, its form chosen by the number
const tc = (key, n, named = {}) => t(key, { ...named, count: n.toLocaleString() }, n)

/** The part of `after` that differs from `before`, for marking it. */
function diff(before, after) {
  let a = 0
  while (a < before.length && a < after.length && before[a] === after[a]) a++
  let b = 0
  while (b < before.length - a && b < after.length - a && before[before.length - 1 - b] === after[after.length - 1 - b]) b++
  return [after.slice(0, a), after.slice(a, after.length - b), after.slice(after.length - b)]
}

/** How many lines a name takes in a cell this wide, breaking between words as the browser does, and anywhere inside a word longer than a line. */
let pen = null
function lines(text, width, font) {
  pen ||= document.createElement('canvas').getContext('2d')
  pen.font = font
  if (width <= 0) return 1
  let n = 1
  let line = 0
  const space = pen.measureText(' ').width
  for (const word of text.split(' ')) {
    const w = pen.measureText(word).width
    if (w > width) {
      // a word wider than the cell breaks where it reaches the edge
      const total = line ? line + space + w : w
      n += Math.floor(total / width)
      line = total % width
      continue
    }
    if (line && line + space + w > width) {
      n++
      line = w
    } else line = line ? line + space + w : w
  }
  return n
}

const items = ref([])
const left = ref(new Set())
const loading = ref(null)
// the methods as last set, and the presets, kept with Curio's settings
const setup = reactive(complete(S.ui.rename || {}))
// one method open at a time
const open = ref(METHODS.find((m) => setup[m].on) || 'newName')
const presets = computed(() => (Array.isArray(S.ui.renamePresets) ? S.ui.renamePresets : []))
const naming = ref(null)
const progress = ref(null)
const done = ref(null)
const table = ref(null)
const width = ref(800)
const font = ref('14px sans-serif')
const fontPx = ref(14)

watch(() => S.toolWindow.seq, async () => {
  const ids = S.toolWindow.run?.items || []
  done.value = null
  left.value = new Set()
  items.value = []
  loading.value = { done: 0, total: ids.length }
  const got = await itemsOf(ids, (n) => (loading.value = { done: n, total: ids.length })).catch((e) => (toast(errorText(e), 'error'), []))
  await loadRows(got.map((i) => i.id))
  items.value = got
  loading.value = null
}, { immediate: true })

watch(setup, () => {
  done.value = null
  S.ui.rename = structuredClone(toRaw(setup))
  saveUi()
}, { deep: true })

const planned = computed(() => plan(setup))
const bad = computed(() => planned.value.bad)

// numbers count over the items being renamed, so leaving one out closes the gap
const kept = computed(() => items.value.filter((it) => !left.value.has(it.id)))
const rows = computed(() => {
  const names = planned.value.names(kept.value)
  const list = kept.value.map((it, i) => {
    const { stem, ext } = names[i]
    const after = stem ? stem + ext : ''
    return { id: it.id, it, before: it.name, after, state: !stem ? 'empty' : after === it.name ? 'same' : 'ok' }
  })
  // names that would be the same as another's, among these and the ones left as they are
  const count = new Map()
  for (const r of list) {
    const k = (r.state === 'empty' ? r.before : r.after).toLowerCase()
    count.set(k, (count.get(k) || 0) + 1)
  }
  for (const r of list) {
    if (r.state === 'ok' && count.get(r.after.toLowerCase()) > 1) r.state = 'dup'
    else if (r.state === 'ok' && r.after.length > 255) r.state = 'long'
  }
  return list
})
const changing = computed(() => rows.value.filter((r) => r.state === 'ok' || r.state === 'dup' || r.state === 'long'))
const blocked = computed(() => METHODS.some((m) => setup[m].on && bad.value[m]))

// each row as tall as its longer name needs, so the virtual scroller places rows where they are drawn
const nameW = computed(() => Math.max(80, (width.value - rem(THUMB_W) - rem(DROP_W)) / 2 - rem(CELL_PAD)))
const heights = computed(() => {
  const lh = fontPx.value * LINE
  return rows.value.map((r) => Math.round(Math.max(rem(ROW_MIN), Math.max(lines(r.before, nameW.value, font.value), lines(r.after || r.before, nameW.value - (STATES[r.state] ? rem(MARK_W) : 0), font.value)) * lh + rem(ROW_PAD))))
})
let ro = null
onMounted(() => {
  ro = new ResizeObserver(() => {
    const el = table.value?.$el
    if (!el) return
    width.value = el.clientWidth
    const cs = getComputedStyle(el)
    font.value = `${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`
    fontPx.value = parseFloat(cs.fontSize) || 14
  })
  const watchTable = () => table.value?.$el && ro.observe(table.value.$el)
  watchTable()
  watch(table, watchTable)
  addEventListener('keydown', onKey)
})
onBeforeUnmount(() => {
  ro?.disconnect()
  removeEventListener('keydown', onKey)
})

/** Renames in parts as a job: its bar here and in Curio's status area, where it can be cancelled between parts. */
async function rename(list, jobTitle) {
  progress.value = { done: 0, total: list.length }
  try {
    return await asJob(jobTitle, list, CHUNK, (part) => call('item.rename', { items: part }), (n) => (progress.value = { done: n, total: list.length }))
  } finally {
    progress.value = null
  }
}
async function refresh() {
  items.value = await itemsOf(items.value.map((i) => i.id))
}
async function apply() {
  const list = changing.value.map((r) => ({ id: r.id, name: r.after }))
  if (!list.length) return
  const before = changing.value.map((r) => ({ id: r.id, name: r.before }))
  try {
    const n = await rename(list, tc('tools.rename.job.renaming', list.length))
    done.value = { count: n, before: before.slice(0, n), at: Date.now() }
    toast(n < list.length ? tc('tools.rename.notify.stopped', n) : tc('tools.rename.notify.renamed', n))
    await refresh()
  } catch (e) {
    toast(errorText(e), 'error')
  }
}
async function undo() {
  const back = done.value.before
  try {
    await rename(back, tc('tools.rename.job.undoing', back.length))
    toast(t('tools.rename.notify.undone'))
    done.value = null
    await refresh()
  } catch (e) {
    toast(errorText(e), 'error')
  }
}
// Undo is offered for a while after renaming
const now = ref(Date.now())
const clock = setInterval(() => (now.value = Date.now()), 1000)
onBeforeUnmount(() => clearInterval(clock))
const canUndo = computed(() => !!done.value && now.value - done.value.at < UNDO_MS)
const leave = (id) => (left.value = new Set([...left.value, id]))
const close = () => getCurrentWindow().close().catch(() => {})
function onKey(e) {
  if (S.apiAsk || S.picker) return
  if (e.key === 'Escape' && !document.querySelector('.p-select-overlay')) close()
  if (e.key === 'Enter' && (e.ctrlKey || e.metaKey) && changing.value.length && !progress.value) apply()
}

// tags go in where the caret was in the last tag field
let target = null
const remember = (m, key) => (e) => (target = { m, key, el: e.target })
function insertTag(tag) {
  const at0 = target || { m: 'newName', key: 'pattern', el: null }
  const v = setup[at0.m][at0.key]
  const at = at0.el && at0.el.selectionStart != null ? [at0.el.selectionStart, at0.el.selectionEnd] : [v.length, v.length]
  setup[at0.m][at0.key] = v.slice(0, at[0]) + tag + v.slice(at[1])
  setup[at0.m].on = true
  if (at0.el)
    nextTick(() => {
      at0.el.focus()
      at0.el.setSelectionRange(at[0] + tag.length, at[0] + tag.length)
    })
}
const tagOptions = () => [...new Set(TAGS.map((x) => x[2]))].flatMap((g) => [{ label: t(`tools.rename.tagGroups.${g}`), heading: true }, ...TAGS.filter((x) => x[2] === g).map(([tag, id]) => ({ value: tag, label: tag, hint: t(`tools.rename.tags.${id}`) }))])
// the list is Curio's own, drawn under the field
async function showTags(e, m, key) {
  const group = e.currentTarget.closest('.p-inputgroup')
  if (!target || target.m !== m || target.key !== key) target = { m, key, el: group.querySelector('input') }
  const { chosen, value } = await choose(group.getBoundingClientRect(), { options: tagOptions(), search: true })
  if (chosen) insertTag(value)
}

const addPlaces = computed(() => opts(ADD, (w) => t(`tools.rename.add.places.${w}`)))
const whichOptions = computed(() => opts(['all', 'first', 'last'], (w) => t(`tools.rename.which.${w}`)))
const removeTypes = computed(() => opts(REMOVE, removeName))
const trimWhere = computed(() => opts(['both', 'start', 'end'], (w) => t(`tools.rename.trim.where.${w}`)))
const caseNames = computed(() => opts(CASES, caseName))
const extCases = computed(() => [{ value: 'keep', label: t('tools.rename.case.extKeep') }, { value: 'lower', label: 'abc' }, { value: 'upper', label: 'ABC' }])
function removePair(i) {
  const p = setup.list.pairs[i]
  if (setup.list.pairs.length > 1) setup.list.pairs.splice(i, 1)
  else Object.assign(p, { find: '', with: '' })
}
function summary(m) {
  const s = setup[m]
  if (m === 'newName') return s.pattern
  if (m === 'add') return s.text
  if (m === 'replace') return s.find ? `${s.find} → ${s.with}` : ''
  if (m === 'list') {
    const n = s.pairs.filter((p) => p.find).length
    return n ? tc('tools.rename.list.pairs', n) : ''
  }
  if (m === 'remove') return s.type === 'text' ? s.text : removeName(s.type)
  if (m === 'trim') return s.chars.replace(/ /g, '␣')
  return caseName(s.name)
}
// a row as Settings › Plugins draws one: a click opens it, the switch at its end turns the method on or off
const flip = (m) => (open.value = open.value === m ? null : m)
function headKey(e, m) {
  if ((e.key === 'Enter' || e.key === ' ') && e.target === e.currentTarget) {
    e.preventDefault()
    flip(m)
  }
}
function reset() {
  Object.assign(setup, complete())
  open.value = 'newName'
}

// presets: the methods saved under a name, chosen again in the footer
const current = computed(() => presets.value.find((p) => same(p.setup, setup)) || null)
function savePresets(list) {
  S.ui.renamePresets = list
  saveUi()
}
function usePreset(id) {
  const p = presets.value.find((x) => x.id === id)
  if (!p) return
  Object.assign(setup, complete(p.setup))
  open.value = METHODS.find((m) => setup[m].on) || 'newName'
}
function savePreset() {
  const name = (naming.value || '').trim()
  naming.value = null
  if (!name) return
  const others = presets.value.filter((p) => p.name.toLowerCase() !== name.toLowerCase())
  savePresets([...others, { id: Math.random().toString(36).slice(2, 10), name, setup: structuredClone(toRaw(setup)) }].sort((a, b) => a.name.localeCompare(b.name)))
}
function nameKey(e) {
  e.stopPropagation()
  if (e.key === 'Enter') savePreset()
  else if (e.key === 'Escape') naming.value = null
}
const presetButton = () => (naming.value === null ? (naming.value = current.value?.name || '') : savePreset())
const focusField = (el) => el?.$el?.focus?.()

const parts = (r) => diff(r.before, r.after)
const virtual = computed(() => ({ itemSize: Math.round(rem(ROW_MIN)), getItemSize: (i) => heights.value[i] ?? Math.round(rem(ROW_MIN)) }))
const fraction = (p) => (p.total ? Math.round((p.done / p.total) * 100) : 0)
const footText = computed(() => (items.value.length ? (left.value.size ? tc('tools.rename.foot.itemsLeft', items.value.length, { left: left.value.size.toLocaleString() }) : tc('tools.rename.foot.items', items.value.length)) : ''))
</script>

<template>
  <ToolPage class="rename-tool">
    <template #side>
      <div class="methods">
        <div v-for="m in METHODS" :key="m" class="method" :class="{ open: open === m, on: setup[m].on, bad: setup[m].on && bad[m] }" :data-method="m">
          <div class="method-head" role="button" tabindex="0" :aria-expanded="open === m" @click="flip(m)" @keydown="headKey($event, m)">
            <span class="method-name">{{ title(m) }}</span>
            <span class="method-sum faint">{{ open === m ? '' : summary(m) }}</span>
            <span @click.stop><ToggleSwitch v-model="setup[m].on" :aria-label="$t('tools.rename.use', { method: title(m) })" /></span>
          </div>
          <!-- typing in a method that is off turns it on -->
          <div v-if="open === m" class="method-body" @input="setup[m].on = true">
            <template v-if="m === 'newName'">
              <div class="opt">
                <span class="side-label">{{ $t('tools.rename.newName.name') }}</span>
                <InputGroup>
                  <InputText v-model="setup.newName.pattern" :spellcheck="false" :placeholder="NAME_EXAMPLE" @focus="remember('newName', 'pattern')($event)" @keyup="remember('newName', 'pattern')($event)" @click="remember('newName', 'pattern')($event)" />
                  <InputGroupAddon><button class="icon-btn" :title="$t('tools.rename.insertTag')" :aria-label="$t('tools.rename.insertTag')" @mousedown.prevent @click="showTags($event, 'newName', 'pattern')"><AppIcon name="code" :size="16" /></button></InputGroupAddon>
                </InputGroup>
                <p class="hint">{{ $t('tools.rename.newName.hint', { tag: COUNTER_EXAMPLE }) }}</p>
              </div>
            </template>
            <template v-else-if="m === 'add'">
              <div class="opt">
                <span class="side-label">{{ $t('tools.rename.add.text') }}</span>
                <InputGroup>
                  <InputText v-model="setup.add.text" :spellcheck="false" @focus="remember('add', 'text')($event)" @keyup="remember('add', 'text')($event)" @click="remember('add', 'text')($event)" />
                  <InputGroupAddon><button class="icon-btn" :title="$t('tools.rename.insertTag')" :aria-label="$t('tools.rename.insertTag')" @mousedown.prevent @click="showTags($event, 'add', 'text')"><AppIcon name="code" :size="16" /></button></InputGroupAddon>
                </InputGroup>
              </div>
              <div class="opt"><span class="side-label">{{ $t('tools.rename.add.where') }}</span><Select v-model="setup.add.where" :options="addPlaces" option-label="label" option-value="value" /></div>
              <template v-if="setup.add.where === 'position'">
                <div class="opt"><span class="side-label">{{ $t('tools.rename.add.afterCharacter') }}</span><InputNumber :model-value="setup.add.at" :min="0" :use-grouping="false" show-buttons @update:model-value="(v) => (setup.add.at = v ?? 0)" /></div>
                <label class="tool-check"><Checkbox v-model="setup.add.fromEnd" binary /><span>{{ $t('tools.rename.fromEnd') }}</span></label>
              </template>
              <template v-if="setup.add.where === 'before' || setup.add.where === 'after'">
                <div class="opt"><span class="side-label">{{ $t('tools.rename.add.find') }}</span><InputText v-model="setup.add.find" :spellcheck="false" :class="{ 'bad-field': bad.add }" /><p class="hint">{{ $t('tools.rename.add.firstMatch') }}</p></div>
                <p v-if="bad.add" class="hint bad">{{ $t('tools.rename.badRegex') }}</p>
                <div class="checks"><label class="tool-check"><Checkbox v-model="setup.add.matchCase" binary /><span>{{ $t('tools.rename.matchCase') }}</span></label><label class="tool-check"><Checkbox v-model="setup.add.regex" binary /><span>{{ $t('tools.rename.regex') }}</span></label></div>
              </template>
            </template>
            <template v-else-if="m === 'replace'">
              <div class="opt"><span class="side-label">{{ $t('tools.rename.find') }}</span><InputText v-model="setup.replace.find" :spellcheck="false" :class="{ 'bad-field': bad.replace }" /></div>
              <div class="opt">
                <span class="side-label">{{ $t('tools.rename.replaceWith') }}</span>
                <InputGroup>
                  <InputText v-model="setup.replace.with" :spellcheck="false" @focus="remember('replace', 'with')($event)" @keyup="remember('replace', 'with')($event)" @click="remember('replace', 'with')($event)" />
                  <InputGroupAddon><button class="icon-btn" :title="$t('tools.rename.insertTag')" :aria-label="$t('tools.rename.insertTag')" @mousedown.prevent @click="showTags($event, 'replace', 'with')"><AppIcon name="code" :size="16" /></button></InputGroupAddon>
                </InputGroup>
                <p v-if="setup.replace.regex" class="hint">{{ $t('tools.rename.replaceGroup') }}</p>
              </div>
              <p v-if="bad.replace" class="hint bad">{{ $t('tools.rename.badRegex') }}</p>
              <SelectButton v-model="setup.replace.which" :options="whichOptions" option-label="label" option-value="value" :allow-empty="false" />
              <div class="checks"><label class="tool-check"><Checkbox v-model="setup.replace.matchCase" binary /><span>{{ $t('tools.rename.matchCase') }}</span></label><label class="tool-check"><Checkbox v-model="setup.replace.regex" binary /><span>{{ $t('tools.rename.regex') }}</span></label></div>
            </template>
            <template v-else-if="m === 'list'">
              <div class="pairs">
                <div v-for="(p, i) in setup.list.pairs" :key="i" class="pair">
                  <InputText v-model="p.find" :spellcheck="false" :placeholder="$t('tools.rename.find')" :aria-label="$t('tools.rename.find')" :class="{ 'bad-field': bad.list }" />
                  <AppIcon name="arrow-right" :size="14" class="faint" />
                  <InputText v-model="p.with" :spellcheck="false" :placeholder="$t('tools.rename.replace')" :aria-label="$t('tools.rename.replaceWith')" />
                  <button class="icon-btn" :title="$t('tools.rename.list.removePair')" :aria-label="$t('tools.rename.list.removePair')" :disabled="setup.list.pairs.length < 2 && !p.find && !p.with" @click="removePair(i)"><AppIcon name="x" :size="16" /></button>
                </div>
              </div>
              <Button :label="$t('tools.rename.list.addPair')" size="small" severity="secondary" text class="add-pair" @click="setup.list.pairs.push({ find: '', with: '' })"><template #icon><AppIcon name="plus" :size="14" /></template></Button>
              <p v-if="bad.list" class="hint bad">{{ $t('tools.rename.badRegex') }}</p>
              <div class="checks"><label class="tool-check"><Checkbox v-model="setup.list.matchCase" binary /><span>{{ $t('tools.rename.matchCase') }}</span></label><label class="tool-check"><Checkbox v-model="setup.list.regex" binary /><span>{{ $t('tools.rename.regex') }}</span></label></div>
            </template>
            <template v-else-if="m === 'remove'">
              <div class="opt"><span class="side-label">{{ $t('tools.rename.remove.what') }}</span><Select v-model="setup.remove.type" :options="removeTypes" option-label="label" option-value="value" /></div>
              <template v-if="setup.remove.type === 'text'">
                <div class="opt"><span class="side-label">{{ $t('tools.rename.remove.text') }}</span><InputText v-model="setup.remove.text" :spellcheck="false" :placeholder="setup.remove.regex ? '' : $t('tools.rename.remove.textPlaceholder')" :class="{ 'bad-field': bad.remove }" /></div>
                <p v-if="bad.remove" class="hint bad">{{ $t('tools.rename.badRegex') }}</p>
                <div class="checks"><label class="tool-check"><Checkbox v-model="setup.remove.matchCase" binary /><span>{{ $t('tools.rename.matchCase') }}</span></label><label class="tool-check"><Checkbox v-model="setup.remove.regex" binary /><span>{{ $t('tools.rename.regex') }}</span></label></div>
              </template>
              <template v-if="setup.remove.type === 'position'">
                <div class="two">
                  <div class="opt"><span class="side-label">{{ $t('tools.rename.remove.from') }}</span><InputNumber :model-value="setup.remove.start" :min="1" :use-grouping="false" show-buttons @update:model-value="(v) => (setup.remove.start = v ?? 1)" /></div>
                  <div class="opt"><span class="side-label">{{ $t('tools.rename.remove.count') }}</span><InputNumber :model-value="setup.remove.count" :min="0" :use-grouping="false" show-buttons @update:model-value="(v) => (setup.remove.count = v ?? 0)" /></div>
                </div>
                <label class="tool-check"><Checkbox v-model="setup.remove.fromEnd" binary /><span>{{ $t('tools.rename.fromEnd') }}</span></label>
              </template>
              <template v-if="setup.remove.type === 'chars'">
                <div class="opt"><span class="side-label">{{ $t('tools.rename.remove.characters') }}</span><InputText v-model="setup.remove.chars" :spellcheck="false" placeholder="#!~" /></div>
                <label class="tool-check"><Checkbox v-model="setup.remove.matchCase" binary /><span>{{ $t('tools.rename.matchCase') }}</span></label>
              </template>
              <p v-if="setup.remove.type === 'symbols'" class="hint">{{ $t('tools.rename.remove.symbolsHint') }}</p>
              <p v-if="setup.remove.type === 'accents'" class="hint">{{ $t('tools.rename.remove.accentsHint') }}</p>
              <p v-if="setup.remove.type === 'brackets'" class="hint">{{ $t('tools.rename.remove.bracketsHint') }}</p>
            </template>
            <template v-else-if="m === 'trim'">
              <div class="opt"><span class="side-label">{{ $t('tools.rename.trim.characters') }}</span><InputText v-model="setup.trim.chars" :spellcheck="false" placeholder=" ._-" /><p class="hint">{{ $t('tools.rename.trim.hint') }}</p></div>
              <SelectButton v-model="setup.trim.where" :options="trimWhere" option-label="label" option-value="value" :allow-empty="false" />
            </template>
            <template v-else-if="m === 'case'">
              <div class="opt"><span class="side-label">{{ $t('tools.rename.case.name') }}</span><Select v-model="setup.case.name" :options="caseNames" option-label="label" option-value="value" /></div>
              <div class="opt"><span class="side-label">{{ $t('tools.rename.case.keep') }}</span><InputText v-model="setup.case.keep" :spellcheck="false" :placeholder="$t('tools.rename.case.keepPlaceholder')" /></div>
              <div class="opt"><span class="side-label">{{ $t('tools.rename.case.extension') }}</span><SelectButton v-model="setup.case.ext" :options="extCases" option-label="label" option-value="value" :allow-empty="false" /></div>
            </template>
          </div>
        </div>
      </div>
      <div class="side-foot">
        <p class="hint">{{ $t('tools.rename.methodsHint') }}</p>
        <button class="icon-btn" :title="$t('tools.rename.resetAll')" :aria-label="$t('tools.rename.resetAll')" @click="reset"><AppIcon name="restore" :size="16" /></button>
      </div>
    </template>

    <div v-if="loading" class="working"><span>{{ tc('tools.rename.loading', loading.total) }}</span><ProgressBar :value="fraction(loading)" :show-value="false" /></div>
    <div v-if="progress" class="working"><span>{{ $t('tools.rename.progress', { done: progress.done.toLocaleString(), total: progress.total.toLocaleString() }) }}</span><ProgressBar :value="fraction(progress)" :show-value="false" /></div>
    <Message v-if="done && !progress" severity="success" :closable="false" class="renamed">{{ tc('tools.rename.done', done.count) }}</Message>
    <div class="rows">
      <DataTable ref="table" :value="rows" data-key="id" size="small" class="tool-table rename-table" striped-rows scrollable scroll-height="flex" :virtual-scroller-options="virtual">
        <Column header="" :style="{ width: `${THUMB_W}rem` }">
          <template #body="{ data }"><div class="thumb-cell"><ToolThumbnail :item="data.id" :size="40" /></div></template>
        </Column>
        <Column :header="$t('tools.rename.columns.current')">
          <template #body="{ data }"><span class="name">{{ data.before }}</span></template>
        </Column>
        <Column :header="$t('tools.rename.columns.new')">
          <template #body="{ data }">
            <span v-if="data.state === 'empty' || data.state === 'same'" class="name faint">{{ data.state === 'empty' ? data.before : data.after }}</span>
            <span v-else class="name after changed">{{ parts(data)[0] }}<mark v-if="parts(data)[1]">{{ parts(data)[1] }}</mark>{{ parts(data)[2] }}<span v-if="STATES[data.state]" v-tooltip.top="STATES[data.state]()" class="mark warn" :aria-label="STATES[data.state]()"><AppIcon name="alert-triangle" :size="15" /></span></span>
          </template>
        </Column>
        <Column header="" :style="{ width: `${DROP_W}rem` }">
          <template #body="{ data }"><button class="icon-btn leave" :title="$t('tools.rename.columns.skip')" :aria-label="$t('tools.rename.columns.skipNamed', { name: data.before })" @click="leave(data.id)"><AppIcon name="x" :size="16" /></button></template>
        </Column>
      </DataTable>
    </div>

    <template #foot>
      <span class="presets">
        <Select v-if="naming === null" :model-value="current?.id ?? null" :options="presets" option-label="name" option-value="id" :placeholder="$t('tools.rename.presets.preset')" :disabled="!presets.length" class="preset-pick" :aria-label="$t('tools.rename.presets.preset')" @update:model-value="usePreset" />
        <InputText v-else :ref="focusField" v-model="naming" :placeholder="$t('tools.rename.presets.name')" maxlength="40" :spellcheck="false" class="preset-pick" :aria-label="$t('tools.rename.presets.name')" @keydown="nameKey" />
        <button class="icon-btn" :title="$t('tools.rename.presets.save')" :aria-label="$t('tools.rename.presets.save')" @click="presetButton"><AppIcon name="device-floppy" :size="16" /></button>
        <button v-if="current && naming === null" class="icon-btn" :title="$t('tools.rename.presets.delete')" :aria-label="$t('tools.rename.presets.delete')" @click="savePresets(presets.filter((p) => p.id !== current.id))"><AppIcon name="trash" :size="16" /></button>
      </span>
      <span class="grow faint">{{ footText }}</span>
      <Button v-if="left.size" :label="$t('tools.rename.foot.includeAll')" size="small" severity="secondary" text @click="left = new Set()" />
      <Button v-if="canUndo" :label="$t('tools.rename.foot.undo')" severity="secondary" :disabled="!!progress" @click="undo" />
      <Button :label="done ? $t('tools.rename.foot.close') : $t('tools.rename.foot.cancel')" severity="secondary" @click="close" />
      <Button :label="changing.length ? tc('tools.rename.foot.rename', changing.length) : $t('tools.rename.foot.nothing')" :disabled="!changing.length || !!progress || blocked" :loading="!!progress" @click="apply" />
    </template>
  </ToolPage>
</template>

<style>
.rename-tool .method.bad .method-name { color: var(--danger); }
.rename-tool .method-body { display: flex; flex-direction: column; gap: calc(0.56rem * var(--g)); padding: calc(0.84rem * var(--g)) calc(0.98rem * var(--g)) calc(0.98rem * var(--g)); }
.rename-tool .method-body .p-inputtext, .rename-tool .method-body .p-inputnumber, .rename-tool .method-body .p-select { width: 100%; }
.rename-tool .method-body .p-inputgroup .p-inputtext { width: auto; flex: 1; min-width: 0; }
.rename-tool .method-body .p-inputnumber-input { width: 100%; min-width: 0; }
.rename-tool .method-body .p-selectbutton { display: flex; }
.rename-tool .method-body .p-selectbutton .p-togglebutton { flex: 1; }
.rename-tool .checks { display: flex; flex-wrap: wrap; gap: 0.42rem 1.12rem; }
.rename-tool .two { display: grid; grid-template-columns: 1fr 1fr; gap: 0.56rem; }
.rename-tool .pairs { display: flex; flex-direction: column; gap: 0.28rem; }
.rename-tool .pair { display: flex; align-items: center; gap: 0.28rem; }
.rename-tool .pair .p-inputtext { flex: 1; min-width: 0; }
.rename-tool .side-foot { display: flex; align-items: flex-start; gap: 0.56rem; padding: calc(0.7rem * var(--g)) calc(0.56rem * var(--g)) calc(0.7rem * var(--g)) calc(0.98rem * var(--g)); }
.rename-tool .side-foot .hint { flex: 1; }
.rename-tool .presets { display: flex; align-items: center; gap: 0.14rem; }
.rename-tool .preset-pick { width: round(12.59rem, 1px); }
.rename-tool .bad { color: var(--danger); }
.rename-tool .bad-field { border-color: var(--danger) !important; }
.rename-tool .renamed { margin: 0.7rem 0.98rem; }
.rename-tool .rows { flex: 1; min-height: 0; }
.rename-tool .rows .p-datatable { height: 100%; }
.rename-tool .rename-table td { vertical-align: middle; }
.rename-tool .rename-table .p-datatable-tbody > tr > td:nth-child(2), .rename-tool .rename-table .p-datatable-tbody > tr > td:nth-child(3) { padding-inline: 0.84rem; }
.rename-tool .thumb-cell { width: round(2.8rem, 1px); height: round(2.24rem, 1px); }
.rename-tool .thumb-cell .tool-thumb { width: round(2.8rem, 1px) !important; height: round(2.24rem, 1px) !important; border-radius: calc(4px * var(--r)); }
.rename-tool .name { display: block; overflow-wrap: anywhere; line-height: 1.3; }
.rename-tool .after.changed { color: var(--accent); }
.rename-tool .after mark { background: color-mix(in srgb, var(--accent) 25%, transparent); color: inherit; border-radius: calc(2px * var(--r)); }
.rename-tool .mark { display: inline-flex; vertical-align: -2px; margin-left: 0.42rem; }
.rename-tool .mark.warn { color: var(--warning); }
.rename-tool .rename-table tr:hover .leave, .rename-tool .leave:focus-visible { visibility: visible; }
</style>

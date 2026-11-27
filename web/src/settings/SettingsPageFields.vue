<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import Select from 'openvue/select'
import InputText from 'openvue/inputtext'
import Button from 'openvue/button'
import Checkbox from 'openvue/checkbox'
import Dialog from 'openvue/dialog'
import ToggleSwitch from 'openvue/toggleswitch'
import Tag from 'openvue/tag'
import { S } from '../app/state.js'
import { run, toast } from '../app/notify.js'
import { ownerName, filterKindOf } from '../filters/query.js'
import { saveUi } from '../app/preferences.js'
import { pollStatus } from '../library/lifecycle.js'
import { call, errorText } from '../platform/ipc.js'
import AppIcon from '../shared/AppIcon.vue'
import AppMenu from '../shared/AppMenu.vue'
import { t } from '../shared/i18n.js'

// Settings › Fields: the person's own fields, the fields read from files by source with how far they are read, the ones switched off, and Clean up.
const props = defineProps({ plugins: { type: Array, default: () => [] } })
const n = (x) => (x || 0).toLocaleString()
const readsFiles = (f) => !!f.readers?.length
const fields = computed(() => S.sidebar.fields)
const own = computed(() => fields.value.filter((f) => f.owner === 'user'))
const iconOf = (owner) => props.plugins.find((p) => p.id === owner)?.icon || 'puzzle'
function groupsOf(list) {
  const m = new Map()
  for (const f of list) m.has(f.owner) ? m.get(f.owner).push(f) : m.set(f.owner, [f])
  return [...m].map(([owner, items]) => ({ owner, label: ownerName(owner), icon: iconOf(owner), items: items.sort((a, b) => a.label.localeCompare(b.label)) })).sort((a, b) => a.label.localeCompare(b.label))
}
const sourceGroups = computed(() => groupsOf(fields.value.filter((f) => f.owner !== 'user')))
const offFields = computed(() => fields.value.filter((f) => f.indexed === false))
const onCount = (g) => g.items.filter((f) => f.indexed !== false).length

// coverage: when the page opens, when fields are switched, and after a read ends
const cov = ref({})
async function loadCoverage() {
  if (!S.libOpen) return
  const r = await call('field.coverage', {}).catch(() => null)
  if (r) cov.value = Object.fromEntries(r.fields.map((x) => [x.key, x]))
}
onMounted(loadCoverage)
watch(() => fields.value.map((f) => f.key + (f.indexed === false ? '-' : '+')).join(), loadCoverage)
watch(() => (S.status.jobs || []).map((j) => j.id), (now, before) => before?.some((id) => !now.includes(id)) && loadCoverage())
const keysOf = (list) => list.filter((f) => f.indexed !== false && readsFiles(f)).map((f) => f.key)
const sumOf = (list, k) => list.reduce((s, f) => s + (cov.value[f.key]?.[k] || 0), 0)
const kept = computed(() => offFields.value.filter((f) => f.owner !== 'user').reduce((s, f) => s + (cov.value[f.key]?.withValue || 0), 0))

// reading again, per group or per field, with the job it started
const jobs = ref({})
const running = (tag) => (S.status.jobs || []).find((j) => j.id === jobs.value[tag])
async function readAgain(tag, keys, mode) {
  const r = await run('field.rescan', { keys, mode }).catch(() => null)
  if (!r) return
  if (!r.files) return toast(t('settings.fields.nothingToRescan'))
  if (r.job) jobs.value[tag] = r.job
  pollStatus()
}
const menu = ref()
const menuModel = ref([])
function readMenu(e, tag, keys) {
  menuModel.value = [
    { label: t('settings.fields.rescan.missing'), icon: 'file-plus', command: () => readAgain(tag, keys, 'missing') },
    { label: t('settings.fields.rescan.all'), icon: 'refresh', command: () => readAgain(tag, keys, 'all') },
  ]
  menu.value.show(e)
}

// a row opens to its details
const open = ref(null)
const toggle = (key) => (open.value = open.value === key ? null : key)
const update = (f, patch) => run('field.define', { key: f.key, ...patch })
const KINDS = ['list', 'range', 'text', 'has']
// how a field filters, wherever it is picked from; `own_filter` lists it with the main filters and smart folder rules rather than in its group
const kindOptions = (f) => [['auto', filterKindOf({ ...f, filter_kind: '' })], ...KINDS.map((k) => [k])]
const kindName = (o) => (o[0] === 'auto' ? t('settings.fields.kinds.auto', { kind: t(`settings.fields.kinds.${o[1]}`) }) : t(`settings.fields.kinds.${o[0]}`))
function setListed(f, on) {
  update(f, { ownFilter: on })
  const id = 'field:' + f.key
  const pins = S.ui.pinnedFilters
  if (on && !pins.includes(id)) pins.push(id)
  if (!on && pins.includes(id)) pins.splice(pins.indexOf(id), 1)
  saveUi()
}

// your own fields
const nf = ref({ label: '', key: '', type: 'text', options: '' })
const TYPES = ['text', 'integer', 'float', 'boolean', 'date', 'datetime', 'single_select', 'multi_select', 'json', 'url', 'file_ref']
// a type a plugin's field may have and Curio has no name for shows its id
const typeName = (type) => (TYPES.includes(type) ? t(`settings.fields.types.${type}`) : type)
async function createField() {
  const x = nf.value
  if (!x.label.trim()) return
  const key = x.key.trim() || x.label.trim().toLowerCase().replace(/[^a-z0-9_.]+/g, '_')
  const def = { key, label: x.label.trim(), type: x.type, description: '', defaultValue: null, options: x.options.split(',').map((s) => s.trim()).filter(Boolean), validation: { required: false }, format: '', widget: '', visible: true, sortOrder: 0 }
  await run('field.define', def, t('settings.fields.created', { name: def.label }))
  nf.value = { label: '', key: '', type: 'text', options: '' }
}
const askDelete = (f) => (S.dialogs.confirm = { title: t('settings.fields.delete.title', { name: f.label }), body: t('settings.fields.delete.body'), okLabel: t('settings.fields.delete.ok'), ok: () => run('field.delete', { key: f.key }) })

// switching off: what names the field (and its pair) is listed first, and left as it is
const offAsk = ref(null)
const pairOf = (f) => fields.value.find((x) => x.key !== f.key && (x.key === f.pair || x.pair === f.key))
// a field filtered beside another's values, as a LoRA's strength in the LoRA filter
const pairedTo = (f) => (f.pair ? fields.value.find((x) => x.key === f.pair && x.key !== f.key) : null)
async function askOff(f) {
  const pair = pairOf(f)
  const keys = [f.key, pair?.key].filter(Boolean)
  const used = await Promise.all(keys.map((key) => call('field.usedBy', { key }).catch(() => null)))
  const deps = []
  for (const u of used.filter(Boolean)) {
    for (const s of u.smartFolders) deps.push({ icon: 'folder-bolt', label: s.name })
    for (const x of u.filters) deps.push({ icon: 'filter', label: [x.label, ownerName(x.plugin)].join(' · ') })
    for (const x of u.sorts) deps.push({ icon: 'arrows-sort', label: [x.label, ownerName(x.plugin)].join(' · ') })
  }
  const grid = S.ui.grid || {}
  for (const k of keys) {
    const label = fields.value.find((x) => x.key === k)?.label || k
    if (S.ui.pinnedFilters.includes('field:' + k)) deps.push({ icon: 'pin', label: t('settings.fields.off.pinned', { name: label }) })
    if ((grid.listCols || []).some((c) => c.id === 'field:' + k)) deps.push({ icon: 'columns', label: t('settings.fields.off.column', { name: label }) })
    if (grid.sort?.key === 'field' && grid.sort.field === k) deps.push({ icon: 'arrows-sort', label: t('settings.fields.off.sort', { name: label }) })
  }
  offAsk.value = { f, pair, deps }
}
async function switchOff() {
  const { f } = offAsk.value
  offAsk.value = null
  await run('field.define', { key: f.key, indexed: false }).catch(() => {})
}

// switching on: read the files now, or later
const onAsk = ref(null)
const isPluginGone = (f) => !readsFiles(f) && f.owner !== 'user' && !f.owner.startsWith('builtin-') && !props.plugins.some((p) => p.id === f.owner)
function toggleOn(f) {
  if (readsFiles(f)) onAsk.value = f
  else run('field.define', { key: f.key, indexed: true }).catch(() => {})
}
async function switchOn(now) {
  const f = onAsk.value
  onAsk.value = null
  const ok = await run('field.define', { key: f.key, indexed: true, ...(now ? {} : { needsScan: true }) }).then(() => true, () => false)
  if (ok && now) readAgain(f.owner, [f.key, ...(pairOf(f) ? [pairOf(f).key] : [])], 'missing')
}

// what readers failed on
const failures = ref(null)
async function showFailures(keys) {
  const r = await call('field.failures', { keys }).catch((e) => (toast(errorText(e), 'error'), null))
  if (r) failures.value = { keys, list: r.failures }
}
async function tryAgain() {
  const { keys, list } = failures.value
  failures.value = null
  const r = await run('field.rescan', { keys, items: [...new Set(list.map((x) => x.id))], mode: 'all' }).catch(() => null)
  if (r?.job) (jobs.value[keys.join()] = r.job), pollStatus()
}

// Clean up: the values kept for fields switched off; a field whose plugin is gone is left unticked
const clean = ref(null)
async function askClean() {
  const keys = offFields.value.filter((f) => f.owner !== 'user').map((f) => f.key)
  const r = await call('field.cleanup', { keys, dryRun: true }).catch((e) => (toast(errorText(e), 'error'), null))
  if (!r) return
  const rows = r.fields.filter((x) => x.values).map((x) => ({ ...x, label: fields.value.find((f) => f.key === x.key)?.label || x.key, on: !x.pluginGone }))
  if (!rows.length) return toast(t('settings.fields.clean.nothing'))
  clean.value = rows
}
async function doClean() {
  const keys = clean.value.filter((x) => x.on).map((x) => x.key)
  clean.value = null
  if (!keys.length) return
  const r = await run('field.cleanup', { keys }).catch(() => null)
  if (r) toast(t('settings.fields.clean.removed', { count: n(r.removed) }, r.removed))
  loadCoverage()
}
const cleanCount = computed(() => (clean.value || []).filter((x) => x.on).reduce((s, x) => s + x.values, 0))
</script>

<template>
  <section class="s-section">
    <div class="s-section-head">
      <span class="s-section-title">{{ $t('settings.fields.own.title') }} <Tag severity="secondary" :value="String(own.length)" /></span>
    </div>
    <div class="s-card">
      <div class="s-subblock create-bar">
        <div class="row gap wrap">
          <InputText v-model="nf.label" :placeholder="$t('settings.fields.own.label')" class="grow-in" @keydown.enter="createField" />
          <InputText v-model="nf.key" :placeholder="$t('settings.fields.own.key')" class="narrow" />
          <Select v-model="nf.type" :options="TYPES" :optionLabel="typeName" class="type-in" />
          <InputText v-if="nf.type.endsWith('select')" v-model="nf.options" placeholder="a, b, c" class="narrow" />
          <Button :label="$t('settings.fields.own.create')" size="small" :disabled="!nf.label.trim()" @click="createField" />
        </div>
      </div>
      <template v-for="f in own" :key="f.key">
        <div v-if="f.indexed !== false" class="s-row frow" :class="{ open: open === f.key }" :data-field="f.key" @click="toggle(f.key)">
          <AppIcon name="chevron-right" :size="15" class="s-exp-chev" />
          <div class="s-meta">
            <span class="s-label">{{ f.label }} <Tag severity="secondary" :value="typeName(f.type)" /></span>
          </div>
          <div class="s-ctrl" @click.stop>
            <ToggleSwitch :modelValue="true" :aria-label="f.label" @update:modelValue="() => askOff(f)" />
            <Button severity="secondary" text size="small" v-tooltip="$t('settings.fields.own.delete')" :aria-label="$t('settings.fields.own.deleteField')" @click="askDelete(f)"><template #icon><AppIcon name="trash" :size="15" /></template></Button>
          </div>
        </div>
        <div v-else class="s-row frow off" :data-field="f.key">
          <AppIcon name="chevron-right" :size="15" class="chev-pad" />
          <div class="s-meta"><span class="s-label">{{ f.label }}</span></div>
          <div class="s-ctrl">
            <ToggleSwitch :modelValue="false" :aria-label="f.label" @update:modelValue="() => toggleOn(f)" />
            <Button severity="secondary" text size="small" v-tooltip="$t('settings.fields.own.delete')" :aria-label="$t('settings.fields.own.deleteField')" @click="askDelete(f)"><template #icon><AppIcon name="trash" :size="15" /></template></Button>
          </div>
        </div>
        <div v-if="f.indexed !== false && open === f.key" class="fbody">
          <span class="s-desc">{{ $t('settings.fields.row.key') }}</span><span><code>{{ f.key }}</code> <span class="s-desc">{{ typeName(f.type) }}<template v-if="f.options?.length"> · {{ f.options.join(', ') }}</template></span></span>
          <span class="s-desc">{{ $t('settings.fields.row.shown') }}</span><label class="opt"><Checkbox :modelValue="f.visible" binary @update:modelValue="(v) => update(f, { visible: v })" /> {{ $t('settings.fields.row.inspector') }}</label>
          <span class="s-desc">{{ $t('settings.fields.row.filter') }}</span><Select :modelValue="f.filter_kind || 'auto'" :options="kindOptions(f)" :optionLabel="kindName" :optionValue="(o) => o[0]" size="small" class="fkind" @update:modelValue="(v) => update(f, { filterKind: v === 'auto' ? '' : v })" />
          <span class="s-desc">{{ $t('settings.fields.row.listed') }}</span><label class="opt"><Checkbox :modelValue="!!f.own_filter" binary @update:modelValue="(v) => setListed(f, v)" /> {{ $t('settings.fields.row.mainFilters') }}</label>
        </div>
      </template>
    </div>
  </section>

  <section v-for="g in sourceGroups" :key="g.owner" class="s-section" :data-group="g.owner">
    <div class="s-section-head">
      <span class="s-section-title">
        <AppIcon :name="g.icon" :size="15" />
        <span>{{ g.label }}</span>
        <Tag severity="secondary" :value="onCount(g) === g.items.length ? String(g.items.length) : `${onCount(g)}/${g.items.length}`" />
      </span>
      <div class="s-section-actions">
        <Button v-if="sumOf(g.items, 'failed')" :label="$t('settings.fields.failed', { count: n(sumOf(g.items, 'failed')) }, sumOf(g.items, 'failed'))" severity="danger" text size="small" @click="showFailures(keysOf(g.items))" />
        <template v-if="running(g.owner)">
          <span class="s-mono s-desc">{{ n(running(g.owner).done) }}/{{ n(running(g.owner).total) }}</span>
          <Button :label="$t('settings.fields.group.stop')" severity="secondary" size="small" @click="run('job_cancel', { id: running(g.owner).id })"><template #icon><AppIcon name="player-stop" :size="14" /></template></Button>
        </template>
        <Button v-else-if="keysOf(g.items).length" :label="$t('settings.fields.group.rescan')" severity="secondary" size="small" @click="readMenu($event, g.owner, keysOf(g.items))"><template #icon><AppIcon name="refresh" :size="14" /></template></Button>
      </div>
    </div>
    <div class="s-card">
      <div v-if="running(g.owner)" class="bar"><div class="fill" :style="{ width: (running(g.owner).total ? (100 * running(g.owner).done) / running(g.owner).total : 0) + '%' }" /></div>
      <template v-for="f in g.items" :key="f.key">
        <div v-if="f.indexed !== false" class="s-row frow" :class="{ open: open === f.key }" :data-field="f.key" @click="toggle(f.key)">
          <AppIcon name="chevron-right" :size="15" class="s-exp-chev" />
          <div class="s-meta">
            <span class="s-label">
              <span>{{ f.label }}</span>
              <span class="s-desc s-mono">{{ f.key }}</span>
            </span>
          </div>
          <div class="s-ctrl" @click.stop>
            <template v-if="readsFiles(f)">
              <Button v-if="open === f.key && cov[f.key]?.failed" :label="$t('settings.fields.failed', { count: n(cov[f.key].failed) }, cov[f.key].failed)" severity="danger" text size="small" @click="showFailures([f.key])" />
              <Button v-if="open === f.key" :label="$t('settings.fields.group.rescan')" severity="secondary" size="small" @click="readMenu($event, g.owner, [f.key])" />
              <Button v-else-if="f.needs_scan" :label="$t('settings.fields.row.notScanned')" severity="secondary" text size="small" @click="readAgain(g.owner, [f.key], 'missing')" />
              <Tag v-else-if="cov[f.key]?.waiting" severity="info" :value="$t('settings.fields.waiting', { count: n(cov[f.key].waiting) }, cov[f.key].waiting)" />
            </template>
            <ToggleSwitch :modelValue="true" :aria-label="f.label" @update:modelValue="() => askOff(f)" />
          </div>
        </div>
        <div v-else class="s-row frow off" :data-field="f.key">
          <AppIcon name="chevron-right" :size="15" class="chev-pad" />
          <div class="s-meta"><span class="s-label">{{ f.label }}</span></div>
          <div class="s-ctrl">
            <ToggleSwitch v-if="!isPluginGone(f)" :modelValue="false" :aria-label="f.label" @update:modelValue="() => toggleOn(f)" />
            <Tag v-else severity="secondary" :value="$t('settings.fields.pluginGone')" />
          </div>
        </div>
        <div v-if="f.indexed !== false && open === f.key" class="fbody">
          <span class="s-desc">{{ $t('settings.fields.row.key') }}</span><span><code>{{ f.key }}</code> <span class="s-desc">{{ f.type }}</span></span>
          <template v-if="f.description"><span class="s-desc">{{ $t('settings.fields.row.about') }}</span><span class="s-desc">{{ f.description }}</span></template>
          <span class="s-desc">{{ $t('settings.fields.row.shown') }}</span><label class="opt"><Checkbox :modelValue="f.visible" binary @update:modelValue="(v) => update(f, { visible: v })" /> {{ $t('settings.fields.row.inspector') }}</label>
          <template v-if="pairedTo(f)"><span class="s-desc">{{ $t('settings.fields.row.filter') }}</span><span class="s-desc">{{ $t('settings.fields.pairedFilter', { name: pairedTo(f).label }) }}</span></template>
          <template v-else>
            <span class="s-desc">{{ $t('settings.fields.row.filter') }}</span><Select :modelValue="f.filter_kind || 'auto'" :options="kindOptions(f)" :optionLabel="kindName" :optionValue="(o) => o[0]" size="small" class="fkind" @update:modelValue="(v) => update(f, { filterKind: v === 'auto' ? '' : v })" />
            <span class="s-desc">{{ $t('settings.fields.row.listed') }}</span><label class="opt"><Checkbox :modelValue="!!f.own_filter" binary @update:modelValue="(v) => setListed(f, v)" /> {{ $t('settings.fields.row.mainFilters') }}</label>
          </template>
        </div>
      </template>
    </div>
  </section>

  <div v-if="kept" class="s-card">
    <div class="s-row">
      <div class="s-meta"><span class="s-desc">{{ $t('settings.fields.kept', { count: n(kept) }, kept) }}</span></div>
      <div class="s-ctrl"><Button :label="$t('settings.fields.clean.button')" severity="secondary" size="small" @click="askClean" /></div>
    </div>
  </div>

  <AppMenu ref="menu" :model="() => menuModel" />

  <Dialog :visible="!!offAsk" @update:visible="(v) => !v && (offAsk = null)" modal :header="$t('settings.fields.off.title', { name: offAsk?.f.label })" :style="{ width: 'min(33.57rem, 92vw)' }">
    <p v-if="offAsk?.pair">{{ $t('settings.fields.off.pair', { name: offAsk.pair.label }) }}</p>
    <p>{{ $t('settings.fields.off.body') }}</p>
    <template v-if="offAsk?.deps.length">
      <p class="faint small">{{ $t('settings.fields.off.namedBy') }}</p>
      <div class="deps"><div v-for="(d, i) in offAsk.deps" :key="i" class="row"><AppIcon :name="d.icon" class="faint" /><span class="ellipsis">{{ d.label }}</span></div></div>
    </template>
    <div class="row dfoot"><span class="grow" /><Button :label="$t('settings.fields.row.switchOff')" severity="danger" autofocus @click="switchOff" /><Button :label="$t('common.cancel')" severity="secondary" @click="offAsk = null" /></div>
  </Dialog>

  <Dialog :visible="!!onAsk" @update:visible="(v) => !v && (onAsk = null)" modal :header="$t('settings.fields.on.title', { name: onAsk?.label })" :style="{ width: 'min(33.57rem, 92vw)' }">
    <p>{{ $t('settings.fields.on.body') }}</p>
    <div class="row dfoot"><span class="grow" /><Button :label="$t('settings.fields.on.now')" autofocus @click="switchOn(true)" /><Button :label="$t('settings.fields.on.later')" severity="secondary" @click="switchOn(false)" /></div>
  </Dialog>

  <Dialog :visible="!!failures" @update:visible="(v) => !v && (failures = null)" modal :header="$t('settings.fields.failures.title')" :style="{ width: 'min(44.76rem, 94vw)' }">
    <div class="fails">
      <div v-for="(x, i) in failures?.list" :key="i" class="fail">
        <div class="row"><span class="grow ellipsis">{{ x.name }}</span><span class="faint small mono">{{ x.reader }}</span></div>
        <div class="faint small">{{ x.error }}</div>
      </div>
      <p v-if="!failures?.list.length" class="faint">{{ $t('settings.fields.failures.none') }}</p>
    </div>
    <div class="row dfoot"><span class="grow" /><Button v-if="failures?.list.length" :label="$t('settings.fields.failures.retry')" @click="tryAgain" /><Button :label="$t('settings.fields.failures.close')" severity="secondary" @click="failures = null" /></div>
  </Dialog>

  <Dialog :visible="!!clean" @update:visible="(v) => !v && (clean = null)" modal :header="$t('settings.fields.clean.button')" :style="{ width: 'min(33.57rem, 92vw)' }">
    <p>{{ $t('settings.fields.clean.body') }}</p>
    <div class="deps">
      <label v-for="x in clean" :key="x.key" class="opt"><Checkbox v-model="x.on" binary /><span class="grow ellipsis">{{ x.label }}<span v-if="x.pluginGone" class="faint"> · {{ $t('settings.fields.clean.pluginGone') }}</span></span><span class="mono small faint">{{ n(x.values) }}</span></label>
    </div>
    <div class="row dfoot"><span class="grow" /><Button :label="$t('settings.fields.clean.remove', { n: n(cleanCount) })" severity="danger" :disabled="!cleanCount" @click="doClean" /><Button :label="$t('common.cancel')" severity="secondary" @click="clean = null" /></div>
  </Dialog>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.grow-in { flex: 1 1 11.19rem; min-width: 0; }
.narrow { width: round(8.39rem, 1px); }
.type-in { width: round(9.79rem, 1px); }
.frow { cursor: pointer; padding: calc(0.56rem * var(--g)) calc(0.98rem * var(--g)); min-height: round(2.8rem, 1px); gap: calc(0.63rem * var(--g)); }
.frow:hover { background: var(--hover); }
.frow.open { background: var(--surface-3); }
.frow.open .s-exp-chev { transform: rotate(90deg); }
.frow.off { cursor: default; }
.frow.off .s-meta { opacity: 0.55; }
.chev-pad { visibility: hidden; flex: none; }
.fbody { display: grid; grid-template-columns: 5.5rem minmax(0, 1fr); align-items: center; gap: calc(0.56rem * var(--g)) calc(0.7rem * var(--g)); padding: calc(0.35rem * var(--g)) calc(1.05rem * var(--g)) calc(0.84rem * var(--g)) calc(2.45rem * var(--g)); background: var(--surface-3); }
.fkind { width: round(11.2rem, 1px); }
.bar { height: 0.21rem; background: var(--surface-3); overflow: hidden; }
.bar .fill { height: 100%; background: var(--accent); transition: width 0.2s linear; }
.deps { display: flex; flex-direction: column; gap: calc(0.42rem * var(--g)); max-height: 40vh; overflow-y: auto; margin-bottom: calc(0.56rem * var(--g)); }
.deps .opt { display: flex; }
.fails { display: flex; flex-direction: column; gap: calc(0.42rem * var(--g)); max-height: 55vh; overflow-y: auto; }
.fail { padding: calc(0.49rem * var(--g)) calc(0.7rem * var(--g)); border-radius: calc(6px * var(--r)); background: var(--surface-2); }
.dfoot { gap: calc(0.56rem * var(--g)); margin-top: calc(0.84rem * var(--g)); }
p { margin: 0 0 calc(0.56rem * var(--g)); }
</style>

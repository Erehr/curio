<script setup>
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import Select from 'openvue/select'
import InputText from 'openvue/inputtext'
import InputGroup from 'openvue/inputgroup'
import Button from 'openvue/button'
import ToggleSwitch from 'openvue/toggleswitch'
import Tag from 'openvue/tag'
import Message from 'openvue/message'
import { S } from '../app/state.js'
import { openPicker, tagTint } from '../tags/tags.js'
import { toast } from '../app/notify.js'
import { call, errorText, saidText } from '../platform/ipc.js'
import AppIcon from '../shared/AppIcon.vue'
import AppChip from '../shared/AppChip.vue'
import SettingsPageAutoImportRuleDialog from './SettingsPageAutoImportRuleDialog.vue'
import { t } from '../shared/i18n.js'

// Each watched folder expands to its core fields, an options repeater for non-default overrides, and a flat list of rules edited in a modal dialog.
const cfg = ref(null)
const report = ref([])
const open = reactive({})
const watchOpts = reactive({})
const editingRule = ref(null)
const SAVE_DELAY_MS = 450
const IGNORE_EXAMPLE = '*.txt, temp_*, raw/**'

const OPT_KEYS = ['folder', 'tags', 'subfolders', 'after', 'extensions', 'ignore']
const SUB = [['off'], ['flat'], ['mirror'], ['match']]
const AFTER = [['move'], ['delete'], ['keep']]
const subName = (o) => t(`settings.autoImport.subfolders.${o[0]}`)
const afterName = (o) => t(`settings.autoImport.after.${o[0]}`)
const optLabel = (k) => t(`settings.autoImport.options.${k}`)

let lastSaved = ''
const snapshot = () => JSON.stringify(cfg.value)
onMounted(async () => {
  const r = await call('auto_import_get').catch((e) => {
    toast(errorText(e), 'error')
    return { config: { enabled: false, watches: [] }, watches: [] }
  })
  cfg.value = r.config
  report.value = r.watches
  lastSaved = snapshot()
})

// no Apply button: what is on screen is what is in force, a moment after typing stops
let timer = null
async function save() {
  const now = snapshot()
  if (now === lastSaved) return
  try {
    const r = await call('auto_import_save', { config: JSON.parse(now) })
    lastSaved = now
    report.value = r.watches
  } catch (e) {
    toast(errorText(e), 'error')
  }
}
watch(cfg, () => {
  clearTimeout(timer)
  timer = setTimeout(save, SAVE_DELAY_MS)
}, { deep: true })
onBeforeUnmount(() => {
  clearTimeout(timer)
  save()
})

const stateOf = (id) => report.value.find((r) => r.id === id)
const newId = () => 'w' + Date.now().toString(36) + Math.random().toString(36).slice(2, 6)
const folderById = (id) => S.sidebar.folders.find((f) => f.id === id)

function optsFor(w) {
  if (!watchOpts[w.id]) {
    const list = []
    if (w.folder != null) list.push('folder')
    if (w.tags?.length) list.push('tags')
    if (w.subfolders && w.subfolders !== 'flat') list.push('subfolders')
    if (w.after && w.after !== 'move') list.push('after')
    if (w.extensions) list.push('extensions')
    if (w.ignore) list.push('ignore')
    watchOpts[w.id] = list
  }
  return watchOpts[w.id]
}
function resetOpt(w, k) {
  if (k === 'folder') w.folder = null
  else if (k === 'tags') w.tags = []
  else if (k === 'subfolders') w.subfolders = 'flat'
  else if (k === 'after') w.after = 'move'
  else if (k === 'extensions') w.extensions = ''
  else if (k === 'ignore') w.ignore = ''
}
const unusedOpts = (w) => OPT_KEYS.filter((k) => !optsFor(w).includes(k))
const availableOpts = (w, cur) => OPT_KEYS.filter((k) => k === cur || !optsFor(w).includes(k)).map((v) => ({ v, l: optLabel(v) }))
function addOpt(w, afterIdx = null) {
  const next = unusedOpts(w)[0]
  if (!next) return
  if (afterIdx == null) optsFor(w).push(next)
  else optsFor(w).splice(afterIdx + 1, 0, next)
}
function removeOpt(w, idx) {
  const [k] = optsFor(w).splice(idx, 1)
  resetOpt(w, k)
}
function changeOpt(w, idx, nextKey) {
  const prev = optsFor(w)[idx]
  if (prev === nextKey) return
  resetOpt(w, prev)
  optsFor(w)[idx] = nextKey
}

async function addWatch() {
  const p = await call('pick', { kind: 'folder' })
  if (!p) return
  const id = newId()
  // off until the person has set it up and switches it on
  cfg.value.watches.push({ id, enabled: false, name: p[0].split(/[\\/]/).pop(), path: p[0], subfolders: 'flat', extensions: '', ignore: '', folder: null, tags: [], after: 'move', rules: [] })
  open[id] = true
}
async function choosePath(w) {
  const p = await call('pick', { kind: 'folder' })
  if (p) w.path = p[0]
}
function removeWatch(w) {
  delete watchOpts[w.id]
  cfg.value.watches.splice(cfg.value.watches.indexOf(w), 1)
}
function openNewRule(w) {
  editingRule.value = { watch: w, index: -1, rule: { enabled: true, name: '', matching: 'all', conditions: [{ field: 'name', op: 'contains', value: '' }], folder: null, tags: [], rating: 0, note: '', skip: false, stop: false } }
}
function openEditRule(w, i) {
  editingRule.value = { watch: w, index: i, rule: w.rules[i] }
}
function saveRule(rule) {
  const { watch: w, index } = editingRule.value
  if (index >= 0) w.rules[index] = rule
  else w.rules.push(rule)
  editingRule.value = null
}
function moveRule(w, i, d) {
  const j = i + d
  if (j < 0 || j >= w.rules.length) return
  ;[w.rules[i], w.rules[j]] = [w.rules[j], w.rules[i]]
}
function chooseFolder(e, o) {
  openPicker('folders', e, { ids: [], canCreate: true, onPick: (r) => (o.folder = r.id) })
}
function chooseTags(e, o) {
  o.tags ||= []
  openPicker('tags', e, {
    value: [...o.tags],
    canCreate: true,
    onToggle: (name) => {
      const i = o.tags.indexOf(name)
      if (i >= 0) o.tags.splice(i, 1)
      else o.tags.push(name)
    },
  })
}
async function allow(w) {
  const r = await call('auto_import_allow', { id: w.id })
  report.value = r.watches
}
function summary(w) {
  const parts = []
  parts.push(w.folder && folderById(w.folder) ? t('settings.autoImport.summary.to', { folder: folderById(w.folder).name }) : t('settings.autoImport.summary.noFolder'))
  if (w.tags.length) parts.push(t('settings.autoImport.summary.tags', { count: w.tags.length }, w.tags.length))
  if (w.rules.length) parts.push(t('settings.autoImport.summary.rules', { count: w.rules.length }, w.rules.length))
  if (['move', 'delete', 'keep'].includes(w.after)) parts.push(t(`settings.autoImport.summary.after.${w.after}`))
  return parts.join(' · ')
}
function ruleSummary(r) {
  const condCount = r.conditions?.length || 0
  const condText = t('settings.autoImport.rule.condCount', { count: condCount }, condCount)
  if (r.skip) return `${condText} → ${t('settings.autoImport.rule.skip')}`
  const acts = []
  if (r.folder && folderById(r.folder)) acts.push(folderById(r.folder).name)
  if (r.tags?.length) acts.push(t('settings.autoImport.summary.tags', { count: r.tags.length }, r.tags.length))
  if (r.rating > 0) acts.push('★'.repeat(r.rating))
  if (r.note) acts.push(t('settings.autoImport.rule.note'))
  if (r.stop) acts.push(t('settings.autoImport.rule.stopShort'))
  return acts.length ? `${condText} → ${acts.join(', ')}` : condText
}
const ruleTitle = (r, i) => r.name || t('settings.autoImport.rule.untitled', { n: i + 1 })
const allOn = computed({ get: () => !!cfg.value?.enabled, set: (v) => (cfg.value.enabled = v) })
</script>

<template>
  <template v-if="cfg">
    <section class="s-section">
      <div class="s-section-head"><span class="s-section-title">{{ $t('settings.autoImport.serviceTitle') }}</span></div>
      <div class="s-card">
        <div class="s-row">
          <div class="s-meta"><span class="s-label">{{ $t('settings.autoImport.enabled') }}</span></div>
          <div class="s-ctrl">
            <ToggleSwitch v-model="allOn" :aria-label="$t('settings.autoImport.enabled')" />
          </div>
        </div>
      </div>
    </section>

    <section class="s-section">
      <div class="s-section-head">
        <span class="s-section-title">{{ $t('settings.autoImport.foldersTitle') }} <Tag v-if="cfg.watches.length" severity="secondary" :value="String(cfg.watches.length)" /></span>
        <div class="s-section-actions">
          <Button :label="$t('settings.autoImport.addFolder')" size="small" severity="secondary" @click="addWatch"><template #icon><AppIcon name="plus" :size="15" /></template></Button>
        </div>
      </div>

      <div v-if="!cfg.watches.length" class="s-card s-empty">{{ $t('settings.autoImport.empty') }}</div>
      <div v-else class="s-exp-list">
        <div v-for="w in cfg.watches" :key="w.id" class="s-exp-item" :class="{ open: open[w.id], off: !w.enabled || !cfg.enabled }">
          <div class="s-exp-head" @click="open[w.id] = !open[w.id]">
            <AppIcon name="chevron-right" :size="15" class="s-exp-chev" />
            <AppIcon name="folder-open" :size="18" class="watch-ico" />
            <div class="s-meta">
              <span class="s-label">
                <span class="ellipsis">{{ w.name || w.path || $t('settings.autoImport.watch.untitled') }}</span>
                <span class="watch-path s-mono ellipsis">{{ w.path }}</span>
              </span>
              <span class="s-desc ellipsis">{{ summary(w) }}</span>
            </div>
            <Tag v-if="stateOf(w.id)?.problem" severity="danger" :value="$t('settings.autoImport.errorTag')" v-tooltip.bottom="saidText(stateOf(w.id).problem)" />
            <Tag v-else-if="stateOf(w.id) && !stateOf(w.id).allowed" severity="warn" :value="$t('settings.autoImport.waiting')" />
            <span @click.stop><ToggleSwitch v-model="w.enabled" :aria-label="w.name || w.path" /></span>
          </div>

          <div v-if="open[w.id]" class="s-exp-body">
            <Message v-if="stateOf(w.id) && !stateOf(w.id).allowed" severity="warn" :closable="false">
              <div class="msg-row">
                <span class="grow">{{ $t('settings.autoImport.paused') }}</span>
                <Button :label="$t('settings.autoImport.allow')" size="small" severity="secondary" @click="allow(w)" />
              </div>
            </Message>
            <Message v-if="stateOf(w.id)?.problem" severity="error" :closable="false">{{ saidText(stateOf(w.id).problem) }}</Message>

            <div class="s-form-grid">
              <div class="s-field">
                <label class="s-field-label">{{ $t('settings.autoImport.watch.name') }}</label>
                <InputText v-model="w.name" :placeholder="$t('settings.autoImport.watch.name')" />
              </div>
              <div class="s-field">
                <label class="s-field-label">{{ $t('settings.autoImport.watch.source') }}</label>
                <InputGroup>
                  <InputText v-model="w.path" class="s-mono" :placeholder="$t('settings.autoImport.watch.sourcePlaceholder')" />
                  <Button severity="secondary" size="small" :label="$t('settings.autoImport.watch.choose')" @click="choosePath(w)"><template #icon><AppIcon name="folder-search" :size="15" /></template></Button>
                </InputGroup>
              </div>
            </div>

            <div class="ai-sub">
              <div class="ai-sub-head">
                <span class="ai-sub-title">{{ $t('settings.autoImport.optionsTitle') }} <Tag v-if="optsFor(w).length" severity="secondary" :value="String(optsFor(w).length)" /></span>
                <Button :label="$t('settings.autoImport.addOption')" severity="secondary" size="small" :disabled="!unusedOpts(w).length" @click="addOpt(w)"><template #icon><AppIcon name="plus" :size="14" /></template></Button>
              </div>
              <div v-if="!optsFor(w).length" class="s-desc">{{ $t('settings.autoImport.optionsDefault') }}</div>
              <div v-else class="opt-rows">
                <div v-for="(ok, idx) in optsFor(w)" :key="ok" class="opt-row">
                  <Select :modelValue="ok" :options="availableOpts(w, ok)" optionLabel="l" optionValue="v" class="opt-sel" @update:modelValue="(v) => changeOpt(w, idx, v)" />
                  <div class="grow opt-val">
                    <div v-if="ok === 'folder'" class="pickbox" tabindex="0" role="button" @click="chooseFolder($event, w)" @keydown.enter.space.prevent="chooseFolder($event, w)">
                      <AppChip v-if="w.folder" :label="folderById(w.folder)?.name || '?'" removable @click.stop @remove="w.folder = null" />
                      <span v-else class="faint">{{ $t('settings.autoImport.watch.chooseFolder') }}</span>
                    </div>
                    <div v-else-if="ok === 'tags'" class="pickbox" tabindex="0" role="button" @click="chooseTags($event, w)" @keydown.enter.space.prevent="chooseTags($event, w)">
                      <AppChip v-for="tag in w.tags" :key="tag" :label="tag" :style="tagTint(tag)" removable @click.stop @remove="w.tags.splice(w.tags.indexOf(tag), 1)" />
                      <span v-if="!w.tags?.length" class="faint">{{ $t('actions.dialog.chooseTags') }}</span>
                    </div>
                    <Select v-else-if="ok === 'subfolders'" v-model="w.subfolders" :options="SUB" :optionLabel="subName" :optionValue="(o) => o[0]" class="full" />
                    <Select v-else-if="ok === 'after'" v-model="w.after" :options="AFTER" :optionLabel="afterName" :optionValue="(o) => o[0]" class="full" />
                    <InputText v-else-if="ok === 'extensions'" v-model="w.extensions" :placeholder="$t('settings.autoImport.watch.allTypes')" class="full" />
                    <InputText v-else-if="ok === 'ignore'" v-model="w.ignore" :placeholder="IGNORE_EXAMPLE" class="full" />
                  </div>
                  <Button severity="secondary" size="small" class="step-btn" :aria-label="$t('settings.autoImport.removeOption')" @click="removeOpt(w, idx)"><template #icon><AppIcon name="minus" :size="15" /></template></Button>
                  <Button severity="secondary" size="small" class="step-btn" :disabled="!unusedOpts(w).length" :aria-label="$t('settings.autoImport.addOption')" @click="addOpt(w, idx)"><template #icon><AppIcon name="plus" :size="15" /></template></Button>
                </div>
              </div>
            </div>

            <div class="ai-sub">
              <div class="ai-sub-head">
                <span class="ai-sub-title">{{ $t('settings.autoImport.rules.title') }} <Tag v-if="w.rules.length" severity="secondary" :value="String(w.rules.length)" /></span>
                <Button :label="$t('settings.autoImport.rules.add')" severity="secondary" size="small" @click="openNewRule(w)"><template #icon><AppIcon name="plus" :size="14" /></template></Button>
              </div>
              <span v-if="w.rules.length" class="s-desc">{{ $t('settings.autoImport.rules.order') }}</span>
              <div v-if="w.rules.length" class="rule-list">
                <div v-for="(r, i) in w.rules" :key="i" class="rule-row" :class="{ off: !r.enabled }" @click="openEditRule(w, i)">
                  <div class="s-meta">
                    <span class="s-label ellipsis">{{ ruleTitle(r, i) }}</span>
                    <span class="s-desc ellipsis">{{ ruleSummary(r) }}</span>
                  </div>
                  <div class="s-actions" @click.stop>
                    <Button severity="secondary" text size="small" :disabled="i === 0" v-tooltip="$t('settings.autoImport.rule.up')" :aria-label="$t('settings.autoImport.rule.up')" @click="moveRule(w, i, -1)"><template #icon><AppIcon name="arrow-up" :size="14" /></template></Button>
                    <Button severity="secondary" text size="small" :disabled="i === w.rules.length - 1" v-tooltip="$t('settings.autoImport.rule.down')" :aria-label="$t('settings.autoImport.rule.down')" @click="moveRule(w, i, 1)"><template #icon><AppIcon name="arrow-down" :size="14" /></template></Button>
                    <Button severity="secondary" text size="small" v-tooltip="$t('common.edit')" :aria-label="$t('common.edit')" @click="openEditRule(w, i)"><template #icon><AppIcon name="pencil" :size="14" /></template></Button>
                    <Button severity="secondary" text size="small" v-tooltip="$t('settings.autoImport.rule.remove')" :aria-label="$t('settings.autoImport.rule.remove')" @click="w.rules.splice(i, 1)"><template #icon><AppIcon name="trash" :size="14" /></template></Button>
                    <ToggleSwitch v-model="r.enabled" :aria-label="ruleTitle(r, i)" />
                  </div>
                </div>
              </div>
            </div>

            <div class="s-actions watch-foot">
              <Button :label="$t('settings.autoImport.stopWatching')" severity="danger" text size="small" @click="removeWatch(w)"><template #icon><AppIcon name="trash" :size="15" /></template></Button>
            </div>
          </div>
        </div>
      </div>
    </section>

    <SettingsPageAutoImportRuleDialog :model="editingRule" @save="saveRule" @close="editingRule = null" />
  </template>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.s-exp-item.off > .s-exp-head .s-meta { opacity: 0.55; }
.watch-ico { color: var(--text-muted); flex: none; }
.watch-path { color: var(--text-faint); font-weight: 400; font-size: 0.84rem; }
.msg-row { display: flex; align-items: center; gap: calc(0.7rem * var(--g)); width: 100%; }
.ai-sub { display: flex; flex-direction: column; gap: calc(0.49rem * var(--g)); padding-top: calc(0.42rem * var(--g)); }
.ai-sub-head { display: flex; align-items: center; justify-content: space-between; gap: calc(0.56rem * var(--g)); }
.ai-sub-title { font-size: 0.88rem; font-weight: 500; display: inline-flex; align-items: center; gap: calc(0.42rem * var(--g)); }
.opt-rows { display: flex; flex-direction: column; gap: calc(0.42rem * var(--g)); }
.opt-row { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); }
.opt-sel { width: round(11.2rem, 1px); flex: none; }
.opt-val { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); min-width: 0; }
.full { width: 100%; }
.pickbox { flex: 1; min-width: 0; display: flex; flex-wrap: wrap; align-items: center; gap: calc(0.35rem * var(--g)); min-height: var(--control-h); padding: 0.14rem calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); background: var(--input); border: 1px solid var(--input-border); cursor: pointer; }
.pickbox:focus { border-color: var(--accent); outline: 0; }
.step-btn { width: var(--control-h); min-width: var(--control-h); height: var(--control-h); padding: 0; flex: none; }
.rule-list { display: flex; flex-direction: column; border-radius: calc(7px * var(--r)); background: var(--surface-2); overflow: hidden; }
.rule-row { display: flex; align-items: center; gap: calc(0.7rem * var(--g)); padding: calc(0.56rem * var(--g)) calc(0.77rem * var(--g)); cursor: pointer; }
.rule-row:hover { background: var(--hover); }
.rule-row.off .s-meta { opacity: 0.55; }
.watch-foot { justify-content: flex-end; padding-top: calc(0.28rem * var(--g)); }
</style>

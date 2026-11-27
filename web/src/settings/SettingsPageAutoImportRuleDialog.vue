<script setup>
import { computed, ref, watch } from 'vue'
import Dialog from 'openvue/dialog'
import Select from 'openvue/select'
import InputText from 'openvue/inputtext'
import Button from 'openvue/button'
import Checkbox from 'openvue/checkbox'
import { S } from '../app/state.js'
import { openPicker, tagTint } from '../tags/tags.js'
import AppIcon from '../shared/AppIcon.vue'
import AppChip from '../shared/AppChip.vue'
import { t } from '../shared/i18n.js'

const props = defineProps({ model: { type: Object, default: null } })
const emit = defineEmits(['save', 'close'])

const SIZE_EXAMPLE = '50KB, 2.5MB'
const FIELDS = [['name', 'text'], ['filename', 'text'], ['ext', 'text'], ['relpath', 'text'], ['fullpath', 'text'], ['size', 'num'], ['width', 'num'], ['height', 'num'], ['ageDays', 'num']]
const TEXT_OPS = [['contains'], ['notcontains'], ['is'], ['isnot'], ['startswith'], ['endswith'], ['regex'], ['notregex']]
const NUM_OPS = [['gt', '>'], ['gte', '≥'], ['lt', '<'], ['lte', '≤'], ['eq', '=']]
const STARS = [[0], [1, '★'], [2, '★★'], [3, '★★★'], [4, '★★★★'], [5, '★★★★★']]
const ACTION_KEYS = ['folder', 'tags', 'rating', 'note']

const MATCHING = computed(() => ['all', 'any'].map((v) => ({ v, l: t(`settings.autoImport.rule.matching.${v}`) })))
const fieldName = (o) => t(`settings.autoImport.fields.${o[0]}`)
const opName = (o) => o[1] ?? t(`settings.autoImport.ops.${o[0]}`)
const starsName = (o) => o[1] ?? t('settings.autoImport.rule.noRating')
const actionLabel = (k) => t(`settings.autoImport.rule.actions.${k}`)
const fieldType = (id) => FIELDS.find((f) => f[0] === id)?.[1] || 'text'
const opsFor = (id) => (fieldType(id) === 'text' ? TEXT_OPS : NUM_OPS)
const folderById = (id) => S.sidebar.folders.find((f) => f.id === id)

const draft = ref(null)
const actionKeys = ref([])

watch(() => props.model, (m) => {
  if (!m) {
    draft.value = null
    actionKeys.value = []
    return
  }
  const r = JSON.parse(JSON.stringify(m.rule))
  r.tags ||= []
  r.conditions = r.conditions?.length ? r.conditions : [{ field: 'name', op: 'contains', value: '' }]
  draft.value = r
  const keys = []
  if (r.folder != null) keys.push('folder')
  if (r.tags.length) keys.push('tags')
  if (r.rating > 0) keys.push('rating')
  if (r.note) keys.push('note')
  actionKeys.value = keys
}, { immediate: true })

function fieldChanged(c) {
  c.op = opsFor(c.field)[0][0]
}
function regexBad(c) {
  if (c.op !== 'regex' && c.op !== 'notregex') return false
  const m = c.value.match(/^\/(.*)\/([a-z]*)$/)
  try {
    new RegExp(m ? m[1] : c.value)
    return false
  } catch {
    return true
  }
}
const hasBadRegex = computed(() => !!draft.value?.conditions.some(regexBad))

function resetAction(k) {
  if (!draft.value) return
  if (k === 'folder') draft.value.folder = null
  else if (k === 'tags') draft.value.tags = []
  else if (k === 'rating') draft.value.rating = 0
  else if (k === 'note') draft.value.note = ''
}
const unusedActions = computed(() => ACTION_KEYS.filter((k) => !actionKeys.value.includes(k)))
const availableActions = (cur) => ACTION_KEYS.filter((k) => k === cur || !actionKeys.value.includes(k)).map((v) => ({ v, l: actionLabel(v) }))
function addAction(afterIdx = null) {
  const next = unusedActions.value[0]
  if (!next) return
  if (afterIdx == null) actionKeys.value.push(next)
  else actionKeys.value.splice(afterIdx + 1, 0, next)
}
function removeAction(idx) {
  const [k] = actionKeys.value.splice(idx, 1)
  resetAction(k)
}
function changeAction(idx, nextKey) {
  const prev = actionKeys.value[idx]
  if (prev === nextKey) return
  resetAction(prev)
  actionKeys.value[idx] = nextKey
}
function chooseFolder(e) {
  openPicker('folders', e, { ids: [], canCreate: true, onPick: (r) => (draft.value.folder = r.id) })
}
function chooseTags(e) {
  draft.value.tags ||= []
  openPicker('tags', e, {
    value: [...draft.value.tags],
    canCreate: true,
    onToggle: (name) => {
      const i = draft.value.tags.indexOf(name)
      if (i >= 0) draft.value.tags.splice(i, 1)
      else draft.value.tags.push(name)
    },
  })
}
function save() {
  if (!draft.value || hasBadRegex.value) return
  emit('save', JSON.parse(JSON.stringify(draft.value)))
}
</script>

<template>
  <Dialog :visible="!!model" modal :header="model?.index >= 0 ? $t('settings.autoImport.rule.editTitle') : $t('settings.autoImport.rule.createTitle')" :style="{ width: 'min(44rem, 94vw)' }" @update:visible="(v) => !v && emit('close')">
    <div v-if="draft" class="rd">
      <div class="s-field">
        <label class="s-field-label">{{ $t('settings.autoImport.rule.nameLabel') }}</label>
        <InputText v-model="draft.name" :placeholder="$t('settings.autoImport.rule.name')" class="full" />
      </div>

      <div class="rd-block">
        <div class="rd-block-head">
          <i18n-t keypath="settings.autoImport.rule.when" tag="div" class="rd-match"><template #choice><Select v-model="draft.matching" :options="MATCHING" optionLabel="l" optionValue="v" class="match-sel" /></template></i18n-t>
        </div>
        <div class="rd-rows">
          <div v-for="(c, k) in draft.conditions" :key="k" class="rd-row">
            <Select v-model="c.field" :options="FIELDS" :optionLabel="fieldName" :optionValue="(o) => o[0]" class="col-field" @change="fieldChanged(c)" />
            <Select v-model="c.op" :options="opsFor(c.field)" :optionLabel="opName" :optionValue="(o) => o[0]" class="col-op" />
            <InputText v-model="c.value" class="grow" :invalid="regexBad(c)" :placeholder="c.field === 'size' ? SIZE_EXAMPLE : ''" />
            <Button severity="secondary" size="small" class="step-btn" :disabled="draft.conditions.length < 2" :aria-label="$t('settings.autoImport.rule.removeCondition')" @click="draft.conditions.splice(k, 1)"><template #icon><AppIcon name="minus" :size="15" /></template></Button>
            <Button severity="secondary" size="small" class="step-btn" :aria-label="$t('settings.autoImport.rule.addCondition')" @click="draft.conditions.splice(k + 1, 0, { field: c.field, op: c.op, value: '' })"><template #icon><AppIcon name="plus" :size="15" /></template></Button>
          </div>
        </div>
      </div>

      <div class="rd-block">
        <div class="rd-block-head">
          <span class="rd-head-title">{{ $t('settings.autoImport.rule.then') }}</span>
          <label class="opt"><Checkbox v-model="draft.skip" binary /> {{ $t('settings.autoImport.rule.skip') }}</label>
        </div>
        <div v-if="!draft.skip" class="rd-rows">
          <div v-for="(ak, idx) in actionKeys" :key="ak" class="rd-row">
            <Select :modelValue="ak" :options="availableActions(ak)" optionLabel="l" optionValue="v" class="col-action" @update:modelValue="(v) => changeAction(idx, v)" />
            <div class="grow act-val">
              <div v-if="ak === 'folder'" class="pickbox" tabindex="0" role="button" @click="chooseFolder($event)" @keydown.enter.space.prevent="chooseFolder($event)">
                <AppChip v-if="draft.folder" :label="folderById(draft.folder)?.name || '?'" removable @click.stop @remove="draft.folder = null" />
                <span v-else class="faint">{{ $t('settings.autoImport.rule.unchanged') }}</span>
              </div>
              <div v-else-if="ak === 'tags'" class="pickbox" tabindex="0" role="button" @click="chooseTags($event)" @keydown.enter.space.prevent="chooseTags($event)">
                <AppChip v-for="tag in draft.tags" :key="tag" :label="tag" :style="tagTint(tag)" removable @click.stop @remove="draft.tags.splice(draft.tags.indexOf(tag), 1)" />
                <span v-if="!draft.tags?.length" class="faint">{{ $t('actions.dialog.chooseTags') }}</span>
              </div>
              <Select v-else-if="ak === 'rating'" v-model="draft.rating" :options="STARS" :optionLabel="starsName" :optionValue="(o) => o[0]" class="full" />
              <InputText v-else-if="ak === 'note'" v-model="draft.note" :placeholder="$t('settings.autoImport.rule.note')" class="full" />
            </div>
            <Button severity="secondary" size="small" class="step-btn" :aria-label="$t('settings.autoImport.rule.removeAction')" @click="removeAction(idx)"><template #icon><AppIcon name="minus" :size="15" /></template></Button>
            <Button severity="secondary" size="small" class="step-btn" :disabled="!unusedActions.length" :aria-label="$t('settings.autoImport.rule.addAction')" @click="addAction(idx)"><template #icon><AppIcon name="plus" :size="15" /></template></Button>
          </div>
          <div v-if="!actionKeys.length" class="rd-empty">
            <span class="s-desc">{{ $t('settings.autoImport.rule.noActions') }}</span>
            <Button :label="$t('settings.autoImport.rule.addAction')" severity="secondary" size="small" @click="addAction()"><template #icon><AppIcon name="plus" :size="15" /></template></Button>
          </div>
        </div>
      </div>

      <label class="opt"><Checkbox v-model="draft.stop" binary /> {{ $t('settings.autoImport.rule.stop') }}</label>
    </div>

    <template #footer>
      <Button :label="$t('settings.autoImport.rule.save')" :disabled="hasBadRegex" @click="save" />
      <Button :label="$t('common.cancel')" severity="secondary" @click="emit('close')" />
    </template>
  </Dialog>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.rd { display: flex; flex-direction: column; gap: calc(0.91rem * var(--g)); }
.full { width: 100%; }
.rd-block { background: var(--surface-2); border-radius: calc(8px * var(--r)); overflow: hidden; }
.rd-block-head { display: flex; align-items: center; justify-content: space-between; gap: calc(0.56rem * var(--g)); padding: calc(0.56rem * var(--g)) calc(0.84rem * var(--g)); background: var(--surface-3); }
.rd-head-title { font-weight: 500; font-size: 0.92rem; }
.rd-match { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); font-size: 0.92rem; }
.match-sel { width: round(5.2rem, 1px); }
.rd-rows { display: flex; flex-direction: column; gap: calc(0.49rem * var(--g)); padding: calc(0.77rem * var(--g)) calc(0.84rem * var(--g)); }
.rd-row { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); }
.col-field { width: round(10.5rem, 1px); flex: none; }
.col-op { width: round(8.8rem, 1px); flex: none; }
.col-action { width: round(10.5rem, 1px); flex: none; }
.act-val { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); min-width: 0; }
.pickbox { flex: 1; min-width: 0; display: flex; flex-wrap: wrap; align-items: center; gap: calc(0.35rem * var(--g)); min-height: var(--control-h); padding: 0.14rem calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); background: var(--input); border: 1px solid var(--input-border); cursor: pointer; }
.pickbox:focus { border-color: var(--accent); outline: 0; }
.step-btn { width: var(--control-h); min-width: var(--control-h); height: var(--control-h); padding: 0; flex: none; }
.rd-empty { display: flex; align-items: center; justify-content: space-between; gap: calc(0.56rem * var(--g)); }
</style>

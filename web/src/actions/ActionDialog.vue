<script setup>
import { computed, ref, watch } from 'vue'
import { language, t } from '../shared/i18n.js'
import InputText from 'openvue/inputtext'
import Select from 'openvue/select'
import Dialog from 'openvue/dialog'
import Button from 'openvue/button'
import InputGroup from 'openvue/inputgroup'
import InputGroupAddon from 'openvue/inputgroupaddon'
import Message from 'openvue/message'
import { S } from '../app/state.js'
import { openPicker, tagTint } from '../tags/tags.js'
import { saveUi } from '../app/preferences.js'
import { STEP_TYPES, newStep, saveActions, stepValue } from './actions.js'
import { keyHolders, pluginActions } from '../plugins/pluginActions.js'
import { comboOf } from '../app/shortcuts.js'
import { keyText } from '../shared/keys.js'
import { beginDrag, finalIndex } from '../app/dragDrop.js'
import AppIcon from '../shared/AppIcon.vue'
import AppChip from '../shared/AppChip.vue'

// Create or edit an Action: a name, a shortcut and its steps.
const form = ref(null)
const open = computed(() => S.dialogs.action)
watch(open, (a) => {
  form.value = a ? JSON.parse(JSON.stringify(a.id ? a : { id: null, name: '', shortcut: '', steps: [newStep()] })) : null
})
const canSave = computed(() => form.value?.name.trim() && form.value.steps.length)
// what else has the key typed: saving takes it from them, as Settings › Shortcuts does
const holders = computed(() => (form.value?.shortcut ? keyHolders(form.value.shortcut, form.value.id ? `action:${form.value.id}` : null) : []))
const holderNames = computed(() => {
  return new Intl.ListFormat(language(), { type: 'conjunction' }).format(holders.value.map((h) => h.label))
})
function close() {
  S.dialogs.action = null
  S.recordingKey = false
}
function save() {
  if (!canSave.value) return
  const f = form.value
  f.name = f.name.trim()
  for (const h of holders.value) {
    if (h.id.startsWith('action:')) S.actions.find((a) => `action:${a.id}` === h.id).shortcut = ''
    else S.ui.keys[h.id] = ''
  }
  if (holders.value.length) saveUi()
  if (f.id) S.actions.splice(S.actions.findIndex((x) => x.id === f.id), 1, f)
  else S.actions.push({ ...f, id: 'a' + Date.now().toString(36) })
  saveActions()
  close()
}
// shortcut: press the keys in the box
function recordKey(e) {
  if (e.key === 'Tab') return
  e.preventDefault()
  e.stopPropagation()
  if (e.key === 'Backspace' || e.key === 'Delete') {
    form.value.shortcut = ''
    return
  }
  const c = comboOf(e)
  if (c) form.value.shortcut = c
}
function pickTags(e, step) {
  openPicker('tags', e, {
    value: [...step.tags],
    canCreate: true,
    onToggle: (name) => {
      const i = step.tags.indexOf(name)
      if (i >= 0) step.tags.splice(i, 1)
      else step.tags.push(name)
    },
  })
}
function pickFolders(e, step) {
  const names = () => step.folders.map((id) => S.sidebar.folders.find((f) => f.id === id)?.name).filter(Boolean)
  openPicker('folders', e, {
    value: names(),
    onToggle: (name) => {
      const f = S.sidebar.folders.find((x) => x.name === name)
      if (!f) return
      const i = step.folders.indexOf(f.id)
      if (i >= 0) step.folders.splice(i, 1)
      else step.folders.push(f.id)
    },
  })
}
const folderName = (id) => S.sidebar.folders.find((f) => f.id === id)?.name || '?'
function moveStep(i, d) {
  const j = i + d
  if (j < 0 || j >= form.value.steps.length) return
  ;[form.value.steps[i], form.value.steps[j]] = [form.value.steps[j], form.value.steps[i]]
}
function dragStep(e, i) {
  beginDrag(e, {
    drop: (t, zone) => {
      const [x] = form.value.steps.splice(i, 1)
      form.value.steps.splice(finalIndex(i, t, zone), 0, x)
    },
  }, stepTypes.value.find((s) => s[0] === form.value.steps[i].type)?.[1] || '', { sort: true, lift: e.currentTarget.closest('.step'), accept: (t) => t.startsWith('astep:') })
}
const fields = computed(() => S.sidebar.fields.filter((f) => f.owner === 'user' || f.visible))
// plugins' programs offered as steps, after Curio's own
const stepTypes = computed(() => [...STEP_TYPES.map(([id, value]) => [id, t(`actions.steps.${id}`), value]), ...pluginActions('step').map((a) => [`plugin:${a.plugin}::${a.id}`, a.title, null])])
</script>

<template>
  <Dialog :visible="!!form" modal dismissableMask :header="form?.id ? $t('actions.dialog.editTitle') : $t('actions.dialog.createTitle')" :style="{ width: 'min(51rem, 94vw)' }" class="action-dialog" @update:visible="(v) => !v && close()">
    <div v-if="form" class="ad-body">
      <div class="head-row">
        <div class="ad-field grow">
          <label class="lbl">{{ $t('actions.dialog.name') }} <i>*</i></label>
          <InputGroup>
            <InputGroupAddon><button class="addon-btn" type="button" :aria-label="$t('actions.dialog.icon')" v-tooltip="$t('actions.dialog.icon')" @click="S.dialogs.icon = { kind: 'action', form, current: form.icon }"><AppIcon :name="form.icon || 'bolt'" :size="18" /></button></InputGroupAddon>
            <InputText v-model="form.name" v-focus :placeholder="$t('actions.dialog.namePlaceholder')" spellcheck="false" />
          </InputGroup>
        </div>
        <div class="ad-field keys">
          <label class="lbl">{{ $t('actions.dialog.shortcut') }}</label>
          <InputGroup>
            <InputText class="s-mono" :modelValue="keyText(form.shortcut, '+')" :placeholder="$t('actions.dialog.pressKeys')" readonly @keydown="recordKey" @focus="S.recordingKey = true" @blur="S.recordingKey = false" />
            <Button v-if="form.shortcut" severity="secondary" size="small" :aria-label="$t('settings.shortcuts.remove')" @click="form.shortcut = ''"><template #icon><AppIcon name="x" :size="14" /></template></Button>
          </InputGroup>
        </div>
      </div>

      <Message v-if="holders.length" severity="warn" :closable="false">{{ $t('actions.dialog.keyTaken', { key: keyText(form.shortcut, '+'), names: holderNames }) }}</Message>

      <div class="steps-card">
        <div class="steps-head">
          <span class="lbl">{{ $t('actions.dialog.steps') }} <i>*</i></span>
          <span class="grow" />
          <Button :label="$t('actions.dialog.addStep')" severity="secondary" size="small" @click="form.steps.push(newStep())"><template #icon><AppIcon name="plus" :size="15" /></template></Button>
        </div>
        <div class="steps-list">
          <div v-for="(s, i) in form.steps" :key="i" class="step" :data-drop="'astep:' + i">
            <span class="grip" @pointerdown="dragStep($event, i)"><AppIcon name="grip-vertical" :size="16" class="faint" /></span>
            <Select v-model="s.type" :options="stepTypes" :optionLabel="(o) => o[1]" :optionValue="(o) => o[0]" class="step-type" />
            <div v-if="stepValue(s.type) === 'tags'" class="pickbox" tabindex="0" role="button" @click="pickTags($event, s)" @keydown.enter.space.prevent="pickTags($event, s)">
              <AppChip v-for="tag in s.tags" :key="tag" :label="tag" :style="tagTint(tag)" removable @click.stop @remove="s.tags.splice(s.tags.indexOf(tag), 1)" />
              <span v-if="!s.tags.length" class="faint">{{ $t('actions.dialog.chooseTags') }}</span>
            </div>
            <div v-else-if="stepValue(s.type) === 'folders'" class="pickbox" tabindex="0" role="button" @click="pickFolders($event, s)" @keydown.enter.space.prevent="pickFolders($event, s)">
              <AppChip v-for="f in s.folders" :key="f" :label="folderName(f)" removable @click.stop @remove="s.folders.splice(s.folders.indexOf(f), 1)" />
              <span v-if="!s.folders.length" class="faint">{{ $t('actions.dialog.chooseFolders') }}</span>
            </div>
            <Select v-else-if="stepValue(s.type) === 'rating'" v-model="s.rating" :options="[5, 4, 3, 2, 1, 0]" :optionLabel="(n) => (n ? '★'.repeat(n) : $t('actions.dialog.noRating'))" class="grow" />
            <InputText v-else-if="stepValue(s.type) === 'text'" v-model="s.text" class="grow" spellcheck="false" />
            <template v-else-if="stepValue(s.type) === 'field'">
              <Select v-model="s.field" :options="fields" optionLabel="label" optionValue="key" :placeholder="$t('actions.dialog.field')" filter autoFilterFocus class="step-field" />
              <InputText v-model="s.value" class="grow" :placeholder="$t('actions.dialog.value')" spellcheck="false" />
            </template>
            <span v-else class="grow" />
            <Button severity="secondary" text size="small" class="step-btn" :disabled="i === 0" :aria-label="$t('settings.autoImport.rule.up')" @click="moveStep(i, -1)"><template #icon><AppIcon name="arrow-up" :size="14" /></template></Button>
            <Button severity="secondary" text size="small" class="step-btn" :disabled="i === form.steps.length - 1" :aria-label="$t('settings.autoImport.rule.down')" @click="moveStep(i, 1)"><template #icon><AppIcon name="arrow-down" :size="14" /></template></Button>
            <Button severity="secondary" size="small" class="step-btn" :disabled="form.steps.length < 2" :aria-label="$t('actions.dialog.removeStep')" @click="form.steps.splice(i, 1)"><template #icon><AppIcon name="minus" :size="15" /></template></Button>
            <Button severity="secondary" size="small" class="step-btn" :aria-label="$t('actions.dialog.addBelow')" @click="form.steps.splice(i + 1, 0, newStep())"><template #icon><AppIcon name="plus" :size="15" /></template></Button>
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <Button :label="form?.id ? $t('common.saveChanges') : $t('actions.dialog.createTitle')" :disabled="!canSave" @click="save" />
      <Button :label="$t('common.cancel')" severity="secondary" @click="close" />
    </template>
  </Dialog>
</template>

<style scoped>
/* laid out as the smart folder rules are: one row per step, its kind, then what it takes */
.ad-body { display: flex; flex-direction: column; gap: calc(0.84rem * var(--g)); }
.head-row { display: flex; gap: calc(0.84rem * var(--g)); align-items: flex-end; }
.ad-field { display: flex; flex-direction: column; gap: calc(0.28rem * var(--g)); min-width: 0; }
.keys { width: round(14.2rem, 1px); flex: none; }
.s-mono { font-family: var(--mono-font); font-size: 0.85rem; }
.lbl { font-size: 0.82rem; font-weight: 500; color: var(--text-muted); }
.lbl i { font-style: normal; color: var(--text-faint); }
.steps-card { background: var(--surface-2); border-radius: calc(8px * var(--r)); overflow: hidden; }
.steps-head { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); padding: calc(0.49rem * var(--g)) calc(0.77rem * var(--g)); background: var(--surface-3); }
.steps-head .lbl { color: var(--text); font-size: 0.92rem; }
.steps-list { display: flex; flex-direction: column; gap: calc(0.49rem * var(--g)); padding: calc(0.77rem * var(--g)); }
.step { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); border-radius: calc(6px * var(--r)); }
.step.drop-hover { outline: 1px solid var(--accent); outline-offset: 2px; }
.step > .grow { min-width: 0; }
.grip { display: inline-flex; flex: none; cursor: grab; }
.step-type { width: round(11.4rem, 1px); flex: none; }
.step-field { width: round(10.2rem, 1px); flex: none; }
.pickbox { flex: 1; min-width: 0; display: flex; flex-wrap: wrap; align-items: center; gap: calc(0.35rem * var(--g)); min-height: var(--control-h); padding: 0.14rem calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); background: var(--input); border: 1px solid var(--input-border); cursor: pointer; }
.pickbox:focus { border-color: var(--accent); outline: 0; }
.step-btn { width: var(--control-h); min-width: var(--control-h); height: var(--control-h); padding: 0; flex: none; }
</style>

<script setup>
import { computed, ref, watch } from 'vue'
import Select from 'openvue/select'
import ToggleSwitch from 'openvue/toggleswitch'
import InputText from 'openvue/inputtext'
import Textarea from 'openvue/textarea'
import { S } from '../app/state.js'
import { edit } from './edits.js'
import { toast } from '../app/notify.js'
import { filterByField } from './navigation.js'
import { indexedFields } from '../filters/query.js'
import AppIcon from '../shared/AppIcon.vue'
import AppChip from '../shared/AppChip.vue'
import { t } from '../shared/i18n.js'

// Type-aware editor for one custom field (or, with `batch`, a picker that sets any field
// on many items). Values are validated by the core (types, options, min/max, patterns).
// `readonly`: values a plugin reads from the file, shown as text; a list of values shows a row each, `suffixes` adding something in brackets to each (a LoRA's strength).
const props = defineProps({ field: Object, values: { type: Array, default: () => [] }, ids: Array, batch: Boolean, readonly: Boolean, suffixes: Array })
const chosenKey = ref(null)
const f = computed(() => (props.batch ? S.sidebar.fields.find((x) => x.key === chosenKey.value) : props.field))
const KNOWN_TYPES = new Set(['text', 'integer', 'float', 'boolean', 'date', 'datetime', 'single_select', 'multi_select', 'json', 'url', 'file_ref'])
const unknownType = computed(() => f.value && !KNOWN_TYPES.has(f.value.type))
// a field readers fill is offered only when every selected file is of a kind they read
const batchFields = computed(() => {
  if (!props.batch || S.itemsVer < 0) return []
  const exts = new Set((props.ids || []).map((id) => S.items.get(id)?.ext).filter(Boolean).map((x) => String(x).toLowerCase()))
  return indexedFields().filter((x) => KNOWN_TYPES.has(x.type) && (!x.exts || [...exts].every((e) => x.exts.includes(e))))
})
watch(batchFields, (list) => {
  if (chosenKey.value && !list.some((x) => x.key === chosenKey.value)) chosenKey.value = null
})

function display(field, vals) {
  if (!field) return ''
  return vals
    .map((v) => {
      switch (field.type) {
        case 'boolean': return v.num ? 'true' : 'false'
        case 'integer': return v.num != null ? String(Math.round(v.num)) : ''
        case 'float': return v.num != null ? String(+v.num.toFixed(6)) : ''
        case 'date': case 'datetime': return v.text || (v.num ? new Date(v.num).toISOString().slice(0, 10) : '')
        default: return v.text ?? (v.num != null ? String(v.num) : '')
      }
    })
    .join(', ')
}
const current = computed(() => display(f.value, props.batch ? [] : props.values))
// a read-only box starts as tall as its text, up to four lines (about this many characters to a line in a sidebar)
const RO_LINE_CHARS = 40
const roRows = computed(() => Math.min(4, Math.max(1, String(current.value || '').split('\n').reduce((n, l) => n + Math.max(1, Math.ceil(l.length / RO_LINE_CHARS)), 0))))
const text = ref('')
watch(current, (c) => (text.value = c), { immediate: true })
const block = computed(() => f.value && (f.value.widget === 'block' || f.value.widget === 'code' || f.value.type === 'json' || (!(props.readonly && (props.values.length > 1 || props.suffixes?.length)) && current.value.length > 48)))
const multi = computed(() => f.value?.type === 'multi_select')
const chips = computed(() => (props.batch ? [] : props.values.map((v) => v.text).filter(Boolean)))
const newChip = ref('')

function commit(value) {
  if (!f.value) return
  edit.field(props.ids, f.value.key, value === '' ? null : value)
}
function save() {
  if (text.value === current.value) return
  if (['integer', 'float'].includes(f.value.type) && text.value.includes(',')) commit(text.value.split(',').map((x) => x.trim()))
  else commit(text.value)
}
function addChip() {
  const parts = newChip.value.split(',').map((x) => x.trim()).filter(Boolean)
  newChip.value = ''
  if (parts.length) commit([...chips.value, ...parts])
}
// the selected words when there are some, else the start of the text
function find(e) {
  const box = e.currentTarget.closest('.fe')?.querySelector('textarea')
  const picked = box ? box.value.slice(box.selectionStart, box.selectionEnd).trim() : ''
  filterByField(f.value.key, picked || current.value.slice(0, 60).trim(), e.ctrlKey || e.metaKey)
}
const copy = (s = current.value) => {
  navigator.clipboard?.writeText(s)
  toast(t('inspector.copied'), 'info', 750)
}
// one row per value, each found by itself
const roRows2 = computed(() => props.values.map((v, i) => {
  const shown = display(f.value, [v])
  const value = ['integer', 'float', 'boolean'].includes(f.value?.type) ? v.num : v.text
  const s = props.suffixes?.[i]
  return { value, label: s ? shown + ' (' + s + ')' : shown, copy: shown }
}).filter((r) => r.label))
const rowLabel = computed(() => f.value?.label)
// a value set by hand on a field readers fill; clearing it takes the file's value again
// switched off: read from the file just now, so it can't be filtered by or edited
const off = computed(() => f.value?.indexed === false)
const byHand = computed(() => !props.batch && props.ids?.length === 1 && f.value?.readers?.length && S.detail?.asset?.id === props.ids[0] && (S.detail.overridden || []).includes(f.value.id))
const hint = computed(() => ({ date: 'YYYY-MM-DD', datetime: 'YYYY-MM-DD HH:MM', integer: t('inspector.field.number'), float: t('inspector.field.number'), url: 'https://example.com', file_ref: t('inspector.field.path') })[f.value?.type] || '-')
// for whoever writes queries and plugins: the field's key, type and owner
const fieldTip = (x) => [x.key, x.type, x.owner].join(' · ') + (x.description ? '\n' + x.description : '')
</script>

<template>
  <div class="fe" :class="{ 'ro-long': readonly && block }">
    <Select v-if="batch" v-model="chosenKey" :options="batchFields" optionLabel="label" optionValue="key" filter autoFilterFocus :placeholder="$t('inspector.field.choose')" fluid />
    <!-- read-only: a long value in a box with find and copy, a short one as a row like Properties -->
    <template v-if="f && (readonly || off || unknownType)">
      <template v-if="block">
        <div class="lab row block ro">
          <span class="name ellipsis">{{ f.label }}</span>
          <button v-if="byHand" class="icon-btn mini" v-tooltip="$t('inspector.field.restoreTip')" :aria-label="$t('inspector.field.restore')" @click="commit(null)"><AppIcon name="pencil-check" :size="13" /></button>
          <button v-if="!off" class="icon-btn mini" v-tooltip="$t('inspector.field.find')" @click="find"><AppIcon name="search" :size="13" /></button>
          <button class="icon-btn mini" v-tooltip="$t('inspector.field.copy')" @click="copy()"><AppIcon name="copy" :size="13" /></button>
        </div>
        <textarea class="ro-block" :class="{ mono: f.type === 'json' || f.widget === 'code' }" :value="current" :rows="roRows" readonly spellcheck="false" />
      </template>
      <template v-else><div v-for="(r, i) in roRows2" :key="i" class="ro-row" :class="{ off }" v-tooltip="off ? { value: $t('inspector.field.notIndexed'), showDelay: 800 } : undefined" @click="!off && filterByField(f.key, r.value, $event.ctrlKey || $event.metaKey)" @contextmenu.prevent="copy(r.copy)">
        <span class="ro-name">{{ rowLabel }}</span><span class="ro-val" :title="r.label">{{ r.label }}</span>
        <button v-if="byHand && i === 0" class="icon-btn mini" v-tooltip="$t('inspector.field.restoreTip')" :aria-label="$t('inspector.field.restore')" @click.stop="commit(null)"><AppIcon name="pencil-check" :size="13" /></button>
      </div></template>
    </template>
    <template v-else-if="f">
      <div class="lab row" :class="{ block }">
        <span class="name ellipsis" v-tooltip.bottom="{ value: fieldTip(f), showDelay: 500 }">{{ f.label }}</span>
        <button v-if="byHand" class="icon-btn mini" v-tooltip="$t('inspector.field.restoreTip')" :aria-label="$t('inspector.field.restore')" @click="commit(null)"><AppIcon name="pencil-check" :size="13" /></button>
        <template v-if="block && !batch && current">
          <button class="icon-btn mini" v-tooltip="$t('inspector.field.find')" @click="find"><AppIcon name="search" :size="13" /></button>
          <button class="icon-btn mini" v-tooltip="$t('inspector.field.copy')" @click="copy()"><AppIcon name="copy" :size="13" /></button>
        </template>
        <template v-if="!block">
          <ToggleSwitch v-if="f.type === 'boolean'" :modelValue="!!values[0]?.num" @update:modelValue="(b) => commit(b)" />
          <Select v-else-if="f.type === 'single_select' && f.options.length" :modelValue="current || null" :options="f.options" showClear class="val" @update:modelValue="(v) => commit(v || null)" />
          <InputText v-else-if="!multi" v-model="text" class="val field" :placeholder="hint" @blur="save" @keydown.enter="$event.target.blur()" />
        </template>
      </div>
      <Textarea v-if="block" v-model="text" class="field" :class="{ mono: f.type === 'json' || f.widget === 'code' }" :spellcheck="f.type !== 'json' && f.widget !== 'code'" :rows="f.type === 'json' ? 5 : 3" @blur="save" />
      <div v-if="multi" class="chips">
        <AppChip v-for="(c, i) in chips" :key="i + ':' + c" :label="c" removable @remove="commit(chips.filter((_, j) => j !== i))" />
        <InputText v-model="newChip" class="field small" :placeholder="$t('inspector.field.add')" @keydown.enter="addChip" @blur="addChip" />
      </div>
      <button v-if="batch" class="clear" @click="commit(null)">{{ $t('inspector.field.clearAll') }}</button>
    </template>
  </div>
</template>

<style scoped>
.fe { display: flex; flex-direction: column; gap: calc(0.28rem * var(--g)); }
.fe.ro-long { margin-bottom: calc(0.56rem * var(--g)); }
.lab { min-height: round(2.1rem, 1px); gap: calc(0.42rem * var(--g)); }
.lab .name { flex: 0 0 6.71rem; color: var(--text-faint); font-size: 0.92rem; }
.lab.block .name { flex: 1; }
.ro-row { display: flex; justify-content: space-between; align-items: center; gap: calc(1.12rem * var(--g)); min-height: round(2.1rem * var(--row), 1px); margin: 0 calc(-0.42rem * var(--g)); padding: 0 calc(0.42rem * var(--g)); border-radius: calc(6px * var(--r)); line-height: 1.3; font-size: 1rem; cursor: pointer; }
/* read-only values look as Properties' rows do: the name in the text's colour, the value muted in the fixed-width font */
.ro-name { flex: none; color: var(--text); white-space: nowrap; }
.ro-val { min-width: 0; text-align: right; color: var(--text-muted); font-family: var(--mono-font); font-size: 0.92rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.lab.ro .name { color: var(--text); font-size: 1rem; }
.ro-row:not(.off):hover { background: var(--hover); }
.ro-row.off { cursor: default; }
/* dragging its corner shows more */
.ro-block { display: block; width: 100%; box-sizing: border-box; resize: vertical; background: var(--input); border: 1px solid var(--input-border); border-radius: calc(6px * var(--r)); padding: calc(0.49rem * var(--g)) calc(0.7rem * var(--g)); color: var(--text); font: inherit; font-size: 0.92rem; line-height: 1.4; white-space: pre-wrap; word-break: break-word; overflow-y: auto; outline: none; cursor: text; }
.ro-block:focus { border-color: var(--accent); }
.ro-block.mono { font-family: var(--mono-font); font-size: 0.85rem; }
.val { flex: 1; min-width: 0; padding: calc(0.28rem * var(--g)) calc(0.56rem * var(--g)); }
.icon-btn.mini { min-width: round(1.54rem, 1px); height: round(1.54rem, 1px); }
textarea.field { font-size: 0.92rem; }
textarea.mono { font-family: var(--mono-font); font-size: 0.85rem; }
.chips { display: flex; flex-wrap: wrap; gap: calc(0.28rem * var(--g)); align-items: center; }
.field.small { width: round(6.29rem, 1px); padding: calc(0.21rem * var(--g)) calc(0.42rem * var(--g)); }
.clear { align-self: flex-start; border: 0; background: transparent; color: var(--text-faint); font: inherit; font-size: 0.85rem; cursor: pointer; padding: 0; }
.clear:hover { color: var(--danger); }
</style>

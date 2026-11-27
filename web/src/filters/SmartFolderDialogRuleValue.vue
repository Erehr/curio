<script setup>
import { computed, ref, watch } from 'vue'
import Select from 'openvue/select'
import InputText from 'openvue/inputtext'
import Popover from 'openvue/popover'
import InputNumber from 'openvue/inputnumber'
import DatePicker from 'openvue/datepicker'
import { S } from '../app/state.js'
import { call } from '../platform/ipc.js'
import { SHAPES, KINDS, SIZE_UNITS, DURATION_UNITS, AGE_UNITS, ruleKey, isListRule } from './smartRules.js'
import { openPicker, tagTint } from '../tags/tags.js'
import AppChip from '../shared/AppChip.vue'
import { LOOK_COLORS, colorName } from '../sidebar/sidebar.js'
import ColorPicker from 'openvue/colorpicker'

// The value of one smart folder rule, in the control that suits its type.
// `paired`: the field holding one value for each of this one's (a LoRA's strength), whose range goes beside the value
const props = defineProps({ rule: Object, type: String, options: { type: Array, default: () => [] }, paired: { type: Object, default: null } })
const pop = ref()
const STARS = [5, 4, 3, 2, 1, 0]
const r = computed(() => props.rule)
const FOLDER_COLORS = LOOK_COLORS.filter(([, c]) => c)
const typed = computed(() => String(r.value.op).includes('CONTAINS'))
// a list field without fixed options, as LoRA or Model, offers the values the library has, and the one saved
const ALL_ITEMS = { scope: 'all', scope_id: 0, scope_ids: [], scope_path: '', history: '', text: '', sort_key: 'imported', sort_field: '', descending: false, subfolders: false }
const inUse = ref([])
watch(() => (props.type === 'select' && !props.options.length ? ruleKey(r.value) : null), async (key) => {
  inUse.value = []
  if (!key) return
  const res = await call('field_facets', { req: ALL_ITEMS, key }).catch(() => null)
  inUse.value = (res?.values || []).map(([v]) => String(v))
}, { immediate: true })
const choices = computed(() => {
  const list = props.options.length ? props.options : inUse.value
  const v = r.value.value
  return v !== '' && v != null && !list.includes(v) ? [v, ...list] : list
})
const hidden = computed(() => r.value.op.startsWith('IS '))
const units = computed(() => (props.type === 'size' ? SIZE_UNITS : props.type === 'duration' ? DURATION_UNITS : r.value.op === 'WITHIN' ? AGE_UNITS : null))
const listed = computed(() => (props.type === 'tags' || props.type === 'folders') && isListRule(r.value))
// the window's own Tags or Folders picker, ticking the names this row holds
function pick(e) {
  const names = r.value.value
  openPicker(props.type, e, { value: [...names], onToggle: (name) => (names.includes(name) ? names.splice(names.indexOf(name), 1) : names.push(name)) })
}
const dateInput = computed(() => props.type === 'date' && r.value.op !== 'WITHIN')
const isHex = computed(() => /^#[0-9a-f]{6}$/i.test(String(r.value.value)))
const toNum = (v) => (v === '' || v == null || !Number.isFinite(Number(v)) ? null : Number(v))
const toDate = (v) => (/^\d{4}-\d{2}-\d{2}$/.test(String(v)) ? new Date(`${v}T00:00:00`) : null)
const pad = (n) => String(n).padStart(2, '0')
const fromDate = (d) => (d instanceof Date ? `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` : '')
</script>

<template>
  <div v-if="!hidden" class="rv">
    <Select v-if="type === 'rating'" v-model="r.value" :options="STARS" :optionLabel="(n) => (n ? '★'.repeat(n) : $t('filters.value.none'))" class="grow" />
    <Select v-else-if="type === 'shape'" v-model="r.value" :options="SHAPES" :optionLabel="(o) => $t(`filters.shapes.${o}`)" class="grow" />
    <Select v-else-if="type === 'kind'" v-model="r.value" :options="KINDS" :optionLabel="(o) => $t(`filters.kinds.${o}`)" class="grow" />
    <Select v-else-if="type === 'bool'" v-model="r.value" :options="['true', 'false']" :optionLabel="(o) => (o === 'true' ? $t('filters.value.yes') : $t('filters.value.no'))" class="grow" />
    <Select v-else-if="type === 'yesno'" v-model="r.value" :options="['1', '0']" :optionLabel="(o) => (o === '1' ? $t('filters.value.yes') : $t('filters.value.no'))" class="grow" />
    <Select v-else-if="type === 'foldercolor'" v-model="r.value" :options="FOLDER_COLORS" :optionLabel="(o) => colorName(o[0])" :optionValue="(o) => o[1]" class="grow">
      <template #option="{ option }"><span class="color-option"><span class="swatch" :style="{ background: option[1] }" />{{ option[0] }}</span></template>
    </Select>
    <span v-else-if="type === 'query'" class="faint grow">{{ $t('filters.value.kept') }}</span>
    <template v-else-if="type === 'select' && !typed">
      <Select v-model="r.value" :options="choices" :filter="choices.length > 8" autoFilterFocus class="grow" />
      <template v-if="paired">
        <InputNumber :modelValue="toNum(r.pmin)" :useGrouping="false" :maxFractionDigits="6" :placeholder="$t('filters.value.min')" class="rv-pair" :aria-label="$t('filters.value.from', { name: paired.label })" v-tooltip.bottom="$t('filters.value.from', { name: paired.label })" @update:modelValue="(v) => (r.pmin = v ?? '')" />
        <span class="faint">-</span>
        <InputNumber :modelValue="toNum(r.pmax)" :useGrouping="false" :maxFractionDigits="6" :placeholder="$t('filters.value.max')" class="rv-pair" :aria-label="$t('filters.value.to', { name: paired.label })" v-tooltip.bottom="$t('filters.value.to', { name: paired.label })" @update:modelValue="(v) => (r.pmax = v ?? '')" />
      </template>
    </template>
    <div v-else-if="listed" class="pickbox" tabindex="0" role="button" @click="pick" @keydown.enter.space.prevent="pick">
      <AppChip v-for="n in r.value" :key="n" :label="n" :style="type === 'tags' ? tagTint(n) : undefined" removable @click.stop @remove="r.value.splice(r.value.indexOf(n), 1)" />
      <span v-if="!r.value.length" class="faint">{{ type === 'tags' ? $t('filters.value.chooseTags') : $t('filters.value.chooseFolders') }}</span>
    </div>

    <template v-else-if="type === 'color'">
      <button class="color-field" @click="pop.toggle($event)">
        <span class="swatch" :style="{ background: isHex ? r.value : 'transparent' }" />
        <span class="mono">{{ r.value }}</span>
      </button>
      <Popover ref="pop">
        <div class="color-pop">
          <ColorPicker inline :modelValue="String(r.value || '#808080').replace('#', '')" @update:modelValue="(v) => (r.value = '#' + v.toUpperCase())" />
          <InputText v-model="r.value" class="mono" placeholder="#3395E1" />
        </div>
      </Popover>
    </template>

    <template v-else>
      <template v-for="k in r.op === 'BETWEEN' ? ['value', 'value2'] : ['value']" :key="k">
        <span v-if="k === 'value2'" class="faint">-</span>
        <DatePicker v-if="dateInput" :modelValue="toDate(r[k])" dateFormat="yy-mm-dd" showIcon iconDisplay="input" class="rv-in" @update:modelValue="(d) => (r[k] = fromDate(d))" />
        <InputNumber v-else-if="units || type === 'num' || k === 'value2'" :modelValue="toNum(r[k])" :min="units ? 0 : undefined" :useGrouping="false" :maxFractionDigits="6" class="rv-in" @update:modelValue="(v) => (r[k] = v ?? '')" />
        <InputText v-else v-model="r[k]" class="rv-in" :placeholder="type === 'words' ? $t('filters.value.words') : ''" spellcheck="false" />
      </template>
      <Select v-if="units" v-model="r.unit" :options="units" :optionLabel="(u) => $t(`filters.units.${u}`)" class="rv-unit" />
    </template>
  </div>
  <div v-else class="rv" />
</template>

<style scoped>
.rv { flex: 1; min-width: 0; display: flex; align-items: center; gap: calc(0.42rem * var(--g)); }
.rv-in { flex: 1; min-width: 0; }
/* as the action dialog's (ActionDialog.vue) */
.pickbox { flex: 1; min-width: 0; display: flex; flex-wrap: wrap; align-items: center; gap: calc(0.35rem * var(--g)); min-height: var(--control-h); padding: 0.14rem calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); background: var(--input); border: 1px solid var(--input-border); cursor: pointer; }
.pickbox:focus { border-color: var(--accent); outline: 0; }
.rv-in :deep(input) { width: 100%; min-width: 0; }
.rv-unit { width: round(7.83rem, 1px); flex: none; }
.rv-pair { width: round(4.9rem, 1px); flex: none; }
.rv-pair :deep(input) { width: 100%; min-width: 0; }
.color-field { flex: 1; min-width: 0; display: flex; align-items: center; gap: calc(0.56rem * var(--g)); height: var(--control-h); padding: 0 calc(0.7rem * var(--g)); border-radius: calc(6px * var(--r)); border: 1px solid var(--input-border); background: var(--input); color: var(--text); font: inherit; cursor: pointer; }
.swatch { width: round(1.4rem, 1px); height: round(0.98rem, 1px); border-radius: calc(4px * var(--r)); flex: none; }
.color-option { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); }
.color-pop { width: round(15.38rem, 1px); display: flex; flex-direction: column; gap: calc(0.56rem * var(--g)); }
</style>

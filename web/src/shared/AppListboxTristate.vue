<script setup>
import { ref } from 'vue'
import Listbox from 'openvue/listbox'
import Checkbox from 'openvue/checkbox'
import AppIcon from './AppIcon.vue'

// A filter's values as OpenVue's Listbox, each with three states: chosen, left out, or neither.
// A click or Space chooses or unchooses; a right click, Shift+Enter or Delete leaves out. `options`: [{ value, label, count, tag, only }], `only` for a value that can be chosen but not left out.
// The `extra` slot puts controls of a row's own after its count; keys typed into them stay theirs.
const props = defineProps({ options: { type: Array, required: true }, chosen: { type: Array, required: true }, out: { type: Array, default: () => [] }, filter: Boolean, scrollHeight: { type: String, default: '18.18rem' }, empty: { type: String, default: '' } })
const emit = defineEmits(['choose', 'leaveOut'])
const list = ref()

// the Listbox hands back the whole selection; the value that changed is the one told
function changed(next) {
  const was = new Set(props.chosen)
  const now = new Set(next || [])
  for (const v of now) if (!was.has(v)) return emit('choose', v)
  for (const v of was) if (!now.has(v)) return emit('choose', v)
}
function leaveOut(o) {
  if (!o.only) emit('leaveOut', o.value)
}
// caught on the way down, before the Listbox takes Enter as a choice
function key(e) {
  if (e.target.closest?.('input')) return
  if (e.key !== 'Delete' && !(e.key === 'Enter' && e.shiftKey)) return
  const o = list.value?.visibleOptions?.[list.value.focusedOptionIndex]
  if (!o) return
  e.preventDefault()
  e.stopPropagation()
  leaveOut(o)
}
</script>

<template>
  <Listbox ref="list" class="tri-list" :options="options" optionLabel="label" optionValue="value" multiple :metaKeySelection="false" :modelValue="chosen" :filter="filter" :scrollHeight="scrollHeight" :emptyMessage="empty || $t('shared.nothingHere')" :emptyFilterMessage="empty || $t('shared.nothingHere')" @update:modelValue="changed" @keydown.capture="key">
    <template #option="{ option, selected }">
      <div class="tri-row" @contextmenu.prevent.stop="leaveOut(option)">
        <Checkbox :modelValue="selected" :indeterminate="out.includes(option.value)" binary readonly tabindex="-1" class="tri-box" :class="{ out: out.includes(option.value) }">
          <template #icon="{ checked, indeterminate }"><AppIcon v-if="indeterminate" name="x" :size="12" class="tri-ico" /><AppIcon v-else-if="checked" name="check" :size="12" class="tri-ico" /></template>
        </Checkbox>
        <span class="grow ellipsis" :title="option.label"><slot name="label" :option="option">{{ option.label }}</slot><span v-if="option.tag" class="tri-tag">{{ option.tag }}</span></span>
        <span v-if="option.count != null" class="count">{{ option.count }}</span>
        <slot name="extra" :option="option" :selected="selected" />
      </div>
    </template>
  </Listbox>
</template>

<style>
.tri-list.p-listbox { border: 0; background: transparent; box-shadow: none; }
.tri-list .p-listbox-list { padding: 0; }
.tri-list .p-listbox-option { padding: 0 calc(0.42rem * var(--g)); border-radius: calc(6px * var(--r)); }
/* the box says what is chosen, so a chosen row isn't filled as well */
/* as specific as OpenVue's own rules for these rows, so they win */
.tri-list.p-listbox:not(.p-disabled) .p-listbox-option.p-listbox-option-selected.p-focus, .tri-list.p-listbox:not(.p-disabled) .p-listbox-option.p-listbox-option-selected, .tri-list.p-listbox:not(.p-disabled) .p-listbox-option:not(.p-listbox-option-selected):not(.p-disabled).p-focus { background: transparent; color: var(--text); }
/* the list takes focus when its menu opens; the focused row lights only for the keyboard or the pointer */
.tri-list.p-listbox .p-listbox-list:focus-visible .p-listbox-option.p-focus, .tri-list.p-listbox .p-listbox-option:not(.p-disabled):hover { background: var(--hover) !important; }
.tri-row { display: flex; align-items: center; gap: calc(0.63rem * var(--g)); width: 100%; height: round(2.17rem, 1px); }
.tri-tag { margin-left: calc(0.42rem * var(--g)); padding: 0 calc(0.35rem * var(--g)); border-radius: calc(4px * var(--r)); background: var(--button); color: var(--text-muted); font-size: 0.85rem; }
.tri-row .count { color: var(--text-faint); flex: none; }
</style>

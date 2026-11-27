<script setup>
import Checkbox from 'openvue/checkbox'
import AppIcon from './AppIcon.vue'

// One value with three states, as a row of an AppListboxTristate: true chosen, false left out, null neither.
// A click or Space chooses; a right click or Shift+Enter leaves out; doing either again clears it.
const model = defineModel({ default: null })
defineProps({ label: String, count: Number })
const choose = () => (model.value = model.value === true ? null : true)
const leaveOut = () => (model.value = model.value === false ? null : false)
function key(e) {
  if (e.key !== 'Enter' || !e.shiftKey) return
  e.preventDefault()
  leaveOut()
}
</script>

<template>
  <label class="tri-one" @contextmenu.prevent="leaveOut" @keydown="key">
    <Checkbox :modelValue="model === true" :indeterminate="model === false" binary class="tri-box" :class="{ out: model === false }" @update:modelValue="choose">
      <template #icon="{ checked, indeterminate }"><AppIcon v-if="indeterminate" name="x" :size="12" class="tri-ico" /><AppIcon v-else-if="checked" name="check" :size="12" class="tri-ico" /></template>
    </Checkbox>
    <span class="grow">{{ label }}</span><span v-if="count != null" class="count">{{ count }}</span>
  </label>
</template>

<style>
.tri-one { display: flex; align-items: center; gap: calc(0.63rem * var(--g)); height: round(2.17rem, 1px); padding: 0 calc(0.42rem * var(--g)); border-radius: calc(6px * var(--r)); cursor: default; }
.tri-one:hover { background: var(--hover); }
.tri-one .count { color: var(--text-faint); flex: none; }
</style>

<script setup>
import SelectButton from 'openvue/selectbutton'
import AppIcon from './AppIcon.vue'

// OpenVue's SelectButton with Curio's icons: `options` [{ value, label, icon, count }]; with `iconOnly` the label is the tooltip and the name read out.
const model = defineModel()
defineProps({ options: { type: Array, required: true }, iconOnly: Boolean, size: { type: Number, default: 16 }, allowEmpty: Boolean })
</script>

<template>
  <SelectButton v-model="model" class="segments" :class="{ 'icon-only': iconOnly }" :options="options" optionLabel="label" optionValue="value" dataKey="value" :allowEmpty="allowEmpty">
    <template #option="{ option }">
      <span class="seg-opt" :aria-label="iconOnly ? option.label : undefined" v-tooltip.bottom="iconOnly ? option.label : undefined">
        <AppIcon v-if="option.icon" :name="option.icon" :size="size" /><span v-if="!iconOnly">{{ option.label }}</span><span v-if="option.count != null" class="count">{{ option.count }}</span>
      </span>
    </template>
  </SelectButton>
</template>

<style>
.segments .seg-opt { display: inline-flex; align-items: center; gap: calc(0.42rem * var(--g)); white-space: nowrap; }
.segments .seg-opt .count { color: var(--text-faint); font-size: 0.85rem; }
.segments.icon-only .p-togglebutton-content { padding: 0 calc(0.35rem * var(--g)); }
</style>

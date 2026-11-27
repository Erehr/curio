<script setup>
import { computed, ref, useAttrs } from 'vue'
import InputText from 'openvue/inputtext'
import AppIcon from './AppIcon.vue'

// The search at the top of a list that floats (a popover's list, a menu): flush with its edges and a line under it, with controls of its own in the slot at its right; drawn by the preset (preset.js), as OpenVue's lists draw their filters.
// Class and style go to the row, everything else (placeholder, listeners) to the input.
defineOptions({ inheritAttrs: false })
const model = defineModel()
const attrs = useAttrs()
const inputAttrs = computed(() => {
  const { class: _c, style: _s, ...rest } = attrs
  return rest
})
const input = ref()
defineExpose({ focus: () => input.value?.$el.focus(), input: () => input.value?.$el })
</script>

<template>
  <div class="list-search" :class="attrs.class" :style="attrs.style">
    <AppIcon name="search" :size="17" class="list-search-icon" />
    <InputText ref="input" v-model="model" :placeholder="$t('shared.search')" spellcheck="false" v-bind="inputAttrs" />
    <slot />
  </div>
</template>

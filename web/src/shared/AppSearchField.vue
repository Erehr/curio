<script setup>
import { computed, ref, useAttrs } from 'vue'
import IconField from 'openvue/iconfield'
import InputIcon from 'openvue/inputicon'
import InputText from 'openvue/inputtext'
import InputGroup from 'openvue/inputgroup'
import InputGroupAddon from 'openvue/inputgroupaddon'
import AppIcon from './AppIcon.vue'

// A text field with an icon in front, or with `menu` a button joined to its front: the app's search and filter boxes. Class and style go to the field's frame, everything else (id, placeholder, listeners) to the input.
defineOptions({ inheritAttrs: false })
const model = defineModel()
// `menu`: the icon becomes a button, as the main search's scope, emitting `menu`; a middle click on it emits `menuClear`, as on a filter chip
defineProps({ icon: { type: String, default: 'search' }, menu: Boolean, menuOn: Boolean, menuLabel: { type: String, default: '' } })
const emit = defineEmits(['menu', 'menuClear'])
const attrs = useAttrs()
const inputAttrs = computed(() => {
  const { class: _c, style: _s, ...rest } = attrs
  return rest
})
const input = ref()
defineExpose({ focus: () => input.value?.$el.focus(), input: () => input.value?.$el })
</script>

<template>
  <InputGroup v-if="menu" :class="['search-field', attrs.class]" :style="attrs.style">
    <InputGroupAddon><button type="button" class="addon-btn" :class="{ on: menuOn }" :aria-label="menuLabel || $t('shared.searchField.where')" @click="emit('menu', $event)" @mousedown.middle.prevent @auxclick.prevent="$event.button === 1 && emit('menuClear')"><AppIcon :name="icon" :size="15" /></button></InputGroupAddon>
    <InputText ref="input" v-model="model" spellcheck="false" v-bind="inputAttrs" />
  </InputGroup>
  <IconField v-else :class="['search-field', attrs.class]" :style="attrs.style">
    <InputIcon><AppIcon :name="icon" :size="15" /></InputIcon>
    <InputText ref="input" v-model="model" spellcheck="false" fluid v-bind="inputAttrs" />
  </IconField>
</template>

<style>
.search-field { flex: 1; min-width: 0; }
.search-field .p-inputtext { padding-top: 0; padding-bottom: 0; }
.search-field .p-inputicon { display: flex; align-items: center; margin-top: -0.52rem; color: var(--text-faint); }
.search-field.p-inputgroup { height: var(--control-h); }
.search-field.p-inputgroup .p-inputtext { min-width: 0; }
</style>

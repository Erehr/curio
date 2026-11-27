<script setup>
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { attachFrame, pageUrl } from './bridge.js'

// A plugin's page in a sandboxed frame of its own origin; the host's menus, pickers and dialogs open over it.
// The origin is the plugin's alone (pageOrigin), so `allow-same-origin` reaches nothing but its own storage and the frames it makes, as a book reader needs; it can't reach this window or another plugin.
// `item` is what a viewer shows now, told as `plugin.item`; `selection` what a widget's window has selected, told as `plugin.selection`; `visible` says whether it can be seen.
const props = defineProps({
  plugin: { type: String, required: true },
  page: { type: String, required: true },
  role: { type: String, default: 'window' },
  run: { type: Object, default: () => ({}) },
  item: { type: Object, default: null },
  selection: { type: Object, default: null },
  visible: { type: Boolean, default: true },
  onAnswer: { type: Function, default: null },
})
const emit = defineEmits(['height', 'title'])
const frame = ref(null)
const heightOf = (e) => emit('height', e.detail)
const titleOf = (e) => emit('title', e.detail)
// hidden until the page has taken the theme: until then its own colour scheme differs from the window's, and the frame would paint an opaque backdrop
const themed = ref(false)
let showTimer = null
const loaded = () => {
  clearTimeout(showTimer)
  showTimer = setTimeout(() => (themed.value = true), 60)
}
let handle = null
const attach = () => {
  handle?.detach()
  handle = frame.value ? attachFrame(frame.value, props.plugin, { role: props.role, run: props.run, onAnswer: props.onAnswer }) : null
  if (handle && props.item) handle.item(props.item)
  if (handle && props.selection) handle.selection(props.selection)
}
onMounted(attach)
watch(() => props.plugin, attach)
// told only when what it says changes: the viewer makes a new item object whenever the item's fields change, and a page would open it again
watch(() => (props.item ? JSON.stringify(props.item) : ''), (v) => v && handle?.item(props.item))
watch(() => props.selection, (v) => v && handle?.selection(v), { deep: true })
watch(() => props.visible, (v) => handle?.visible(v))
onBeforeUnmount(() => {
  clearTimeout(showTimer)
  handle?.detach()
})
defineExpose({ tell: (name, data) => handle?.tell(name, data), run: (data) => handle?.run(data) })
</script>

<template>
  <iframe ref="frame" class="plugin-frame" :class="{ themed }" @curio-height="heightOf" @curio-title="titleOf" :src="pageUrl(plugin, page)" sandbox="allow-scripts allow-same-origin" referrerpolicy="no-referrer" :data-plugin="plugin" :data-role="role" @load="loaded" />
</template>

<style scoped>
/* the frame takes the window's colour scheme, as the page does from the theme, so what is behind the page shows through */
.plugin-frame { display: block; width: 100%; height: 100%; border: 0; background: transparent; opacity: 0; }
.plugin-frame.themed { opacity: 1; }
</style>

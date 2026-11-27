<script setup>
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { overlay, closePopover } from './pageOverlay.js'
import AppMenu from '../shared/AppMenu.vue'
import PluginFrame from './PluginFrame.vue'

// What a plugin page opens outside its frame (plugins/pageOverlay.js): a select's list, as the window's own menus are, and a popover showing another of the plugin's pages, under the page's field.
const list = ref()
const searchList = ref()
const GAP = 2
const EDGE = 4

const menuModel = () => {
  const c = overlay.choose
  if (!c) return []
  const icons = c.options.some((o) => o.icon)
  return c.options.map((o) => (o.heading ? { label: o.label, heading: true } : { label: o.label, icon: o.icon || (icons ? 'point' : undefined), shortcut: o.hint || undefined, current: o.value === c.value, disabled: o.disabled, command: () => answer({ chosen: true, value: o.value }) }))
}
function answer(r) {
  const c = overlay.choose
  if (!c) return
  overlay.choose = null
  c.resolve(r)
}
// the menu closes before it runs the chosen entry
function listHidden() {
  const c = overlay.choose
  setTimeout(() => overlay.choose === c && answer({ chosen: false }), 0)
}
watch(() => overlay.choose, async (c) => {
  if (!c) return
  await nextTick()
  const m = c.search ? searchList.value : list.value
  m?.openAt(c.rect.left, c.rect.bottom, { field: c.rect })
})

// the popover: under the field, or over it where there is no room below; as tall as its page says, within the window
const pop = ref(null)
const fit = ref(0)
const place = ref({})
watch(() => overlay.popover, (p) => {
  fit.value = 0
  if (p) layout()
})
watch(fit, () => overlay.popover && layout())
function layout() {
  const p = overlay.popover
  const r = p.rect
  const h = Math.min(innerHeight - 2 * EDGE, fit.value || p.height || 200)
  const left = Math.max(EDGE, Math.min(r.left, innerWidth - p.width - EDGE))
  const below = r.bottom + GAP + h <= innerHeight - EDGE || r.top - GAP - h < EDGE
  const top = below ? Math.min(r.bottom + GAP, innerHeight - h - EDGE) : r.top - GAP - h
  place.value = { left: left + 'px', top: Math.max(EDGE, top) + 'px', width: p.width + 'px', height: h + 'px' }
}
// a press anywhere else closes it: in this window, or in another plugin page (its own is the popover's page)
function outside(e) {
  // a list, picker or question its page opened over the window belongs to it
  if (overlay.popover && !pop.value?.contains(e.target) && !e.target.closest?.('.app-menu, .picker, .p-dialog-mask')) closePopover(null)
}
function frameDown(e) {
  if (overlay.popover && !pop.value?.contains(e.detail)) closePopover(null)
}
function key(e) {
  if (e.key === 'Escape' && overlay.popover) closePopover(null)
}
window.addEventListener('pointerdown', outside, true)
window.addEventListener('curio-frame-down', frameDown)
window.addEventListener('keydown', key)
onBeforeUnmount(() => {
  window.removeEventListener('pointerdown', outside, true)
  window.removeEventListener('curio-frame-down', frameDown)
  window.removeEventListener('keydown', key)
  closePopover(null)
  answer({ chosen: false })
})
</script>

<template>
  <AppMenu ref="list" :model="menuModel" @hide="listHidden" />
  <AppMenu ref="searchList" :model="menuModel" searchable @hide="listHidden" />
  <div v-if="overlay.popover" ref="pop" class="page-pop" :style="place">
    <PluginFrame :key="overlay.popover.plugin + overlay.popover.page" :plugin="overlay.popover.plugin" :page="overlay.popover.page" role="popover" :run="overlay.popover.data ?? {}" @height="(h) => (fit = h)" />
  </div>
</template>

<style>
.page-pop { position: fixed; z-index: 1200; overflow: hidden; background: var(--surface); border: 1px solid var(--float-border); border-radius: calc(8px * var(--r)); }
html.fx-see-through .page-pop { background: color-mix(in srgb, var(--surface) 92%, transparent); backdrop-filter: blur(18px); }
</style>

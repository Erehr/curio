<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { S } from '../app/state.js'
import { L } from '../layout/arrangement.js'
import { saveUi } from '../app/preferences.js'
import { call } from '../platform/ipc.js'
import { baseOf, widgetInfo } from '../layout/widgets.js'
import PluginFrame from './PluginFrame.vue'
import AppWidgetHeading from '../shared/AppWidgetHeading.vue'

// A Widget plugin's page in a sidebar, placed like Curio's own widgets. It hears the selection as `plugin.selection`.
// `popup`: opened from its icon, so it shows in full; `headless` widgets show without their heading anywhere
const props = defineProps({ id: String, popup: Boolean })
const emit = defineEmits(['empty'])
const info = computed(() => widgetInfo(props.id, S.contrib))
const w = computed(() => info.value?.plugin)
const state = computed(() => {
  const l = L()
  l.widgetState ||= {}
  return (l.widgetState[props.id] ||= {})
})
const collapsed = computed(() => !props.popup && !w.value?.headless && !!state.value.collapsed)
function toggle() {
  state.value.collapsed = !collapsed.value
  saveUi()
}
const one = computed(() => (S.selected.length === 1 ? S.detail?.asset : null))
// `kinds` and `exts` limit it to items of those: the one selected, or every one of several
const applies = computed(() => {
  const kinds = w.value?.kinds
  const exts = w.value?.exts
  const ext = (x) => String(x).toLowerCase()
  if (one.value) return (!kinds?.length || kinds.includes(one.value.kind)) && (!exts?.length || exts.includes(ext(one.value.ext)))
  if (kinds?.length && !(S.selected.length > 1 && !!S.summary?.kinds?.length && S.summary.kinds.every((k) => kinds.includes(k.kind)))) return false
  // several selected: those loaded in the grid stand for the rest
  return !exts?.length || (S.selected.length > 1 && S.selected.every((id) => !S.items.has(id) || exts.includes(ext(S.items.get(id).ext))))
})
const active = computed(() => !!(w.value && applies.value && !collapsed.value))
// kept mounted briefly after hiding so stepping across mixed items avoids reloading, then unloaded to free its webview memory
const CLOSE_AFTER_MS = 30000
const kept = ref(false)
let idleTimer = null
watch(active, (on) => {
  clearTimeout(idleTimer)
  if (on) kept.value = true
  else if (kept.value) idleTimer = setTimeout(() => {
    kept.value = false
    fit.value = null
    customTitle.value = ''
    selection.value = null
  }, CLOSE_AFTER_MS)
}, { immediate: true })

// the selected items' ids, the first thousand, read once the selection settles; skipped while the widget is hidden or collapsed
const LIMIT = 1000
const selection = ref(null)
let timer = null
let seq = 0
const snapshot = (ids) => ({
  count: S.selected.length,
  ids,
  item: one.value ? { id: one.value.uid, kind: one.value.kind, ext: one.value.ext, name: one.value.name } : null,
})
watch(() => [active.value, S.selected.length, S.selected[0], S.selected[S.selected.length - 1], one.value?.uid], () => {
  clearTimeout(timer)
  if (!active.value) return
  if (one.value) return (selection.value = snapshot([one.value.uid]))
  if (!S.selected.length) return (selection.value = snapshot([]))
  const n = ++seq
  timer = setTimeout(async () => {
    const items = await call('item.getMany', { ids: S.selected.slice(0, LIMIT) }).catch(() => [])
    if (n === seq && active.value) selection.value = snapshot(items.map((i) => i.id))
  }, 150)
}, { immediate: true })

// as tall as its page says it is, until it is given a height by dragging its edge; a page of no height has nothing to show
const fit = ref(null)
const customTitle = ref('')
const empty = computed(() => fit.value === 0)
// told to an icon standing for it, which goes while there is nothing to show, as the widget does in an open sidebar
watch(() => !w.value || !applies.value || empty.value, (v) => emit('empty', v), { immediate: true })
const height = computed(() => state.value.height || fit.value || w.value.height || 240)
let dragOff = null
function resize(e) {
  dragOff?.()
  const y0 = e.clientY
  const h0 = height.value
  const move = (ev) => (state.value.height = Math.max(80, Math.min(1200, h0 + ev.clientY - y0)))
  const up = () => {
    dragOff?.()
    saveUi()
  }
  dragOff = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
    dragOff = null
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
}
onBeforeUnmount(() => {
  clearTimeout(idleTimer)
  clearTimeout(timer)
  dragOff?.()
})
</script>

<template>
  <!-- kept briefly while hidden so switching back avoids reloading; empty, it is folded to nothing rather than hidden, so its page keeps a width to lay itself out in -->
  <div v-if="w && (applies || kept)" v-show="applies" class="plugin-widget" :class="{ 'pw-empty': empty }" :data-part="baseOf(id)">
    <AppWidgetHeading v-if="!w.headless" :title="customTitle || w.title" :collapsed="collapsed" :foldable="!popup" @toggle="toggle" />
    <div class="pw-body" :style="{ height: (collapsed ? 0 : height) + 'px' }">
      <PluginFrame v-if="active || kept" :plugin="w.plugin" :page="w.page" role="widget" :selection="selection" :visible="active" @height="(h) => (fit = h)" @title="(t) => (customTitle = t)" />
    </div>
    <div v-if="!collapsed" class="pw-resize" @pointerdown.prevent="resize" />
  </div>
</template>

<style scoped>
.plugin-widget { display: flex; flex-direction: column; }
.plugin-widget.pw-empty { height: 0; padding: 0; overflow: hidden; visibility: hidden; }
/* in line with Curio's own widgets: the page draws on the sidebar, from its edge */
.plugin-widget { padding: 0 0.14rem; }
.pw-body { overflow: hidden; }
.pw-resize { height: 0.42rem; cursor: ns-resize; }
</style>

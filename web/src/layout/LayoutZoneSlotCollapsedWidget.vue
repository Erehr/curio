<script setup>
import { onBeforeUnmount, ref, watch } from 'vue'
import Popover from 'openvue/popover'
import { S } from '../app/state.js'
import AppIcon from '../shared/AppIcon.vue'

// A widget with no room to show: its icon, opening the widget itself in a popup.
// Below the icon in a toolbar; beside it in a sidebar of icons, away from the window's edge.
// `keep`: a widget kept loaded elsewhere, moved into the popup while it is open, so a plugin's page is not loaded again on every opening
const props = defineProps({ info: { type: Object, required: true }, disabled: Boolean, keep: Object })
const pop = ref()
const btn = ref()
const body = ref()
let home = null
let held = null
// moveBefore keeps a frame's page as it is, where moving it otherwise loads it again
const put = (to, el) => (to.moveBefore ? to.moveBefore(el, null) : to.appendChild(el))
const open = ref(false)
const GAP = 6
const EDGE = 4
function besideRail() {
  const el = pop.value?.container
  const b = btn.value
  if (!el || !b?.closest('.zone.rail')) return false
  const r = b.getBoundingClientRect()
  const left = r.left > innerWidth / 2 ? r.left - GAP - el.offsetWidth : r.right + GAP
  el.style.insetInlineStart = el.style.left = Math.max(EDGE, left) + scrollX + 'px'
  // level with the icon's top, or near the window's bottom with its bottom
  const h = el.offsetHeight
  el.style.top = Math.max(EDGE, r.top + h > innerHeight - EDGE ? r.bottom - h : r.top) + scrollY + 'px'
  return true
}
// the popover aligns itself again whenever its content changes size; beside a rail that alignment is this one
watch(pop, (p) => {
  if (!p) return
  const own = p.alignOverlay
  p.alignOverlay = () => besideRail() || own()
})

// in a sidebar of icons it opens on hover when Settings › Layout says so, as the Library's menus do, and closes once the pointer has left both
const LEAVE_MS = 120
let leaveTimer = null
const onHover = () => S.ui.railOpen !== 'click' && !!btn.value?.closest('.zone.rail')
function enter(e) {
  clearTimeout(leaveTimer)
  if (props.disabled || S.customize || !onHover() || open.value) return
  if (current && current !== close) current()
  current = close
  pop.value.show(e, btn.value)
}
function click(e) {
  if (props.disabled || S.customize) return
  if (onHover()) enter(e)
  else pop.value.toggle(e)
}
watch(() => props.disabled, (d) => { if (d) close() })
// it stays while one of its fields is being typed in or something opened from it is in use
function busy() {
  const el = document.activeElement
  return (!!pop.value?.container?.contains(el) && /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName)) || !!document.querySelector('.app-menu, .p-select-overlay, .picker')
}
function leave() {
  if (!onHover()) return
  clearTimeout(leaveTimer)
  const later = () => (leaveTimer = setTimeout(() => (busy() ? later() : close()), LEAVE_MS))
  later()
}
function close() {
  clearTimeout(leaveTimer)
  if (current === close) current = null
  pop.value?.hide()
}
function shown() {
  open.value = true
  if (props.keep && body.value) {
    home = props.keep.parentNode
    held = body.value
    put(held, props.keep)
    pop.value?.alignOverlay()
  }
  const el = pop.value?.container
  el?.addEventListener('pointerenter', () => clearTimeout(leaveTimer))
  el?.addEventListener('pointerleave', leave)
}
function hidden() {
  open.value = false
  // the popup's own ref is already gone when it starts to close, the element still in place
  if (home && props.keep?.parentNode === held) put(home, props.keep)
  home = held = null
}
onBeforeUnmount(() => {
  clearTimeout(leaveTimer)
  if (current === close) current = null
  hidden()
})
</script>

<script>
// the one popup open on hover, across the icons of the compact sidebars
let current = null
</script>

<template>
  <button ref="btn" class="icon-btn part-btn" :class="{ on: open }" :disabled="disabled" :aria-label="info.label" v-tooltip.bottom="info.label" @click="click" @pointerenter="enter" @pointerleave="leave"><AppIcon :name="info.icon" :size="18" /></button>
  <Popover ref="pop" class="part-pop" @show="shown" @hide="hidden">
    <div ref="body" class="part-pop-body"><slot /></div>
  </Popover>
</template>

<style>
.zone.rail .part-btn { flex: none; align-self: center; width: round(2.38rem, 1px); height: round(2.38rem, 1px); margin: 1px auto; }
/* comes and goes at once, as the menus beside the Library's icons do */
.part-pop:is(.p-anchored-overlay-enter-active, .p-anchored-overlay-leave-active) { transition: none !important; animation: none !important; }
.part-pop .p-popover-content { padding: calc(0.7rem * var(--g)) 0; }
/* as wide as a sidebar, and never taller than the window */
.part-pop-body { width: round(20.98rem, 1px); max-height: min(70vh, 44.76rem); overflow-y: auto; overflow-x: hidden; padding: 0 calc(0.7rem * var(--g)); }
/* the room a widget leaves under itself in a sidebar is the popup's own padding here */
.part-pop .part-pop-body .w-body { padding-bottom: 0; }
/* widgets kept for their popups, as wide as they show there; in the window though unseen, as a frame out of view runs throttled and would tell its height late */
.kept-widgets { position: fixed; left: 0; top: 0; z-index: -1; width: round(20.98rem, 1px); padding: 0 calc(0.7rem * var(--g)); opacity: 0; pointer-events: none; }
.part-pop .part-pop-body .w-head { padding-top: 0; }
</style>

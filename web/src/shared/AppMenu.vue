<script setup>
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import ContextMenu from 'openvue/contextmenu'
import AppIcon from './AppIcon.vue'
import AppHighlight from './AppHighlight.vue'
import { fuzzy } from './text.js'
import { t } from './i18n.js'
import { keyText } from './keys.js'

// A menu at the pointer, under a button or beside a sidebar icon: OpenVue's ContextMenu with Curio's rows. `model` items:
// `checked` marks a setting that is on, `current` the one chosen of several (lit as a list's chosen option).
// `more` (items, or a function giving them) is the row's own options, opened from a button at its end or a right click on it, as a second menu.
// `drag(event, row)` starts dragging the row (beginDrag, lifting `row`) from a grip drawn at its start, the one part of it that drags, and `drop` names it as a place to drop on; `refresh()` reads a function's items again while the menu is open.
// { label, icon, color, shortcut, count, checked, current, danger, disabled, visible, separator, heading, items, more, command, drag, drop, indent, dots: [{ label, color, selected, command }] }
const props = defineProps({ model: [Array, Function], searchable: Boolean })
const emit = defineEmits(['hide'])
const cm = ref()
const source = ref([])
const q = ref('')
const shown = ref(false)
const hover = ref(null)
// opened as a field's list: under the field, at least as wide as it
const field = ref(null)
// opened beside a rail icon by a click: it stays when the pointer leaves
let stays = false
let leaveTimer = null
// the button that opened it: a press there leaves the menu to the button's click, which toggles it closed rather than opening it again
let opener = null
// just long enough to cross from the icon onto its menu
const LEAVE_MS = 70
const MENU_GAP = 2
// a row lights a little more than a list's under the pointer
const LIT = 'color-mix(in srgb, var(--text) 10%, transparent)'
const TOKENS = { item: { focusBackground: LIT, activeBackground: LIT } }
// a menu's rows dragged to reorder them make room in its list (app/dragDrop.js)
const PT = { rootList: { 'data-drop-list': '' } }
const UNDER_GAP = 4
const id = 'm' + Math.random().toString(36).slice(2, 8)

const visible = (list) => (list || []).filter((i) => i.visible !== false)
function flatten(list, path = []) {
  return visible(list).flatMap((it) => (it.separator || it.dots || it.heading ? [] : it.items ? flatten(it.items, [...path, it.label]) : [{ ...it, path: path.join(' › ') }]))
}
function prepare(it) {
  if (it.separator) return { separator: true }
  const p = { ...it, disabled: it.disabled || it.heading, class: [it.danger && 'danger', it.current && 'cur', it.dots && 'dots', it.heading && 'heading'].filter(Boolean).join(' ') || undefined }
  if (it.items) p.items = tidy(it.items)
  // a row with a submenu and its own action (a folder) does the action and closes
  if (it.command) p.command = it.items ? (e) => { hide(); it.command(e.originalEvent) } : (e) => it.command(e.originalEvent)
  return p
}
// no doubled or dangling separators when items are hidden
function tidy(list) {
  const out = []
  for (const it of visible(list)) {
    if (it.separator && (!out.length || out[out.length - 1].separator)) continue
    out.push(prepare(it))
  }
  while (out.length && out[out.length - 1].separator) out.pop()
  return out
}
const model = computed(() => {
  const term = q.value.trim()
  // a submenu's name finds its entries too: "copy" lists Copy › Path and the rest
  const list = tidy(term ? flatten(source.value).filter((i) => fuzzy(i.path ? `${i.path} › ${i.label}` : i.label, term)) : source.value)
  if (!props.searchable) return list
  // the search row is disabled, so the list's own keys and hover pass over it
  return [{ search: true, label: t('shared.search'), disabled: true }, ...(list.length ? list : [{ label: t('shared.menu.nothingFound'), disabled: true, none: true }])]
})

function at(x, y) {
  return { pageX: x - 1 + window.scrollX, pageY: y - 1 + window.scrollY, stopPropagation() {}, preventDefault() {} }
}
function openAt(x, y, opts = {}) {
  opener = opts.opener || null
  q.value = ''
  source.value = typeof props.model === 'function' ? props.model() : props.model
  hover.value = opts.from || null
  field.value = opts.field || null
  stays = !!opts.stay
  shown.value = true
  cm.value.show(at(x, y))
  // OpenVue closes the menu on a click anywhere but its target, which it never sets itself
  cm.value.target = opener
}
function show(e) {
  const r = e.currentTarget?.getBoundingClientRect?.()
  const pointer = e.type === 'contextmenu' || !r
  openAt(pointer ? e.clientX : r.left, pointer ? e.clientY : r.bottom + UNDER_GAP, { opener: pointer ? null : e.currentTarget })
}
/** Open beside a rail button; closes by itself once the pointer has left, unless `stay` (opened by a click). */
function hoverAt(el, stay = false) {
  clearTimeout(leaveTimer)
  if (hoverOpen && hoverOpen !== hide) hoverOpen()
  hoverOpen = hide
  const r = el.getBoundingClientRect()
  openAt(r.right + MENU_GAP, r.top, { from: r, stay })
}
function hide() {
  clearTimeout(leaveTimer)
  if (hoverOpen === hide) hoverOpen = null
  if (shown.value) cm.value?.hide()
}
const container = () => document.querySelector(`.app-menu.${id}`)
const EDGE = 4
// Below the pointer; above it when it fits there whole; otherwise as low as the window allows, rather than OpenVue's flip that pins a tall menu to the top.
// Beside a rail icon: to its right, or to its left when the sidebar sits on the right, level with the icon's top or, near the bottom, its bottom.
function position() {
  const c = this.container
  if (!c) return
  // under a field or a wide control (the library card) it is at least as wide as what opened it, never wider than the window
  const under = field.value?.width ?? opener?.getBoundingClientRect().width
  c.style.minWidth = under ? Math.min(under, innerWidth - 2 * EDGE) + 'px' : ''
  const list = c.querySelector('.p-contextmenu-root-list')
  if (list) list.style.maxHeight = ''
  const w = c.offsetWidth
  const h = c.offsetHeight
  const r = hover.value
  const f = field.value
  let left, top
  if (f) {
    left = f.left
    const below = innerHeight - EDGE - f.bottom - MENU_GAP
    const above = f.top - MENU_GAP - EDGE
    if (h <= below) top = f.bottom + MENU_GAP
    else if (h <= above) top = f.top - MENU_GAP - h
    else {
      // a list taller than the room on either side goes on the roomier one and scrolls within it
      const room = Math.max(below, above)
      if (list) list.style.maxHeight = Math.max(80, room - (h - list.offsetHeight)) + 'px'
      top = below >= above ? f.bottom + MENU_GAP : f.top - MENU_GAP - c.offsetHeight
    }
  } else if (r) {
    left = r.left > innerWidth / 2 ? r.left - w - MENU_GAP : r.right + MENU_GAP
    top = r.top + h > innerHeight - EDGE ? r.bottom - h : r.top
  } else {
    const x = this.pageX + 1 - window.scrollX
    const y = this.pageY + 1 - window.scrollY
    left = x + w > innerWidth - EDGE ? x - w : x
    top = y + h <= innerHeight - EDGE ? y : y - h >= EDGE ? y - h : innerHeight - h - EDGE
  }
  left = Math.max(EDGE, Math.min(left, innerWidth - w - EDGE))
  top = Math.max(EDGE, top)
  c.style.left = left + window.scrollX + 'px'
  c.style.top = top + window.scrollY + 'px'
}
watch(cm, (m) => m && (m.position = position.bind(m)))
// submenus are fixed to the window so that any list can scroll without clipping them: each is placed beside its row, flipped or lifted at the edges
function placeSub(ul) {
  const row = ul.parentElement?.getBoundingClientRect()
  if (!row) return
  const w = ul.offsetWidth
  const h = ul.offsetHeight
  let left = row.right
  if (left + w > innerWidth - EDGE) left = Math.max(EDGE, row.left - w)
  let top = row.top - parseFloat(getComputedStyle(ul).paddingTop || 0)
  if (top + h > innerHeight - EDGE) top = Math.max(EDGE, innerHeight - h - EDGE)
  ul.style.insetInlineStart = ul.style.left = left + 'px'
  ul.style.top = top + 'px'
}
let watcher = null
// a scrolled list closes the submenus opened from it, which would no longer sit beside their rows
function scrolled(e) {
  const list = e.target
  if (!list.matches?.('.p-contextmenu-root-list, .p-contextmenu-submenu') || !cm.value) return
  let depth = 0
  for (let el = list.parentElement; el && !el.classList.contains('app-menu'); el = el.parentElement) if (el.classList.contains('p-contextmenu-submenu')) depth++
  if (list.classList.contains('p-contextmenu-submenu')) depth++
  cm.value.activeItemPath = cm.value.activeItemPath.filter((p) => p.level < depth)
}
// ContextMenu focuses its list before it is placed, so a list taller than a small window scrolls the page, which has no scrolling of its own
const unscroll = () => document.scrollingElement && (document.scrollingElement.scrollTop || document.scrollingElement.scrollLeft) && document.scrollingElement.scrollTo(0, 0)
function opened() {
  unscroll()
  requestAnimationFrame(unscroll)
  window.addEventListener('pointerdown', outside, true)
  const el = container()
  watcher = new MutationObserver((list) => list.forEach((m) => m.addedNodes.forEach((n) => n.nodeType === 1 && (n.matches('.p-contextmenu-submenu') ? [n] : n.querySelectorAll('.p-contextmenu-submenu')).forEach(placeSub))))
  if (el) watcher.observe(el, { childList: true, subtree: true })
  el?.addEventListener('scroll', scrolled, true)
  if (props.searchable) requestAnimationFrame(() => el?.querySelector('.m-search input')?.focus({ preventScroll: true }))
  // a field's list takes the keys from the page it was opened from
  else if (field.value) requestAnimationFrame(() => el?.querySelector('.p-contextmenu-root-list')?.focus({ preventScroll: true }))
}
function closed() {
  watcher?.disconnect()
  watcher = null
  shown.value = false
  hover.value = null
  field.value = null
  opener = null
  window.removeEventListener('pointerdown', outside, true)
  moreModel.value = null
  emit('hide')
}
function outside(e) {
  if (!e.target.closest?.('.app-menu') && !opener?.contains?.(e.target)) hide()
}
function enter() {
  clearTimeout(leaveTimer)
}
function leave() {
  if (hover.value && !stays) leaveTimer = setTimeout(hide, LEAVE_MS)
}
// the search field keeps its keys; down moves into the list, Enter takes the first result
function searchKey(e) {
  if (e.key === 'Escape') hide()
  else if (e.key === 'Enter') {
    const first = model.value.find((i) => !i.search && !i.separator && !i.disabled && !i.dots && !i.items)
    if (first?.command) {
      hide()
      first.command({ originalEvent: e })
    }
  } else if (e.key === 'ArrowDown') {
    const list = container()?.querySelector('.p-contextmenu-root-list')
    list?.focus()
    nextTick(() => list?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', code: 'ArrowDown', bubbles: true })))
  } else {
    e.stopPropagation()
    return
  }
  e.preventDefault()
  e.stopPropagation()
}
function pickDot(d) {
  hide()
  d.command()
}
const moreMenu = ref()
const moreModel = ref(null)
function openMore(e, item, atPointer = false) {
  moreModel.value = item.more
  if (atPointer) return nextTick(() => moreMenu.value?.openAt(e.clientX, e.clientY))
  // beside this menu, level with the row, where a submenu would open
  const menu = e.currentTarget.closest('.app-menu').getBoundingClientRect()
  const list = e.currentTarget.closest('.p-contextmenu-root-list, .p-contextmenu-submenu')
  const row = e.currentTarget.closest('.p-contextmenu-item').getBoundingClientRect()
  const lift = parseFloat(getComputedStyle(list).paddingTop) + 1
  const from = { left: menu.left, right: menu.right, top: row.top - lift, bottom: row.bottom + lift }
  nextTick(() => moreMenu.value?.openAt(from.right + MENU_GAP, from.top, { from, stay: true }))
}
// a press inside a plugin's page closes the menu, as one anywhere else in the window does
window.addEventListener('curio-frame-down', hide)
onBeforeUnmount(() => {
  window.removeEventListener('curio-frame-down', hide)
  // the menu may go before it says it closed
  window.removeEventListener('pointerdown', outside, true)
  watcher?.disconnect()
  clearTimeout(leaveTimer)
  hide()
})
function refresh() {
  if (shown.value && typeof props.model === 'function') source.value = props.model()
}
const hasIcon = computed(() => model.value.some((i) => !i.search && !i.separator && !i.dots && !i.heading && (i.icon || i.checked != null)))
defineExpose({ show, toggle: (e) => (shown.value ? hide() : show(e)), hide, openAt, hoverAt, leave, refresh, isOpen: () => shown.value })
</script>

<script>
// the one menu open on hover, across all the icons of the compact sidebar
let hoverOpen = null
</script>

<template>
  <ContextMenu ref="cm" :model="model" :dt="TOKENS" :pt="PT" :class="['app-menu', id]" breakpoint="1px" @show="opened" @hide="closed" @pointerenter="enter" @pointerleave="leave" @contextmenu.prevent>
    <template #item="{ item, props: p, hasSubmenu }">
      <div v-if="item.search" class="m-search" @click.stop @pointerdown.stop>
        <AppIcon name="search" :size="17" class="faint" />
        <input v-model="q" :placeholder="$t('shared.search')" spellcheck="false" @keydown="searchKey" />
      </div>
      <div v-else-if="item.dots" class="m-dots swatches" @click.stop>
        <button v-for="d in item.dots" :key="d.color || 'none'" class="swatch-dot" :class="{ none: !d.color, sel: d.selected }" :style="{ background: d.color || undefined }" :aria-label="d.label" v-tooltip="d.label" @click="pickDot(d)" />
      </div>
      <div v-else-if="item.none" class="m-row m-empty faint">{{ item.label }}</div>
      <div v-else-if="item.heading" class="m-head section-title">{{ item.label }}</div>
      <a v-else class="m-row" v-bind="p.action" :data-drop="item.drop || undefined" @contextmenu.prevent.stop="item.more && openMore($event, item, true)">
        <span v-if="item.drag" class="m-grip" aria-hidden="true" @pointerdown.stop="item.drag($event, $event.currentTarget.closest('.m-row'))" @click.stop.prevent><AppIcon name="grip-vertical" :size="15" /></span>
        <span v-if="item.indent" :style="{ width: item.indent * 0.98 + 'rem' }" class="m-indent" />
        <AppIcon v-if="hasSubmenu && hover" name="caret-right-filled" :size="9" class="m-caret" />
        <AppIcon v-if="item.checked != null && !item.icon" :name="item.checked ? 'check' : 'point'" :size="16" :class="{ blank: !item.checked }" />
        <AppIcon v-else-if="hasIcon" :name="item.icon || 'point'" :size="18" :class="{ blank: !item.icon }" :style="{ color: item.color || item.iconColor || undefined }" />
        <span class="m-label"><span v-if="item.path" class="faint"><AppHighlight :text="item.path" :q="q" /> › </span><AppHighlight :text="item.label" :q="q" /></span>
        <AppIcon v-if="item.checked && item.icon" name="check" :size="15" class="m-tick" />
        <span v-if="item.count != null" class="count">{{ item.count }}</span>
        <span v-if="item.shortcut" class="m-key">{{ keyText(item.shortcut) }}</span>
        <AppIcon v-if="hasSubmenu && !hover" name="chevron-right" :size="15" class="m-arrow" />
        <button v-if="item.more" class="icon-btn m-more" :aria-label="$t('shared.menu.more')" v-tooltip="$t('shared.menu.more')" @pointerdown.stop @click.stop="openMore($event, item)"><AppIcon name="dots" :size="16" /></button>
      </a>
    </template>
  </ContextMenu>
  <AppMenu v-if="moreModel" ref="moreMenu" :model="moreModel" @hide="moreModel = null" />
</template>

<style>
.app-menu.p-contextmenu { min-width: round(16.08rem, 1px); max-width: 32.17rem; font-size: 1rem; }
/* a menu comes and goes at once: no two show while moving between the compact sidebar's icons, and a see-through window's blur is there from the start (the system skips it while the menu fades in) */
.app-menu:is(.p-anchored-overlay-enter-active, .p-anchored-overlay-leave-active) { transition: none !important; animation: none !important; }
.app-menu .p-contextmenu-root-list, .app-menu .p-contextmenu-submenu { padding: calc(0.28rem * var(--g)); gap: 0; outline: none; }
.app-menu .p-contextmenu-submenu { min-width: round(16.08rem, 1px); max-width: 32.17rem; }
.app-menu .p-contextmenu-submenu { position: fixed; }
.app-menu .p-contextmenu-root-list, .app-menu .p-contextmenu-submenu { max-height: calc(100vh - 0.56rem); overflow-y: auto; overflow-x: hidden; }
.app-menu .p-contextmenu-item-content { border-radius: calc(6px * var(--r)); }
.app-menu .p-contextmenu-item.cur > .p-contextmenu-item-content,
.app-menu .p-contextmenu-item.cur.p-focus > .p-contextmenu-item-content,
.app-menu .p-contextmenu-item.cur:not(.p-disabled) > .p-contextmenu-item-content:hover { background: var(--selection); }
.app-menu .p-contextmenu-item.danger .m-row { color: var(--danger); }
.app-menu .p-contextmenu-item.p-disabled { opacity: 1; }
.app-menu .p-contextmenu-item.p-disabled .m-row { color: var(--text-faint); }
.app-menu .p-contextmenu-item.p-disabled .m-row .ico { opacity: .5; }
.app-menu .p-contextmenu-item:has(.m-search) > .p-contextmenu-item-content { pointer-events: auto; background: transparent; cursor: default; }
.app-menu .p-contextmenu-item.dots > .p-contextmenu-item-content { background: transparent; cursor: default; }
.app-menu .p-contextmenu-separator { margin: calc(0.28rem * var(--g)); }
/* rows moving aside for a dragged one don't leave the pointer's light behind */
body.dragging .app-menu .p-contextmenu-item:not(.cur) > .p-contextmenu-item-content { background: transparent; }
.m-search { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); height: round(2.66rem, 1px); margin: calc(-0.28rem * var(--g)) calc(-0.28rem * var(--g)) calc(0.28rem * var(--g)); padding: 0 calc(0.98rem * var(--g)); border-bottom: 1px solid var(--border); }
.app-menu .m-search, .app-menu .m-search * { pointer-events: auto; user-select: text; }
.m-search input { flex: 1; min-width: 0; border: 0; outline: 0; padding: 0; background: transparent; color: var(--text); font: inherit; }
.m-search input::placeholder { color: var(--text-faint); }
.m-row { display: flex; align-items: center; gap: calc(0.63rem * var(--g)); height: round(2.1rem, 1px); padding: 0 calc(0.56rem * var(--g)); color: inherit; text-decoration: none; cursor: default; white-space: nowrap; }
.m-row .ico { color: var(--text-muted); flex: none; }
.m-row .ico.blank { visibility: hidden; }
.m-indent { flex: none; }
/* what drags a row that can be reordered, in the margin before its icon */
.m-grip { display: inline-flex; flex: none; margin: 0 calc(-0.28rem * var(--g)) 0 calc(-0.21rem * var(--g)); cursor: grab; }
.m-row .m-grip .ico { color: var(--text-faint); }
.m-row .m-grip:hover .ico { color: var(--text); }
.m-label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
.m-key { flex: none; margin-left: auto; padding-left: calc(1.82rem * var(--g)); color: var(--text-faint); font-size: 0.92em; }
.m-row .count { margin-left: auto; padding-left: calc(1.26rem * var(--g)); color: var(--text-faint); }
.m-row .m-arrow { margin-left: auto; margin-right: -0.14rem; }
/* beside the compact sidebar a row marks its submenu as the sidebar's tree does, in the margin, so the counts stay in line */
.m-row { position: relative; }
.m-row .ico.m-caret { position: absolute; left: 0; color: var(--text-faint); }
.m-key + .m-arrow, .count + .m-arrow { margin-left: calc(0.42rem * var(--g)); }
.m-row .m-tick { color: var(--accent); }
.m-row .m-more { flex: none; min-width: round(1.68rem, 1px); height: round(1.68rem, 1px); padding: 0; margin-left: auto; margin-right: calc(-0.28rem * var(--g)); color: var(--text-faint); }
.m-row .m-more:hover { color: var(--text); }
.m-key + .m-more, .count + .m-more { margin-left: calc(0.42rem * var(--g)); }
/* the first dot under the icons of the rows above */
.m-dots { flex-wrap: nowrap; padding: calc(0.42rem * var(--g)) calc(0.56rem * var(--g)) calc(0.42rem * var(--g)) calc(0.84rem * var(--g)); }
.m-empty { padding: 0 calc(0.7rem * var(--g)); }
/* a section's name over its entries, as the sidebar's own headings */
.app-menu .p-contextmenu-item.heading > .p-contextmenu-item-content { background: transparent; cursor: default; }
.m-head { display: flex; align-items: center; height: round(1.82rem, 1px); padding: calc(0.14rem * var(--g)) calc(0.56rem * var(--g)) 0; }
</style>

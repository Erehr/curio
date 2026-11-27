<script setup>
import { computed, nextTick, ref, watch } from 'vue'
import Popover from 'openvue/popover'
import Checkbox from 'openvue/checkbox'
import AppListSearch from '../shared/AppListSearch.vue'
import AppMenu from '../shared/AppMenu.vue'
import { S } from '../app/state.js'
import { L } from '../layout/arrangement.js'
import { saveUi } from '../app/preferences.js'
import { allColumns, nextSort, sortedBy } from './listColumns.js'
import { beginDrag, finalIndex } from '../app/dragDrop.js'
import { fuzzy } from '../shared/text.js'
import AppIcon from '../shared/AppIcon.vue'
import { rem } from '../shared/rem.js'
import { t } from '../shared/i18n.js'

// The list view's header: click a column to sort by it (again to turn it around), drag the edge on its right to resize it, drag the name to move it, right-click for fitting and hiding. The + opens the list of every column there is.
const props = defineProps({ pad: Number, scrollbar: Number, contentWidth: Number, room: Number, offset: Number })
const lay = computed(() => L())
const every = computed(() => allColumns(S.sidebar.fields, S.contrib.columns))
const cols = computed(() => lay.value.listCols.map((c) => ({ ...every.value.find((x) => x.id === c.id), w: c.w })).filter((c) => c.id))
const pop = ref()
const menu = ref()
const head = ref()
const q = ref('')
const NAME_SORT = { id: 'name', sort: 'name' }
// rem, but for MAX_FIT
const MIN_COL = 3.5
const MAX_FIT = 600
const SORT_ICON = 1.19
const CHIP_GAP = 0.28
// room after the last column for the + button, as the rows keep (DisplayGrid.vue)
const ROW_END = 2.38

function clickColumn(c) {
  if (!c.sort) return
  lay.value.sort = nextSort(lay.value.sort, c)
  saveUi()
}
const isOn = (id) => lay.value.listCols.some((c) => c.id === id)
function toggle(c) {
  const list = lay.value.listCols
  const i = list.findIndex((x) => x.id === c.id)
  if (i >= 0) list.splice(i, 1)
  else list.push({ id: c.id, w: c.w })
  saveUi()
}
const entryOf = (c) => lay.value.listCols.find((x) => x.id === c.id)
const cellsOf = (c) => document.querySelectorAll(`.lcell[data-col="${CSS.escape(c.id)}"]`)
// a tag is never cut in half: the column is at least as wide as the widest tag in view
function minWidth(c) {
  let w = rem(MIN_COL)
  if (c.render === 'tags') for (const cell of cellsOf(c)) for (const t of cell.querySelectorAll('.ltag')) w = Math.max(w, t.offsetWidth)
  return w
}
// what the column needs for its heading and the rows in view
function fitWidth(c) {
  const label = head.value?.querySelector(`.lh[data-col="${CSS.escape(c.id)}"] .ellipsis`)
  let w = (label?.scrollWidth || 0) + (c.sort ? rem(SORT_ICON) : 0)
  for (const cell of cellsOf(c)) {
    const inner = cell.firstElementChild
    if (!inner) continue
    if (c.render === 'tags') {
      const chips = [...inner.children]
      w = Math.max(w, chips.reduce((n, t) => n + t.offsetWidth, 0) + rem(CHIP_GAP) * Math.max(0, chips.length - 1))
    } else w = Math.max(w, inner.scrollWidth)
  }
  return Math.min(MAX_FIT, Math.max(minWidth(c), Math.ceil(w) + 2))
}
function fit(list, grow = false) {
  for (const c of list) {
    const e = entryOf(c)
    if (!e) continue
    const w = fitWidth(c)
    if (!grow || w > e.w) e.w = w
  }
  saveUi()
}
// auto fit: what each column's content needs, by id; columns get it while the list is wide enough and share the room in proportion when it isn't
const wanted = {}
function share() {
  const parts = cols.value.map((c) => ({ e: entryOf(c), want: wanted[c.id] ?? entryOf(c).w, min: minWidth(c) })).filter((p) => p.e)
  for (const p of parts) p.min = Math.min(p.min, p.want)
  const want = parts.reduce((n, p) => n + p.want, 0)
  const min = parts.reduce((n, p) => n + p.min, 0)
  const k = want <= props.room ? 1 : Math.max(0, (props.room - min) / Math.max(1, want - min))
  let changed = false
  for (const p of parts) {
    // down, so the rounded widths never add up past the room
    const w = Math.floor(p.min + (p.want - p.min) * k)
    if (p.e.w === w) continue
    p.e.w = w
    changed = true
  }
  if (changed) saveUi()
}
// measuring only widens columns while the same results scroll by, so they don't jump; new results are measured afresh
let fitTimer = null
function autoFit(fresh) {
  if (!lay.value.listAutoFit) return
  clearTimeout(fitTimer)
  fitTimer = setTimeout(() => nextTick(() => {
    for (const c of cols.value) wanted[c.id] = fresh ? fitWidth(c) : Math.max(wanted[c.id] || 0, fitWidth(c))
    share()
  }), 150)
}
let roomFrame = 0
watch(() => props.room, () => {
  if (!lay.value.listAutoFit || roomFrame) return
  roomFrame = requestAnimationFrame(() => {
    roomFrame = 0
    share()
  })
})
watch(() => S.resultsVer, () => autoFit(true))
watch(() => S.cellsVer, () => autoFit(false))
watch(() => lay.value.listAutoFit, (on) => on && autoFit(true))

function resize(e, c, dir) {
  e.stopPropagation()
  const x0 = e.clientX
  const entry = entryOf(c)
  const w0 = entry.w
  const min = minWidth(c)
  const move = (ev) => (entry.w = Math.max(min, w0 + dir * (ev.clientX - x0)))
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
    // with auto fit, the width set by hand is what the column wants from then on
    if (lay.value.listAutoFit) wanted[c.id] = entry.w
    saveUi()
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
}
function dragColumn(e, i) {
  beginDrag(e, {
    drop: (t, zone) => {
      const list = lay.value.listCols
      const [x] = list.splice(i, 1)
      list.splice(finalIndex(i, t, zone), 0, x)
      saveUi()
    },
  }, cols.value[i].label, { sort: true, accept: (t) => t.startsWith('lcol:') })
}
const shown = computed(() => every.value.filter((c) => !q.value || fuzzy(c.label, q.value)))
const menuFor = ref(null)
function headMenu(e, c = null) {
  menuFor.value = c
  menu.value.show(e)
}
const menuItems = () => {
  const c = menuFor.value
  return [
    { label: c ? t('grid.header.hideColumn', { name: c.label }) : t('grid.header.hide'), icon: 'eye-off', visible: !!c, command: () => toggle(c) },
    { label: t('grid.header.fit'), icon: 'arrows-horizontal', visible: !!c, command: () => fit([c]) },
    { label: t('grid.header.fitAll'), icon: 'columns-3', command: () => fit(cols.value) },
    { label: t('grid.header.autoFit'), icon: 'wand', checked: !!lay.value.listAutoFit, command: () => { lay.value.listAutoFit = !lay.value.listAutoFit; saveUi() } },
    { separator: true },
    { label: t('grid.header.columns'), icon: 'plus', command: (ev) => pop.value.show(ev, head.value.querySelector('.lh-add')) },
  ]
}
</script>

<template>
  <div ref="head" class="list-head" @contextmenu.prevent="headMenu($event)">
    <div class="lh-inner" :style="{ width: contentWidth + 'px', transform: `translateX(${-offset}px)`, paddingLeft: pad + 'px', paddingRight: pad + Math.round(rem(ROW_END)) + 'px' }">
    <button class="lh name" :class="{ on: sortedBy(lay.sort, NAME_SORT) }" @click="clickColumn(NAME_SORT)">
      {{ $t('grid.header.name') }} <AppIcon v-if="sortedBy(lay.sort, NAME_SORT)" :name="lay.sort.desc ? 'arrow-down' : 'arrow-up'" :size="13" />
    </button>
    <div v-for="(c, i) in cols" :key="c.id" class="lh" :class="{ on: sortedBy(lay.sort, c), fixed: !c.sort }" :style="{ width: c.w + 'px' }" :data-col="c.id" :data-drop="'lcol:' + i" @pointerdown="dragColumn($event, i)" @click="clickColumn(c)" @contextmenu.prevent.stop="headMenu($event, c)">
      <span class="ellipsis">{{ c.label }}</span>
      <AppIcon v-if="sortedBy(lay.sort, c)" :name="lay.sort.desc ? 'arrow-down' : 'arrow-up'" :size="13" />
      <span v-if="i === 0" class="edge before" @pointerdown="resize($event, c, -1)" @click.stop />
      <span class="edge after" @pointerdown="resize($event, c, 1)" @click.stop @dblclick.stop="fit([c])" />
    </div>
    </div>
    <button class="icon-btn lh-add" :style="{ right: scrollbar + 2 + 'px' }" v-tooltip.bottom="$t('grid.header.columns')" @click="pop.toggle($event)"><AppIcon name="plus" :size="18" /></button>
    <Popover ref="pop" class="list-pop">
      <div class="col-menu">
        <AppListSearch v-focus v-model="q" />
        <div class="col-list">
          <label v-for="c in shown" :key="c.id" class="col-row">
            <Checkbox :modelValue="isOn(c.id)" binary @update:modelValue="toggle(c)" />
            <span class="ellipsis grow">{{ c.label }}</span>
            <AppIcon v-if="c.id.startsWith('field:')" name="forms" :size="14" class="faint" />
          </label>
        </div>
      </div>
    </Popover>
    <AppMenu ref="menu" :model="menuItems" />
  </div>
</template>

<style scoped>
.list-head { position: relative; flex: none; height: round(2.52rem, 1px); overflow: hidden; border-top: 1px solid var(--border); border-bottom: 1px solid var(--border); font-size: 1rem; color: var(--text-muted); }
.lh-inner { display: flex; align-items: center; gap: calc(0.84rem * var(--g)); height: 100%; }
.lh { position: relative; display: flex; align-items: center; gap: calc(0.28rem * var(--g)); flex: none; height: 100%; min-width: 0; padding: 0; border: 0; background: transparent; color: inherit; font: inherit; cursor: pointer; text-align: left; }
.lh.name { flex: 1; }
.lh.fixed { cursor: default; }
.lh.on { color: var(--text); font-weight: 500; }
.lh:hover:not(.fixed) { color: var(--text); }
.lh.drop-hover { color: var(--accent); }
/* the handle sits in the gap after the column and resizes that column; the first one's left edge, beside the name, resizes it too */
.edge { position: absolute; top: 0.42rem; bottom: 0.42rem; width: round(0.56rem, 1px); cursor: col-resize; }
.edge.after { right: -0.7rem; border-right: 1px solid transparent; }
.edge.before { left: -0.7rem; border-left: 1px solid transparent; }
.lh:hover .edge.after { border-right-color: var(--border); }
.lh:hover .edge.before { border-left-color: var(--border); }
.lh-add { position: absolute; top: 0.28rem; height: round(1.96rem, 1px); min-width: round(1.96rem, 1px); background: var(--bg); }
.col-menu { width: round(15.38rem, 1px); display: flex; flex-direction: column; font-size: 1rem; }
.col-list { max-height: 23.78rem; overflow-y: auto; padding: calc(0.28rem * var(--g)); }
.col-row { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); height: round(2.1rem, 1px); padding: 0 calc(0.42rem * var(--g)); border-radius: calc(5px * var(--r)); cursor: default; }
.col-row:hover { background: var(--hover); }
</style>

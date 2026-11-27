<script setup>
import { computed, inject } from 'vue'
import { S } from '../app/state.js'
import { L, activeZones, dockSide } from './arrangement.js'
import { baseOf, widgetInfo, ZONES, PANELS, PIN_TOP, PIN_BOTTOM } from './widgets.js'
import LayoutZoneSlot from './LayoutZoneSlot.vue'
import MacWindowButtons from '../app/MacWindowButtons.vue'

// One space of the window. mode: panel (sidebars, full width), bar (toolbars, in a row),
// rail (the compact sidebar: icons with menus that open on hover).
const props = defineProps({ name: String, mode: { type: String, default: 'panel' } })
const list = computed(() => activeZones()[props.name] || [])
// the zones a Mac's window buttons lead, chosen by App.vue: they belong to the window, not to the layout, so nothing in the editor moves or removes them
const lights = inject('lightsAt', null)
const leads = computed(() => !!lights?.value.has(props.name))
// the Inspector docked under the grid can spread its widgets out
const dock = computed(() => (props.name === 'inspector' && dockSide() === 'bottom' && !S.customize ? L().dockLayout || 'column' : null))
// spread over columns inside the scroller, so the columns grow downwards and scroll
const cols = computed(() => dock.value === 'grid' || dock.value === 'masonry')
const hasElastic = computed(() => list.value.some((id) => baseOf(id) === 'spacer'))

const isPin = (id) => id === PIN_TOP || id === PIN_BOTTOM

// lists and details next to each other share one padded column
function grouped(entries) {
  const out = []
  for (const e of entries) {
    const scroll = props.mode !== 'bar' && !!widgetInfo(e.id, S.contrib, S.sidebar.fields, S.actions)?.scroll
    const last = out[out.length - 1]
    if (scroll && last?.scroll) last.items.push(e)
    else out.push({ scroll, items: [e] })
  }
  return out
}
// a widget no longer known (a plugin's that is gone) would split the padded column it sits in
const entries = computed(() => list.value.map((id, i) => ({ id, i })).filter((e) => (S.customize || !isPin(e.id)) && widgetInfo(e.id, S.contrib, S.sidebar.fields, S.actions)))
// a sidebar: a pinned top, a middle that scrolls and a pinned bottom
const sections = computed(() => {
  if (props.mode === 'bar' || !PANELS.includes(props.name) || (dock.value && dock.value !== 'column')) return null
  const t = list.value.indexOf(PIN_TOP)
  const b = list.value.indexOf(PIN_BOTTOM)
  const midFrom = t + 1
  const midTo = b < 0 ? list.value.length : b
  const part = (from, to) => grouped(entries.value.filter((e) => e.i >= from && e.i < to))
  // each marker is the inner edge of its pinned part, where the edit mode draws it as a line to drag
  return [
    { kind: 'top', groups: part(0, midFrom) },
    { kind: 'mid', groups: part(midFrom, midTo) },
    { kind: 'bottom', groups: part(midTo, Infinity) },
  ]
})
const groups = computed(() => {
  if (dock.value && dock.value !== 'column') return [{ scroll: true, items: entries.value }]
  return grouped(entries.value)
})
</script>

<template>
  <div class="zone" data-drop-list :class="[name, mode, dock ? 'dock-' + dock : '', { customizing: S.customize, 'has-elastic': hasElastic }]" :style="dock ? { '--dock-w': (L().dockItemWidth || 300) + 'px', '--dock-cols': L().dockColumns || 3 } : {}">
    <MacWindowButtons v-if="leads" />
    <template v-if="sections">
      <div v-for="sec in sections" :key="sec.kind" class="zone-sec" :class="sec.kind">
        <template v-for="(g, gi) in sec.groups" :key="gi">
          <div v-if="g.scroll" class="zone-scroll">
            <LayoutZoneSlot v-for="p in g.items" :id="p.id" :key="p.id" :zone="name" :index="p.i" :mode="mode" />
          </div>
          <LayoutZoneSlot v-for="p in g.scroll ? [] : g.items" v-else :id="p.id" :key="p.id" :zone="name" :index="p.i" :mode="mode" />
        </template>
      </div>
    </template>
    <template v-for="(g, gi) in sections ? [] : groups" v-else :key="gi">
      <div v-if="g.scroll" class="zone-scroll">
        <div v-if="cols" class="dock-cols">
          <LayoutZoneSlot v-for="p in g.items" :id="p.id" :key="p.id" :zone="name" :index="p.i" :mode="mode" />
        </div>
        <LayoutZoneSlot v-for="p in cols ? [] : g.items" :id="p.id" :key="p.id" :zone="name" :index="p.i" :mode="mode" />
      </div>
      <LayoutZoneSlot v-for="p in g.scroll ? [] : g.items" v-else :id="p.id" :key="p.id" :zone="name" :index="p.i" :mode="mode" />
    </template>
    <div v-if="S.customize" class="slot-end" :class="{ empty: !list.length }" :data-drop="`zone:${name}:${list.length}`">{{ list.length ? '' : ZONES.has(name) ? $t('layout.zones.dropHereIn', { zone: $t(`layout.zones.${name}`) }) : $t('layout.zones.dropHere') }}</div>
  </div>
</template>

<style>
.zone { display: flex; min-width: 0; min-height: 0; }
.zone.panel, .zone.rail { flex-direction: column; height: 100%; }
.zone.bar { flex-direction: row; align-items: center; gap: calc(0.28rem * var(--g)); flex-wrap: wrap; }
.zone-scroll { flex: 0 1 auto; min-height: 0; overflow-y: auto; overflow-x: hidden; padding: 0 calc(0.7rem * var(--g)) calc(0.7rem * var(--g)); }
.zone-sec { display: flex; flex-direction: column; flex: none; min-height: 0; }
.zone-sec.mid { flex: 1 1 auto; overflow-y: auto; overflow-x: hidden; }
.zone-sec > .zone-scroll { flex: none; overflow: visible; }
/* under the grid the widgets fill the room */
.dock-grid > .zone-scroll, .dock-masonry > .zone-scroll, .dock-row > .zone-scroll { flex: 1; }
.rail .zone-scroll { display: flex; flex-direction: column; padding: 0 calc(0.42rem * var(--g)); }
.slot { display: contents; }
/* in a toolbar the filters take the room before an elastic gap does, so their chips stay on one line */
.bar .part-filters { display: flex; flex: 1000 1 0; min-width: 0; }
.bar .part-spacer { min-width: 0; }

/* nothing but room */
.room-gap { flex: none; width: round(1.96rem, 1px); height: round(1.96rem, 1px); }
.part-spacer { display: flex; flex: 1; min-width: round(0.56rem, 1px); min-height: round(0.56rem, 1px); }
.room-elastic { flex: 1; }
.room-line { flex: none; }
.panel .room-line, .rail .room-line { height: 1px; margin: calc(0.42rem * var(--g)) calc(0.7rem * var(--g)); background: var(--border); }
/* in a toolbar or a row of widgets the line stands upright */
.bar .room-line, .row-zone .room-line, .rail .row-zone .room-line { width: 1px; height: round(1.26rem, 1px); margin: 0 calc(0.42rem * var(--g)); background: var(--border); }
.rail .row-zone .room-line { width: round(1.26rem, 1px); height: 1px; margin: calc(0.42rem * var(--g)) auto; }
.bar .room-line { width: 1px; height: round(1.26rem, 1px); margin: 0 calc(0.28rem * var(--g)); background: var(--border); }
.zone-scroll .room-line { margin: calc(0.42rem * var(--g)) 0; }

/* inspector under the grid */
.dock-grid > .zone-scroll, .dock-masonry > .zone-scroll { padding-top: calc(0.56rem * var(--g)); }
.dock-grid .dock-cols { display: grid; grid-template-columns: repeat(var(--dock-cols), minmax(0, 1fr)); gap: 0 calc(1.26rem * var(--g)); align-items: start; }
.dock-grid .slot, .dock-masonry .slot, .dock-row .slot { display: block; min-width: 0; }
.dock-grid .part-w-batch, .dock-grid .part-w-summary { grid-column: 1 / -1; }
.dock-masonry .dock-cols { column-count: var(--dock-cols); column-gap: calc(1.26rem * var(--g)); }
.dock-masonry .slot { break-inside: avoid; }
.dock-row > .zone-scroll { display: flex; gap: calc(1.26rem * var(--g)); overflow-x: auto; overflow-y: hidden; padding-top: calc(0.56rem * var(--g)); }
.dock-row .slot { flex: none; width: var(--dock-w); height: 100%; overflow-y: auto; }

/* a row of widgets in a sidebar */
.slot.part-row { display: block; flex: none; }
.zone.row-zone { height: round(3.36rem * var(--row), 1px); flex-wrap: nowrap; padding: 0 calc(0.7rem * var(--g)); min-width: 0; }
.rail .zone.row-zone { flex-direction: column; height: auto; padding: calc(0.7rem * var(--g)) 0 calc(0.42rem * var(--g)); }
.rail .row-zone .part-spacer { display: none; }
/* widgets that start a sidebar away from the window's top edge get room above them */
.zone.library.panel > .zone-sec.top:not(:has(> .slot:first-child.part-row)) { padding-top: calc(0.7rem * var(--g)); }

/* the edit mode: the real widgets, each with a place to take hold of */
.zone.customizing .slot { display: block; position: relative; min-height: round(1.96rem, 1px); flex: none; }
.zone.customizing.bar .slot { display: inline-flex; align-items: center; flex: none; }
.zone.customizing.bar .slot.part-filters { display: flex; flex: 1 1 0; min-width: 0; }
.zone.customizing.bar .slot.part-libfilter { display: flex; flex: 1 1 0; min-width: 0; }
.zone.customizing.row-zone > .slot-end { flex: 0 0 0.84rem; min-width: round(0.84rem, 1px); padding: 0; }
.zone.customizing.row-zone > .slot-end.empty { flex: 1; min-width: 0; min-height: round(2.1rem, 1px); padding: 0 calc(0.42rem * var(--g)); justify-content: flex-start; white-space: nowrap; overflow: hidden; }
.zone.customizing.bar .slot.part-spacer { display: flex; flex: 1; min-width: round(2.8rem, 1px); min-height: round(1.96rem, 1px); }
.zone.customizing .part-spacer, .zone.customizing .part-gap, .zone.customizing .part-separator { outline: 1px dashed color-mix(in srgb, var(--text) 22%, transparent); outline-offset: -2px; border-radius: calc(5px * var(--r)); }
.zone.customizing .zone-scroll { flex: none; overflow: visible; }
.zone.customizing .zone-sec.mid { flex: 1 0 auto; overflow: visible; }
.pin-line { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); height: round(1.54rem, 1px); margin: 0.14rem calc(0.7rem * var(--g)); padding: 0 calc(0.42rem * var(--g)); border-top: 1px dashed var(--accent); color: var(--text-faint); font-size: 0.85rem; }
/* an elastic gap takes the spare room here too */
.zone.customizing.panel .slot.part-spacer { display: flex; flex: 1 0 1.96rem; }
.zone.customizing { outline: 1px dashed color-mix(in srgb, var(--accent) 55%, transparent); outline-offset: -2px; overflow-y: auto; overflow-x: hidden; }
.zone.customizing.row-zone { overflow: visible; min-height: round(2.8rem, 1px); }
.zone.customizing .part-spacer { min-height: round(1.96rem, 1px); }
.zone.customizing .slot.part-row { padding-left: calc(1.12rem * var(--g)); }
.edit-head { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); height: round(2.1rem, 1px); padding: 0 calc(0.56rem * var(--g)); color: var(--text-muted); font-size: 1rem; border-bottom: 1px solid var(--border); }
.slot-over { position: absolute; inset: 0; z-index: 40; border-radius: calc(5px * var(--r)); border: 1px solid transparent; cursor: grab; }
.slot-over:hover { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 10%, transparent); }
.slot-over.row-grip { inset: 0 auto 0 0; width: round(1.12rem, 1px); display: flex; align-items: center; justify-content: center; color: var(--text-faint); background: color-mix(in srgb, var(--text) 6%, transparent); }
.slot-x { all: unset; position: absolute; top: -0.35rem; right: -0.35rem; width: round(1.05rem, 1px); height: round(1.05rem, 1px); display: none; align-items: center; justify-content: center; border-radius: var(--round); background: var(--text-muted); color: var(--bg); cursor: pointer; }
.slot-over:hover .slot-x { display: inline-flex; }
.slot-over.row-grip .slot-x { top: 0; right: auto; left: 0; }
.slot.drop-hover { outline: none !important; background: none !important; }
.slot.drop-hover::before { content: ''; position: absolute; z-index: 41; left: 0; right: 0; top: -0.14rem; height: 0.21rem; border-radius: calc(2px * var(--r)); background: var(--accent); }
.zone.bar .slot.drop-hover::before { left: -0.14rem; right: auto; top: 0; bottom: 0; width: 0.21rem; height: auto; }
.slot-end { flex: 1; min-height: round(1.82rem, 1px); min-width: round(2.8rem, 1px); display: flex; align-items: center; justify-content: center; color: var(--text-faint); font-size: 0.92rem; padding: 0 calc(0.56rem * var(--g)); }
/* with an elastic gap the end needs no room of its own: a thin strip still takes drops */
.zone.has-elastic > .slot-end { flex: 0 0 0.7rem; min-width: round(0.7rem, 1px); min-height: round(0.7rem, 1px); padding: 0; }
.zone.customizing.filter-zone > .slot-end { flex: 0 0 0.7rem; min-width: round(0.7rem, 1px); padding: 0; }
/* an empty bar keeps a bar's height, with its hint on one line */
.zone.customizing.filter-zone > .slot-end.empty { flex: 1; min-width: 0; min-height: round(1.54rem, 1px); padding: 0 calc(0.28rem * var(--g)); justify-content: flex-start; white-space: nowrap; overflow: hidden; }
.slot-end.drop-hover { outline: 1px solid var(--accent) !important; }
</style>

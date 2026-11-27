<script setup>
import { computed, ref } from 'vue'
import { S } from '../app/state.js'
import { L, editZones } from './arrangement.js'
import { saveUi } from '../app/preferences.js'
import { baseOf, fitsZone, formOf, hidePart, orderPinMarkers, widgetInfo, placePart } from './widgets.js'
import { beginDrag } from '../app/dragDrop.js'
import AppIcon from '../shared/AppIcon.vue'
import BrowseWidgets from '../sidebar/BrowseWidgets.vue'
import ToolbarWidgets from './ToolbarWidgets.vue'
import ViewerTool from '../plugins/ViewerTool.vue'
import { supplied } from '../plugins/viewerTools.js'
import ItemWidgets from '../library/ItemWidgets.vue'
import SelectionWidgets from '../library/SelectionWidgets.vue'
import DisplaySpectrumMap from '../display/DisplaySpectrumMap.vue'
import TagWidgets from '../tags/TagWidgets.vue'
import FilterBar from '../filters/FilterBar.vue'
import LayoutZone from './LayoutZone.vue'
import PluginWidgets from '../plugins/PluginWidgets.vue'
import LayoutZoneSlotCollapsedWidget from './LayoutZoneSlotCollapsedWidget.vue'
import { widgetEmpty } from '../library/itemSelectors.js'

// One placed widget. While editing, the real widget itself can be dragged to another place, or onto
// the edit window to remove it; lists and details fold down to their headings so the layout is easy
// to read. Widgets about the selection only show while they apply.
// `popup`: drawn inside the popup of its own icon, open in full and with nothing of the edit mode.
const props = defineProps({ id: String, zone: String, index: Number, mode: String, popup: Boolean })
const BROWSE = ['header', 'libswitcher', 'libcard', 'favorites', 'smart', 'folders', 'tags', 'locations', 'libraries', 'panels', 'libfilter', 'b.menu', 'b.add', 'b.libswitch', 'b.library', 'm.quick', 'm.favorites', 'm.smart', 'm.folders', 'm.tags', 'm.locations', 'm.libraries']
const base = computed(() => baseOf(props.id))
const info = computed(() => widgetInfo(props.id, S.contrib, S.sidebar.fields, S.actions))
const applies = computed(() => {
  const w = info.value?.when
  if (!w) return true
  const n = S.selected.length
  return w === 'one' ? n === 1 : w === 'many' ? n > 1 : w === 'some' ? n > 0 : n === 0
})
// an icon for a widget that would show nothing for this item goes too, as the widget does in an open sidebar
const empty = computed(() => collapsed.value && !S.customize && !!info.value?.when && applies.value && (info.value.page ? pageEmpty.value : widgetEmpty(info.value, base.value)))
// a plugin's widget behind an icon stays loaded, out of sight, and is moved into the popup when it opens: its page then shows at once and can say it has nothing to show
const kept = computed(() => collapsed.value && applies.value && !!info.value?.page && !S.customize)
const keep = ref(null)
const pageEmpty = ref(false)
// the Filters button shows and hides the filters wherever they are placed, not only in the filter bar
const filtersOff = computed(() => base.value === 'filters' && L().filterbar === false && !S.customize)
// a viewer page's control shows while the page gives it
const toolOff = computed(() => !!info.value?.tool && !S.customize && !supplied(info.value.tool))
// a widget for one view of the items shows while they are in it
const viewOff = computed(() => !!info.value?.view && !S.customize && L().view !== info.value.view)
const headOnly = computed(() => !props.popup && S.customize && props.mode !== 'bar' && (info.value?.scroll || info.value?.when))
// too wide for where it sits: its icon, opening it in a popup (in a toolbar a widget about the selection waits, greyed; in a compact sidebar it hides like in an open one)
const collapsed = computed(() => {
  if (props.popup) return false
  const form = formOf(info.value)
  return (props.mode === 'rail' && (form === 'panel' || form === 'inline')) || (props.mode === 'bar' && form === 'panel')
})

function dragPart(e) {
  beginDrag(e, {
    drop: (target, where) => {
      const zones = editZones()
      if (target === 'palette') hidePart(zones, props.id)
      else {
        const [, zone, at] = target.split(':')
        placePart(zones, props.id, zone, Number(at) + (where === 'after' ? 1 : 0))
        orderPinMarkers(zones[zone])
      }
      saveUi()
    },
  }, info.value?.label || props.id, { sort: (t) => t.startsWith('zone:'), lift: e.currentTarget.closest('.slot'), accept: (t) => t === 'palette' || (t.startsWith('zone:') && fitsZone(info.value, t.split(':')[1])) })
}
function remove() {
  hidePart(editZones(), props.id)
  saveUi()
}
</script>

<template>
  <div v-if="info && (applies || S.customize || (collapsed && mode === 'bar')) && !empty && !filtersOff && !toolOff && !viewOff" class="slot" :class="['part-' + base.replace('.', '-'), { 'popup-part': popup }]" :data-drop="S.customize && !popup ? `zone:${zone}:${index}` : undefined">
    <LayoutZoneSlotCollapsedWidget v-if="collapsed" :info="info" :disabled="!applies && !S.customize" :keep="kept ? keep : null"><LayoutZoneSlot v-if="!kept" :id="id" :zone="zone" :index="index" mode="panel" popup /></LayoutZoneSlotCollapsedWidget>
    <div v-else-if="headOnly" class="edit-head"><AppIcon :name="info.icon" :size="16" /><span class="ellipsis grow">{{ info.label }}</span><AppIcon name="chevron-right" :size="15" class="faint" /></div>
    <template v-else>
      <div v-if="info.fixed" class="pin-line"><AppIcon name="pin" :size="12" />{{ base === 'pin.top' ? $t('layout.pinTop') : $t('layout.pinBottom') }}</div>
      <div v-else-if="base === 'gap'" class="room-gap" />
      <div v-else-if="base === 'spacer'" class="room-elastic" />
      <div v-else-if="base === 'separator'" class="room-line" />
      <LayoutZone v-else-if="base === 'row'" :name="id" mode="bar" class="row-zone" data-window-drag />
      <BrowseWidgets v-else-if="BROWSE.includes(base) || base.startsWith('q.') || base.startsWith('tab:')" :id="base" :mode="mode" />
      <PluginWidgets v-else-if="info.page" :id="id" :popup="popup" />
      <FilterBar v-else-if="base === 'filters'" :mode="mode" />
      <SelectionWidgets v-else-if="base === 'w.batch' || base === 'w.summary'" :many="base === 'w.batch'" />
      <DisplaySpectrumMap v-else-if="base === 'w.spectrum'" />
      <TagWidgets v-else-if="base === 't.best'" />
      <ItemWidgets v-else-if="info.when" :id="id" :popup="popup" />
      <ViewerTool v-else-if="info.tool" :id="info.tool.id" :tool="info.tool" />
      <ToolbarWidgets v-else :id="base" :mode="mode" />
    </template>
    <!-- the pinned edges stay where they are: widgets move across them -->
    <div v-if="S.customize && !popup && !info.fixed" class="slot-over" :class="{ 'row-grip': base === 'row' }" :title="info.label" @pointerdown="dragPart">
      <AppIcon v-if="base === 'row'" name="grip-vertical" :size="14" />
      <button class="slot-x" v-tooltip="$t('common.remove')" @pointerdown.stop @click="remove"><AppIcon name="x" :size="11" /></button>
    </div>
  </div>
  <Teleport v-if="kept" to="body">
    <div class="kept-widgets" inert><div ref="keep" class="kept-widget"><PluginWidgets :id="id" popup @empty="(v) => (pageEmpty = v)" /></div></div>
  </Teleport>
</template>

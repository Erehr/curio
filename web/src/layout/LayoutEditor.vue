<script setup>
import AppSegments from '../shared/AppSegments.vue'
import { computed, onMounted, reactive, ref } from 'vue'
import { t } from '../shared/i18n.js'
import { layoutLabel } from './layouts.js'
import { S } from '../app/state.js'
import { activeZones, currentScreen, editZones, resetZones, setEditAspect, currentAspect } from './arrangement.js'
import { saveUi } from '../app/preferences.js'
import { localizePlugins } from '../plugins/pluginText.js'
import { call } from '../platform/ipc.js'
import { GROUPS, WIDGETS, allScreens, fitsScreen, fitsZone, isPlaced, newInstance, widgetInfo, placePart, tabKey, toolKey, winKey } from './widgets.js'
import { beginDrag } from '../app/dragDrop.js'
import { fuzzy } from '../shared/text.js'
import Select from 'openvue/select'
import Button from 'openvue/button'
import AppIcon from '../shared/AppIcon.vue'
import AppSearchField from '../shared/AppSearchField.vue'

// The edit mode's window: every widget there is, as a grid of tiles or a list, ready to be dragged
// onto the real window. Each drag adds one more, so a widget can appear several times; dragging one
// back here removes it. Each screen (main, folder, tags, an open image…) has a layout of its own.
const pos = reactive({ x: null, y: 130 })
const width = computed(() => 640)
const q = ref('')
const group = ref('all')
const origin = ref('all')
// in a short window it opens higher, and it never runs past the window's foot, where its Done button is
const top = computed(() => Math.max(8, Math.min(pos.y, window.innerHeight - 360)))
const style = computed(() => ({ left: (pos.x ?? window.innerWidth / 2 - width.value / 2) + 'px', top: top.value + 'px', width: width.value + 'px', maxHeight: `min(620px, calc(100vh - ${top.value + 12}px))` }))

const all = computed(() => {
  // the font sample's field only while a plugin gives the list a Font sample column
  const fontColumn = (S.contrib.columns || []).some((c) => c.render === 'font')
  const out = Object.entries(WIDGETS).filter(([, p]) => p.group && (p.needs !== 'fontColumn' || fontColumn)).map(([id]) => ({ id, ...widgetInfo(id) }))
  for (const w of S.contrib.widgets) out.push({ id: w.id, ...widgetInfo(w.id, S.contrib, S.sidebar.fields) })
  for (const tab of S.contrib.tabs || []) out.push({ id: tabKey(tab), ...widgetInfo(tabKey(tab), S.contrib) })
  for (const w of S.contrib.windows || []) out.push({ id: winKey(w), hint: t('layout.editor.windowHint'), ...widgetInfo(winKey(w), S.contrib) })
  for (const tool of S.contrib.viewerTools || []) out.push({ id: toolKey(tool), hint: tool.plugin, ...widgetInfo(toolKey(tool), S.contrib) })
  for (const f of S.sidebar.fields.filter((x) => x.owner === 'user')) out.push({ id: 'field:' + f.key, ...widgetInfo('field:' + f.key, S.contrib, S.sidebar.fields) })
  for (const a of S.actions) out.push({ id: 'action:' + a.id, label: a.name, icon: a.icon || 'bolt', group: 'actions', hint: t('layout.editor.actionHint') })
  return out
})
// which plugin a widget comes from; Curio's own, and the built-in readers of metadata, are Curio's
const originOf = (p) => {
  const id = p.plugin?.plugin || p.tab?.plugin || p.tool?.plugin || p.win?.plugin
  return id && !id.startsWith('builtin.') ? id : 'curio'
}
const pluginNames = ref({})
onMounted(async () => {
  const list = await call('plugin_list').then(localizePlugins).catch(() => [])
  pluginNames.value = Object.fromEntries(list.map((x) => [x.id, x.name]))
})
const fits = computed(() => all.value.filter((p) => fitsScreen(p, currentScreen())))
const originOptions = computed(() => [['all', t('layout.editor.allSources')], ['curio', 'Curio'], ...[...new Set(fits.value.map(originOf))].filter((o) => o !== 'curio').map((o) => [o, pluginNames.value[o] || o]).sort((a, b) => a[1].localeCompare(b[1]))])
// only what can show on the screen being edited: selected items' widgets on the screens of item kinds, the rest where nothing is selected
const shown = computed(() => fits.value.filter((p) => (group.value === 'all' || p.group === group.value) && (origin.value === 'all' || originOf(p) === origin.value) && (!q.value || fuzzy(p.label, q.value))))
const sections = computed(() => GROUPS.map((id) => ({ id, label: t(`layout.groups.${id}`), rows: shown.value.filter((p) => p.group === id) })).filter((s) => s.rows.length))
const groupOptions = computed(() => [['all', t('layout.editor.allWidgets')], ...GROUPS.filter((id) => fits.value.some((p) => p.group === id)).map((id) => [id, t(`layout.groups.${id}`)])])
const used = (id) => isPlaced(activeZones(), id)

function dragNew(e, p) {
  beginDrag(e, {
    drop: (target, where) => {
      const zones = editZones()
      const [, zone, at] = target.split(':')
      const id = newInstance(zones, p.id)
      placePart(zones, id, zone, Number(at) + (where === 'after' ? 1 : 0))
      if (p.isRow && !zones[id]) zones[id] = []
      saveUi()
    },
  }, p.label, { sort: (t) => t.startsWith('zone:'), accept: (t) => t.startsWith('zone:') && fitsZone(p, t.split(':')[1]) })
}
// moved by any part of its edges that isn't a control
function moveWindow(e) {
  if (e.button !== 0 || e.target.closest('button, input, select, .p-select')) return
  const x0 = e.clientX - (pos.x ?? window.innerWidth / 2 - width.value / 2)
  const y0 = e.clientY - top.value
  const move = (ev) => {
    pos.x = Math.max(0, Math.min(window.innerWidth - width.value, ev.clientX - x0))
    pos.y = Math.max(0, Math.min(window.innerHeight - 60, ev.clientY - y0))
  }
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
}
const screens = computed(() => allScreens(S.contrib).map(([value, label, icon]) => ({ value, label, icon })))
const VIEWS = computed(() => [{ value: 'grid', label: t('layout.editor.tiles'), icon: 'layout-grid' }, { value: 'list', label: t('layout.editor.list'), icon: 'list' }])
// each shape edits the layout chosen for it in Settings › Layout, which may be the same one
const ASPECTS = computed(() => {
  const f = S.ui.layoutFor
  const name = (n) => (f.horizontal === f.vertical ? t('layout.editor.shared', { name: layoutLabel(n) }) : layoutLabel(n))
  return [{ value: 'horizontal', label: t('layout.editor.landscape', { name: name(f.horizontal) }), icon: 'device-desktop' }, { value: 'vertical', label: t('layout.editor.portrait', { name: name(f.vertical) }), icon: 'device-mobile' }]
})
function setView(v) {
  S.ui.editView = v
  saveUi()
}
</script>

<template>
  <div class="edit-window" :style="style" data-drop="palette">
    <div class="ew-head" @pointerdown="moveWindow">
      <!-- a list, as Format plugins add screens of their own -->
      <Select class="ew-sel" :modelValue="currentScreen()" :options="screens" optionLabel="label" optionValue="value" :ariaLabel="$t('layout.editor.screen')" @update:modelValue="(v) => (S.editScreen = v)">
        <template #value="{ value }"><span class="ew-opt"><AppIcon :name="screens.find((x) => x.value === value)?.icon || 'layout'" :size="15" />{{ screens.find((x) => x.value === value)?.label }}</span></template>
        <template #option="{ option }"><span class="ew-opt"><AppIcon :name="option.icon" :size="15" />{{ option.label }}</span></template>
      </Select>
      <Select v-model="origin" :options="originOptions" :optionLabel="(o) => o[1]" :optionValue="(o) => o[0]" class="ew-sel" :ariaLabel="$t('layout.editor.source')" />
      <Select v-model="group" :options="groupOptions" :optionLabel="(o) => o[1]" :optionValue="(o) => o[0]" class="ew-sel" :ariaLabel="$t('layout.editor.group')" />
    </div>
    <div class="ew-tools" @pointerdown="moveWindow">
      <AppSearchField v-model="q" :placeholder="$t('layout.editor.search')" />
      <AppSegments :modelValue="S.ui.editView === 'list' ? 'list' : 'grid'" :options="VIEWS" iconOnly :size="15" @update:modelValue="setView" />
      <AppSegments :modelValue="currentAspect()" :options="ASPECTS" iconOnly :size="15" @update:modelValue="setEditAspect" />
    </div>
    <div class="ew-list" :class="S.ui.editView === 'list' ? 'as-list' : 'as-tiles'">
      <template v-for="s in sections" :key="s.id">
        <div class="ew-group">{{ s.label }}</div>
        <div class="ew-rows">
          <div v-for="p in s.rows" :key="p.id" class="ew-part" :class="{ used: used(p.id) }" v-tooltip.bottom="{ value: p.hint, showDelay: 500 }" @pointerdown="dragNew($event, p)">
            <span v-if="p.id === 'search'" class="tile wide"><AppIcon name="search" :size="14" class="faint" /><i>{{ $t('layout.editor.searchTile') }}</i></span>
            <span v-else-if="p.id === 'zoom' || p.id === 'v.zoom' || p.tool?.kind === 'slider'" class="tile wide slider"><i class="knob" /></span>
            <span v-else class="tile"><AppIcon :name="p.icon" :size="20" /></span>
            <span class="ew-label">{{ p.label }}</span>
            <AppIcon v-if="used(p.id)" name="check" :size="13" class="ew-used" />
          </div>
        </div>
      </template>
      <div v-if="!sections.length" class="faint none">{{ $t('layout.editor.none') }}</div>
    </div>
    <div class="ew-foot" @pointerdown="moveWindow">
      <span class="grow faint">{{ $t('layout.editor.about') }}</span>
      <Button :label="$t('layout.editor.done')" size="small" @click="S.customize = false" />
      <Button :label="$t('layout.editor.reset')" severity="secondary" size="small" v-tooltip.top="$t('layout.editor.resetTip')" @click="resetZones" />
    </div>
  </div>
</template>

<style scoped>
.edit-window { position: fixed; z-index: 1500; display: flex; flex-direction: column; border-radius: calc(8px * var(--r)); background: var(--surface); border: 1px solid var(--float-border); font-size: 1rem; }
.edit-window.drop-hover { border-color: var(--danger); outline-color: var(--danger) !important; background: color-mix(in srgb, var(--danger) 10%, var(--surface)) !important; }
.ew-head { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); padding: calc(0.7rem * var(--g)) calc(0.84rem * var(--g)) calc(0.56rem * var(--g)); cursor: move; flex: none; }
.ew-sel { flex: 1 1 0; min-width: 0; }
.ew-tools { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); padding: 0 calc(0.84rem * var(--g)) calc(0.56rem * var(--g)); flex: none; cursor: move; }
.ew-tools .search-field { flex: 1; min-width: 0; cursor: auto; }
.ew-opt { display: inline-flex; align-items: center; gap: calc(0.56rem * var(--g)); }
.ew-list { flex: 1; min-height: round(8.39rem, 1px); overflow-y: auto; padding: 0 calc(0.84rem * var(--g)) calc(0.56rem * var(--g)); }
.ew-group { margin: calc(0.42rem * var(--g)) 0 calc(0.28rem * var(--g)); color: var(--text-faint); font-weight: 500; }
.as-tiles .ew-rows { display: grid; grid-template-columns: repeat(auto-fill, minmax(6.43rem, 1fr)); gap: calc(0.56rem * var(--g)) calc(0.28rem * var(--g)); }
.as-tiles .ew-part { position: relative; display: flex; flex-direction: column; align-items: center; gap: calc(0.28rem * var(--g)); padding: 0.14rem; text-align: center; cursor: grab; border-radius: calc(8px * var(--r)); }
.as-tiles .ew-part:hover .tile { border-color: var(--accent); }
.tile { display: inline-flex; align-items: center; justify-content: center; gap: calc(0.28rem * var(--g)); width: round(2.94rem, 1px); height: round(2.94rem, 1px); border-radius: calc(9px * var(--r)); background: var(--button); border: 1px solid color-mix(in srgb, var(--text) 12%, transparent); color: var(--text-muted); flex: none; }
.tile.wide { width: round(5.59rem, 1px); }
.tile i { font-style: normal; font-size: 0.92rem; color: var(--text-faint); }
.tile.slider { position: relative; }
.tile.slider::before { content: ''; position: absolute; left: 0.84rem; right: 0.84rem; top: 50%; height: 0.21rem; margin-top: -1px; border-radius: calc(2px * var(--r)); background: color-mix(in srgb, var(--text) 30%, transparent); }
.tile.slider .knob { position: relative; width: round(0.84rem, 1px); height: round(0.84rem, 1px); border-radius: var(--round); background: var(--text); }
.ew-label { line-height: 1.2; color: var(--text); overflow-wrap: anywhere; }
.ew-used { position: absolute; top: 0.14rem; right: 0.56rem; color: var(--accent); }
.as-list .ew-rows { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0 calc(0.56rem * var(--g)); }
.as-list .ew-part { position: relative; display: flex; align-items: center; gap: calc(0.56rem * var(--g)); height: round(2.1rem, 1px); padding: 0 calc(1.54rem * var(--g)) 0 calc(0.28rem * var(--g)); border-radius: calc(6px * var(--r)); cursor: grab; }
.as-list .ew-part:hover { background: var(--hover); }
.as-list .tile { width: round(1.68rem, 1px); height: round(1.68rem, 1px); border: 0; background: transparent; }
.as-list .tile.wide { width: round(1.68rem, 1px); overflow: hidden; }
.as-list .tile i { display: none; }
.as-list .tile.slider::before { left: 0.21rem; right: 0.21rem; }
.as-list .ew-label { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.as-list .ew-used { top: 0.56rem; right: 0.42rem; }
.none { padding: calc(1.26rem * var(--g)); text-align: center; }
.ew-foot { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); padding: calc(0.56rem * var(--g)) calc(0.84rem * var(--g)); border-top: 1px solid var(--border); font-size: 0.92rem; flex: none; cursor: move; }
</style>

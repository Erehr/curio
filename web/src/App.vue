<script setup>
import { computed, onMounted, onBeforeUnmount, provide, ref, watch } from 'vue'
import { S } from './app/state.js'
import { L, activeZones, dockSide, currentAspect, liveScreen } from './layout/arrangement.js'
import { saveUi } from './app/preferences.js'
import { requestSearch } from './library/results.js'
import { fileDropText } from './app/drops.js'
import { installDrops } from './app/dropHandler.js'
import { installKeyboard } from './app/keyboard.js'
import SelectButton from 'openvue/selectbutton'
import LayoutZone from './layout/LayoutZone.vue'
import DisplayGrid from './display/DisplayGrid.vue'
import DisplaySpectrum from './display/DisplaySpectrum.vue'
import ItemViewer from './viewer/ItemViewer.vue'
import TagsView from './tags/TagsView.vue'
import WelcomeScreen from './library/WelcomeScreen.vue'
import SettingsWindow from './settings/SettingsWindow.vue'
import TheDialogs from './app/TheDialogs.vue'
import TagFolderPicker from './tags/TagFolderPicker.vue'
import LayoutEditor from './layout/LayoutEditor.vue'
import WindowControls from './app/WindowControls.vue'
import MacWindowButtons from './app/MacWindowButtons.vue'
import PageOverlay from './plugins/PageOverlay.vue'
import TheToasts from './app/TheToasts.vue'
import { reportCodecs } from './formats/mediaEngine.js'
import ActionDialog from './actions/ActionDialog.vue'
import ApiAsk from './app/ApiAsk.vue'
import AppIcon from './shared/AppIcon.vue'
import { rem } from './shared/rem.js'
import PluginWindow from './plugins/PluginWindow.vue'
import ToolWindow from './tools/ToolWindow.vue'
import PluginFrame from './plugins/PluginFrame.vue'
import PageJobFrames from './plugins/PageJobFrames.vue'
import { startPageJobs } from './plugins/pageJobs.js'
import { PIN_TOP, baseOf, tabKey } from './layout/widgets.js'

const lay = computed(() => L())
const gridRef = ref(null)
// Tab plugins opened so far; a tab whose plugin went away returns the view to the grid
const visitedTabs = ref([])
const tabsNow = computed(() => (S.contrib.tabs || []).map((t) => ({ ...t, key: tabKey(t) })))
const openTabs = computed(() => tabsNow.value.filter((t) => visitedTabs.value.includes(t.key)))
watch(() => S.view, (v) => v.startsWith('tab:') && !visitedTabs.value.includes(v) && visitedTabs.value.push(v), { immediate: true })
watch(tabsNow, (list) => {
  if (S.view.startsWith('tab:') && !list.some((t) => t.key === S.view)) S.view = 'grid'
})
// an action or an address opened a tab for something: a tab already loaded hears plugin.run again, one loading gets it as it starts
const tabFrames = {}
const tabRunOf = (t) => (S.tabRun?.key === t.key ? S.tabRun.data : { tab: t.id })
watch(() => S.tabRun?.seq, () => S.tabRun && tabFrames[S.tabRun.key]?.run(S.tabRun.data))
const mergedTab = ref('library')
// the rail of icons a narrow sidebar becomes
const railWidth = () => Math.round(rem(3.5))
const SPLIT_MIN = 0.15

const showLibrary = computed(() => lay.value.library || (lay.value.merged && lay.value.inspector) || S.customize)
// a window too narrow for the grid beside both sidebars shows the Inspector as its icons, then the Library too, without changing either setting
const GRID_MIN_PX = 360
const winWidth = ref(window.innerWidth)
const trackWidth = () => (winWidth.value = window.innerWidth)
onMounted(() => window.addEventListener('resize', trackWidth))
onBeforeUnmount(() => window.removeEventListener('resize', trackWidth))
const fullWidth = (shown, w) => (shown ? w : 0)
// an Inspector docked under the grid or merged into the Library takes no width of its own
const inspectorBeside = () => lay.value.inspector && !lay.value.merged && side.value !== 'bottom'
const inspectorNarrow = computed(() => inspectorBeside() && winWidth.value - fullWidth(lay.value.library && !lay.value.libraryCompact, lay.value.libraryWidth) - fullWidth(!lay.value.inspectorCompact, lay.value.inspectorWidth) < GRID_MIN_PX)
const libraryNarrow = computed(() => winWidth.value - fullWidth(lay.value.library, lay.value.libraryWidth) - fullWidth(inspectorBeside(), railWidth()) < GRID_MIN_PX)
const compact = computed(() => (lay.value.libraryCompact || libraryNarrow.value) && !lay.value.merged && !S.customize)
// compact as icons, each opening its list or widget beside it
const rail = computed(() => compact.value)
// a hidden sidebar shows over the window while the pointer is at the window's edge on its side, with Settings › Layout
const REVEAL_OPEN_MS = 150
const REVEAL_CLOSE_MS = 280
const revealed = ref(null)
const revealEl = ref(null)
let revealTimer = null
let revealInside = false
const libraryHidden = computed(() => S.ui.edgeReveal && S.libOpen && !showLibrary.value && !S.customize)
const inspectorHidden = computed(() => S.ui.edgeReveal && S.libOpen && !lay.value.inspector && inspectorParts.value && !lay.value.merged && side.value !== 'bottom' && !S.customize)
function revealEnter(which) {
  revealInside = true
  clearTimeout(revealTimer)
  if (revealed.value !== which) revealTimer = setTimeout(() => (revealed.value = which), revealed.value ? 0 : REVEAL_OPEN_MS)
}
// it stays while something opened from it is in use: a menu, a popover, a picker, a field being typed in, or its scrollbar, which floats outside it
function revealBusy() {
  const el = document.activeElement
  return !!document.querySelector('.app-menu, .p-popover, .picker, .p-dialog-mask, .ov-thumb:hover, .ov-thumb.drag') || (!!revealEl.value?.contains(el) && /^(INPUT|TEXTAREA)$/.test(el.tagName))
}
function revealLeave() {
  revealInside = false
  clearTimeout(revealTimer)
  const later = () => (revealTimer = setTimeout(() => (revealInside ? null : revealBusy() ? later() : (revealed.value = null)), REVEAL_CLOSE_MS))
  later()
}
const revealSide = (which) => (which === 'library' ? lay.value.libraryDock || 'left' : side.value)
// the window's own layer is cut away under a revealed sidebar, so it lies on the window's material as a docked one does
const revealClip = computed(() => {
  if (!revealed.value) return null
  const w = revealed.value === 'library' ? lay.value.libraryWidth : lay.value.inspectorWidth
  return { clipPath: revealSide(revealed.value) === 'right' ? `inset(0 ${w}px 0 0)` : `inset(0 0 0 ${w}px)` }
})
const inspectorParts = computed(() => activeZones().inspector.length > 0)
const side = computed(() => dockSide())
// the width of the panes on the right, past which the window's controls end the toolbar
const rightPaneWidth = computed(() => {
  let w = 0
  if (showInspector.value && side.value === 'right') w += inspectorRail.value ? railWidth() : lay.value.inspectorWidth
  if (showLibrary.value && lay.value.libraryDock === 'right') w += compact.value ? railWidth() : lay.value.libraryWidth
  return w
})
// The window's controls sit in a strip above the pane at the window's right edge when it is full width; down to its icons, or with no pane there, they end the toolbar and the icons start at the top.
const controlsOver = computed(() => {
  if (S.ui.windowButtons === false) return false
  if (showLibrary.value && lay.value.libraryDock === 'right') return !compact.value
  if (showInspector.value && side.value === 'right') return !inspectorRail.value
  return false
})
// the sidebar at the window's top left at its full width
const leftPane = computed(() => {
  if (showLibrary.value && lay.value.libraryDock !== 'right') return compact.value ? null : 'library'
  if (showInspector.value && side.value === 'left') return inspectorRail.value ? null : 'inspector'
  return null
})
// a sidebar's row that opens its pinned top, so it never scrolls away
const firstRow = (zone) => {
  const list = activeZones()[zone]
  return list.length && baseOf(list[0]) === 'row' && list.includes(PIN_TOP) ? list[0] : null
}
// A Mac's window buttons lead the sidebar at the top left as a Mac app's own sidebar has them: in its first row, which the layout may move or remove, else in a strip of their own above it.
// With no sidebar there, or one down to its icons, they lead the toolbar.
const lightsAt = computed(() => {
  const at = new Set()
  if (!S.lights) return at
  const pane = S.itemWindow ? null : leftPane.value
  if (!pane) at.add('toolbar')
  else at.add((!(pane === 'library' && tabsOn.value) && firstRow(pane)) || 'strip:' + pane)
  if (revealed.value && revealSide(revealed.value) === 'left') at.add(firstRow(revealed.value) || 'strip:reveal')
  return at
})
provide('lightsAt', lightsAt)
// the Inspector as its widgets' icons, each opening its widget beside it
const inspectorRail = computed(() => (!!lay.value.inspectorCompact || inspectorNarrow.value) && side.value !== 'bottom' && !S.customize)
const showInspector = computed(() => (S.customize && !splitOn.value) || (lay.value.inspector && inspectorParts.value && !lay.value.merged && S.libOpen))
watch([libraryHidden, inspectorHidden], ([l, i]) => {
  if ((revealed.value === 'library' && !l) || (revealed.value === 'inspector' && !i)) revealed.value = null
})
// the right panel as the lower half of the left one
const splitOn = computed(() => !!lay.value.merged && lay.value.mergeMode === 'split' && (S.customize || (lay.value.inspector && inspectorParts.value && S.libOpen)))
const tabsOn = computed(() => !!lay.value.merged && lay.value.mergeMode !== 'split' && lay.value.inspector && inspectorParts.value)
const showFilter = computed(() => S.customize || (lay.value.filterbar && S.libOpen && !S.viewer && S.view === 'grid' && activeZones().filterbar.length > 0))

// ---------------------------------------------------------------- panel resizing
function resize(e, which) {
  const start = e.clientX
  const startY = e.clientY
  const l = L()
  const w0 = which === 'library' ? l.libraryWidth : which === 'inspector' ? l.inspectorWidth : l.inspectorHeight
  const pane = which === 'split' ? e.currentTarget.parentElement.getBoundingClientRect() : null
  const move = (ev) => {
    if (which === 'split') {
      l.splitRatio = Math.max(SPLIT_MIN, Math.min(1 - SPLIT_MIN, (ev.clientY - pane.top) / pane.height))
      return
    }
    if (which === 'inspector-h') {
      l.inspectorHeight = Math.max(140, Math.min(900, w0 - (ev.clientY - startY)))
      return
    }
    let d = ev.clientX - start
    if ((which === 'library' && l.libraryDock === 'right') || (which === 'inspector' && dockSide() !== 'left')) d = -d
    const v = Math.max(which === 'library' ? 180 : 200, Math.min(620, w0 + d))
    if (which === 'library') l.libraryWidth = v
    else l.inspectorWidth = v
  }
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
    saveUi()
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
}

installDrops()

// The web view's own menu (Back, Refresh, Print) would reload or leave the app. Text fields and selected text keep it for copy and paste.
window.addEventListener('contextmenu', (e) => {
  const text = e.target.closest?.('input, textarea, [contenteditable]') || String(window.getSelection?.() || '').trim()
  if (!e.defaultPrevented && !text) e.preventDefault()
})

// the main window tells FFmpeg what its player reads, and makes plugin pages' thumbnails
const stopWaiting = watch(() => S.ready, (ready) => {
  if (!ready) return
  if (!S.prefsWindow && !S.itemWindow && !S.pluginWindow && !S.toolWindow) {
    reportCodecs()
    startPageJobs()
  }
  queueMicrotask(() => stopWaiting())
}, { immediate: true })

// Something dropped on the window never makes it leave the app. Text boxes still take dropped text; files arrive through the program itself.
function keepPageOnDrop(e) {
  if (!e.target?.closest?.('input, textarea')) e.preventDefault()
}

let removeKeyboard
onMounted(() => {
  window.addEventListener('dragover', keepPageOnDrop)
  window.addEventListener('drop', keepPageOnDrop)
  removeKeyboard = installKeyboard({ grid: () => gridRef.value })
})
onBeforeUnmount(() => removeKeyboard())
watch(
  () => [
    lay.value.sort.key,
    lay.value.sort.field,
    lay.value.sort.desc,
    lay.value.view === 'spectrum' ? 'none' : lay.value.group?.key || 'none',
    lay.value.view === 'spectrum' ? '' : lay.value.group?.field || '',
    lay.value.view === 'spectrum' ? false : !!lay.value.group?.desc,
    !!lay.value.subfolders,
    lay.value.showNote && lay.value.view !== 'list',
  ],
  () => requestSearch(),
)
watch(() => S.customize, (on) => {
  if (on) S.editScreen = liveScreen()
  else {
    S.editAspect = null
    S.editScreen = null
  }
})
void currentAspect
</script>

<template>
  <!-- "Open in New Window": just the item -->
  <div v-if="S.ready && S.itemWindow" class="app"><main class="center viewing" style="--controls-pad: var(--wc-width, 0px)"><LayoutZone name="toolbar" mode="bar" class="top-zone" data-window-drag /><div class="content"><ItemViewer v-if="S.viewer" /></div></main><TheDialogs /><PageOverlay /><WindowControls /></div>

  <!-- a Window plugin's page -->
  <PluginWindow v-else-if="S.ready && S.pluginWindow" />

  <!-- one of Curio's tools -->
  <ToolWindow v-else-if="S.ready && S.toolWindow" />

  <!-- Settings, in its own window -->
  <div v-else-if="S.ready && S.prefsWindow" class="shell"><SettingsWindow /><TheDialogs /><TagFolderPicker /><ApiAsk /><PageOverlay /><WindowControls edges-only /><TheToasts /></div>

  <div v-else-if="S.ready" class="shell">
    <LayoutEditor v-if="S.customize" />
    <div class="app" :class="{ 'side-right': lay.libraryDock === 'right' }" :style="revealClip">
      <template v-if="showLibrary">
        <aside class="pane side" :class="{ 'rail-pane': rail, 'on-right': lay.libraryDock === 'right' }" :style="{ width: (compact ? railWidth() : lay.libraryWidth) + 'px', order: lay.libraryDock === 'right' ? 8 : 0 }">
          <!-- the window's controls over a full pane at the right edge, a Mac's window buttons over one on the left that no row there takes them in -->
          <div v-if="lay.libraryDock === 'right' && controlsOver" class="win-head" data-window-drag />
          <div v-else-if="lightsAt.has('strip:library')" class="win-head" data-window-drag><MacWindowButtons /></div>
          <template v-if="splitOn">
            <div class="split-part" :style="{ flex: `0 0 ${(lay.splitRatio || 0.5) * 100}%` }"><LayoutZone name="library" mode="panel" class="grow" /></div>
            <div class="resizer-h split-line" @pointerdown.prevent="resize($event, 'split')" />
            <div class="split-part grow"><LayoutZone name="inspector" class="grow" /></div>
          </template>
          <template v-else>
            <SelectButton v-if="tabsOn" v-model="mergedTab" class="panel-tabs" :options="[{ v: 'library', l: $t('layout.zones.library') }, { v: 'inspector', l: $t('layout.zones.inspector') }]" optionLabel="l" optionValue="v" :allowEmpty="false" />
            <LayoutZone v-if="tabsOn && mergedTab === 'inspector'" name="inspector" class="grow" />
            <LayoutZone v-else name="library" :mode="rail ? 'rail' : 'panel'" class="grow" />
          </template>
        </aside>
        <div v-if="!compact" class="resizer" :style="{ order: lay.libraryDock === 'right' ? 7 : 1 }" @pointerdown.prevent="resize($event, 'library')" />
      </template>
      <main class="center" :class="{ viewing: S.viewer }" :style="{ order: 4, '--controls-pad': controlsOver ? `max(0px, calc(var(--wc-width, 0px) - ${rightPaneWidth}px))` : 'var(--wc-width, 0px)' }">
        <LayoutZone name="toolbar" mode="bar" class="top-zone" data-window-drag />
        <LayoutZone v-if="showFilter" name="filterbar" mode="bar" class="filter-zone" />
        <div class="content">
          <WelcomeScreen v-if="!S.libOpen" />
          <template v-else>
            <TagsView v-if="!S.viewer && S.view === 'tags'" />
            <DisplaySpectrum v-else-if="!S.view.startsWith('tab:') && S.view !== 'tags' && lay.view === 'spectrum'" ref="gridRef" :class="{ behind: !!S.viewer }" :inert="!!S.viewer" />
            <DisplayGrid v-else-if="!S.view.startsWith('tab:') && S.view !== 'tags'" ref="gridRef" :class="{ behind: !!S.viewer }" :inert="!!S.viewer" />
            <ItemViewer v-if="S.viewer" />
          </template>
          <!-- a Tab plugin's page stays loaded once opened, so going back to it finds it as it was -->
          <div v-for="t in openTabs" v-show="S.libOpen && !S.viewer && S.view === t.key" :key="t.key" class="tab-page"><PluginFrame :ref="(el) => (tabFrames[t.key] = el)" :plugin="t.plugin" :page="t.page" role="tab" :visible="!S.viewer && S.view === t.key" :run="tabRunOf(t)" /></div>
          <div v-if="S.fileDrag" class="file-drop"><div class="file-drop-label"><AppIcon name="download" :size="22" />{{ fileDropText({ folder: S.fileDrag.folder, library: S.fileDrag.library }) }}</div></div>
        </div>
        <template v-if="showInspector && side === 'bottom'">
          <div class="resizer-h" @pointerdown.prevent="resize($event, 'inspector-h')" />
          <LayoutZone name="inspector" class="bottom-inspector" :style="{ height: lay.inspectorHeight + 'px' }" />
        </template>
      </main>
      <template v-if="showInspector && side !== 'bottom'">
        <div v-if="!inspectorRail" class="resizer" :style="{ order: side === 'left' ? 3 : 5 }" @pointerdown.prevent="resize($event, 'inspector')" />
        <aside class="pane side" :class="{ 'rail-pane': inspectorRail, 'on-right': inspectorRail && side === 'right' }" :style="{ width: (inspectorRail ? railWidth() : lay.inspectorWidth) + 'px', order: side === 'left' ? 2 : 6 }">
          <div v-if="side === 'right' && controlsOver && !inspectorRail" class="win-head" data-window-drag />
          <div v-else-if="lightsAt.has('strip:inspector')" class="win-head" data-window-drag><MacWindowButtons /></div>
          <LayoutZone name="inspector" :mode="inspectorRail ? 'rail' : 'panel'" class="grow" />
        </aside>
      </template>
    </div>
    <!-- the edges a hidden sidebar comes out of, and the sidebar itself, over the window -->
    <template v-for="which in ['library', 'inspector']" :key="which">
      <div v-if="which === 'library' ? libraryHidden : inspectorHidden" class="reveal-edge" :class="revealSide(which)" @pointerenter="revealEnter(which)" @pointerleave="revealLeave" />
    </template>
    <aside v-if="revealed" ref="revealEl" class="reveal side" :class="revealSide(revealed)" :style="{ width: (revealed === 'library' ? lay.libraryWidth : lay.inspectorWidth) + 'px' }" @pointerenter="revealEnter(revealed)" @pointerleave="revealLeave">
      <div v-if="revealSide(revealed) === 'right' && S.ui.windowButtons !== false" class="win-head" data-window-drag />
      <div v-else-if="lightsAt.has('strip:reveal')" class="win-head" data-window-drag><MacWindowButtons /></div>
      <LayoutZone :name="revealed" mode="panel" class="grow" />
    </aside>
    <TheDialogs />
    <TagFolderPicker />
    <ApiAsk />
    <PageOverlay />
    <PageJobFrames />
    <ActionDialog />
    <WindowControls main :right="controlsOver ? 0 : rightPaneWidth" />
    <TheToasts />
  </div>
</template>

<style scoped>
.tab-page { position: absolute; inset: 0; }
.shell { display: flex; flex-direction: column; height: 100vh; width: 100vw; }
.app { flex: 1; min-height: 0; display: flex; height: 100%; width: 100%; background: var(--bg); }
.pane { flex: none; height: 100%; }
.side { display: flex; flex-direction: column; background: var(--panel); min-width: 0; }
.panel-tabs { display: flex; flex: none; margin: calc(0.56rem * var(--g)) calc(0.7rem * var(--g)) calc(0.42rem * var(--g)); }
.panel-tabs :deep(.p-togglebutton) { flex: 1; }
.center { flex: 1; min-width: 0; display: flex; flex-direction: column; background: var(--bg); }
.top-zone { min-height: round(3.36rem * var(--row), 1px); flex: none; padding: 0.49rem calc(0.7rem + var(--controls-pad, 0px)) 0.49rem 0.7rem; }
.top-zone :deep(.icon-btn:not(.mini):not(.small)) { height: var(--control-h); min-width: var(--control-h); }
.win-head { display: flex; align-items: center; height: round(3.36rem, 1px); flex: none; padding-left: 0.7rem; }
/* A sidebar of icons that doesn't start with a row of buttons (which has its own room above) starts level with the toolbar's buttons (34px each with a 1px margin), or under the window's buttons where an open sidebar's first widget does. */
.side:not(:has(> .win-head)) > :deep(.zone.rail > .zone-sec.top:not(:has(> .slot:first-child.part-row))) { padding-top: calc(1.68rem * var(--row) - 1.26rem); }
.win-head + :deep(.zone.rail > .zone-sec.top:not(:has(> .slot:first-child.part-row))) { padding-top: calc(0.7rem * var(--g) - 1px); }
.rail-pane { border-right: 1px solid var(--border); }
.rail-pane.on-right { border-right: 0; border-left: 1px solid var(--border); }
/* a hidden sidebar, out over the window from its edge; the strip it comes out of reaches a little inside, so a window that isn't maximised needn't be met at its very last pixel */
.reveal-edge { position: fixed; top: 0; bottom: 0; z-index: 60; width: round(0.7rem, 1px); }
.reveal-edge.left { left: 0; }
.reveal-edge.right { right: 0; top: 3.36rem; }
.reveal { position: fixed; top: 0; bottom: 0; z-index: 70; height: auto; }
.reveal.left { left: 0; border-right: 1px solid var(--border); }
.reveal.right { right: 0; border-left: 1px solid var(--border); }
.filter-zone { flex: none; padding: 0.14rem calc(0.84rem * var(--g)) calc(0.56rem * var(--g)); min-height: round(2.1rem, 1px); }
.content { flex: 1; min-height: 0; position: relative; }
.behind { visibility: hidden; pointer-events: none; }
.resizer { flex: none; width: 0.28rem; margin: 0 -0.14rem; z-index: 5; cursor: col-resize; background: transparent; position: relative; }
.resizer::after { content: ''; position: absolute; left: 1px; top: 0; bottom: 0; width: 1px; background: var(--border); }
.resizer:hover::after { background: var(--accent); width: 0.14rem; }
.resizer-h { height: 0.28rem; margin: -0.14rem 0; cursor: row-resize; z-index: 5; position: relative; }
.resizer-h::after { content: ''; position: absolute; top: 1px; left: 0; right: 0; height: 1px; background: var(--border); }
.bottom-inspector { flex: none; border-top: 1px solid var(--border); background: var(--panel); }
.file-drop { position: absolute; inset: 0.56rem; z-index: 30; border-radius: calc(10px * var(--r)); background: color-mix(in srgb, var(--accent) 16%, transparent); display: flex; align-items: center; justify-content: center; pointer-events: none; }
.file-drop-label { border: 1px solid var(--border); display: inline-flex; align-items: center; gap: calc(0.7rem * var(--g)); padding: calc(0.7rem * var(--g)) calc(1.26rem * var(--g)); border-radius: calc(10px * var(--r)); background: var(--surface); color: var(--text); font-size: 1.15rem; }
.split-part { display: flex; flex-direction: column; min-height: 0; min-width: 0; }
.split-line { flex: none; }
</style>

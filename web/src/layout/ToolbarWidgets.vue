<script setup>
import { t } from '../shared/i18n.js'
import InputText from 'openvue/inputtext'
import InputGroup from 'openvue/inputgroup'
import InputGroupAddon from 'openvue/inputgroupaddon'
import { computed, reactive, ref } from 'vue'
import Slider from 'openvue/slider'
import Popover from 'openvue/popover'
import Select from 'openvue/select'
import ToggleSwitch from 'openvue/toggleswitch'
import Button from 'openvue/button'
import ButtonGroup from 'openvue/buttongroup'
import { badgeShown, L, cycleLibrary, cycleInspector, dockSide, folderViewState, resetFolderView } from './arrangement.js'
import { openPrefs, saveUi } from '../app/preferences.js'
import { S } from '../app/state.js'
import { goBack, goForward, scopeTitle, setScope } from '../library/navigation.js'
import { requestSearch, reshuffle, viewRequest } from '../library/results.js'
import { SEARCH_SCOPES, plainWords, searchableFields, sortsBy, fitsView, ownerName, filterQuery } from '../filters/query.js'
import { call } from '../platform/ipc.js'
import { toggleOnTop } from '../app/windowChrome.js'
import { stepViewer, itemFiles, stepFile, viewerFile, viewerPosition } from '../viewer/session.js'
import { withKey } from '../app/shortcuts.js'
import { closeViewer, fit, flip, isMarkdown, isPicture, isText, isVideo, liveClip, oneToOne, openExternally, pairFile, percent, revealItem, rotate, setTextView, textAsWritten, textView, toggleStrip, toggleText, V, zoomSlider } from '../viewer/viewerControls.js'
import { runUserAction } from '../actions/actions.js'
import AppIcon from '../shared/AppIcon.vue'
import AppSegments from '../shared/AppSegments.vue'
import PluginsMenu from '../plugins/PluginsMenu.vue'
import AppMenu from '../shared/AppMenu.vue'
import { subfoldersApply } from '../sidebar/sidebar.js'
import ActionButton from '../actions/ActionButton.vue'
import { openTool } from '../tools/tools.js'
import { actionFits, openPluginWindow, pluginActions, runPluginAction } from '../plugins/pluginActions.js'

// The small controls of the toolbar. Each can also sit in a sidebar.
const props = defineProps({ id: String, mode: { type: String, default: 'bar' } })
const lay = computed(() => L())
const layoutPop = ref()
const histPop = ref()
const pluginMenu = ref()
const stacked = computed(() => props.mode !== 'bar')

const VIEWS = computed(() => [['masonry', 'layout-dashboard'], ['justified', 'layout-rows'], ['grid', 'layout-grid'], ['list', 'list'], ['spectrum', 'color-filter']].map(([value, icon]) => ({ value, icon, label: t(`layout.views.${value}`) })))
// the kinds of file the view holds, read as the layout menu opens: Sort by offers what every item there can have
const viewHolds = ref({ exts: null, kinds: null })
async function readViewHolds() {
  const r = await call('filter_facets', { req: viewRequest() }).catch(() => null)
  viewHolds.value = r ? { exts: new Set(Object.keys(r.exts || {})), kinds: new Set(Object.keys(r.kinds || {}).filter((k) => r.kinds[k] > 0)) } : { exts: null, kinds: null }
}
const TIMED = ['video', 'audio']
const SORTS = computed(() => {
  const current = sortValue.value
  const fits = (value, ok) => ok || value === current
  const kinds = viewHolds.value.kinds
  const base = [['imported', 'calendar-plus'], ['modified', 'calendar-event'], ['created', 'calendar'], ['name', 'letter-case'], ['ext', 'file-code'], ['size', 'file'], ['resolution', 'dimensions'], ['rating', 'star'], ['duration', 'clock'], ['random', 'arrows-shuffle']].map(([value, icon]) => ({ value, icon, label: t(`layout.sorts.${value}`) })).filter((s) => s.value !== 'duration' || fits(s.value, !kinds || [...kinds].every((k) => TIMED.includes(k))))
  // nothing reorders a folder's items by hand, so its order (an Eagle library's) stays listed only while in use
  if (current === 'manual') base.unshift({ value: 'manual', label: t('layout.sorts.manual'), icon: 'hand-move' })
  for (const f of S.sidebar.fields.filter(sortsBy)) if (fits('field:' + f.key, fitsView(f, viewHolds.value.exts))) base.push({ value: 'field:' + f.key, label: f.label, icon: 'forms' })
  // a plugin's sort by a field switched off stays listed, saying so
  for (const s of S.contrib.sorts || []) if (s.field && !base.some((b) => b.value === 'field:' + s.field) && fits('field:' + s.field, fitsView(S.sidebar.fields.find((f) => f.key === s.field) || {}, viewHolds.value.exts))) base.push({ value: 'field:' + s.field, label: S.sidebar.fields.some((f) => f.key === s.field && f.indexed === false) ? t('layout.sorts.notIndexed', { name: s.label }) : s.label, icon: 'puzzle' })
  return base
})
const sortValue = computed({
  get: () => (lay.value.sort.key === 'field' ? 'field:' + lay.value.sort.field : lay.value.sort.key),
  set: (v) => {
    if (v.startsWith('field:')) lay.value.sort = { ...lay.value.sort, key: 'field', field: v.slice(6) }
    else lay.value.sort = { ...lay.value.sort, key: v, field: '' }
    saveUi()
  },
})
const DESC_GROUPS = new Set(['imported', 'modified', 'created', 'rating', 'size'])
const GROUPS = computed(() => {
  const current = groupValue.value
  const fits = (value, ok) => ok || value === current
  const base = [['none', 'minus'], ['kind', 'category'], ['ext', 'file-code'], ['imported', 'calendar-plus'], ['modified', 'calendar-event'], ['created', 'calendar'], ['rating', 'star'], ['size', 'file'], ['name', 'letter-case'], ['location', 'device-desktop']].map(([value, icon]) => ({ value, icon, label: t(`layout.groupBy.${value}`) }))
  for (const f of S.sidebar.fields.filter(sortsBy)) if (fits('field:' + f.key, fitsView(f, viewHolds.value.exts))) base.push({ value: 'field:' + f.key, label: f.label, icon: 'forms' })
  return base
})
const groupValue = computed({
  get: () => {
    const g = lay.value.group || { key: 'none', field: '' }
    return g.key === 'field' ? 'field:' + g.field : g.key || 'none'
  },
  set: (v) => {
    if (v.startsWith('field:')) {
      const key = v.slice(6)
      const f = S.sidebar.fields.find((x) => x.key === key)
      const desc = f?.type === 'date' || f?.type === 'datetime'
      lay.value.group = { key: 'field', field: key, desc }
    } else {
      lay.value.group = { key: v, field: '', desc: DESC_GROUPS.has(v) }
    }
    saveUi()
  },
})
const groupDesc = computed(() => !!lay.value.group?.desc)
function setGroupDesc(desc) {
  lay.value.group = { ...(lay.value.group || { key: 'none', field: '' }), desc }
  saveUi()
}
const CARD_TOGGLES = new Set(['showName', 'showExt', 'showInfo', 'showNote'])
const toggles = ['showName', 'showExt', 'showInfo', 'showNote', 'showSubfolders', 'subfolders']
// Spectrum draws no captions, and a list row always has its name and has columns for the rest: only the extension's toggle reaches it
const cardOff = (k) => CARD_TOGGLES.has(k) && (lay.value.view === 'spectrum' || (lay.value.view === 'list' && k !== 'showExt'))
const shownToggles = computed(() => toggles.filter((k) => !cardOff(k) && (k === 'showSubfolders' ? S.scope.kind !== 'folders' && subfoldersApply() : k !== 'subfolders' || subfoldersApply())))
// a sidebar as the toolbar's button cycles it: full, compact (its icons), hidden; the Inspector docked under the grid has no compact form
const SIDE_STATES = computed(() => ['full', 'icons', 'hidden'].map((value) => ({ value, label: t(`layout.sides.${value}`) })))
const sideState = (shown, icons) => (!shown ? 'hidden' : icons ? 'icons' : 'full')
const libraryState = computed({ get: () => sideState(lay.value.library, lay.value.libraryCompact), set: (v) => { lay.value.library = v !== 'hidden'; lay.value.libraryCompact = v === 'icons'; saveUi() } })
const inspectorState = computed({ get: () => sideState(lay.value.inspector, lay.value.inspectorCompact && dockSide() !== 'bottom'), set: (v) => { lay.value.inspector = v !== 'hidden'; lay.value.inspectorCompact = v === 'icons'; saveUi() } })
const inspectorStates = computed(() => (dockSide() === 'bottom' ? SIDE_STATES.value.filter((s) => s.value !== 'icons') : SIDE_STATES.value))
const badgeModel = computed({ get: () => badgeShown(lay.value), set: (v) => { lay.value.extBadge = v; saveUi() } })
const ownView = computed(() => folderViewState())
const sizeModel = computed({ get: () => lay.value.size, set: (v) => { lay.value.size = v; saveUi() } })
const recentWords = computed(() => S.sidebar.history.filter(plainWords).slice(0, 6))
function pickHistory(h) {
  S.search = h
  requestSearch()
  histPop.value.hide()
}
// ---------------------------------------------------------------- one search box for whatever is on show
const inTags = computed(() => S.view === 'tags' && !S.viewer)
const searchable = computed(() => S.customize || !S.viewer)
const searchText = computed({
  get: () => (inTags.value ? S.tagSearch : S.search),
  set: (v) => {
    if (inTags.value) S.tagSearch = v
    else S.search = v
  },
})
function searchTyped() {
  if (!inTags.value) requestSearch(350)
}
function clearSearch() {
  searchText.value = ''
  if (!inTags.value) requestSearch()
}
// fields come in groups by where they come from, folded until opened
const scopeGroups = computed(() => {
  const m = new Map()
  for (const f of searchableFields()) {
    if (!m.has(f.owner)) m.set(f.owner, { owner: f.owner, name: ownerName(f.owner), fields: [] })
    m.get(f.owner).fields.push(f)
  }
  return [...m.values()]
})
const openGroups = reactive({})
const groupOn = (g) => g.fields.filter((f) => scopeOn('field:' + f.key)).length
function toggleGroup(g) {
  const on = groupOn(g) < g.fields.length
  S.ui.searchScope = { ...S.ui.searchScope, ...Object.fromEntries(g.fields.map((f) => ['field:' + f.key, on])) }
  saveUi()
  if (S.search.trim()) requestSearch()
}

// ---------------------------------------------------------------- the tag manager's controls
const tagView = computed(() => (S.ui.tagView ||= { layout: 'grid', sort: 'az', groupBy: 'alphabet' }))
const tagSortPop = ref()
const TAG_LAYOUTS = computed(() => [{ value: 'grid', label: t('layout.tags.grid'), icon: 'layout-grid' }, { value: 'list', label: t('layout.tags.list'), icon: 'menu-2' }])
const TAG_SORTS = ['az', 'za', 'high', 'low']
function setTagView(k, v) {
  tagView.value[k] = v
  saveUi()
}

// ---------------------------------------------------------------- an opened item's controls
const viewerZoom = computed({ get: () => zoomSlider.value, set: (x) => (zoomSlider.value = x) })
const showTurn = computed(() => S.customize || isPicture.value || isVideo.value)
const showZoom = computed(() => S.customize || isPicture.value || isText.value)
function toggleFitActual() {
  if (!isPicture.value) return oneToOne()
  if (Math.abs(V.zoom - 1) < 0.01) oneToOne()
  else fit()
}
const TEXT_MODES = computed(() => [['rendered', 'markdown'], ['source', 'code'], ['split', 'layout-columns']].map(([value, icon]) => ({ value, icon, label: t(`layout.textModes.${value}`) })))
const viewPos = computed(() => {
  void S.resultsVer
  return S.viewer ? viewerPosition() : { pos: 0, length: 1 }
})
const viewCount = computed(() => (S.viewer ? `${viewPos.value.pos + 1} / ${viewPos.value.length.toLocaleString()}` : '1 / 1'))
// the files of an item holding several, as the viewer steps through them
const fileAt = computed(() => itemFiles().indexOf(viewerFile()))
const fileCount = computed(() => (itemFiles().length ? `${fileAt.value + 1} / ${itemFiles().length}` : '1 / 2'))
// plugins' buttons for the opened item's kind or extension (`where: viewer`)
const viewerActions = computed(() => (S.viewer ? pluginActions('viewer').filter((a) => !a.disabled && actionFits(a, [S.viewer.id])) : []))
void V

// ---------------------------------------------------------------- breadcrumb
const crumbs = computed(() => {
  const out = []
  const { kind, id } = S.scope
  const here = (label, go = null) => out.push({ label, go })
  if (S.view === 'tags') here(scopeTitle())
  else if (kind === 'folder') {
    const chain = []
    let f = S.sidebar.folders.find((x) => x.id === id)
    while (f) {
      chain.unshift(f)
      f = S.sidebar.folders.find((x) => x.id === f.parent_id)
    }
    here(t('layout.crumbs.all'), () => setScope('all'))
    for (const x of chain) here(x.name, () => setScope('folder', x.id))
  } else if (kind === 'root' || kind === 'rootdir') {
    const r = S.sidebar.roots.find((x) => x.id === id)
    here(t('layout.crumbs.all'), () => setScope('all'))
    here(r?.label || t('library.views.root'), () => setScope('root', id))
    if (kind === 'rootdir' && S.scope.path) {
      const parts = S.scope.path.split('/').filter(Boolean)
      parts.forEach((p, i) => {
        const sub = parts.slice(0, i + 1).join('/')
        here(p, () => setScope('rootdir', id, null, sub))
      })
    }
  } else if (kind === 'tag') {
    here(t('layout.crumbs.allTags'), () => {
      S.view = 'tags'
      S.viewer = null
    })
    here(scopeTitle().replace(/^# /, ''))
  } else if (kind === 'all') here(t('layout.crumbs.all'), () => setScope('all'))
  else {
    here(t('layout.crumbs.all'), () => setScope('all'))
    here(scopeTitle())
  }
  if (S.view === 'grid' && !S.viewer && (S.search.trim() || filterQuery())) here(t('layout.crumbs.results', { n: S.results.ids.length.toLocaleString() }))
  if (S.viewer) {
    const it = S.items.get(S.viewer.id)
    // a file grouped into the item comes after it, and the item's crumb goes back to its own file
    const file = S.viewer.file != null ? S.items.get(S.viewer.file) : null
    if (it) here(it.title || it.name, file ? () => (S.viewer.file = null) : null)
    if (file) here(file.title || file.name)
  }
  return out
})
const lastCrumb = (i) => i === crumbs.value.length - 1

// ---------------------------------------------------------------- an action as a button
const actionOf = computed(() => (props.id.startsWith('action:') ? S.actions.find((a) => a.id === props.id.slice(7)) : null))
// a Window plugin's window as a button
const winOf = computed(() => (props.id.startsWith('win:') ? (S.contrib.windows || []).find((w) => `win:${w.plugin}/${w.id}` === props.id) : null))
// the text the list's Font sample column and the font viewer write, while a plugin gives the list that column
const fontColumn = computed(() => (S.contrib.columns || []).some((c) => c.render === 'font'))
const fontSampleText = computed({ get: () => S.ui.fontSample?.text || '', set: (v) => { S.ui.fontSample = { text: v }; saveUi() } })
const scopeOn = (k) => S.ui.searchScope?.[k] !== false
function toggleScope(k) {
  S.ui.searchScope = { ...S.ui.searchScope, [k]: !scopeOn(k) }
  saveUi()
  if (S.search.trim()) requestSearch()
}
</script>

<template>
  <button v-if="id === 'librarytoggle'" v-show="!lay.library || S.customize" class="icon-btn" v-tooltip.bottom="$t('layout.tips.showLibrary')" @click="cycleLibrary"><AppIcon name="layout-sidebar" :size="18" /></button>

  <span v-else-if="id === 'nav'" class="row tight" :class="{ pad: stacked }">
    <template v-if="S.viewer">
      <button class="icon-btn step-btn" :disabled="viewPos.pos <= 0" v-tooltip.bottom="$t('layout.tips.previousItem')" @click="stepViewer(-1)"><AppIcon name="chevron-left" :size="18" /></button>
      <button class="icon-btn step-btn" :disabled="viewPos.pos >= viewPos.length - 1" v-tooltip.bottom="$t('layout.tips.nextItem')" @click="stepViewer(1)"><AppIcon name="chevron-right" :size="18" /></button>
    </template>
    <template v-else>
      <button class="icon-btn" :disabled="!S.back.length" v-tooltip.bottom="withKey($t('layout.tips.back'), 'nav.back')" @click="goBack"><AppIcon name="chevron-left" :size="18" /></button>
      <button class="icon-btn" :disabled="!S.fwd.length" v-tooltip.bottom="withKey($t('layout.tips.forward'), 'nav.forward')" @click="goForward"><AppIcon name="chevron-right" :size="18" /></button>
    </template>
  </span>

  <span v-else-if="id === 'title'" class="row tight title-wrap" :class="{ pad: stacked }">
    <span class="title ellipsis">{{ S.libOpen ? scopeTitle() : 'Curio' }}</span>
  </span>

  <nav v-else-if="id === 'breadcrumb'" class="crumbs" :class="{ pad: stacked }" :aria-label="$t('layout.crumbs.label')">
    <template v-for="(c, i) in crumbs" :key="i">
      <span v-if="i" class="crumb-sep">/</span>
      <button v-if="c.go && !lastCrumb(i)" class="crumb" @click="c.go()">{{ c.label }}</button>
      <span v-else class="crumb here ellipsis">{{ c.label }}</span>
    </template>
  </nav>

  <AppSegments v-else-if="id === 'tagview'" class="tag-seg" :modelValue="tagView.layout" :options="TAG_LAYOUTS" iconOnly @update:modelValue="(v) => setTagView('layout', v)" />

  <template v-else-if="id === 'tagsort'">
    <button class="icon-btn" v-tooltip.bottom="$t('layout.tags.sortTip')" @click="tagSortPop.toggle($event)"><AppIcon name="arrows-sort" :size="18" /></button>
    <Popover ref="tagSortPop">
      <div class="sort-menu">
        <div v-for="v in TAG_SORTS" :key="v" class="sort-row" :class="{ on: tagView.sort === v }" @click="setTagView('sort', v)">{{ $t(`layout.tags.sorts.${v}`) }}</div>
        <hr />
        <div class="section-title sort-title">{{ $t('layout.tags.groupBy') }}</div>
        <div class="sort-row" :class="{ on: tagView.groupBy === 'alphabet' }" @click="setTagView('groupBy', 'alphabet')">{{ $t('layout.tags.alphabet') }}</div>
        <div class="sort-row" :class="{ on: tagView.groupBy === 'groups' }" @click="setTagView('groupBy', 'groups')">{{ $t('layout.tags.groups') }}</div>
      </div>
    </Popover>
  </template>

  <button v-else-if="id === 'v.back'" class="icon-btn" v-tooltip.bottom="withKey($t('layout.tips.close'), 'item.open', 'Esc')" @click="S.viewer && closeViewer()"><AppIcon name="arrow-left" :size="18" /></button>
  <span v-else-if="id === 'v.count'" class="v-count-row">
    <span class="faint mono v-count">{{ viewCount }}</span>
    <button v-for="a in viewerActions" :key="a.plugin + a.id" class="icon-btn" :data-action="a.plugin + '::' + a.id" v-tooltip.bottom="a.title" @click="runPluginAction(a, [S.viewer.id]).catch(() => {})"><AppIcon :name="a.icon || 'puzzle'" :size="18" /></button>
  </span>
  <span v-else-if="id === 'v.group'" v-show="S.customize || itemFiles().length" class="row tight v-group">
    <button class="icon-btn file-btn" :disabled="fileAt <= 0" v-tooltip.bottom="withKey($t('layout.tips.previousFile'), 'viewer.fileBack')" :aria-label="$t('layout.tips.previousFile')" @click="stepFile(-1)"><AppIcon name="chevron-left" :size="18" /></button>
    <span class="faint mono v-files" v-tooltip.bottom="$t('layout.tips.files')"><AppIcon name="files" :size="15" />{{ fileCount }}</span>
    <button class="icon-btn file-btn" :disabled="fileAt >= itemFiles().length - 1" v-tooltip.bottom="withKey($t('layout.tips.nextFile'), 'viewer.fileAhead')" :aria-label="$t('layout.tips.nextFile')" @click="stepFile(1)"><AppIcon name="chevron-right" :size="18" /></button>
  </span>
  <span v-else-if="id === 'v.zoom'" v-show="showZoom" class="v-zoom">
    <Slider v-model="viewerZoom" :min="0" :max="100" :step="0.25" class="zoom-range" v-tooltip.bottom="$t('layout.tips.zoom')" />
    <button type="button" class="icon-btn mono pct" v-tooltip.bottom="isPicture ? $t('layout.tips.toggleZoom') : $t('layout.tips.actualSize')" @click="toggleFitActual">{{ percent }}%</button>
  </span>
  <button v-else-if="id === 'v.reveal'" class="icon-btn" v-tooltip.bottom="$t('layout.tips.reveal')" @click="revealItem"><AppIcon name="folder-open" :size="19" /></button>
  <button v-else-if="id === 'v.open'" class="icon-btn" v-tooltip.bottom="withKey($t('layout.tips.openExternal'), 'item.openExternal')" @click="openExternally"><AppIcon name="external-link" :size="19" /></button>
  <button v-else-if="id === 'v.live'" v-show="S.customize || liveClip" class="icon-btn" :class="{ on: V.live }" v-tooltip.bottom="$t('layout.tips.live')" @click="V.live = !V.live"><AppIcon name="live-photo" :size="19" /></button>
  <button v-else-if="id === 'v.pair'" v-show="S.customize || pairFile" class="icon-btn" :class="{ on: V.alt }" v-tooltip.bottom="$t('layout.tips.pair')" @click="V.alt = !V.alt"><AppIcon name="switch-horizontal" :size="19" /></button>
  <button v-else-if="id === 'v.flip'" v-show="showTurn" class="icon-btn" :class="{ on: V.flip }" v-tooltip.bottom="withKey($t('layout.tips.mirror'), 'viewer.flip')" @click="flip"><AppIcon name="flip-vertical" :size="19" /></button>
  <button v-else-if="id === 'v.rotate'" v-show="showTurn" class="icon-btn" v-tooltip.bottom="withKey($t('layout.tips.turn'), 'viewer.rotate')" @click="S.viewer && rotate()"><AppIcon name="rotate-clockwise-2" :size="19" /></button>
  <button v-else-if="id === 'v.actual'" v-show="showZoom" class="icon-btn" v-tooltip.bottom="isText ? withKey($t('layout.tips.actualSize'), 'viewer.zoomReset') : $t('layout.tips.actualSize')" @click="oneToOne"><AppIcon name="circle-dashed-number-1" :size="19" /></button>
  <button v-else-if="id === 'v.fit'" v-show="S.customize || isPicture" class="icon-btn" v-tooltip.bottom="withKey($t('layout.tips.fit'), 'viewer.zoomReset')" @click="fit"><AppIcon name="arrows-maximize" :size="19" /></button>
  <AppSegments v-else-if="id === 't.mode'" v-show="S.customize || isMarkdown" :modelValue="textView.mode" :options="TEXT_MODES" iconOnly @update:modelValue="(v) => v && setTextView('mode', v)" />
  <button v-else-if="id === 't.wrap'" v-show="S.customize || textAsWritten" class="icon-btn" :class="{ on: textView.wrap }" v-tooltip.bottom="withKey($t('layout.tips.wrap'), 'text.wrap')" @click="toggleText('wrap')"><AppIcon name="text-wrap" :size="19" /></button>
  <button v-else-if="id === 't.lines'" v-show="S.customize || textAsWritten" class="icon-btn" :class="{ on: textView.lines }" v-tooltip.bottom="withKey($t('layout.tips.lines'), 'text.lines')" @click="toggleText('lines')"><AppIcon name="list-numbers" :size="19" /></button>
  <button v-else-if="id === 'v.strip'" v-show="!S.itemWindow" class="icon-btn" :class="{ on: S.ui.viewerStrip }" v-tooltip.bottom="$t('layout.tips.strip')" @click="toggleStrip"><AppIcon :name="{ left: 'layout-sidebar', right: 'layout-sidebar-right' }[S.ui.viewerStripSide] || 'layout-bottombar'" :size="19" /></button>

  <button v-else-if="id === 'b.prefs'" class="icon-btn" v-tooltip.bottom="$t('layout.tips.settings')" @click="openPrefs()"><AppIcon name="settings" :size="18" /></button>
  <button v-else-if="id === 'b.edit'" class="icon-btn" v-tooltip.bottom="$t('layout.tips.editLayout')" @click="S.customize = true"><AppIcon name="layout-board" :size="18" /></button>
  <button v-else-if="id === 'shuffle' && (S.customize || (S.libOpen && S.scope.kind === 'random' && S.view === 'grid' && !S.viewer))" class="icon-btn" v-tooltip.bottom="$t('layout.tips.shuffle')" @click="reshuffle"><AppIcon name="refresh" :size="16" /></button>
  <button v-else-if="id === 'b.ontop'" class="icon-btn" :class="{ on: S.onTop }" v-tooltip.bottom="$t('layout.tips.onTop')" @click="toggleOnTop"><AppIcon :name="S.onTop ? 'pinned-filled' : 'pin'" :size="18" /></button>
  <button v-else-if="id === 'b.duplicates'" class="icon-btn" v-tooltip.bottom="$t('library.menu.items.findDuplicates')" @click="openTool('duplicates', S.selected, { action: 'find' })"><AppIcon name="copy-check" :size="18" /></button>
  <button v-else-if="id === 'b.rename'" class="icon-btn" :disabled="!S.customize && S.selected.length < 2" v-tooltip.bottom="withKey($t('library.menu.items.renameAll'), 'item.rename')" @click="openTool('rename', S.selected)"><AppIcon name="cursor-text" :size="18" /></button>
  <button v-else-if="id === 'b.compare'" class="icon-btn" :disabled="!S.customize && S.selected.length < 2" v-tooltip.bottom="$t('library.menu.items.compare')" @click="openTool('compare', S.selected.slice(0, 2))"><AppIcon name="layout-columns" :size="18" /></button>
  <button v-else-if="id === 'b.inspector'" class="icon-btn" v-tooltip.bottom="!lay.inspector ? $t('layout.tips.showInspector') : lay.inspectorCompact || dockSide() === 'bottom' ? $t('layout.tips.hideInspector') : $t('layout.tips.compactInspector')" @click="cycleInspector"><AppIcon name="layout-sidebar-right" :size="18" /></button>

  <button v-else-if="winOf" class="icon-btn" :class="{ wide: stacked }" v-tooltip.bottom="winOf.title" @click="openPluginWindow(winOf.plugin, winOf.id)"><AppIcon :name="winOf.icon || 'app-window'" :size="18" /><span v-if="stacked" class="action-name ellipsis">{{ winOf.title }}</span></button>
  <button v-else-if="actionOf" class="icon-btn" :class="{ wide: stacked }" v-tooltip.bottom="actionOf.name" @click="runUserAction(actionOf)"><AppIcon :name="actionOf.icon || 'bolt'" :size="18" /><span v-if="stacked" class="action-name ellipsis">{{ actionOf.name }}</span></button>

  <div v-else-if="id === 'zoom' && S.libOpen" class="zoom row" :class="{ pad: stacked }">
    <button class="icon-btn small" @click="sizeModel = Math.max(60, sizeModel - 20)"><AppIcon name="minus" :size="14" /></button>
    <Slider v-model="sizeModel" :min="60" :max="640" class="size-slider" />
    <button class="icon-btn small" @click="sizeModel = Math.min(640, sizeModel + 20)"><AppIcon name="plus" :size="14" /></button>
  </div>

  <div v-else-if="id === 'spacer'" class="spacer" />

  <template v-else-if="id === 'plugins' && S.libOpen">
    <button id="plugins-btn" class="icon-btn" v-tooltip.bottom="$t('layout.tips.plugins')" @click="pluginMenu.toggle($event)"><AppIcon name="puzzle" :size="18" /></button>
    <PluginsMenu ref="pluginMenu" />
  </template>

  <ActionButton v-else-if="id === 'actions' && S.libOpen" />


  <template v-else-if="id === 'layout'">
    <button id="layout-btn" class="icon-btn" v-tooltip.bottom="$t('layout.tips.layout')" @click="layoutPop.toggle($event)"><AppIcon name="layout" :size="18" /></button>
    <Popover ref="layoutPop" @show="readViewHolds">
      <div class="layout-pop">
        <div class="lp-row"><span>{{ $t('layout.pop.layout') }}</span>
          <div class="row">
            <button v-if="ownView" class="icon-btn" v-tooltip="$t('layout.pop.resetTo', { name: ownView.inherits || $t('layout.crumbs.all') })" @click="resetFolderView"><AppIcon name="restore" :size="18" /></button>
            <Select v-model="lay.view" :options="VIEWS" optionLabel="label" optionValue="value" class="lp-select" @change="saveUi">
              <template #value="{ value }"><span class="row"><AppIcon :name="VIEWS.find((v) => v.value === value)?.icon || 'layout'" /> {{ VIEWS.find((v) => v.value === value)?.label }}</span></template>
              <template #option="{ option }"><span class="row"><AppIcon :name="option.icon" /> {{ option.label }}</span></template>
            </Select>
          </div>
        </div>
        <div class="lp-row"><span>{{ $t('layout.pop.size') }}</span>
          <span class="lp-zoom"><button class="icon-btn small" @click="sizeModel = Math.max(60, sizeModel - 20)"><AppIcon name="minus" :size="14" /></button><Slider v-model="sizeModel" :min="60" :max="640" class="lp-size" /><button class="icon-btn small" @click="sizeModel = Math.min(640, sizeModel + 20)"><AppIcon name="plus" :size="14" /></button></span>
        </div>
        <hr />
        <div v-if="lay.view === 'spectrum'" class="lp-row"><span>{{ $t('layout.pop.sortBy') }}</span>
          <InputGroup class="lp-sort">
            <Select :modelValue="'color'" :options="[{ value: 'color', label: $t('layout.pop.color'), icon: 'palette' }]" optionLabel="label" optionValue="value" class="lp-select" disabled>
              <template #value><span class="row"><AppIcon name="palette" /> {{ $t('layout.pop.color') }}</span></template>
            </Select>
            <InputGroupAddon><button class="addon-btn" :class="{ on: !lay.spectrumDark }" v-tooltip="$t('layout.pop.lightMiddle')" :aria-label="$t('layout.pop.lightMiddle')" @click="lay.spectrumDark = false; saveUi()"><AppIcon name="sort-ascending" :size="17" /></button></InputGroupAddon>
            <InputGroupAddon><button class="addon-btn" :class="{ on: lay.spectrumDark }" v-tooltip="$t('layout.pop.darkMiddle')" :aria-label="$t('layout.pop.darkMiddle')" @click="lay.spectrumDark = true; saveUi()"><AppIcon name="sort-descending" :size="17" /></button></InputGroupAddon>
          </InputGroup>
        </div>
        <div v-else class="lp-row"><span>{{ $t('layout.pop.sortBy') }}</span>
          <InputGroup class="lp-sort">
            <Select v-model="sortValue" :options="SORTS" optionLabel="label" optionValue="value" class="lp-select" filter autoFilterFocus>
              <template #value="{ value }"><span class="row"><AppIcon :name="SORTS.find((v) => v.value === value)?.icon || 'arrows-sort'" /> {{ SORTS.find((v) => v.value === value)?.label }}</span></template>
              <template #option="{ option }"><span class="row"><AppIcon :name="option.icon" /> {{ option.label }}</span></template>
            </Select>
            <InputGroupAddon><button class="addon-btn" :class="{ on: !lay.sort.desc }" v-tooltip="$t('layout.pop.ascending')" :aria-label="$t('layout.pop.ascending')" @click="lay.sort = { ...lay.sort, desc: false }; saveUi()"><AppIcon name="sort-ascending" :size="17" /></button></InputGroupAddon>
            <InputGroupAddon><button class="addon-btn" :class="{ on: lay.sort.desc }" v-tooltip="$t('layout.pop.descending')" :aria-label="$t('layout.pop.descending')" @click="lay.sort = { ...lay.sort, desc: true }; saveUi()"><AppIcon name="sort-descending" :size="17" /></button></InputGroupAddon>
          </InputGroup>
        </div>
        <div v-if="lay.view !== 'spectrum'" class="lp-row"><span>{{ $t('layout.pop.groupBy') }}</span>
          <InputGroup class="lp-sort">
            <Select v-model="groupValue" :options="GROUPS" optionLabel="label" optionValue="value" class="lp-select" filter autoFilterFocus>
              <template #value="{ value }"><span class="row"><AppIcon :name="GROUPS.find((v) => v.value === value)?.icon || 'category'" /> {{ GROUPS.find((v) => v.value === value)?.label }}</span></template>
              <template #option="{ option }"><span class="row"><AppIcon :name="option.icon" /> {{ option.label }}</span></template>
            </Select>
            <InputGroupAddon><button class="addon-btn" :class="{ on: !groupDesc && groupValue !== 'none' }" :disabled="groupValue === 'none'" v-tooltip="$t('layout.pop.ascending')" :aria-label="$t('layout.pop.ascending')" @click="setGroupDesc(false)"><AppIcon name="sort-ascending" :size="17" /></button></InputGroupAddon>
            <InputGroupAddon><button class="addon-btn" :class="{ on: groupDesc && groupValue !== 'none' }" :disabled="groupValue === 'none'" v-tooltip="$t('layout.pop.descending')" :aria-label="$t('layout.pop.descending')" @click="setGroupDesc(true)"><AppIcon name="sort-descending" :size="17" /></button></InputGroupAddon>
          </InputGroup>
        </div>
        <template v-if="shownToggles.length || lay.view !== 'spectrum'">
          <hr />
          <div v-for="k in shownToggles" :key="k" class="lp-row"><span>{{ $t(`layout.toggles.${k}`) }}</span><ToggleSwitch v-model="lay[k]" @change="saveUi" /></div>
          <div v-if="lay.view !== 'spectrum' && lay.view !== 'list'" class="lp-row"><span>{{ $t('layout.pop.badges') }}</span><ToggleSwitch v-model="badgeModel" /></div>
        </template>
        <hr />
        <div class="lp-row"><span>{{ $t('layout.pop.library') }}</span><AppSegments v-model="libraryState" :options="SIDE_STATES" class="lp-side" /></div>
        <div class="lp-row"><span>{{ $t('layout.pop.inspector') }}</span><AppSegments v-model="inspectorState" :options="inspectorStates" class="lp-side" /></div>
        <ButtonGroup class="lp-edit">
          <Button :label="$t('layout.pop.edit')" severity="secondary" class="lp-edit-main" @click="S.customize = true; layoutPop.hide()" />
          <Button severity="secondary" v-tooltip.bottom="$t('layout.pop.more')" :aria-label="$t('layout.pop.more')" @click="openPrefs('layout'); layoutPop.hide()"><template #icon><AppIcon name="settings" :size="17" /></template></Button>
        </ButtonGroup>
      </div>
    </Popover>
  </template>

  <button v-else-if="id === 'filtertoggle' && S.libOpen" class="icon-btn" :class="{ on: lay.filterbar }" v-tooltip.bottom="withKey($t('layout.tips.filters'), 'filters.toggle')" @click="lay.filterbar = !lay.filterbar; saveUi()"><AppIcon name="filter" :size="18" /></button>

  <template v-else-if="id === 'search' && S.libOpen && searchable">
    <InputGroup class="search" :class="{ error: S.queryError && !inTags, stacked }">
      <InputGroupAddon><button v-if="!inTags" class="addon-btn" v-tooltip.bottom="$t('layout.search.scope')" :aria-label="$t('layout.search.scope')" @click="histPop.toggle($event)"><AppIcon name="search" :size="16" /></button><AppIcon v-else name="search" :size="16" /></InputGroupAddon>
      <InputText id="search-box" v-model="searchText" :placeholder="$t('layout.search.placeholder')" spellcheck="false" @input="searchTyped" @keydown.enter="!inTags && requestSearch()" @keydown.esc="clearSearch" />
      <InputGroupAddon v-if="searchText"><button class="addon-btn" :aria-label="$t('layout.search.clear')" @click="clearSearch"><AppIcon name="x" :size="14" /></button></InputGroupAddon>
    </InputGroup>
    <Popover ref="histPop">
      <div class="hist">
        <div class="section-title scope-title">{{ $t('layout.search.in') }}</div>
        <div v-for="[k, ic] in SEARCH_SCOPES" :key="k" class="scope-row" @click="toggleScope(k)">
          <AppIcon :name="ic" :size="18" /><span class="grow">{{ $t(`filters.scopes.${k}`) }}</span>
          <AppIcon v-if="scopeOn(k)" name="check" :size="16" />
        </div>
        <template v-if="scopeGroups.length">
          <hr />
          <div class="scope-fields">
            <template v-for="g in scopeGroups" :key="g.owner">
              <div class="scope-row" @click="openGroups[g.owner] = !openGroups[g.owner]">
                <AppIcon :name="openGroups[g.owner] ? 'chevron-down' : 'chevron-right'" :size="16" /><span class="grow ellipsis">{{ g.name }}</span>
                <span class="count">{{ groupOn(g) }}/{{ g.fields.length }}</span>
                <button class="icon-btn mini" v-tooltip="groupOn(g) < g.fields.length ? $t('layout.search.selectAll') : $t('layout.search.selectNone')" @click.stop="toggleGroup(g)"><AppIcon :name="groupOn(g) === g.fields.length ? 'square-check' : groupOn(g) ? 'square-minus' : 'square'" :size="16" /></button>
              </div>
              <template v-if="openGroups[g.owner]">
                <div v-for="f in g.fields" :key="f.key" class="scope-row sub" @click="toggleScope('field:' + f.key)">
                  <span class="grow ellipsis">{{ f.label }}</span>
                  <AppIcon v-if="scopeOn('field:' + f.key)" name="check" :size="16" />
                </div>
              </template>
            </template>
          </div>
        </template>
        <template v-if="recentWords.length">
          <hr />
          <div class="section-title scope-title">{{ $t('layout.search.recent') }}</div>
          <div v-for="h in recentWords" :key="h" class="hist-row ellipsis" @click="pickHistory(h)">{{ h }}</div>
        </template>
      </div>
    </Popover>
  </template>

  <InputGroup v-else-if="id === 'fontsample' && S.libOpen && (fontColumn || S.customize)" class="search" :class="{ stacked }">
    <InputGroupAddon><AppIcon name="typography" :size="16" /></InputGroupAddon>
    <InputText v-model="fontSampleText" :placeholder="$t('layout.fontSample.placeholder')" :aria-label="$t('layout.fontSample.placeholder')" spellcheck="false" @keydown.esc="fontSampleText = ''" />
    <InputGroupAddon v-if="fontSampleText"><button class="addon-btn" :aria-label="$t('layout.fontSample.clear')" @click="fontSampleText = ''"><AppIcon name="x" :size="14" /></button></InputGroupAddon>
  </InputGroup>

  <button v-else-if="id === 'inspectortoggle' && S.libOpen" v-show="!lay.inspector || S.customize" class="icon-btn" v-tooltip.bottom="$t('layout.tips.showInspector')" @click="lay.inspector = true; saveUi()"><AppIcon name="layout-sidebar-right" :size="18" /></button>
</template>

<style scoped>
.tight { gap: calc(0.28rem * var(--g)); }
.crumbs { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); min-width: 0; margin-left: calc(0.56rem * var(--g)); font-size: 1.08rem; }
.crumb { border: 0; padding: 0.14rem calc(0.28rem * var(--g)); border-radius: calc(5px * var(--r)); background: transparent; color: var(--text-muted); font: inherit; cursor: pointer; white-space: nowrap; }
.crumb:hover { background: var(--hover); color: var(--text); }
.crumb.here { padding: 0.14rem calc(0.28rem * var(--g)); color: var(--text); font-weight: 500; max-width: 30vw; }
.crumb-sep { color: var(--text-faint); }
.sort-menu { width: round(15.38rem, 1px); font-size: 1rem; }
.sort-row { display: flex; align-items: center; height: round(2.1rem, 1px); padding: 0 calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); cursor: default; }
.sort-row:hover { background: var(--hover); }
.sort-row.on { background: var(--selection); }
.sort-title { padding: calc(0.28rem * var(--g)) calc(0.56rem * var(--g)); }
.v-count { margin-left: calc(0.42rem * var(--g)); white-space: nowrap; }
.v-count-row { display: inline-flex; align-items: center; gap: calc(0.28rem * var(--g)); }
.v-group { margin-left: calc(0.56rem * var(--g)); }
.v-files { display: inline-flex; align-items: center; gap: calc(0.28rem * var(--g)); white-space: nowrap; }
.v-zoom { display: inline-flex; align-items: center; gap: calc(0.56rem * var(--g)); margin-left: calc(1.26rem * var(--g)); }
.zoom-range { width: round(9.09rem, 1px); margin: 0 calc(0.56rem * var(--g)); }
.pct { min-width: round(3.08rem, 1px); }
.icon-btn.wide { width: 100%; justify-content: flex-start; gap: calc(0.56rem * var(--g)); padding: 0 calc(0.56rem * var(--g)); }
.action-name { min-width: 0; }
.pad { padding: calc(0.28rem * var(--g)) calc(0.7rem * var(--g)); flex: none; }
.spacer { flex: 1; min-width: round(0.56rem, 1px); }
.title { font-weight: 500; font-size: 1.08rem; margin-left: calc(0.56rem * var(--g)); max-width: 30vw; }
.title-wrap { min-width: 0; }
.zoom { margin-left: calc(1.68rem * var(--g)); gap: 0.14rem; }
.zoom.pad { margin-left: 0; }
.size-slider { width: round(6.99rem, 1px); margin: 0 calc(0.56rem * var(--g)); }
.zoom.pad .size-slider { flex: 1; }
.icon-btn.small { min-width: round(1.54rem, 1px); height: round(1.68rem, 1px); padding: 0 calc(0.21rem * var(--g)); gap: 1px; }
/* narrows with the window, down to a small box, before the toolbar wraps it onto a row of its own */
.search { flex: 1 1 9.79rem; width: auto; max-width: 18.18rem; height: var(--control-h); margin-left: calc(0.42rem * var(--g)); }
.search.stacked { width: auto; margin: calc(0.42rem * var(--g)) calc(0.7rem * var(--g)); flex: none; }
.search .p-inputtext { min-width: 0; }
.search.error :is(.p-inputtext, .p-inputgroupaddon) { border-color: var(--danger); }
.hist { width: round(17.48rem, 1px); display: flex; flex-direction: column; gap: 0.14rem; font-size: 1rem; }
/* the fields scroll by themselves, so the menu never runs off the window */
.scope-fields { max-height: 40vh; overflow-y: auto; display: flex; flex-direction: column; gap: 0.14rem; }
/* rows keep their height in the scrolling list rather than squeezing to fit it */
.scope-fields > .scope-row { flex: none; }
.scope-row.sub { padding-left: calc(2.1rem * var(--g)); }
.scope-row .count { color: var(--text-faint); font-size: 0.92rem; }
.scope-row .icon-btn.mini { min-width: round(1.68rem, 1px); height: round(1.68rem, 1px); padding: 0; }
.scope-title { padding: 0.14rem calc(0.42rem * var(--g)) calc(0.28rem * var(--g)); }
.scope-row { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); height: round(2.1rem, 1px); padding: 0 calc(0.42rem * var(--g)); border-radius: calc(6px * var(--r)); cursor: default; }
.scope-row .ico { color: var(--text-muted); }
.scope-row:hover { background: color-mix(in srgb, var(--text) 8%, transparent); }
.hist-row { padding: calc(0.35rem * var(--g)) calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); cursor: pointer; font-size: 0.85rem; }
.hist-row:hover { background: var(--hover); }
.lp-zoom { display: inline-flex; align-items: center; gap: 0.14rem; }
.lp-size { width: round(9.09rem, 1px); margin: 0 calc(0.56rem * var(--g)); }
.layout-pop { width: round(23.08rem, 1px); display: flex; flex-direction: column; gap: calc(0.7rem * var(--g)); padding: calc(0.28rem * var(--g)); }
.lp-row { display: flex; align-items: center; justify-content: space-between; gap: calc(0.7rem * var(--g)); }
.lp-select { min-width: round(10.49rem, 1px); }
hr { border: 0; border-top: 1px solid var(--border); margin: 0.14rem 0; width: 100%; }
.lp-sort { width: auto; flex: none; }
.lp-edit { display: flex; }
.lp-edit-main { flex: 1; }
/* parted as a split button's halves */
.lp-edit .p-button + .p-button { border-inline-start: 1px solid var(--surface); }
</style>

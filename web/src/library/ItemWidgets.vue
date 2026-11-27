<script setup>
import AppWidgetHeading from '../shared/AppWidgetHeading.vue'
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import InputText from 'openvue/inputtext'
import InputGroup from 'openvue/inputgroup'
import InputGroupAddon from 'openvue/inputgroupaddon'
import Textarea from 'openvue/textarea'
import { tagTint, openPicker } from '../tags/tags.js'
import { S } from '../app/state.js'
import { L } from '../layout/arrangement.js'
import { saveUi } from '../app/preferences.js'
import { edit } from './edits.js'
import { setScope, filterByTag, filterByFolder } from './navigation.js'
import { requestSearch } from './results.js'
import { run, toast } from '../app/notify.js'
import { openItem, exportFiles } from './lifecycle.js'
import { loadDetail } from './selection.js'
import { call, errorText } from '../platform/ipc.js'
import { shortcutLabel, withKey } from '../app/shortcuts.js'
import { widgetInfo, baseOf } from '../layout/widgets.js'
import AppIcon from '../shared/AppIcon.vue'
import AppChip from '../shared/AppChip.vue'
import AppRating from '../shared/AppRating.vue'
import AppMenu from '../shared/AppMenu.vue'
import FieldEditor from './FieldEditor.vue'
import ItemPreview from '../formats/ItemPreview.vue'
import { fitBox } from '../formats/index.js'
import { asset as a, detail as d, fieldValues as values, fmtDate, fmtDur, fmtSize, hasDimensions, pageCount, selectedIds as ids, userFields, pluginFieldsOf, widgetEmpty } from './itemSelectors.js'
import { t, hasText } from '../shared/i18n.js'
import { numberText, percentText } from '../shared/numbers.js'

// The widgets about one selected item. Each is its own entry in the edit window, and any of them can
// be placed anywhere in a sidebar, more than once. (Plugins add theirs through their manifest.)
// `popup`: opened from its icon, so it shows in full whatever its heading's folded state in a sidebar
const props = defineProps({ id: String, popup: Boolean })
const type = computed(() => baseOf(props.id))
const info = computed(() => widgetInfo(props.id, S.contrib, S.sidebar.fields))

// Remembered per placed widget: closed or open, and the preview's height.
const state = computed(() => {
  const l = L()
  l.widgetState ||= {}
  return (l.widgetState[props.id] ||= {})
})
const COLLAPSED_FIRST = new Set(['exif.camera', 'w.file'])
const collapsed = computed(() => !props.popup && (state.value.collapsed ?? COLLAPSED_FIRST.has(type.value)))
function toggleCollapsed() {
  state.value.collapsed = !collapsed.value
  saveUi()
}
const HEADED = new Set(['w.tags', 'w.folders', 'w.properties', 'w.file', 'w.parts'])
const headed = computed(() => HEADED.has(type.value) || (!!info.value?.plugin && !info.value.headless) || !!info.value?.field)
const heading = computed(() => (HEADED.has(type.value) ? t(`inspector.headings.${type.value.slice(2)}`) : info.value?.label))
// the files grouped into the item, with the item's own file first
const parts = computed(() => d.value?.parts || [])
const partsSize = computed(() => parts.value.reduce((n, p) => n + p.size, a.value?.size || 0))
const detachPart = (p) => run('item.detach', { id: p.id }, t('inspector.parts.detached', { name: p.name }))
const forgetPart = (p) => run('item.forgetFile', { id: p.id }, t('inspector.parts.forgotten', { name: p.name })).then(() => loadDetail()).catch(() => {})
// a row dragged out of the window gives that file, as a tile gives its item's
const dragPart = (p) => call('start_drag', { ids: [p.id] }).catch((e) => toast(errorText(e), 'error'))
// a grouped file's role is named by the plugin whose rule grouped it; Curio's own plugins' roles are in the catalog
const roleText = (role) => (hasText(`inspector.parts.roles.${role}`) ? t(`inspector.parts.roles.${role}`) : role)
const splitItem = () => run('item.split', { ids: [a.value.id] }, t('inspector.parts.splitDone'))

const pluginFields = computed(() => pluginFieldsOf(info.value?.plugin))
// the values of the field paired with this one, shown beside its own in order
const suffixesOf = (f) => {
  const s = S.sidebar.fields.find((g) => g.pair === f.key)
  return s ? (values.value[s.id] || []).map((v) => (v.num != null ? numberText(v.num, { digits: 3, grouping: false }) : v.text || '')) : null
}

const nothingToShow = computed(() => widgetEmpty(info.value, type.value))

// several selected: what they share (S.summary), and edits go to all of them
const many = computed(() => S.selected.length > 1)
const sum = computed(() => (many.value ? S.summary : null))
const shown = computed(() => (many.value ? sum.value : a.value))
// named as the sidebar has them now: a tag or folder renamed since the detail was read shows its new name
function named(list, rows) {
  const names = new Map(rows.map((r) => [r.id, r.name]))
  return list.map((x) => (names.has(x.id) && names.get(x.id) !== x.name ? { ...x, name: names.get(x.id) } : x))
}
const tagList = computed(() => named(many.value ? sum.value?.tags || [] : d.value?.tags || [], S.sidebar.tags))
const folderList = computed(() => named(many.value ? sum.value?.folders || [] : d.value?.folders || [], S.sidebar.folders))
const targets = () => (many.value ? ids() : [a.value.id])
// a rating shows when every item has the same one
const rating = computed(() => (many.value ? (sum.value && sum.value.ratingMin === sum.value.ratingMax ? sum.value.ratingMin : 0) : a.value?.rating || 0))
// the Rating answers null for the star already set: one item loses its stars, several keep that rating for all
const setRating = (n) => edit.rating(targets(), n ?? (many.value ? rating.value : 0))
const isFav = computed(() => (many.value ? !!(sum.value && sum.value.favoriteMin === 1) : !!a.value?.favorite))
const isExp = computed(() => (many.value ? !!(sum.value && sum.value.explicitMin === 1) : !!a.value?.explicit))
const toggleFav = () => edit.favorite(targets(), !isFav.value)
const toggleExp = () => edit.explicit(targets(), !isExp.value)
const kindsText = computed(() => (sum.value?.kinds || []).map((k) => t(`inspector.kindCounts.${k.kind}`, { count: k.count })).join(', '))

// editable copies
const name = ref('')
const note = ref('')
const url = ref('')
// the extension belongs to the file, so it isn't part of the name you edit
const extTail = computed(() => {
  const x = a.value
  const n = x ? x.display_name || x.name : ''
  return x?.ext && n.toLowerCase().endsWith('.' + x.ext.toLowerCase()) ? n.slice(n.length - x.ext.length - 1) : ''
})
const bareName = () => {
  const n = a.value ? a.value.display_name || a.value.name : ''
  return extTail.value ? n.slice(0, -extTail.value.length) : n
}
watch([a, sum], ([x, s]) => {
  name.value = bareName()
  note.value = (s ? s.annotation : x?.annotation) || ''
  url.value = (s ? s.url : x?.source_url) || ''
}, { immediate: true })
function saveName() {
  if (!a.value) return
  if (!name.value.trim()) name.value = bareName()
  else if (name.value !== bareName()) edit.text([a.value.id], 'name', name.value + extTail.value)
}
// several items with different notes or addresses keep them until something is typed
function saveNote() {
  if (sum.value) {
    if (note.value !== (sum.value.annotation ?? '') && (note.value || sum.value.annotation != null)) edit.text(ids(), 'annotation', note.value)
  } else if (a.value && note.value !== a.value.annotation) edit.text(ids(), 'annotation', note.value)
}
function saveUrl() {
  if (sum.value) {
    if (url.value !== (sum.value.url ?? '') && (url.value || sum.value.url != null)) edit.text(ids(), 'url', url.value)
  } else if (a.value && url.value !== a.value.source_url) edit.text(ids(), 'url', url.value)
}

// right-click on the Tags heading works on all the tags, on a tag chip on that one
const tagMenu = ref()
const tagMenuItems = ref([])
const renamingTag = ref(null)
function tagsCtx(e) {
  tagMenuItems.value = [
    { label: t('inspector.tagMenu.copyAll'), icon: 'bookmarks', shortcut: shortcutLabel(S.ui.keys, 'tags.copy'), disabled: !tagList.value.length, command: () => edit.copyTagNames(tagList.value.map((t) => t.name)) },
    { label: t('inspector.tagMenu.paste'), icon: 'clipboard-plus', shortcut: shortcutLabel(S.ui.keys, 'tags.paste'), disabled: !S.copiedTags?.length, command: () => edit.pasteTags(targets()) },
    { separator: true },
    { label: t('inspector.tagMenu.removeAll'), icon: 'bookmark-off', disabled: !tagList.value.length, command: () => edit.clearTags(targets()) },
  ]
  tagMenu.value.show(e)
}
function tagCtx(e, tag) {
  tagMenuItems.value = [
    { label: t('inspector.tagMenu.rename'), icon: 'cursor-text', command: () => (renamingTag.value = { id: tag.id, name: tag.name }) },
    { label: t('inspector.tagMenu.copy'), icon: 'copy', command: () => edit.copyTagNames([tag.name]) },
    { label: t('inspector.tagMenu.paste'), icon: 'clipboard-plus', disabled: !S.copiedTags?.length, command: () => edit.pasteTags(targets()) },
    { separator: true },
    { label: t('inspector.tagMenu.remove'), icon: 'x', command: () => edit.removeTag(targets(), tag.id) },
  ]
  tagMenu.value.show(e)
}
// renames the tag itself, on every item that has it
async function finishTagRename() {
  const r = renamingTag.value
  renamingTag.value = null
  if (r && r.name.trim() && r.name.trim() !== tagList.value.find((t) => t.id === r.id)?.name) await edit.tag('rename', { id: r.id, name: r.name.trim() })
}

const exportMenu = ref()
const exportItems = () => [
  { label: t('inspector.export.files'), icon: 'file-export', command: () => exportFiles(ids()).catch(() => {}) },
  { label: t('inspector.export.json'), icon: 'braces', command: () => run('export', { kind: 'json', ids: ids() }) },
  { label: t('inspector.export.csv'), icon: 'table-export', command: () => run('export', { kind: 'csv', ids: ids() }) },
]

// the file type sits on the picture's corner, wherever the picture falls in its box
const BADGE_INSET = 6
const pvBox = ref(null)
const pvSize = ref({ w: 0, h: 0 })
let pvRo = null
watch(pvBox, (el) => {
  pvRo?.disconnect()
  pvRo = null
  if (!el) return
  pvRo = new ResizeObserver(() => (pvSize.value = { w: el.clientWidth, h: el.clientHeight }))
  pvRo.observe(el)
})
onBeforeUnmount(() => pvRo?.disconnect())
const badgeStyle = computed(() => {
  const b = a.value?.width ? fitBox(a.value, pvSize.value.w, pvSize.value.h, null, 0) : null
  if (!b) return { left: BADGE_INSET + 'px', top: BADGE_INSET + 'px' }
  return { left: Math.round((pvSize.value.w - b.w) / 2) + BADGE_INSET + 'px', top: Math.round((pvSize.value.h - b.h) / 2) + BADGE_INSET + 'px' }
})
// colours under this share of the image would be slivers too narrow to see or click
const MIN_COLOR_PCT = 3
const palette = computed(() => {
  const all = a.value?.palette || []
  const list = all.filter((p, i) => i === 0 || (p.ratio || 0) >= MIN_COLOR_PCT)
  const total = list.reduce((t, p) => t + (p.ratio || 0), 0) || 1
  return list.map((p) => ({ ...p, share: (p.ratio || 0) / total }))
})
const hexOf = (c) => '#' + c.map((v) => v.toString(16).padStart(2, '0')).join('')
const colorTip = (p) => `${hexOf(p.color)} · ${percentText(Number(p.ratio) / 100, 2)}`
function copyColor(c) {
  navigator.clipboard?.writeText(hexOf(c))
  toast(t('inspector.copied'), 'info', 750)
}
function pickColor(c) {
  S.filters.color = '#' + c.map((v) => v.toString(16).padStart(2, '0')).join('')
  requestSearch()
}
function resizePreview(e) {
  const y0 = e.clientY
  const h0 = state.value.height || 220
  const move = (ev) => (state.value.height = Math.max(80, Math.min(800, h0 + ev.clientY - y0)))
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
    saveUi()
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
}
const openUrl = (u) => call('open_url', { url: u })
const copyText = (t) => navigator.clipboard?.writeText(t)
// copied into the library, or linked where it is (and what Curio may do there)
const storage = computed(() => {
  const r = S.sidebar.roots.find((x) => x.id === a.value?.root_id)
  if (!r) return ''
  if (r.kind === 'Managed') return t('inspector.storage.managed')
  const may = r.access === 'read_only' ? t('inspector.storage.readOnly') : r.access === 'organize' ? t('inspector.storage.organize') : t('inspector.storage.edit')
  return t('inspector.storage.linked', { place: r.label || r.path, access: may })
})
const statusText = computed(() => (a.value?.status === 'Ok' ? t('inspector.status.ok') : a.value?.status === 'Offline' ? t('inspector.status.offline') : t('inspector.status.missing')))
const partTip = (p) => (p.status === 'Ok' ? p.name : p.status === 'Offline' ? t('inspector.parts.offline', { name: p.name }) : t('inspector.parts.missing', { name: p.name }))
</script>

<template>
  <div v-if="shown && !nothingToShow" class="item-widget" :class="['w-' + type.replace('.', '-'), { many }]">
    <AppWidgetHeading v-if="headed" :title="heading" :collapsed="collapsed" :foldable="!popup" @toggle="toggleCollapsed" @contextmenu="type === 'w.tags' && ($event.preventDefault(), tagsCtx($event))">
      <button v-if="type === 'w.tags'" id="item-tag" class="icon-btn mini" v-tooltip.bottom="withKey($t('inspector.newTag'), 'tags.pick')" @click="openPicker('tags', $event)"><AppIcon name="plus" :size="14" /></button>
      <button v-else-if="type === 'w.folders'" id="item-folder" class="icon-btn mini" v-tooltip.bottom="$t('inspector.addToFolders')" @click="openPicker('folders', $event)"><AppIcon name="plus" :size="14" /></button>
    </AppWidgetHeading>
    <AppMenu v-if="type === 'w.tags'" ref="tagMenu" :model="() => tagMenuItems" />
    <div v-if="!headed || !collapsed" class="w-body" :class="{ list: !!info?.plugin, empty: (type === 'w.tags' && !tagList.length) || (type === 'w.folders' && !folderList.length) }">
      <template v-if="type === 'w.preview' && many">
        <div class="stack" :style="{ height: (state.height || 220) + 'px' }">
          <div v-for="(id, k) in S.selected.slice(-3)" :key="id" class="stack-card" :style="{ transform: `rotate(${(k - 1) * 5}deg)` }"><ItemPreview :id="id" compact /></div>
        </div>
        <div class="count-head"><span class="accent-num">{{ S.selected.length.toLocaleString() }}</span> {{ $t('inspector.selected', S.selected.length) }}</div>
      </template>
      <template v-else-if="type === 'w.preview'">
        <div ref="pvBox" class="pv" :style="{ height: (state.height || 220) + 'px' }">
          <div class="pv-pic"><ItemPreview :id="a.id" compact /></div>
          <span class="ext" :style="badgeStyle">{{ a.ext.toUpperCase() }}</span>
        </div>
        <div class="pv-resize" @pointerdown.prevent="resizePreview" />
      </template>

      <div v-else-if="type === 'w.palette'" class="palette">
        <span v-for="(p, k) in palette" :key="k" class="band" :style="{ background: `rgb(${p.color.join(',')})`, flexGrow: p.share }" v-tooltip.bottom="colorTip(p)" @click="pickColor(p.color)" @contextmenu.prevent="copyColor(p.color)" />
      </div>

      <InputText v-else-if="type === 'w.name'" id="item-name" v-model="name" class="field big" @blur="saveName" @keydown.enter="$event.target.blur()" v-tooltip.bottom="{ value: $t('inspector.displayName'), showDelay: 800 }" />
      <Textarea v-else-if="type === 'w.annotation'" v-model="note" class="field" rows="2" :placeholder="sum && sum.annotation == null ? $t('inspector.notesDiffer') : $t('inspector.notes')" @blur="saveNote" />

      <InputGroup v-else-if="type === 'w.url'">
        <InputText v-model="url" class="field" :placeholder="sum && sum.url == null ? $t('inspector.urlsDiffer') : 'http://'" @blur="saveUrl" @keydown.enter="$event.target.blur()" />
        <InputGroupAddon v-if="(sum ? sum.url : a.source_url)"><button class="addon-btn" v-tooltip.bottom="$t('inspector.openUrl')" :aria-label="$t('inspector.openUrl')" @click="openUrl(sum ? sum.url : a.source_url)"><AppIcon name="external-link" :size="16" /></button></InputGroupAddon>
      </InputGroup>

      <template v-else-if="type === 'w.tags'">
        <div v-if="tagList.length" class="chips">
          <template v-for="t in tagList" :key="t.id">
          <InputText v-if="renamingTag?.id === t.id" v-model="renamingTag.name" v-focus class="chip-rename" size="small" @keydown.enter="$event.target.blur()" @keydown.esc="renamingTag = null" @blur="finishTagRename" />
          <AppChip v-else class="chip-link" removable corner @contextmenu.prevent="tagCtx($event, t)" :style="tagTint(t.name)" v-tooltip="many ? $t('inspector.onAll') : undefined" @mousedown.middle.prevent @auxclick.prevent="$event.button === 1 && edit.removeTag(targets(), t.id)" @remove="edit.removeTag(targets(), t.id)"><span @click="filterByTag(t.name, $event.ctrlKey || $event.metaKey)">{{ t.name }}</span></AppChip>
          </template>
        </div>
      </template>

      <template v-else-if="type === 'w.folders'">
        <div v-if="folderList.length" class="chips">
          <AppChip v-for="f in folderList" :key="f.id" class="chip-link" removable corner v-tooltip="many ? $t('inspector.containsAll') : undefined" @mousedown.middle.prevent @auxclick.prevent="$event.button === 1 && edit.folder('unadd', { id: f.id, ids: targets() })" @remove="edit.folder('unadd', { id: f.id, ids: targets() })"><span @click="($event.ctrlKey || $event.metaKey) ? filterByFolder(f.name, true) : setScope('folder', f.id)">{{ f.name }}</span></AppChip>
        </div>
      </template>

      <div v-else-if="type === 'w.properties' && many" class="props">
        <div class="kv"><span>{{ $t('inspector.props.size') }}</span><span class="mono">{{ fmtSize(sum.size) }}</span></div>
        <div v-if="sum.duration" class="kv"><span>{{ $t('inspector.props.duration') }}</span><span class="mono">{{ fmtDur(sum.duration) }}</span></div>
        <div v-if="kindsText" class="kv"><span>{{ $t('inspector.props.types') }}</span><span class="mono long">{{ kindsText }}</span></div>
      </div>

      <template v-else-if="type === 'w.properties'">
        <div class="props">
          <div v-if="hasDimensions(a)" class="kv"><span>{{ $t('inspector.props.dimensions') }}</span><span class="mono">{{ a.width }} × {{ a.height }}</span></div>
          <div v-if="a.duration" class="kv"><span>{{ $t('inspector.props.duration') }}</span><span class="mono">{{ fmtDur(a.duration) }}</span></div>
          <div class="kv"><span>{{ $t('inspector.props.size') }}</span><span class="mono">{{ fmtSize(a.size) }}</span></div>
          <div class="kv"><span>{{ $t('inspector.props.type') }}</span><span class="mono">{{ a.ext.toUpperCase() }}</span></div>
          <div class="kv"><span>{{ $t('inspector.props.added') }}</span><span class="mono">{{ fmtDate(a.added_at) }}</span></div>
          <div class="kv"><span>{{ $t('inspector.props.created') }}</span><span class="mono">{{ fmtDate(a.ctime) }}</span></div>
          <div class="kv"><span>{{ $t('inspector.props.modified') }}</span><span class="mono">{{ fmtDate(a.mtime) }}</span></div>
        </div>
        <FieldEditor v-for="f in userFields" :key="f.id" :field="f" :values="values[f.id] || []" :ids="[a.id]" />
      </template>

      <div v-else-if="type === 'w.rating'" class="rating-row">
        <AppRating class="rate" :modelValue="rating" @update:modelValue="setRating" />
        <span class="grow" />
        <button class="icon-btn flag-btn" :class="{ fav: isFav }" v-tooltip="withKey($t('inspector.favorite'), 'rating.favorite')" :aria-label="$t('inspector.favorite')" :aria-pressed="isFav" @click="toggleFav"><AppIcon :name="isFav ? 'heart-filled' : 'heart'" :size="17" /></button>
        <button class="icon-btn flag-btn" :class="{ exp: isExp }" v-tooltip="withKey($t('inspector.explicit'), 'rating.explicit')" :aria-label="$t('inspector.explicit')" :aria-pressed="isExp" @click="toggleExp"><AppIcon name="shield-exclamation" :size="17" /></button>
      </div>
      <div v-else-if="type === 'w.dimensions' && hasDimensions(a)" class="kv"><span>{{ $t('inspector.props.dimensions') }}</span><span class="mono">{{ a.width }} × {{ a.height }}</span></div>
      <div v-else-if="type === 'w.duration' && a.duration" class="kv"><span>{{ $t('inspector.props.duration') }}</span><span class="mono">{{ fmtDur(a.duration) }}</span></div>
      <div v-else-if="type === 'w.pages' && pageCount" class="kv"><span>{{ $t('inspector.props.pages') }}</span><span class="mono">{{ pageCount }}</span></div>
      <div v-else-if="type === 'w.size'" class="kv"><span>{{ $t('inspector.props.size') }}</span><span class="mono">{{ fmtSize(shown.size) }}</span></div>
      <div v-else-if="type === 'w.type'" class="kv"><span>{{ $t('inspector.props.type') }}</span><span class="mono">{{ a.ext.toUpperCase() }}</span></div>
      <div v-else-if="type === 'w.added'" class="kv"><span>{{ $t('inspector.props.added') }}</span><span class="mono">{{ fmtDate(a.added_at) }}</span></div>
      <div v-else-if="type === 'w.created'" class="kv"><span>{{ $t('inspector.props.created') }}</span><span class="mono">{{ fmtDate(a.ctime) }}</span></div>
      <div v-else-if="type === 'w.modified'" class="kv"><span>{{ $t('inspector.props.modified') }}</span><span class="mono">{{ fmtDate(a.mtime) }}</span></div>
      <template v-else-if="type === 'w.export'">
        <button class="add-btn wide export" @click="exportMenu.toggle($event)"><AppIcon name="upload" :size="15" /> {{ $t('inspector.export.button') }}</button>
        <AppMenu ref="exportMenu" :model="exportItems" />
      </template>

      <div v-else-if="type === 'w.file'" class="props">
        <div class="kv"><span>{{ $t('inspector.status.label') }}</span><span :class="{ ok: a.status === 'Ok', warn: a.status !== 'Ok' }">{{ statusText }}</span></div>
        <div class="kv"><span>{{ $t('inspector.storage.label') }}</span><span>{{ storage }}</span></div>
        <div class="path mono">{{ a.abs_path }}</div>
        <div class="row wrap">
          <button class="add-btn" @click="openItem(a.id, 'reveal')"><AppIcon name="folder-open" /> {{ $t('inspector.reveal') }}</button>
          <button class="add-btn" @click="copyText(a.abs_path); toast($t('inspector.pathCopied'))"><AppIcon name="clipboard" /> {{ $t('inspector.copyPath') }}</button>
          <button v-if="a.status !== 'Ok'" class="add-btn" @click="run('relink', { id: a.id })"><AppIcon name="link" /> {{ $t('inspector.relink') }}</button>
        </div>
        <div v-if="a.content_hash" class="kv"><span>BLAKE3</span><span class="mono">{{ a.content_hash.slice(0, 16) }}</span></div>
      </div>

      <div v-else-if="type === 'w.parts'" class="props parts">
        <div class="part">
          <span class="pname" :title="a.name">{{ a.name }}</span><span class="meta faint">{{ $t('inspector.parts.item', { size: fmtSize(a.size) }) }}</span>
        </div>
        <div v-for="p in parts" :key="p.id" class="part" :class="{ warn: p.status !== 'Ok' }" :draggable="p.status === 'Ok'" @dragstart.prevent="dragPart(p)">
          <span class="pname" v-tooltip.bottom="partTip(p)">{{ p.name }}</span>
          <span class="meta faint">{{ roleText(p.role) }} · {{ fmtSize(p.size) }}</span>
          <span class="acts">
            <button class="icon-btn mini" v-tooltip.bottom="$t('inspector.parts.open')" :aria-label="$t('inspector.parts.open')" @click="openItem(p.id, 'open')"><AppIcon name="external-link" /></button>
            <button class="icon-btn mini" v-tooltip.bottom="$t('inspector.reveal')" :aria-label="$t('inspector.reveal')" @click="openItem(p.id, 'reveal')"><AppIcon name="folder-open" /></button>
            <button class="icon-btn mini" v-tooltip.bottom="$t('inspector.parts.detach')" :aria-label="$t('inspector.parts.detach')" @click="detachPart(p)"><AppIcon name="unlink" /></button>
            <button v-if="p.status !== 'Ok'" class="icon-btn mini" v-tooltip.bottom="$t('inspector.parts.forget')" :aria-label="$t('inspector.parts.forget')" @click="forgetPart(p)"><AppIcon name="trash-x" /></button>
          </span>
        </div>
        <div class="kv"><span>{{ $t('inspector.parts.total') }}</span><span>{{ fmtSize(partsSize) }}</span></div>
        <div class="row wrap"><button class="add-btn" @click="splitItem"><AppIcon name="arrows-split-2" /> {{ $t('inspector.parts.split') }}</button></div>
      </div>

      <FieldEditor v-else-if="info?.field" :field="info.field" :values="values[info.field.id] || []" :ids="[a.id]" />
      <template v-else-if="info?.plugin">
        <FieldEditor v-for="f in pluginFields" :key="f.id" :field="f" :values="values[f.id] || []" :ids="[a.id]" :suffixes="suffixesOf(f)" readonly />
      </template>
    </div>
  </div>
</template>

<style scoped>
.item-widget { font-size: 1rem; padding: 0 0.14rem; }
.icon-btn.mini { min-width: round(1.54rem, 1px); height: round(1.54rem, 1px); opacity: 0.8; }
.w-body.empty { display: none; }
/* read-only values sit close together, like the rows of Properties */
.w-body.list { gap: 0; }
.w-body { padding-bottom: calc(0.84rem * var(--g)); display: flex; flex-direction: column; gap: calc(0.7rem * var(--g)); }
.w-w-preview .w-body { padding-bottom: calc(0.84rem * var(--g)); padding-top: calc(0.7rem * var(--g)); }
.w-w-palette .w-body { padding-bottom: calc(0.98rem * var(--g)); }
.w-w-name .w-body, .w-w-annotation .w-body { padding-bottom: calc(0.56rem * var(--g)); }
.pv { position: relative; display: flex; flex-direction: column; border-radius: calc(8px * var(--r)); }
.ext { position: absolute; z-index: 1; font-size: 0.72rem; font-weight: 600; line-height: round(1.26rem, 1px); padding: 0 calc(0.42rem * var(--g)); border-radius: calc(4px * var(--r)); background: rgba(0, 0, 0, .55); color: #fff; }
.pv-pic { position: relative; flex: 1; min-height: 0; }
.pv-resize { height: 0.42rem; cursor: row-resize; margin-top: calc(-0.56rem * var(--g)); }
.palette { display: flex; width: 100%; height: round(1.12rem, 1px); border-radius: calc(4px * var(--r)); overflow: hidden; outline: 1px solid color-mix(in srgb, var(--text) 10%, transparent); outline-offset: -1px; }
.band { flex: 1 1 0; min-width: 0.21rem; cursor: pointer; }
.parts .part { display: grid; grid-template-columns: minmax(0, 1fr) auto; grid-template-areas: 'name name' 'meta acts'; align-items: center; column-gap: calc(0.42rem * var(--g)); padding: calc(0.28rem * var(--g)) 0; }
.parts .pname { grid-area: name; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.parts .meta { grid-area: meta; font-size: 0.85rem; }
.parts .acts { grid-area: acts; display: flex; }
.parts .part.warn .pname { color: var(--warning); }
.field { min-height: round(2.38rem, 1px); }
.field.big { font-size: 1rem; }
textarea.field { line-height: 1.5; }
.chips { display: flex; flex-wrap: wrap; gap: calc(0.56rem * var(--g)); align-items: center; }
.chip-rename { width: 12em; }
.chip:hover :deep(.chip-clear) { display: flex; }
.chip span { cursor: pointer; }
.add-btn { display: inline-flex; align-items: center; justify-content: center; gap: calc(0.42rem * var(--g)); height: var(--control-h); padding: 0 calc(0.7rem * var(--g)); border-radius: calc(6px * var(--r)); border: 0; background: var(--button); color: var(--text); font: inherit; font-size: 0.92rem; cursor: pointer; }
.add-btn:hover { background: var(--button-hover); }
.add-btn.wide { width: 100%; }
.add-btn.export { margin-top: calc(0.56rem * var(--g)); }
.stack { position: relative; margin: calc(0.7rem * var(--g)) calc(1.12rem * var(--g)) 0; }
.stack-card { position: absolute; inset: 0 2.1rem; border-radius: calc(8px * var(--r)); overflow: hidden; background: var(--grid-item); }
.count-head { text-align: center; font-size: 1rem; margin-top: calc(0.98rem * var(--g)); }
.accent-num { color: var(--accent); font-weight: 600; }
.props { display: flex; flex-direction: column; gap: 0; }
.rating-row { display: flex; align-items: center; gap: calc(0.28rem * var(--g)); min-height: round(2.1rem * var(--row), 1px); }
.flag-btn { color: var(--text-muted); }
.flag-btn.fav { color: #e5686b; }
.flag-btn.exp { color: var(--warning); }
/* rows as tall and as large as the Library's folders, so the two sidebars' lists read alike */
.kv { display: flex; justify-content: space-between; align-items: center; gap: calc(0.84rem * var(--g)); min-height: round(2.1rem * var(--row), 1px); line-height: 1.3; font-size: 1rem; }
/* a label longer than the room wraps, in any language, and dates and sizes stay whole */
.kv > span:first-child { flex: 0 1 auto; min-width: 0; color: var(--text); overflow-wrap: break-word; }
.kv > span:last-child { min-width: 0; text-align: right; }
.kv .mono { flex-shrink: 0; color: var(--text-muted); font-size: 0.92rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
/* a list of types is the one value long enough to give way: it takes the room the label leaves and ends in an ellipsis */
.kv .mono.long { flex: 1 1 0; min-width: 0; }
.path { font-size: 0.92rem; color: var(--text-muted); word-break: break-all; user-select: text; margin: calc(0.42rem * var(--g)) 0; }
.wrap { flex-wrap: wrap; }
.ok { color: var(--success); }
.warn { color: var(--warning); }
.small { font-size: 0.85rem; }
</style>

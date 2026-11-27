<script setup>
import { t } from '../shared/i18n.js'
import { computed, ref, watch } from 'vue'
import InputText from 'openvue/inputtext'
import { S, folderOpen } from '../app/state.js'
import { L } from '../layout/arrangement.js'
import { edit, isFavorite, toggleFavorite } from '../library/edits.js'
import { toast } from '../app/notify.js'
import { saveUi } from '../app/preferences.js'
import { exportFiles } from '../library/lifecycle.js'
import { sideClick, sideSelected, sideTargets, deleteSide, visibleFolderOrder, startNewFolder, finishNewFolder, newFolderListed, setAllFoldersOpen, moveFolders, folderInScope, sideScoped, hasSubfolders, LOOK_COLORS as COLORS, colorName } from './sidebar.js'
import { openPicker } from '../tags/tags.js'
import { call } from '../platform/ipc.js'
import { beginDrag } from '../app/dragDrop.js'
import { fuzzy } from '../shared/text.js'
import AppIcon from '../shared/AppIcon.vue'
import { rem } from '../shared/rem.js'
import AppHighlight from '../shared/AppHighlight.vue'
import AppMenu from '../shared/AppMenu.vue'
import { shortcutLabel } from '../app/shortcuts.js'

const sc = (id) => shortcutLabel(S.ui.keys, id)

// tail: for each level above this row, whether its branch was the last one there
const props = defineProps({ folder: Object, depth: Number, filter: String, tail: { type: Array, default: () => [] } })
const open = folderOpen
const menu = ref()
const menuItems = ref([])
const renaming = ref(null)
const adding = ref('')
// left out when there is nothing to count
const fmt = (n) => (n ? Number(n).toLocaleString() : '')
// Every icon lines up with the rows above the tree; the arrow takes no room, it sits left of the icon.
// the arrow sits in the row's padding, left of the icon
const PAD = 1.12
const INDENT = 1.12
const CARET = 0.98
const GUIDE = 0.56
// whole px, so guides stay sharp and icons line up
const at = (n) => Math.round(rem(n)) + 'px'

const children = computed(() => S.sidebar.folders.filter((f) => f.parent_id === props.folder.id))
const narrowed = computed(() => !!props.filter || sideScoped())
const visible = computed(() => {
  if (!narrowed.value) return true
  const hit = (f) => (fuzzy(f.name, props.filter || '') && folderInScope(f)) || S.sidebar.folders.some((c) => c.parent_id === f.id && hit(c))
  return hit(props.folder)
})
const isOpen = computed(() => narrowed.value || open[props.folder.id])
const active = computed(() => sideSelected('folder', props.folder.id))
const addingHere = computed(() => S.newFolder?.parent === props.folder.id && !newFolderListed())


// F2 with this folder selected
watch(() => S.renameSide, (r) => {
  if (r?.kind === 'folder' && r.id === props.folder.id) {
    renaming.value = props.folder.name
    S.renameSide = null
  }
})

async function allItems(ids) {
  const lists = await Promise.all(ids.map((id) => call('folder_items', { id })))
  return [...new Set(lists.flat())]
}
function moveTo(ids) {
  openPicker('folders', null, { ids: [], onPick: (r) => moveFolders(ids.filter((id) => id !== r.id), r.id) })
}

async function ctx(e) {
  const point = { type: 'contextmenu', clientX: e.clientX, clientY: e.clientY }
  const ids = sideTargets('folder', props.folder.id)
  const many = ids.length > 1
  const f = props.folder
  const others = S.libraries.filter((x) => !x.current && x.exists)
  const lay = L()
  const fav = isFavorite('folder', f.id)
  const parent = f.parent_id ?? null
  menuItems.value = [
    { label: t('sidebar.menu.newFolder'), icon: 'folder-plus', shortcut: sc('folder.new'), visible: !many, command: () => startNewFolder() },
    { label: t('sidebar.menu.newSubfolder'), icon: 'folder-down', shortcut: sc('folder.newInside'), visible: !many, command: () => startNewFolder(f.id) },
    { label: t('sidebar.menu.moveTo'), icon: 'corner-up-right', command: () => moveTo(ids) },
    { label: t('sidebar.menu.moveToTop'), icon: 'arrow-bar-to-up', visible: many || parent != null, command: () => moveFolders(ids, null) },
    { separator: true },
    { label: fav ? t('sidebar.menu.unpin') : t('sidebar.menu.pin'), icon: fav ? 'pinned-off' : 'pin', visible: !many, command: () => toggleFavorite('folder', f.id) },
    { separator: true },
    { label: t('sidebar.menu.rename'), icon: 'cursor-text', shortcut: 'F2', visible: !many, command: () => (renaming.value = f.name) },
    { label: t('sidebar.menu.autoTag'), icon: 'tag', shortcut: sc('folder.defaultTags'), visible: !many, command: () => (S.dialogs.autoTag = { id: f.id, name: f.name, tags: [...(f.auto_tags || [])] }) },
    {
      label: t('sidebar.menu.orderFolders'), icon: 'arrows-sort', visible: !many,
      items: [
        { label: t('sidebar.menu.sortLevelAz'), icon: 'sort-ascending-letters', command: () => edit.folder('sort', { parent, value: 'asc' }) },
        { label: t('sidebar.menu.sortLevelZa'), icon: 'sort-descending-letters', command: () => edit.folder('sort', { parent, value: 'desc' }) },
        { label: t('sidebar.menu.sortAllAz'), icon: 'sort-ascending-letters', command: () => edit.folder('sort', { value: 'asc-all' }) },
        { label: t('sidebar.menu.sortAllZa'), icon: 'sort-descending-letters', command: () => edit.folder('sort', { value: 'desc-all' }) },
        { separator: true },
        { label: t('sidebar.menu.sortLevelCount'), icon: 'sort-descending-numbers', command: () => edit.folder('sort', { parent, value: 'count-desc' }) },
        { label: t('sidebar.menu.sortLevelCountUp'), icon: 'sort-ascending-numbers', command: () => edit.folder('sort', { parent, value: 'count-asc' }) },
        { label: t('sidebar.menu.sortAllCount'), icon: 'sort-descending-numbers', command: () => edit.folder('sort', { value: 'count-desc-all' }) },
        { separator: true, visible: children.value.length > 0 },
        { label: t('sidebar.menu.sortSubAz'), icon: 'sort-ascending-letters', visible: children.value.length > 0, command: () => edit.folder('sort', { parent: f.id, value: 'asc' }) },
        { label: t('sidebar.menu.sortSubZa'), icon: 'sort-descending-letters', visible: children.value.length > 0, command: () => edit.folder('sort', { parent: f.id, value: 'desc' }) },
        { label: t('sidebar.menu.sortSubCount'), icon: 'sort-descending-numbers', visible: children.value.length > 0, command: () => edit.folder('sort', { parent: f.id, value: 'count-desc' }) },
      ],
    },
    { separator: true },
    { label: t('sidebar.menu.expand'), icon: 'list-tree', visible: !many && children.value.length > 0, command: () => (open[f.id] = !open[f.id]) },
    { label: t('sidebar.menu.expandLevel'), icon: 'list-details', visible: !many, command: () => setAllFoldersOpen(S.sidebar.folders.filter((x) => (x.parent_id ?? null) === parent).map((x) => x.id)) },
    { label: t('sidebar.menu.expandAll'), icon: 'binary-tree', shortcut: '/', command: () => setAllFoldersOpen() },
    { separator: true },
    { label: t('sidebar.menu.duplicate'), icon: 'copy-plus', command: () => ids.forEach((id) => edit.folder('clone', { id })) },
    { separator: true },
    { label: t('sidebar.menu.exportFiles'), icon: 'upload', command: async () => exportFiles(await allItems(ids)).catch(() => {}) },
    { label: t('sidebar.menu.copyTo'), icon: 'copy', visible: others.length > 0, items: others.map((l) => ({ label: l.name, icon: 'books', command: async () => edit.sendTo(l, await allItems(ids), false) })) },
    { label: t('sidebar.menu.moveToLibrary'), icon: 'corner-up-right', visible: others.length > 0, items: others.map((l) => ({ label: l.name, icon: 'books', command: async () => edit.sendTo(l, await allItems(ids), true) })) },
    { separator: true },
    { label: t('sidebar.menu.subfolders'), icon: 'folders', visible: hasSubfolders(ids), checked: !!lay.subfolders, command: () => { lay.subfolders = !lay.subfolders; saveUi() } },
    { separator: true },
    { label: t('sidebar.menu.icon'), icon: 'mood-smile', visible: !many, command: () => (S.dialogs.icon = { kind: 'folder', id: f.id, current: f.icon }) },
    { dots: COLORS.map(([n, c]) => ({ label: colorName(n), color: c, selected: !many && (f.color || null) === c, command: () => ids.forEach((id) => edit.folder('color', { id, value: c })) })) },
    { separator: true },
    { label: many ? t('sidebar.menu.removeFolders', { count: ids.length }) : t('sidebar.menu.removeFolder'), icon: 'folder-x', shortcut: 'Delete', command: () => deleteSide('folder', ids) },
  ]
  menu.value.show(point)
}

function click(e) {
  if (e.target.closest('.caret')) {
    open[props.folder.id] = !open[props.folder.id]
    return
  }
  sideClick(e, 'folder', props.folder.id, visibleFolderOrder())
}
function dbl() {
  if (children.value.length) open[props.folder.id] = !open[props.folder.id]
}
async function finishRename() {
  const name = (renaming.value || '').trim()
  renaming.value = null
  if (name && name !== props.folder.name) await edit.folder('rename', { id: props.folder.id, name })
}
function finishAdd() {
  const name = adding.value
  adding.value = ''
  finishNewFolder(name)
}
function drag(e) {
  const s = S.sideSel
  const ids = s.kind === 'folder' && s.ids.includes(props.folder.id) && s.ids.length > 1 ? [...s.ids] : [props.folder.id]
  beginDrag(e, { type: 'folder', ids }, ids.length > 1 ? `${ids.length} folders` : props.folder.name, {
    // onto a folder nests it; near its top or bottom edge places it before or after
    zones: true,
    count: ids.length,
    accept: (t) => t === 'folders-root' || (t.startsWith('folder:') && !ids.includes(Number(t.split(':')[1]))),
  })
}
</script>

<template>
  <template v-if="visible">
    <div class="item" :data-depth="depth" :class="{ active }" :style="{ paddingLeft: at(PAD + depth * INDENT) }" :data-drop="'folder:' + folder.id"
         @click="click" @dblclick="dbl" @pointerdown="drag" @contextmenu.prevent="ctx">
      <template v-for="d in depth" :key="d">
        <span v-if="d === depth || !tail[d - 1]" class="guide" :class="{ elbow: d === depth && tail[d - 1] }" :style="{ left: at(PAD + GUIDE + (d - 1) * INDENT) }" />
      </template>
      <span v-if="children.length" class="caret" :style="{ left: at(PAD - CARET + depth * INDENT) }"><AppIcon :name="isOpen ? 'chevron-down' : 'chevron-right'" :size="12" /></span>
      <AppIcon :name="folder.icon || (isOpen && children.length ? 'folder-open' : 'folder')" :size="18" :style="{ color: folder.color || undefined }" />
      <InputText v-if="renaming !== null" v-focus v-model="renaming" class="field inline" @keydown.enter="finishRename" @keydown.esc="renaming = null" @blur="finishRename" @click.stop @pointerdown.stop />
      <span v-else class="name"><AppHighlight :text="folder.name" :q="filter" /></span>
      <span class="count">{{ fmt(folder.count) }}</span>
    </div>
    <template v-if="isOpen">
      <BrowseWidgetsFolderNode v-for="(c, i) in children" :key="c.id" :folder="c" :depth="depth + 1" :filter="filter" :tail="[...tail, i === children.length - 1 && !addingHere]" />
    </template>
    <div v-if="addingHere" class="item" :style="{ paddingLeft: at(PAD + (depth + 1) * INDENT) }">
      <AppIcon name="folder" :size="18" />
      <InputText v-if="S.newFolder.name == null" v-focus v-model="adding" class="field inline" :placeholder="$t('sidebar.subfolderName')" @keydown.enter="finishAdd" @keydown.esc="adding = ''; S.newFolder = null" @blur="finishAdd" />
      <span v-else class="name">{{ S.newFolder.name }}</span>
    </div>
    <AppMenu ref="menu" :model="() => menuItems" searchable />
  </template>
</template>

<style scoped>
.item { position: relative; display: flex; align-items: center; gap: calc(0.56rem * var(--g)); height: round(2.1rem * var(--row), 1px); padding-right: calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); color: var(--text); font-size: 1rem; }
.item .ico { color: var(--text-muted); }
.item:hover { background: var(--hover); }
.item.active { background: var(--active); }
.name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.caret { position: absolute; top: 0; bottom: 0; width: round(0.98rem, 1px); display: inline-flex; align-items: center; justify-content: center; color: var(--text-faint); cursor: pointer; }
.guide { position: absolute; top: 0; bottom: 0; width: 1px; background: color-mix(in srgb, var(--text) 16%, transparent); pointer-events: none; }
/* the last branch turns towards its folder */
.guide.elbow { bottom: 50%; width: 0.49rem; background: none; border: solid color-mix(in srgb, var(--text) 16%, transparent); border-width: 0 0 1px 1px; border-bottom-left-radius: calc(5px * var(--r)); }
.field.inline { padding: 0.14rem calc(0.42rem * var(--g)); height: round(1.68rem, 1px); }
</style>

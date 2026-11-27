<script setup>
import { t } from '../shared/i18n.js'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch, watchEffect } from 'vue'
import InputText from 'openvue/inputtext'
import { S } from '../app/state.js'
import { edit } from '../library/edits.js'
import { setScope, resetFilters } from '../library/navigation.js'
import { requestSearch } from '../library/results.js'
import { toast } from '../app/notify.js'
import { LOOK_COLORS, colorName } from '../sidebar/sidebar.js'
import { call, errorText } from '../platform/ipc.js'
import { beginDrag } from '../app/dragDrop.js'
import { fuzzy } from '../shared/text.js'
import AppIcon from '../shared/AppIcon.vue'
import AppHighlight from '../shared/AppHighlight.vue'
import AppMenu from '../shared/AppMenu.vue'

// The tag manager: All, Uncategorized, Starred and the groups on the left; tags on the right, as a
// grid or a list, sorted and grouped as asked. A tag can sit in several groups; its first group's
// colour follows it everywhere in the app.
const view = computed(() => (S.ui.tagView ||= { layout: 'grid', sort: 'az', groupBy: 'alphabet' }))
const place = ref('all') // 'all' | 'none' | 'starred' | group id
// typed into the toolbar's one search box
const q = computed(() => S.tagSearch)
const selected = ref(new Set())
watch([selected, place, () => S.sidebar.tags], () => (S.tagFocus = { names: S.sidebar.tags.filter((t) => selected.value.has(t.id)).map((t) => t.name), place: place.value }), { immediate: true })
onBeforeUnmount(() => (S.tagFocus = null))
let anchor = null
const menu = ref()
const menuItems = ref([])
const groupMenu = ref()
const groupItems = ref([])
const renaming = ref(null)
const renamingGroup = ref(null)
const addingTag = ref(null)
const addingGroup = ref(null)

const groups = computed(() => S.sidebar.tagGroups)
const groupById = computed(() => new Map(groups.value.map((g) => [g.id, g])))
const colorOf = (t) => groupById.value.get(t.group_ids?.[0])?.color || t.color || null
const allTags = computed(() => S.sidebar.tags)
const inGroup = (id) => allTags.value.filter((t) => t.group_ids?.includes(id))
const currentGroup = computed(() => (typeof place.value === 'number' ? groupById.value.get(place.value) : null))
const shown = computed(() => {
  let list = allTags.value
  if (place.value === 'none') list = list.filter((t) => !t.group_ids?.length)
  else if (place.value === 'starred') list = list.filter((t) => t.starred)
  else if (currentGroup.value) list = list.filter((t) => t.group_ids?.includes(place.value))
  if (q.value.trim()) {
    const term = q.value.trim()
    list = list.filter((t) => fuzzy(t.name, term) || t.group_ids?.some((g) => fuzzy(groupById.value.get(g)?.name || '', term)))
  }
  const by = view.value.sort
  const name = (a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' })
  return [...list].sort(by === 'za' ? (a, b) => name(b, a) : by === 'high' ? (a, b) => b.count - a.count || name(a, b) : by === 'low' ? (a, b) => a.count - b.count || name(a, b) : name)
})
const sections = computed(() => {
  // inside a group, or sorted by count, letters would only get in the way
  if (currentGroup.value) return [{ key: 'one', list: shown.value }]
  if (view.value.groupBy === 'groups') {
    const out = groups.value.map((g) => ({ key: 'g' + g.id, name: g.name, color: g.color, list: shown.value.filter((t) => t.group_ids?.includes(g.id)) })).filter((s) => s.list.length)
    const rest = shown.value.filter((t) => !t.group_ids?.length)
    if (rest.length) out.push({ key: 'g0', name: t('tags.view.uncategorized'), list: rest })
    return out
  }
  if (view.value.sort === 'high' || view.value.sort === 'low') return [{ key: 'count', list: shown.value }]
  const map = new Map()
  for (const t of shown.value) {
    const c = t.name[0]?.toUpperCase() || '#'
    const letter = /\p{L}/u.test(c) ? c : '#'
    if (!map.has(letter)) map.set(letter, [])
    map.get(letter).push(t)
  }
  return [...map].map(([name, list]) => ({ key: name, name, list }))
})
const flat = computed(() => sections.value.flatMap((s) => s.list))
watchEffect(() => {
  const name = currentGroup.value?.name || (place.value === 'none' ? t('tags.view.uncategorized') : place.value === 'starred' ? t('tags.view.starred') : t('tags.view.allTags'))
  S.tagTitle = selected.value.size ? t('tags.view.titleSelected', { name, n: selected.value.size }) : t('tags.view.titleCount', { name, n: shown.value.length })
})
onBeforeUnmount(() => {
  S.tagTitle = ''
  S.tagSearch = ''
})
watch(place, () => (selected.value = new Set()))

// ---------------------------------------------------------------- choosing tags
function click(e, t) {
  const next = new Set(e.ctrlKey || e.metaKey || e.shiftKey ? selected.value : [])
  if (e.shiftKey && anchor != null) {
    const list = flat.value
    const [a, b] = [list.findIndex((x) => x.id === anchor), list.findIndex((x) => x.id === t.id)]
    for (const x of list.slice(Math.min(a, b), Math.max(a, b) + 1)) next.add(x.id)
  } else if ((e.ctrlKey || e.metaKey) && next.has(t.id)) next.delete(t.id)
  else next.add(t.id)
  if (!e.shiftKey) anchor = t.id
  selected.value = next
}
const targets = () => allTags.value.filter((t) => selected.value.has(t.id))
const each = (fn) => Promise.all(targets().map(fn))

/** Show the items carrying these tags, through the filter so they can be narrowed further. */
function filterItems(names) {
  setScope('all')
  resetFilters()
  S.filters.tags.push(...names)
  S.view = 'grid'
  requestSearch()
}
function ctx(e, tag) {
  if (!selected.value.has(tag.id)) selected.value = new Set([tag.id])
  const list = targets()
  const one = list.length === 1
  const here = currentGroup.value
  menuItems.value = [
    { label: t('tags.view.menu.showItems'), icon: 'photo-search', command: () => filterItems(list.map((x) => x.name)) },
    { separator: true },
    { label: t('tags.view.menu.star'), icon: 'star', disabled: list.every((x) => x.starred), command: () => each((x) => edit.tag('star', { id: x.id })) },
    { label: t('tags.view.menu.unstar'), icon: 'star-off', disabled: !list.some((x) => x.starred), command: () => each((x) => edit.tag('unstar', { id: x.id })) },
    { label: t('tags.view.menu.rename'), icon: 'cursor-text', shortcut: 'F2', visible: one, command: () => (renaming.value = { id: tag.id, name: tag.name }) },
    { separator: true },
    { label: t('tags.view.menu.addToGroup'), icon: 'bookmark-plus', disabled: !groups.value.length, items: groups.value.map((g) => ({ label: g.name, icon: 'bookmark', iconColor: g.color, checked: list.every((x) => x.group_ids?.includes(g.id)), command: () => each((x) => edit.tag('add_to_group', { id: x.id, group: g.id })) })) },
    { label: t('tags.view.menu.moveToGroup'), icon: 'bookmarks', disabled: !groups.value.length, items: groups.value.map((g) => ({ label: g.name, icon: 'bookmark', iconColor: g.color, command: () => each((x) => edit.tag('group', { id: x.id, group: g.id })) })) },
    { label: t('tags.view.menu.removeFromGroup'), icon: 'bookmark-minus', visible: !!here, command: () => each((x) => edit.tag('remove_from_group', { id: x.id, group: here.id })) },
    { label: t('tags.view.menu.removeFromAll'), icon: 'bookmark-off', visible: !here && list.some((x) => x.group_ids?.length), command: () => each((x) => edit.tag('group', { id: x.id, group: null })) },
    { separator: true },
    { label: one ? t('tags.view.deleteTag') : t('tags.view.deleteTags', { count: list.length }), icon: 'trash', shortcut: 'Delete', command: () => confirmRemove(list) },
  ]
  menu.value.show(e)
}
function confirmRemove(list) {
  S.dialogs.confirm = {
    title: list.length === 1 ? t('tags.view.confirm.one', { name: list[0].name }) : t('tags.view.confirm.many', { count: list.length }),
    body: t('tags.view.confirm.body'),
    ok: () => {
      list.forEach((x) => edit.tag('delete', { id: x.id }))
      selected.value = new Set()
    },
  }
}
async function finishRename() {
  const r = renaming.value
  renaming.value = null
  if (r && r.name.trim()) await edit.tag('rename', { id: r.id, name: r.name.trim() })
}
async function finishAddTag() {
  const name = (addingTag.value || '').trim()
  addingTag.value = null
  if (name) await edit.tag('new_tag', { name, group: currentGroup.value?.id ?? null })
}
function dragTag(e, t) {
  // with Ctrl or Shift the click that follows says what is chosen
  if (!e.ctrlKey && !e.metaKey && !e.shiftKey && !selected.value.has(t.id)) selected.value = new Set([t.id])
  const list = () => (selected.value.has(t.id) ? targets() : [t])
  beginDrag(e, {
    drop: (target) => {
      const g = target.split(':')[1]
      if (target === 'tgroup:star') return list().forEach((x) => edit.tag('star', { id: x.id }))
      list().forEach((x) => edit.tag('group', { id: x.id, group: g === 'none' ? null : Number(g) }))
    },
  }, () => (list().length > 1 ? `${list().length} tags` : t.name), { count: () => list().length, accept: (x) => x.startsWith('tgroup:') })
}

// a band drawn from the empty background chooses the tags it touches; a click there chooses none
const cols = ref()
const band = ref(null)
function bandDown(e) {
  if (e.button !== 0 || e.target.closest('.tag, input, textarea, button, .add-tag')) return
  const el = cols.value
  const at = (ev) => {
    const r = el.getBoundingClientRect()
    return [ev.clientX - r.left + el.scrollLeft, ev.clientY - r.top + el.scrollTop]
  }
  const [x0, y0] = at(e)
  const additive = e.ctrlKey || e.metaKey || e.shiftKey
  const base = additive ? [...selected.value] : []
  const ids = new Map(flat.value.map((t) => [t.name, t.id]))
  let moved = false
  const move = (ev) => {
    const [x1, y1] = at(ev)
    if (!moved && Math.abs(x1 - x0) + Math.abs(y1 - y0) < 5) return
    moved = true
    band.value = { x: Math.min(x0, x1), y: Math.min(y0, y1), w: Math.abs(x1 - x0), h: Math.abs(y1 - y0) }
    const r = el.getBoundingClientRect()
    const box = { left: band.value.x - el.scrollLeft + r.left, top: band.value.y - el.scrollTop + r.top }
    box.right = box.left + band.value.w
    box.bottom = box.top + band.value.h
    const next = new Set(base)
    for (const tag of el.querySelectorAll('.tag')) {
      const t = tag.getBoundingClientRect()
      if (t.left < box.right && t.right > box.left && t.top < box.bottom && t.bottom > box.top) next.add(ids.get(tag.dataset.tag))
    }
    selected.value = next
  }
  const up = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
    band.value = null
    if (!moved && !additive) selected.value = new Set()
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
}
// Escape lets go of the chosen tags wherever the keys are, unless a menu or dialog takes it
function escape(e) {
  if (e.key !== 'Escape' || !selected.value.size || /^(INPUT|TEXTAREA)$/.test(e.target.tagName) || document.querySelector('.app-menu, .p-dialog-mask')) return
  selected.value = new Set()
}
onMounted(() => window.addEventListener('keydown', escape, true))
onBeforeUnmount(() => window.removeEventListener('keydown', escape, true))

// ---------------------------------------------------------------- groups
function ctxGroup(e, g) {
  place.value = g.id
  groupItems.value = [
    { label: t('tags.view.menu.showItems'), icon: 'photo-search', disabled: !inGroup(g.id).length, command: () => filterItems(inGroup(g.id).map((t) => t.name)) },
    { separator: true },
    { label: t('tags.view.menu.rename'), icon: 'cursor-text', shortcut: 'F2', command: () => (renamingGroup.value = { id: g.id, name: g.name }) },
    { label: t('tags.view.menu.changeIcon'), icon: 'mood-smile', command: () => (S.dialogs.icon = { kind: 'tagGroup', id: g.id, current: g.icon, color: g.color }) },
    { label: t('tags.view.menu.remove'), icon: 'trash', shortcut: 'Delete', command: () => removeGroup(g) },
    { separator: true, visible: otherLibraries().length > 0 },
    { label: t('tags.view.menu.copyTo'), icon: 'books', visible: otherLibraries().length > 0, items: otherLibraries().map((l) => ({ label: l.name, icon: 'books', command: async () => call('item.copyToLibrary', { to: l.path, ids: await call('tag_group_items', { id: g.id }) }).then((r) => toast(t('tags.view.copied', { count: r.count, name: l.name }))).catch((err) => toast(errorText(err), 'error')) })) },
    { separator: true },
    { dots: LOOK_COLORS.map(([n, c]) => ({ label: colorName(n), color: c, selected: (g.color || null) === c, command: () => edit.tag('group_look', { id: g.id, color: c, name: g.icon }) })) },
  ]
  groupMenu.value.show(e)
}
const otherLibraries = () => S.libraries.filter((x) => !x.current && x.exists)
function removeGroup(g) {
  S.dialogs.confirm = {
    title: t('tags.view.removeGroup.title', { name: g.name }),
    body: t('tags.view.removeGroup.body'),
    ok: () => {
      edit.tag('delete_group', { id: g.id })
      place.value = 'all'
    },
  }
}
async function finishGroupRename() {
  const r = renamingGroup.value
  renamingGroup.value = null
  if (r && r.name.trim()) await edit.tag('rename_group', { id: r.id, name: r.name.trim() })
}
async function finishAddGroup() {
  const name = (addingGroup.value || '').trim()
  addingGroup.value = null
  if (name) await edit.tag('new_group', { name })
}
watch(() => S.renameSide, (r) => {
  if (r?.kind !== 'tagGroup') return
  S.renameSide = null
  const [one] = targets()
  if (selected.value.size === 1 && one) renaming.value = { id: one.id, name: one.name }
  else if (currentGroup.value) renamingGroup.value = { id: currentGroup.value.id, name: currentGroup.value.name }
})
function dragGroup(e, g) {
  beginDrag(e, {
    drop: (target, zone) => {
      const list = groups.value.map((x) => x.id).filter((id) => id !== g.id)
      const at = list.indexOf(Number(target.split(':')[1])) + (zone === 'after' ? 1 : 0)
      list.splice(at, 0, g.id)
      edit.tag('group_order', { name: list.join(',') })
    },
  }, g.name, { sort: true, accept: (x) => /^tgroup:\d+$/.test(x) })
}
const description = ref('')
watch(currentGroup, (g) => (description.value = g?.description || ''), { immediate: true })
function saveDescription() {
  const g = currentGroup.value
  if (g && description.value !== (g.description || '')) edit.tag('group_description', { id: g.id, name: description.value })
}
function key(e) {
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return
  if (e.key === 'Delete' && selected.value.size) confirmRemove(targets())
  else if (e.key === 'Delete' && currentGroup.value) removeGroup(currentGroup.value)
}
function startAddTag() {
  addingTag.value = ''
  nextTick(() => document.querySelector('.add-tag input')?.focus())
}
</script>

<template>
  <div class="tagsview" tabindex="-1" @keydown="key">
    <aside class="groups">
      <div class="g" :class="{ active: place === 'all' }" @click="place = 'all'"><AppIcon name="bookmark" :size="18" /><span class="grow">{{ $t('tags.view.allTags') }}</span><span class="count">{{ allTags.length }}</span></div>
      <div class="g" :class="{ active: place === 'none' }" data-drop="tgroup:none" @click="place = 'none'"><AppIcon name="folder-question" :size="18" /><span class="grow">{{ $t('tags.view.uncategorized') }}</span><span class="count">{{ allTags.filter((t) => !t.group_ids?.length).length }}</span></div>
      <div class="g" :class="{ active: place === 'starred' }" data-drop="tgroup:star" @click="place = 'starred'"><AppIcon name="star" :size="18" /><span class="grow">{{ $t('tags.view.starred') }}</span><span class="count">{{ allTags.filter((t) => t.starred).length }}</span></div>
      <div class="head row"><span class="section-title grow">{{ $t('tags.view.groups', { n: groups.length }) }}</span><button class="icon-btn mini" v-tooltip="$t('tags.view.newGroup')" @click="addingGroup = ''"><AppIcon name="plus" :size="15" /></button></div>
      <div v-if="addingGroup !== null" class="g"><AppIcon name="bookmark" :size="18" /><InputText v-model="addingGroup" v-focus class="field inline grow" :placeholder="$t('tags.view.groupName')" @keydown.enter="finishAddGroup" @keydown.esc="addingGroup = null" @blur="finishAddGroup" /></div>
      <div v-for="g in groups" :key="g.id" class="g" :class="{ active: place === g.id }" :data-drop="'tgroup:' + g.id" @click="place = g.id" @pointerdown="dragGroup($event, g)" @contextmenu.prevent="ctxGroup($event, g)">
        <AppIcon :name="g.icon || 'bookmark'" :size="18" :style="{ color: g.color || undefined }" />
        <InputText v-if="renamingGroup && renamingGroup.id === g.id" v-model="renamingGroup.name" v-focus class="field inline grow" @click.stop @pointerdown.stop @keydown.enter="finishGroupRename" @keydown.esc="renamingGroup = null" @blur="finishGroupRename" />
        <span v-else class="grow ellipsis">{{ g.name }}</span>
        <span class="count">{{ inGroup(g.id).length }}</span>
      </div>
    </aside>
    <div class="main">
      <div ref="cols" class="cols" @pointerdown="bandDown">
        <template v-if="currentGroup">
          <h2><span>{{ currentGroup.name }}</span> <span class="count big">({{ shown.length }})</span><button class="icon-btn mini" v-tooltip="$t('tags.view.newTag')" @click="startAddTag"><AppIcon name="plus" :size="16" /></button></h2>
          <InputText v-model="description" class="desc" :placeholder="$t('tags.view.description')" spellcheck="false" @blur="saveDescription" @keydown.enter="$event.target.blur()" />
          <hr class="sep" />
        </template>
        <section v-for="s in sections" :key="s.key">
          <h3 v-if="s.name"><span v-if="s.color" class="head-dot" :style="{ background: s.color }" />{{ s.name }} <span class="count big">({{ s.list.length }})</span></h3>
          <div class="tag-list" :class="view.layout">
            <div v-for="t in s.list" :key="t.id" class="tag" :class="{ sel: selected.has(t.id) }" :data-tag="t.name" @click="click($event, t)" @dblclick="filterItems([t.name])" @pointerdown="dragTag($event, t)" @contextmenu.prevent="ctx($event, t)">
              <AppIcon v-if="t.starred" name="star-filled" :size="12" class="star" :style="{ color: colorOf(t) || undefined }" />
              <span v-else class="bullet" :style="{ background: colorOf(t) || undefined }" />
              <InputText v-if="renaming && renaming.id === t.id" v-model="renaming.name" v-focus class="field inline" @click.stop @pointerdown.stop @keydown.enter="finishRename" @keydown.esc="renaming = null" @blur="finishRename" />
              <span v-else class="ellipsis name"><AppHighlight :text="t.name" :q="q" /></span>
              <span class="count">({{ t.count }})</span>
            </div>
          </div>
        </section>
        <div v-if="addingTag !== null" class="add-tag"><InputText v-model="addingTag" class="field inline" :placeholder="$t('tags.view.tagName')" @keydown.enter="finishAddTag" @keydown.esc="addingTag = null" @blur="finishAddTag" /></div>
        <div v-if="band" class="band" :style="{ left: band.x + 'px', top: band.y + 'px', width: band.w + 'px', height: band.h + 'px' }" />
        <div v-if="!flat.length && addingTag === null" class="faint empty">{{ q ? $t('tags.view.noMatch') : currentGroup ? $t('tags.view.emptyGroup') : $t('tags.view.empty') }}</div>
      </div>
    </div>
    <AppMenu ref="menu" :model="() => menuItems" searchable />
    <AppMenu ref="groupMenu" :model="() => groupItems" searchable />
  </div>
</template>

<style scoped>
.tagsview { position: absolute; inset: 0; display: flex; outline: none; }
.groups { width: round(18.32rem, 1px); flex: none; border-right: 1px solid var(--border); padding: calc(0.7rem * var(--g)); overflow-y: auto; }
.g { position: relative; display: flex; align-items: center; gap: calc(0.7rem * var(--g)); height: round(2.31rem, 1px); padding: 0 calc(0.7rem * var(--g)); border-radius: calc(6px * var(--r)); font-size: 1rem; }
.g .ico { color: var(--text-muted); }
.g:hover { background: var(--hover); }
.g.active { background: var(--active); }
.head { margin: calc(1.12rem * var(--g)) 0 calc(0.28rem * var(--g)); padding: 0 calc(0.42rem * var(--g)) 0 calc(0.7rem * var(--g)); }
.icon-btn.mini { min-width: round(1.54rem, 1px); height: round(1.54rem, 1px); padding: 0; }
.main { flex: 1; min-width: 0; display: flex; flex-direction: column; }
.cols { position: relative; flex: 1; min-height: 0; overflow-y: auto; user-select: none; padding: calc(0.98rem * var(--g)) calc(1.68rem * var(--g)) calc(2.1rem * var(--g)); }
h2 { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); margin: 0.14rem 0 calc(0.56rem * var(--g)); font-weight: 500; font-size: 1.45rem; }
.desc { width: 100%; border: 0; outline: 0; background: transparent; color: var(--text); font: inherit; padding: 0.14rem 0; }
.desc::placeholder { color: var(--text-faint); }
.sep { border: 0; border-top: 1px solid var(--border); margin: calc(0.84rem * var(--g)) 0 calc(0.56rem * var(--g)); }
section { padding: calc(0.56rem * var(--g)) 0 calc(0.84rem * var(--g)); }
section + section { border-top: 1px solid var(--border); }
h3 { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); font-weight: 400; font-size: 1.15rem; margin: calc(0.28rem * var(--g)) 0 calc(0.7rem * var(--g)); }
.head-dot { width: round(0.7rem, 1px); height: round(0.7rem, 1px); border-radius: var(--round); }
.count.big { font-size: 0.92rem; }
.tag-list.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(16.78rem, 1fr)); gap: 0.14rem calc(1.68rem * var(--g)); }
.tag-list.list { display: flex; flex-direction: column; gap: 1px; }
.tag { display: flex; align-items: center; gap: calc(0.63rem * var(--g)); height: round(2.1rem, 1px); padding: 0 calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); font-size: 1rem; cursor: default; min-width: 0; }
.tag:hover { background: var(--hover); }
.tag.sel { background: var(--selection); }
.name { min-width: 0; }
.bullet { width: 0.42rem; height: 0.42rem; margin: 0 calc(0.21rem * var(--g)); border-radius: var(--round); background: var(--text-faint); flex: none; }
.star { flex: none; color: var(--star); }
.band { position: absolute; border: 1px solid var(--accent); background: color-mix(in srgb, var(--accent) 15%, transparent); pointer-events: none; }
.empty { padding: calc(2.8rem * var(--g)); text-align: center; }
.add-tag { padding: calc(0.42rem * var(--g)) calc(0.56rem * var(--g)); max-width: 19.58rem; }
.field.inline { padding: 0.14rem calc(0.42rem * var(--g)); height: round(1.82rem, 1px); }
</style>

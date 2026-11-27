<script setup>
import { computed, ref } from 'vue'
import Popover from 'openvue/popover'
import { S } from '../app/state.js'
import { openPrefs, saveUi } from '../app/preferences.js'
import { refreshSidebar } from '../sidebar/sidebar.js'
import { run, toast } from '../app/notify.js'
import { localizePlugins } from './pluginText.js'
import { call } from '../platform/ipc.js'
import { fuzzy } from '../shared/text.js'
import { actionFits, openPluginPage, openPluginWindow, pluginActions, runPluginAction } from './pluginActions.js'
import AppIcon from '../shared/AppIcon.vue'
import AppHighlight from '../shared/AppHighlight.vue'
import AppMenu from '../shared/AppMenu.vue'
import AppListSearch from '../shared/AppListSearch.vue'
import { t } from '../shared/i18n.js'

// The toolbar's Plugins button: every plugin that is on, found by name, by kind or among the last used; a click opens its window or tab for the selection, the ⋯ has the rest; pinned ones come first.
const pop = ref()
const rowMenu = ref()
const q = ref('')
const kind = ref('all')
const list = ref([])
const KINDS = [['all', 'apps'], ['window', 'app-window'], ['tab', 'layout-grid'], ['widget', 'layout-sidebar-right'], ['format', 'file-code'], ['action', 'bolt']]
const RECENT = 5

const kindsOf = (p) => {
  const k = []
  if (p.windows?.length) k.push('window')
  if (p.tabs?.length) k.push('tab')
  if (p.widgets) k.push('widget')
  if (p.formats?.length) k.push('format')
  if (p.actions || p.urls?.length) k.push('action')
  return k
}
const pins = computed(() => S.ui.pluginPins || [])
const recent = computed(() => S.ui.pluginRecent || [])
const mine = computed(() => list.value.filter((p) => p.source !== 'builtin' && p.enabled && !p.problem))
const matching = computed(() => mine.value.filter((p) => (kind.value === 'all' || kindsOf(p).includes(kind.value)) && (!q.value.trim() || fuzzy(p.name, q.value.trim()))))
const sections = computed(() => {
  const all = matching.value
  const byId = (ids) => ids.map((id) => all.find((p) => p.id === id)).filter(Boolean)
  const pinned = byId(pins.value)
  const used = q.value.trim() ? [] : byId(recent.value).filter((p) => !pins.value.includes(p.id))
  const rest = all.filter((p) => !pinned.includes(p) && !used.includes(p)).sort((a, b) => a.name.localeCompare(b.name))
  return [[t('plugins.menu.pinned'), pinned], [t('plugins.menu.recent'), used], [pinned.length || used.length ? t('plugins.menu.all') : '', rest]].filter(([, l]) => l.length)
})

async function show(e) {
  // the button, which the event no longer names once the list has come
  const at = e.currentTarget
  q.value = ''
  list.value = await call('plugin_list').then(localizePlugins).catch(() => [])
  pop.value.show(e, at)
}
function used(p) {
  const r = [p.id, ...recent.value.filter((x) => x !== p.id)].slice(0, RECENT)
  S.ui.pluginRecent = r
  saveUi()
}
function togglePin(p) {
  const list = pins.value.includes(p.id) ? pins.value.filter((x) => x !== p.id) : [...pins.value, p.id]
  S.ui.pluginPins = list
  saveUi()
}
async function openWindow(p, w) {
  used(p)
  pop.value.hide()
  openPluginWindow(p.id, w.id)
}
function openTab(p, tab) {
  used(p)
  pop.value.hide()
  openPluginPage(p.id, `tab:${tab.id}`)
}
const actionsOf = (p) => pluginActions('toolbar').filter((a) => a.plugin === p.id)
// what a click does: its first window, else its first tab, else its menu
function primary(p, e) {
  if (p.windows?.length) return openWindow(p, p.windows[0])
  if (p.tabs?.length) return openTab(p, p.tabs[0])
  openRowMenu(p, e)
}
const menuFor = ref(null)
function openRowMenu(p, e) {
  menuFor.value = p
  rowMenu.value.show(e)
}
const describe = (p) => {
  const k = kindsOf(p)
  return k.map((x) => (x === 'format' ? (p.formats || []).flatMap((f) => f.exts).map((e) => '.' + e).join(' ') : t(`plugins.menu.kinds.${x}`))).filter(Boolean).join(' · ')
}
const rowItems = () => {
  const p = menuFor.value
  if (!p) return []
  return [
    ...(p.windows || []).map((w) => ({ label: t('plugins.menu.open', { title: w.title }), icon: w.icon || 'app-window', command: () => openWindow(p, w) })),
    ...(p.tabs || []).map((tab) => ({ label: t('plugins.menu.show', { title: tab.title }), icon: tab.icon || 'layout-grid', command: () => openTab(p, tab) })),
    ...actionsOf(p).map((a) => ({ label: a.title, icon: a.icon || 'bolt', disabled: !!a.disabled || !actionFits(a, S.selected), command: () => { used(p); runPluginAction(a, [...S.selected]).catch(() => {}) } })),
    { separator: true },
    { label: pins.value.includes(p.id) ? t('plugins.menu.unpin') : t('plugins.menu.pin'), icon: pins.value.includes(p.id) ? 'pinned-off' : 'pin', command: () => togglePin(p) },
    { label: t('plugins.menu.settings'), icon: 'settings', visible: !!p.settings, command: () => (pop.value.hide(), openPrefs('plugins')) },
    { label: t('plugins.menu.reveal'), icon: 'folder-open', command: () => run('plugin_op', { op: 'reveal', id: p.id }) },
    { label: t('plugins.menu.switchOff'), icon: 'power', danger: true, command: async () => { await run('plugin_op', { op: 'set', id: p.id, enabled: false, granted: p.granted }); list.value = await call('plugin_list').then(localizePlugins).catch(() => []); refreshSidebar() } },
    { label: t('plugins.menu.remove'), icon: 'unlink', danger: true, visible: !!p.removable, command: async () => { await run('plugin_op', { op: 'remove', id: p.id }); list.value = await call('plugin_list').then(localizePlugins).catch(() => []); refreshSidebar() } },
  ]
}
async function addFolder() {
  pop.value.hide()
  await run('plugin_op', { op: 'add-dir' })
  await refreshSidebar()
}
function createExample() {
  pop.value.hide()
  S.dialogs.examplePlugin = { kind: 'window' }
}
defineExpose({ show, toggle: (e) => (pop.value?.visible ? pop.value.hide() : show(e)) })
</script>

<template>
  <Popover ref="pop" class="plugins-menu">
    <div class="pm">
      <AppListSearch v-focus v-model="q" :placeholder="$t('plugins.menu.search')">
        <button v-for="[k, ic] in KINDS" :key="k" class="icon-btn" :class="{ on: kind === k }" :aria-pressed="kind === k" :aria-label="$t(`plugins.menu.filters.${k}`)" v-tooltip.bottom="$t(`plugins.menu.filters.${k}`)" @click="kind = k"><AppIcon :name="ic" :size="17" /></button>
      </AppListSearch>
      <div class="pm-list">
        <template v-for="[title, rows] in sections" :key="title">
          <div v-if="title" class="section-title pm-head">{{ title }}</div>
          <div v-for="p in rows" :key="p.id" class="pm-row" :data-plugin="p.id" @click="primary(p, $event)" @contextmenu.prevent="openRowMenu(p, $event)">
            <span class="pm-icon"><AppIcon :name="p.icon || 'puzzle'" :size="20" /></span>
            <span class="pm-text"><span class="pm-name"><AppHighlight :text="p.name" :q="q" /></span><span class="pm-kind">{{ describe(p) }}</span></span>
            <button class="icon-btn mini pm-pin" :class="{ on: pins.includes(p.id) }" v-tooltip.bottom="pins.includes(p.id) ? $t('plugins.menu.unpin') : $t('plugins.menu.pin')" @click.stop="togglePin(p)"><AppIcon :name="pins.includes(p.id) ? 'pin-filled' : 'pin'" :size="15" /></button>
            <button class="icon-btn mini" v-tooltip.bottom="$t('plugins.menu.more')" @click.stop="openRowMenu(p, $event)"><AppIcon name="dots" :size="16" /></button>
          </div>
        </template>
        <div v-if="!sections.length" class="faint pm-none">{{ mine.length ? $t('plugins.menu.noMatches') : $t('plugins.menu.none') }}</div>
      </div>
      <div class="pm-foot">
        <button class="pm-link" @click="addFolder"><AppIcon name="folder-plus" :size="16" /> {{ $t('plugins.menu.addFolder') }}</button>
        <button class="pm-link" @click="createExample"><AppIcon name="sparkles" :size="16" /> {{ $t('plugins.menu.example') }}</button>
        <button class="pm-link" @click="pop.hide(); openPrefs('plugins')"><AppIcon name="settings" :size="16" /> {{ $t('plugins.menu.manage') }}</button>
      </div>
    </div>
    <AppMenu ref="rowMenu" :model="rowItems" />
  </Popover>
</template>

<style>
.plugins-menu .p-popover-content { padding: 0; }
.pm { width: round(25.17rem, 1px); display: flex; flex-direction: column; font-size: 1rem; }
.pm .list-search { gap: 0.14rem; }
.pm .list-search .p-inputtext { margin-inline-start: calc(0.42rem * var(--g)); }
.pm-list { max-height: min(29.37rem, 60vh); overflow-y: auto; padding: calc(0.28rem * var(--g)); }
.pm-head { padding: calc(0.56rem * var(--g)) calc(0.56rem * var(--g)) calc(0.28rem * var(--g)); }
.pm-row { display: flex; align-items: center; gap: calc(0.7rem * var(--g)); padding: calc(0.42rem * var(--g)) calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); cursor: pointer; }
.pm-row:hover { background: var(--hover); }
.pm-icon { flex: none; display: inline-flex; align-items: center; justify-content: center; width: round(2.38rem, 1px); height: round(2.38rem, 1px); border-radius: calc(8px * var(--r)); background: var(--button); color: var(--text); }
.pm-text { flex: 1; min-width: 0; display: flex; flex-direction: column; }
.pm-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.pm-kind { font-size: 0.85rem; color: var(--text-faint); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.pm-row .pm-pin { visibility: hidden; }
.pm-row:hover .pm-pin, .pm-row .pm-pin.on { visibility: visible; }
.pm-row .pm-pin.on { color: var(--accent); }
.pm-none { padding: calc(0.98rem * var(--g)); }
.pm-foot { display: flex; flex-direction: column; padding: calc(0.28rem * var(--g)); border-top: 1px solid var(--border); }
.pm-link { display: flex; align-items: center; gap: calc(0.63rem * var(--g)); height: round(2.24rem, 1px); padding: 0 calc(0.56rem * var(--g)); border: 0; border-radius: calc(6px * var(--r)); background: transparent; color: var(--text); font: inherit; text-align: left; cursor: pointer; }
.pm-link .ico { color: var(--text-muted); }
.pm-link:hover { background: var(--hover); }
</style>

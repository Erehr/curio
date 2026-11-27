<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch, watchEffect } from 'vue'
import Button from 'openvue/button'
import Select from 'openvue/select'
import { getCurrentWindow } from '../platform/windowFrame.js'
import { S } from '../app/state.js'
import { rem } from '../shared/rem.js'
import { saveUi, resetSettings, uiFromElsewhere } from '../app/preferences.js'
import { toast } from '../app/notify.js'
import { call } from '../platform/ipc.js'
import { openLibrary, refreshLibraries } from '../library/lifecycle.js'
import { localizePlugins } from '../plugins/pluginText.js'
import AppIcon from '../shared/AppIcon.vue'
import AppSearchField from '../shared/AppSearchField.vue'
import MacWindowButtons from '../app/MacWindowButtons.vue'
import { provideSettingsData, hasWords } from './settingsData.js'
import SettingsPageGeneral from './SettingsPageGeneral.vue'
import SettingsPageNotifications from './SettingsPageNotifications.vue'
import SettingsPageAppearance from './SettingsPageAppearance.vue'
import SettingsPageLayout from './SettingsPageLayout.vue'
import SettingsPageControls from './SettingsPageControls.vue'
import SettingsPagePreview from './SettingsPagePreview.vue'
import SettingsPageShortcuts from './SettingsPageShortcuts.vue'
import SettingsPageStorage from './SettingsPageStorage.vue'
import SettingsPageBackup from './SettingsPageBackup.vue'
import SettingsPageAutoImport from './SettingsPageAutoImport.vue'
import SettingsPageGrouping from './SettingsPageGrouping.vue'
import SettingsPageFields from './SettingsPageFields.vue'
import SettingsPagePlugins from './SettingsPagePlugins.vue'
import SettingsPageApi from './SettingsPageApi.vue'
import SettingsPageMcp from './SettingsPageMcp.vue'
import SettingsPageAbout from './SettingsPageAbout.vue'
import { t } from '../shared/i18n.js'

const { plugins, loadLib, setCfg, refreshes, resets, query, searching, words, shown } = provideSettingsData()

function closePrefs() {
  if (S.prefsWindow) getCurrentWindow().close().catch(() => {})
  else S.dialogs.prefs = false
}
// the window can stay open while other things change
async function refreshPage() {
  refreshLibraries()
  if (!S.libOpen) return
  await loadLib()
  plugins.value = await call('plugin_list').then(localizePlugins).catch(() => [])
  for (const fn of refreshes) fn()
}
onMounted(() => {
  refreshLibraries()
  if (S.prefsWindow) window.addEventListener('focus', refreshPage)
})
watch(() => S.ui.language, refreshPage)
watch(() => S.startup.current, () => S.libOpen && refreshPage())
onBeforeUnmount(() => window.removeEventListener('focus', refreshPage))

const LIBRARY_TABS = new Set(['storage', 'backup', 'autoimport'])
const libs = computed(() => S.libraries.filter((l) => l.exists))
const libByPath = (p) => libs.value.find((l) => l.path === p)
const libName = computed(() => (S.startup.current || '').split(/[\\/]/).pop()?.replace(/\.curio$/, '') || t('sidebar.library'))
const showLibPicker = computed(() => !searching.value && S.libOpen && LIBRARY_TABS.has(S.dialogs.prefsTab) && libs.value.length > 0)
async function switchLibrary(path) {
  if (!path || path === S.startup.current) return
  await openLibrary(path)
  await refreshPage()
}

// each is made when first shown and keeps its state while another shows, as long as Settings is open; Fields and Auto-Import start afresh, and with each library
const PAGES = { general: SettingsPageGeneral, notifications: SettingsPageNotifications, appearance: SettingsPageAppearance, layout: SettingsPageLayout, controls: SettingsPageControls, preview: SettingsPagePreview, shortcuts: SettingsPageShortcuts, storage: SettingsPageStorage, backup: SettingsPageBackup, grouping: SettingsPageGrouping, plugins: SettingsPagePlugins, api: SettingsPageApi, mcp: SettingsPageMcp, about: SettingsPageAbout }
const NAV_GROUPS = [
  { id: 'preferences', items: [['general', 'settings'], ['notifications', 'bell'], ['appearance', 'palette'], ['layout', 'layout-grid'], ['controls', 'adjustments-horizontal'], ['preview', 'eye'], ['shortcuts', 'command']] },
  { id: 'files', items: [['storage', 'database'], ['backup', 'history'], ['autoimport', 'download'], ['grouping', 'stack-2'], ['fields', 'forms']] },
  { id: 'extensions', items: [['plugins', 'puzzle'], ['api', 'api'], ['mcp', 'plug'], ['about', 'info-circle']] },
]
const PAGE_IDS = NAV_GROUPS.flatMap((g) => g.items.map((n) => n[0]))
const label = (id) => (PAGE_IDS.includes(id) ? t(`settings.pages.${id}`) : '')
const groupVisible = (g) => !searching.value || g.items.some((n) => hits.value[n[0]])
// below this width the page list shows only its icons
const SLIM_BELOW = 53.15
const prefsEl = ref()
const slim = ref(false)
const sized = new ResizeObserver(([e]) => (slim.value = e.contentRect.width < rem(SLIM_BELOW)))
onMounted(() => prefsEl.value && sized.observe(prefsEl.value))
onBeforeUnmount(() => sized.disconnect())

// the search shows every page at once, each with the sections that have all the words typed, or all of them when its name has them
const scrollEl = ref()
const made = reactive(new Set())
watch(() => S.dialogs.prefsTab, (tab) => made.add(tab), { immediate: true })
watchEffect(() => (searching.value = !!query.value.trim()))
// made when the search takes the focus, so the first word typed doesn't wait for them
const makeAll = () => PAGE_IDS.forEach((id) => made.add(id))
watch(searching, (on) => {
  if (on) makeAll()
  if (scrollEl.value) scrollEl.value.scrollTop = 0
})
const hits = ref({})
const found = computed(() => Object.values(hits.value).reduce((a, b) => a + b, 0))
const MARK = 'settings-find'
const canMark = () => !!(globalThis.CSS?.highlights && globalThis.Highlight)
// hidden by an attribute of their own, as Vue sets the class and style of the pages' elements
function find() {
  const ws = words.value
  const counts = {}
  const ranges = []
  for (const page of scrollEl.value?.querySelectorAll(':scope > .find-page') || []) {
    const whole = hasWords(label(page.dataset.page), ws)
    let n = 0
    for (const el of page.children) {
      if (el.classList.contains('find-head')) continue
      const hit = whole || hasWords(el.textContent, ws)
      el.toggleAttribute('data-find-out', !hit)
      if (!hit) continue
      n++
      if (ws.length && canMark()) mark(el, ws, ranges)
    }
    page.toggleAttribute('data-find-out', searching.value && !n)
    counts[page.dataset.page] = n
  }
  hits.value = counts
  if (!canMark()) return
  if (ranges.length) CSS.highlights.set(MARK, new Highlight(...ranges))
  else CSS.highlights.delete(MARK)
}
function mark(el, ws, ranges) {
  const walk = document.createTreeWalker(el, NodeFilter.SHOW_TEXT)
  for (let node = walk.nextNode(); node; node = walk.nextNode()) {
    const s = node.data.toLowerCase()
    for (const w of ws) {
      for (let i = s.indexOf(w); i >= 0; i = s.indexOf(w, i + w.length)) {
        const r = new Range()
        r.setStart(node, i)
        r.setEnd(node, i + w.length)
        ranges.push(r)
      }
    }
  }
}
let findTimer = null
const findSoon = () => {
  clearTimeout(findTimer)
  findTimer = setTimeout(find, 80)
}
// pages fill in as their data comes
const filled = new MutationObserver(() => searching.value && findSoon())
onMounted(() => scrollEl.value && filled.observe(scrollEl.value, { childList: true, subtree: true, characterData: true }))
onBeforeUnmount(() => {
  filled.disconnect()
  clearTimeout(findTimer)
  if (canMark()) CSS.highlights.delete(MARK)
})
watch([words, searching], () => nextTick(findSoon))
let picked = null
function go(id) {
  picked = id
  S.dialogs.prefsTab = id
  picked = null
  if (searching.value) scrollEl.value?.querySelector(`.find-page[data-page="${id}"]`)?.firstElementChild?.scrollIntoView({ block: 'start' })
}
// a page asked for from elsewhere, as by another window, shows by itself; sync, to tell it from a click in the list
watch(() => S.dialogs.prefsTab, (tab) => tab !== picked && (query.value = ''), { flush: 'sync' })
watch(() => S.dialogs.prefs, (open) => {
  if (open) refreshPage()
}, { immediate: true })

// every change counts at once: there is nothing to apply or save
watch(() => S.ui, () => {
  if ((S.dialogs.prefs || S.prefsWindow) && !uiFromElsewhere()) saveUi()
}, { deep: true })
// the arrows step through the pages, as in a list
function stepNav(e, d) {
  const rows = [...e.currentTarget.closest('nav').querySelectorAll('.nav:not(.reset)')]
  const next = rows[rows.indexOf(e.currentTarget) + d]
  if (next) {
    next.focus()
    next.click()
  }
}
function askReset() {
  S.dialogs.confirm = {
    title: t('settings.reset.title'),
    body: t('settings.reset.body'),
    okLabel: t('settings.reset.ok'),
    ok: async () => {
      resetSettings()
      for (const fn of resets) fn()
      await setCfg({ gpu: true, closeToTray: true, linkPreviews: false, mcpEnabled: false, mcpDisabled: [] })
      toast(t('settings.reset.done'))
    },
  }
}
</script>

<template>
  <div ref="prefsEl" class="prefs" :class="{ slim }">
    <nav>
      <div class="ptitle" data-window-drag><MacWindowButtons v-if="S.lights && !slim" />{{ $t('settings.title') }}</div>
      <AppSearchField v-model="query" :placeholder="$t('settings.search')" class="search-in" @focusin="makeAll" />
      <div class="nav-list">
        <template v-for="(g, gi) in NAV_GROUPS" :key="g.id">
          <div v-if="groupVisible(g)" class="nav-group" :class="{ first: gi === 0 }">{{ $t(`settings.groups.${g.id}`) }}</div>
          <hr v-if="gi > 0 && groupVisible(g)" class="slim-sep" />
          <template v-for="n in g.items" :key="n[0]">
            <div v-if="!searching || hits[n[0]]" class="nav" :class="{ active: S.dialogs.prefsTab === n[0] }" tabindex="0" role="link" :aria-current="S.dialogs.prefsTab === n[0] ? 'page' : undefined" :aria-label="label(n[0])" v-tooltip.right="slim ? label(n[0]) : undefined" @click="go(n[0])" @keydown.enter.space.prevent="go(n[0])" @keydown.down.prevent="stepNav($event, 1)" @keydown.up.prevent="stepNav($event, -1)">
              <AppIcon :name="n[1]" :size="17" />
              <span class="nav-l">{{ label(n[0]) }}</span>
              <span v-if="searching && hits[n[0]]" class="nav-cnt">{{ hits[n[0]] }}</span>
            </div>
          </template>
        </template>
      </div>
      <div class="nav-foot">
        <div class="nav reset" tabindex="0" role="button" :aria-label="$t('settings.reset.label')" v-tooltip.right="slim ? $t('settings.reset.label') : undefined" @click="askReset" @keydown.enter.space.prevent="askReset">
          <AppIcon name="restore" :size="17" />
          <span class="nav-l">{{ $t('settings.reset.label') }}</span>
        </div>
      </div>
    </nav>
    <section class="pbody">
      <div class="phead" data-window-drag>
        <MacWindowButtons v-if="S.lights && slim" />
        <span class="grow ellipsis">{{ searching ? $t('settings.results') : label(S.dialogs.prefsTab) }}</span>
        <Select v-if="showLibPicker" :modelValue="S.startup.current" :options="libs" optionLabel="name" optionValue="path" size="small" class="lib-picker" :aria-label="$t('sidebar.library')" @update:modelValue="switchLibrary">
          <template #value="{ value }">
            <span class="lib-opt"><AppIcon :name="libByPath(value)?.icon || 'books'" :size="15" :style="{ color: libByPath(value)?.color || undefined }" /><span class="ellipsis">{{ libByPath(value)?.name || libName }}</span></span>
          </template>
          <template #option="{ option }">
            <span class="lib-opt"><AppIcon :name="option.icon || 'books'" :size="15" :style="{ color: option.color || undefined }" /><span class="ellipsis grow">{{ option.name }}</span><AppIcon v-if="option.openElsewhere" name="app-window" :size="14" class="faint" /></span>
          </template>
        </Select>
        <AppSearchField v-if="slim" v-model="query" :placeholder="$t('settings.search')" class="search-head" @focusin="makeAll" />
        <Button v-if="!S.lights" severity="secondary" text size="small" :aria-label="$t('app.windowControls.close')" @click="closePrefs"><template #icon><AppIcon name="x" :size="17" /></template></Button>
      </div>
      <div ref="scrollEl" class="pscroll">
        <div v-for="id in PAGE_IDS" :key="id" v-show="shown(id)" class="find-page" :data-page="id">
          <div v-if="searching" class="find-head">{{ label(id) }}</div>
          <component :is="PAGES[id]" v-if="PAGES[id] && made.has(id)" />
          <SettingsPageFields v-else-if="id === 'fields' && shown(id)" :key="S.startup.current" :plugins="plugins" />
          <SettingsPageAutoImport v-else-if="id === 'autoimport' && shown(id)" :key="S.startup.current" />
        </div>
        <p v-if="searching && !found" class="faint">{{ $t('settings.noMatch') }}</p>
      </div>
    </section>
  </div>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.prefs { display: flex; height: 100%; }
nav { width: round(15.6rem, 1px); flex: none; padding: 0 calc(0.77rem * var(--g)) calc(0.77rem * var(--g)); background: var(--panel); display: flex; flex-direction: column; min-height: 0; }
.nav-list { flex: 1; overflow-y: auto; }
.nav-foot { flex: none; padding-top: calc(0.49rem * var(--g)); border-top: 1px solid var(--border-subtle); margin-top: calc(0.42rem * var(--g)); }
.ptitle { display: flex; align-items: center; height: round(3.29rem, 1px); flex: none; padding-left: calc(0.42rem * var(--g)); font-weight: 600; font-size: 1.05rem; }
.nav-group { padding: calc(0.77rem * var(--g)) calc(0.49rem * var(--g)) calc(0.28rem * var(--g)); font-size: 0.72rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-faint); user-select: none; }
.nav-group.first { padding-top: calc(0.21rem * var(--g)); }
.slim-sep { display: none; border: 0; border-top: 1px solid var(--border-subtle); margin: calc(0.35rem * var(--g)) 0; }
.nav { display: flex; align-items: center; gap: calc(0.49rem * var(--g)); height: round(2.17rem * var(--row), 1px); margin-bottom: 0.1rem; padding: 0 calc(0.49rem * var(--g)); border-radius: calc(6px * var(--r)); font-size: 0.96rem; color: var(--text-muted); cursor: default; }
.nav .ico { color: var(--text-muted); flex: none; }
.nav:hover { background: var(--hover); color: var(--text); }
.nav.active { background: var(--active); color: var(--text); font-weight: 500; }
.nav.active .ico { color: var(--text); }
.nav:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
.nav-l { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.nav-cnt { font-family: var(--mono-font); font-size: 0.76rem; color: var(--text-faint); background: var(--surface-2); padding: 0.05rem 0.38rem; border-radius: calc(999px * var(--r)); }
.nav.reset { margin-bottom: 0; color: var(--text-muted); }
.pbody { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--surface); border-left: 1px solid var(--border-subtle); }
.phead { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); height: round(3.29rem, 1px); flex: none; padding: 0 calc(0.84rem * var(--g)) 0 calc(1.4rem * var(--g)); font-weight: 600; font-size: 1.08rem; }
.lib-picker { width: round(12.6rem, 1px); flex: none; font-weight: 400; font-size: 0.92rem; }
.lib-opt { display: inline-flex; align-items: center; gap: calc(0.42rem * var(--g)); min-width: 0; width: 100%; }
.pscroll { container-type: inline-size; flex: 1; overflow-y: auto; padding: calc(0.56rem * var(--g)) calc(1.4rem * var(--g)) calc(2.1rem * var(--g)); display: flex; flex-direction: column; gap: calc(1.26rem * var(--g)); font-size: 1rem; }
/* a narrow window: the list of pages keeps its icons; pages adapt rows to actual pane width */
.prefs.slim nav { width: round(3.92rem, 1px); padding: 0 calc(0.49rem * var(--g)) calc(0.56rem * var(--g)); }
.prefs.slim .ptitle { visibility: hidden; }
.prefs.slim .slim-sep { display: block; }
.prefs.slim .lib-picker { width: round(9.2rem, 1px); }
.search-head { flex: 0 1 12.59rem; min-width: round(6.99rem, 1px); }
.prefs.slim .nav { justify-content: center; padding: 0; }
.prefs.slim .pscroll { padding: calc(0.56rem * var(--g)) calc(0.98rem * var(--g)) calc(1.8rem * var(--g)); }
/* a page's sections are its own root elements, so each stands in the column as it does alone */
.find-page { display: contents; }
.find-head { padding-top: calc(0.56rem * var(--g)); font-weight: 600; font-size: 1.05rem; width: 100%; }
.find-page:first-child .find-head { padding-top: 0; }
</style>
<style>
/* the pages' elements, which Settings' scoped rules don't reach */
.pscroll .find-page[data-find-out], .pscroll .find-page > [data-find-out] { display: none !important; }
::highlight(settings-find) { background-color: color-mix(in srgb, var(--accent) 45%, transparent); color: inherit; }
</style>

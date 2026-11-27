// What the pages of Settings share: made in the shell's setup each time Settings opens, and injected by the pages.
import { computed, inject, onBeforeUnmount, provide, ref, watch } from 'vue'
import { S } from '../app/state.js'
import { saveUi } from '../app/preferences.js'
import { refreshTheme } from '../app/appearance.js'
import { run, toast } from '../app/notify.js'
import { call, errorText } from '../platform/ipc.js'

const KEY = Symbol('settings')

export function provideSettingsData() {
  const plugins = ref([])
  const libSettings = ref(null)
  let libSnapshot = null
  const appCfg = ref(null)
  // what pages read again when Settings opens or takes the focus, and what they clear on Reset to default
  const refreshes = new Set()
  const resets = new Set()
  // the search in the list of pages: while it has words every page shows, each with the cards that have them
  const query = ref('')
  const searching = ref(false)
  const words = computed(() => (searching.value ? wordsOf(query.value) : []))
  const shown = (id) => searching.value || S.dialogs.prefsTab === id
  async function loadLib() {
    const v = await call('library_settings', { set: null }).catch(() => null)
    libSettings.value = v
    libSnapshot = v ? JSON.parse(JSON.stringify(v)) : null
    return v
  }
  async function saveLib() {
    if (!libSettings.value) return
    const patch = {}
    for (const [k, v] of Object.entries(libSettings.value)) {
      if (!libSnapshot || JSON.stringify(v) !== JSON.stringify(libSnapshot[k])) patch[k] = v
    }
    if (!Object.keys(patch).length) return
    libSnapshot = JSON.parse(JSON.stringify(libSettings.value))
    return run('library_settings', { set: patch })
  }
  async function setCfg(patch) {
    try {
      appCfg.value = await call('app_config', { set: patch })
    } catch (e) {
      toast(errorText(e), 'error')
      appCfg.value = await call('app_config', { set: null }).catch(() => null)
    }
  }
  watch(() => [S.dialogs.prefs, S.dialogs.prefsTab, searching.value], async () => {
    if (S.dialogs.prefs && ['general', 'developer', 'about', 'preview', 'api', 'mcp'].some(shown)) appCfg.value = await call('app_config', { set: null }).catch(() => null)
  }, { immediate: true })
  const data = { plugins, libSettings, appCfg, loadLib, saveLib, setCfg, refreshes, resets, query, searching, words, shown }
  provide(KEY, data)
  return data
}

export const useSettingsData = () => inject(KEY)

export const wordsOf = (text) => String(text || '').toLowerCase().split(/\s+/).filter(Boolean)
export const hasWords = (text, words) => words.every((w) => text.toLowerCase().includes(w))

function hook(set, fn) {
  set.add(fn)
  onBeforeUnmount(() => set.delete(fn))
}
export const onSettingsRefresh = (fn) => hook(useSettingsData().refreshes, fn)
export const onSettingsReset = (fn) => hook(useSettingsData().resets, fn)

export function setUi(k, v) {
  S.ui[k] = v
  saveUi()
  refreshTheme()
}

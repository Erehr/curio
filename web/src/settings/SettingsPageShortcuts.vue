<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import Button from 'openvue/button'
import Tag from 'openvue/tag'
import Message from 'openvue/message'
import { S } from '../app/state.js'
import { saveUi } from '../app/preferences.js'
import { toast } from '../app/notify.js'
import { localizeContrib } from '../plugins/pluginText.js'
import { call } from '../platform/ipc.js'
import { ACTIONS, DEFAULT_KEYS, actionName, comboOf, groupName } from '../app/shortcuts.js'
import { actionDefaultKey, actionKeyId, keyHolders, viewerKeyId } from '../plugins/pluginActions.js'
import { saveActions } from '../actions/actions.js'
import AppIcon from '../shared/AppIcon.vue'
import AppSearchField from '../shared/AppSearchField.vue'
import { useSettingsData, wordsOf, hasWords } from './settingsData.js'
import { t } from '../shared/i18n.js'
import { keyText } from '../shared/keys.js'

const { searching, words, shown } = useSettingsData()
const scFilter = ref('')
const recording = ref(null)
// one key is waited for at a time, and a page put away or closed meanwhile gives it up unset
let stopRecording = null
function recordKey(id) {
  stopRecording?.()
  recording.value = id
  S.recordingKey = true
  const onKey = (e) => {
    e.preventDefault()
    e.stopPropagation()
    if (e.key === 'Escape') return finish(null)
    const c = comboOf(e)
    if (c) finish(c)
  }
  const stop = () => {
    window.removeEventListener('keydown', onKey, true)
    S.recordingKey = false
    recording.value = null
    stopRecording = null
  }
  const finish = (combo) => {
    stop()
    if (combo !== null) {
      // a key is one action's: Curio's, a plugin's or one of yours; whichever had it gives it up
      for (const h of heldBy(id, combo)) {
        setKey(h.id, '')
        toast(t('settings.shortcuts.taken', { name: h.label }))
      }
      setKey(id, combo)
    }
  }
  stopRecording = stop
  window.addEventListener('keydown', onKey, true)
}
watch(() => shown('shortcuts'), (on) => !on && stopRecording?.())
onBeforeUnmount(() => stopRecording?.())
// plugins' actions and their viewers' keys, each a group of its own after Curio's
const pluginContrib = ref({ actions: [], viewerKeys: [], names: {} })
watch(() => shown('shortcuts'), (on) => {
  if (on) call('contributions').then(localizeContrib).then((c) => (pluginContrib.value = { actions: (c.actions || []).filter((a) => a.where.some((w) => w !== 'step')), viewerKeys: c.viewerKeys || [], names: c.names || {} })).catch(() => {})
}, { immediate: true })
const scRows = computed(() => [
  ...ACTIONS.map(([id, group]) => ({ id, group: groupName(group), label: actionName(id), def: DEFAULT_KEYS[id] || '' })),
  ...pluginContrib.value.actions.map((a) => ({ id: actionKeyId(a), group: pluginContrib.value.names[a.plugin] || a.plugin, label: a.title.replace(/…$/, ''), def: actionDefaultKey(a, S.ui.keys) })),
  ...pluginContrib.value.viewerKeys.map((k) => ({ id: viewerKeyId(k), group: t('settings.shortcuts.pluginViewer', { plugin: pluginContrib.value.names[k.plugin] || k.plugin }), label: k.title, def: k.key || '' })),
  ...(S.actions || []).map((a) => ({ id: `action:${a.id}`, group: t('settings.shortcuts.yourActions'), label: a.name, def: '', mine: true })),
])
const defaultOf = (id) => scRows.value.find((r) => r.id === id)?.def || ''
const mineOf = (id) => (id.startsWith('action:') ? S.actions.find((a) => `action:${a.id}` === id) : null)
const keyOf = (id) => (id.startsWith('action:') ? mineOf(id)?.shortcut || '' : S.ui.keys[id] ?? defaultOf(id))
function setKey(id, combo) {
  const mine = mineOf(id)
  if (mine) {
    mine.shortcut = combo
    saveActions()
  } else if (!id.startsWith('action:')) {
    S.ui.keys[id] = combo
    saveUi()
  }
}
const heldBy = (id, combo) => keyHolders(combo, id, { plugins: pluginContrib.value.actions, viewer: pluginContrib.value.viewerKeys })
// a key two actions have, as one made before this check could be
const sharedWith = (id) => heldBy(id, keyOf(id)).map((h) => h.label).join(', ')
const scGroups = computed(() => {
  const g = {}
  // the Settings search picks rows as this page's own does, unless it names the page
  const ws = searching.value ? (hasWords(t('settings.pages.shortcuts'), words.value) ? [] : words.value) : wordsOf(scFilter.value)
  for (const r of scRows.value) if (hasWords(`${r.label} ${r.group}`, ws)) (g[r.group] ||= []).push(r)
  return Object.entries(g)
})
const hasCustom = computed(() => Object.keys(S.ui.keys || {}).length > 0)
</script>

<template>
  <div v-if="!searching" class="s-toolbar">
    <AppSearchField v-model="scFilter" :placeholder="$t('settings.search')" class="sc-search" />
    <Button :label="$t('settings.shortcuts.resetAll')" severity="secondary" size="small" :disabled="!hasCustom" @click="S.ui.keys = {}; saveUi()" />
  </div>
  <Message v-if="recording" severity="info" :closable="false" class="sc-msg">{{ $t('settings.shortcuts.recording') }}</Message>
  <section v-for="[g, list] in scGroups" :key="g" class="s-section">
    <div class="s-section-head">
      <span class="s-section-title">{{ g }}</span>
      <Tag severity="secondary" :value="String(list.length)" />
    </div>
    <div class="s-card">
      <div v-for="a in list" :key="a.id" class="s-row sc-row">
        <div class="s-meta">
          <span class="s-label">
            {{ a.label }}
            <AppIcon v-if="keyOf(a.id) && sharedWith(a.id)" name="alert-triangle" :size="15" class="warn" v-tooltip="$t('settings.shortcuts.also', { names: sharedWith(a.id) })" />
          </span>
        </div>
        <div class="s-ctrl sc-ctrl">
          <Tag v-if="!a.mine && S.ui.keys[a.id] !== undefined" severity="info" :value="$t('settings.shortcuts.custom')" />
          <Button v-if="!a.mine && S.ui.keys[a.id] !== undefined" severity="secondary" text size="small" v-tooltip="a.def ? $t('settings.shortcuts.default', { key: keyText(a.def, '+') }) : $t('settings.shortcuts.noDefault')" :aria-label="a.def ? $t('settings.shortcuts.default', { key: keyText(a.def, '+') }) : $t('settings.shortcuts.noDefault')" @click="delete S.ui.keys[a.id]; saveUi()"><template #icon><AppIcon name="restore" :size="15" /></template></Button>
          <Button v-if="keyOf(a.id) && recording !== a.id" severity="secondary" text size="small" class="clear" v-tooltip="$t('settings.shortcuts.remove')" :aria-label="$t('settings.shortcuts.remove')" @click="setKey(a.id, '')"><template #icon><AppIcon name="x" :size="15" /></template></Button>
          <button type="button" class="rec" :class="{ on: recording === a.id, unset: !keyOf(a.id) && recording !== a.id }" :aria-label="a.label" @click="recordKey(a.id)">{{ recording === a.id ? $t('settings.shortcuts.recording') : keyText(keyOf(a.id), '+') || '—' }}</button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.sc-search { flex: 1; max-width: 24rem; }
.sc-msg { width: 100%; }
.sc-row { padding: calc(0.42rem * var(--g)) calc(1.05rem * var(--g)); min-height: round(2.66rem, 1px); }
.sc-row:hover { background: var(--hover); }
.sc-ctrl { gap: calc(0.35rem * var(--g)); }
.rec { min-width: round(9.8rem, 1px); height: var(--control-h); border-radius: calc(6px * var(--r)); border: 1px solid var(--input-border); background: var(--input); color: var(--text); font-family: var(--mono-font); font-size: 0.84rem; cursor: pointer; text-align: center; padding: 0 calc(0.56rem * var(--g)); transition: border-color 0.12s, background 0.12s; }
.rec:hover { border-color: var(--border-strong); }
.rec.unset { color: var(--text-faint); }
.rec.on { border-color: var(--accent); background: var(--accent-subtle); color: var(--accent); }
.sc-row:not(:hover):not(:focus-within) .clear { visibility: hidden; pointer-events: none; }
</style>

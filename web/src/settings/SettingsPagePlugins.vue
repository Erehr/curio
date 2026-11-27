<script setup>
import { computed, ref, watch } from 'vue'
import Button from 'openvue/button'
import Checkbox from 'openvue/checkbox'
import ToggleSwitch from 'openvue/toggleswitch'
import Tag from 'openvue/tag'
import Message from 'openvue/message'
import AppSegments from '../shared/AppSegments.vue'
import { refreshSidebar } from '../sidebar/sidebar.js'
import { S } from '../app/state.js'
import { saveUi } from '../app/preferences.js'
import { run, toast } from '../app/notify.js'
import { localizePlugins } from '../plugins/pluginText.js'
import { call } from '../platform/ipc.js'
import { playerCodecs } from '../formats/mediaEngine.js'
import AppIcon from '../shared/AppIcon.vue'
import PluginFrame from '../plugins/PluginFrame.vue'
import { useSettingsData } from './settingsData.js'
import { t, hasText } from '../shared/i18n.js'

const { plugins } = useSettingsData()
async function reloadPlugins() {
  plugins.value = await call('plugin_list').then(localizePlugins).catch(() => [])
  await refreshSidebar()
  // the main window's widgets follow
  saveUi()
}
const PLUGIN_TABS = [['format', 'file-code'], ['widget', 'layout-sidebar-right'], ['window', 'app-window']]
const tabs = computed(() => [...PLUGIN_TABS.map(([value, icon]) => ({ value, label: t(`settings.plugins.tabs.${value}`), icon, count: pluginsOf(value).length })), { value: 'tools', label: t('settings.plugins.tabs.tools'), icon: 'tool' }])
const pluginTab = ref('format')
const pluginOpen = ref(null)
// a plugin's settings page is as tall as it says it is
const settingsHeight = ref({})
let foundCodecs = null
const codecs = () => (foundCodecs ??= playerCodecs())
const pluginsOf = (type) => plugins.value.filter((p) => (p.types || []).includes(type))
// programs plugins use, found by Curio; the list is read when the tab opens, as finding versions runs each program
const tools = ref([])
async function toolOp(op = 'list', extra = {}) {
  tools.value = await run('tool_op', { op, name: null, path: null, ...extra }).catch(() => tools.value)
}
// Rust describes each program in English for the API; the window names Curio's own from the catalog
const toolAbout = (tool) => (hasText(`settings.plugins.tools.about.${tool.name}`) ? t(`settings.plugins.tools.about.${tool.name}`) : tool.about)
async function chooseTool(tool) {
  const [path] = (await call('pick', { kind: 'file', title: t('settings.plugins.tools.locate', { name: tool.title }) })) || []
  if (path) toolOp('set', { name: tool.name, path })
}
watch(pluginTab, (tab) => tab === 'tools' && toolOp())
async function pluginOp(op, extra = {}) {
  const msg = await run('plugin_op', { op, id: null, enabled: null, granted: null, ids: null, kind: null, ...extra })
  if (msg) toast(msg)
  await reloadPlugins()
}
function readAllAgain(p) {
  S.dialogs.confirm = { title: t('settings.plugins.rescan.title', { name: p.name }), body: t('settings.plugins.rescan.body'), okLabel: t('settings.plugins.rescan.ok'), ok: () => pluginOp('read-again', { id: p.id, kind: 'all' }) }
}
// the permissions a plugin can ask for; what each allows is in the catalog under its id
const PERMS = ['metadata.read', 'metadata.write', 'organize', 'files.read', 'files.write', 'files.outside', 'ui', 'exec', 'network']
const overriddenText = (p) => p.overriddenBy.map((o) => t('settings.plugins.more.overriddenEach', { exts: o.exts.map((e) => '.' + e).join(', '), plugin: o.plugin })).join('; ')
const urlsText = (p) => p.urls.map((u) => u.title + ' (' + u.patterns.join(', ') + ')').join('; ')
const countsText = (p) => [t('settings.plugins.more.version', { version: p.version }), ...['widgets', 'fields', 'actions', 'filters'].map((k) => t(`settings.plugins.more.counts.${k}`, { count: p[k] }, p[k])), t('settings.plugins.more.counts.formats', { count: p.formats.length }, p.formats.length)].join(' · ')
function togglePlugin(p, v) {
  const granted = v && !p.configured && !p.granted.length ? p.requested : p.granted
  pluginOp('set', { id: p.id, enabled: v, granted })
}
function togglePerm(p, perm, on) {
  const g = new Set(p.granted)
  on ? g.add(perm) : g.delete(perm)
  pluginOp('set', { id: p.id, enabled: p.enabled, granted: [...g] })
}
</script>

<template>
  <div class="s-toolbar">
    <AppSegments v-model="pluginTab" :options="tabs" />
    <div class="s-actions">
      <Button :label="$t('settings.plugins.addFolder')" severity="secondary" size="small" @click="pluginOp('add-dir')"><template #icon><AppIcon name="folder-plus" :size="15" /></template></Button>
      <Button severity="secondary" text size="small" v-tooltip.bottom="$t('settings.plugins.example')" :aria-label="$t('settings.plugins.example')" @click="S.dialogs.examplePlugin = { kind: 'window' }"><template #icon><AppIcon name="file-plus" :size="17" /></template></Button>
      <Button severity="secondary" text size="small" v-tooltip.bottom="$t('settings.plugins.reload')" :aria-label="$t('settings.plugins.reload')" @click="pluginOp('reload')"><template #icon><AppIcon name="refresh" :size="17" /></template></Button>
    </div>
  </div>

  <section v-if="pluginTab === 'tools'" class="s-section">
    <div class="s-card">
      <div v-for="tool in tools" :key="tool.name" class="s-row" :data-tool="tool.name">
        <span class="plug-icon"><AppIcon :name="tool.found ? 'circle-check' : 'circle-dashed'" :size="18" :class="{ ok: tool.found }" /></span>
        <div class="s-meta">
          <span class="s-label">
            <span>{{ tool.title }}</span>
            <Tag v-if="tool.version" severity="secondary" :value="tool.version" />
            <Tag v-if="!tool.found" severity="warn" :value="$t('settings.plugins.tools.notFound')" />
          </span>
          <span class="s-desc ellipsis" v-tooltip.bottom="tool.path || ''">{{ tool.usedBy.length ? $t('settings.plugins.tools.usedBy', { about: toolAbout(tool), plugins: tool.usedBy.join(', ') }) : toolAbout(tool) }}</span>
        </div>
        <div class="s-ctrl s-actions">
          <Button v-if="!tool.found" :label="$t('settings.plugins.tools.download')" severity="secondary" text size="small" @click="call('open_url', { url: tool.homepage })" />
          <Button :label="$t('settings.plugins.tools.choose')" severity="secondary" size="small" @click="chooseTool(tool)" />
          <Button v-if="tool.chosen" severity="secondary" text size="small" v-tooltip.bottom="$t('settings.plugins.tools.autoDetect')" :aria-label="$t('settings.plugins.tools.autoDetect')" @click="toolOp('set', { name: tool.name })"><template #icon><AppIcon name="restore" :size="15" /></template></Button>
        </div>
      </div>
      <div class="s-subblock"><span class="s-desc">{{ $t('settings.plugins.tools.note') }}</span></div>
    </div>
  </section>

  <section v-else class="s-section">
    <div v-if="!pluginsOf(pluginTab).length" class="s-card s-empty">{{ $t('settings.plugins.none') }}</div>
    <div v-else class="s-exp-list">
      <div v-for="p in pluginsOf(pluginTab)" :key="p.id" class="s-exp-item" :class="{ open: pluginOpen === p.id, off: !p.enabled }">
        <div class="s-exp-head" @click="pluginOpen = pluginOpen === p.id ? null : p.id">
          <AppIcon name="chevron-right" :size="15" class="s-exp-chev" />
          <span class="plug-icon"><AppIcon :name="p.icon || 'puzzle'" :size="18" /></span>
          <div class="s-meta">
            <span class="s-label">
              <span class="ellipsis">{{ p.name }}</span>
              <Tag v-if="p.source === 'builtin'" :value="$t('settings.plugins.core')" severity="secondary" />
              <Tag v-if="p.problem" severity="warn" :value="$t('settings.plugins.issueTag')" v-tooltip.bottom="p.problem" />
            </span>
            <span class="s-desc" :class="{ ellipsis: pluginOpen !== p.id }">{{ p.description }}</span>
          </div>
          <div class="s-ctrl" @click.stop>
            <Button v-if="p.removable" severity="secondary" text size="small" v-tooltip.bottom="$t('settings.plugins.remove')" :aria-label="$t('settings.plugins.remove')" @click="pluginOp('remove', { id: p.id })"><template #icon><AppIcon name="unlink" :size="16" /></template></Button>
            <AppIcon v-if="p.locked" name="lock" :size="15" class="faint" v-tooltip.bottom="$t('settings.plugins.builtIn')" />
            <ToggleSwitch v-else :modelValue="p.enabled" :aria-label="p.name" @update:modelValue="(v) => togglePlugin(p, v)" />
          </div>
        </div>

        <div v-if="pluginOpen === p.id" class="s-exp-body">
          <div v-if="p.detail" class="s-desc">{{ p.detail }}</div>
          <template v-if="p.id === 'builtin.format.video' || p.id === 'builtin.format.audio'">
            <div class="s-desc">{{ $t('settings.plugins.more.plays', { codecs: codecs().plays.join(', ') }) }}</div>
            <div v-if="codecs().not.length" class="s-desc">{{ $t('settings.plugins.more.notPlays', { codecs: codecs().not.join(', ') }) }}</div>
          </template>
          <div v-if="p.id === 'builtin.format.video'" class="s-actions">
            <Button :label="$t('settings.plugins.more.ffmpegFolder')" severity="secondary" size="small" @click.stop="pluginOp('ffmpeg-folder', { id: p.id })" />
            <Button :label="$t('settings.plugins.more.ffmpegDownload')" severity="secondary" text size="small" @click.stop="call('open_url', { url: 'https://ffmpeg.org/download.html' })" />
          </div>
          <Message v-if="p.problem" severity="warn" :closable="false">{{ p.problem }}</Message>
          <div v-if="p.overriddenBy?.length" class="s-desc">{{ $t('settings.plugins.more.overridden', { list: overriddenText(p) }) }}</div>
          <div v-if="p.requested.includes('network')" class="s-desc">{{ p.network?.length ? $t('settings.plugins.more.network', { sites: p.network.join(', ') }) : $t('settings.plugins.more.networkAny') }}</div>
          <div v-if="p.waitingApproval" class="s-actions">
            <Button :label="$t('settings.plugins.more.allow')" severity="secondary" size="small" @click.stop="pluginOp('set', { id: p.id, enabled: true, granted: p.granted.length ? p.granted : p.requested })" />
          </div>
          <div v-if="p.windows?.length && p.enabled && !p.problem" class="s-actions">
            <Button v-for="w in p.windows" :key="w.id" :label="$t('settings.plugins.more.open', { title: w.title })" severity="secondary" size="small" @click.stop="run('plugin_window', { plugin: p.id, window: w.id, items: null, run: null })"><template v-if="w.icon" #icon><AppIcon :name="w.icon" :size="15" /></template></Button>
          </div>
          <div v-if="p.urls?.length" class="s-desc">{{ $t('settings.plugins.more.urls', { list: urlsText(p) }) }}</div>
          <div v-if="p.formats.length" class="s-actions" @click.stop>
            <Button :label="$t('settings.plugins.more.rescanMissing')" severity="secondary" size="small" :disabled="!p.enabled || !!p.problem" @click="pluginOp('read-again', { id: p.id, kind: 'missing' })" />
            <Button :label="$t('settings.plugins.more.rescanAll')" severity="secondary" text size="small" :disabled="!p.enabled || !!p.problem" @click="readAllAgain(p)" />
          </div>
          <!-- the plugin's own settings page -->
          <div v-if="p.settings && p.enabled && !p.problem" class="plug-settings" :style="{ height: (settingsHeight[p.id] ?? 48) + 'px' }" @click.stop><PluginFrame :plugin="p.id" :page="p.settings" role="settings" @height="(h) => (settingsHeight[p.id] = h)" /></div>
          <div class="s-desc">{{ countsText(p) }}</div>
          <div v-if="p.requested.length" class="perm-box">
            <span class="s-field-label">{{ $t('settings.plugins.more.permissions') }}</span>
            <div class="s-check-grid">
              <label v-for="perm in PERMS.filter((x) => p.requested.includes(x))" :key="perm" class="opt small" v-tooltip="$t(`settings.plugins.perms.${perm}`)"><Checkbox :modelValue="p.granted.includes(perm)" binary @update:modelValue="(v) => togglePerm(p, perm, v)" /> <span class="s-mono">{{ perm }}</span></label>
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.s-exp-item.off > .s-exp-head :is(.s-meta, .plug-icon) { opacity: 0.5; }
.plug-icon { flex: none; width: round(2.1rem, 1px); height: round(2.1rem, 1px); display: flex; align-items: center; justify-content: center; border-radius: calc(7px * var(--r)); background: var(--surface-3); color: var(--text-muted); }
.s-exp-item.open .plug-icon { background: var(--surface-2); }
.plug-settings { max-height: 70vh; border-radius: calc(8px * var(--r)); overflow: hidden; background: var(--surface); }
.perm-box { display: flex; flex-direction: column; gap: calc(0.42rem * var(--g)); padding-top: calc(0.28rem * var(--g)); }
</style>

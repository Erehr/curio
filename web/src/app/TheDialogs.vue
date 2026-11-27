<script setup>
import { computed, ref, watch } from 'vue'
import { t } from '../shared/i18n.js'
import Dialog from 'openvue/dialog'
import Select from 'openvue/select'
import InputText from 'openvue/inputtext'
import Button from 'openvue/button'
import Textarea from 'openvue/textarea'
import { S } from './state.js'
import { edit } from '../library/edits.js'
import { run, toast } from './notify.js'
import { openPicker, tagTint } from '../tags/tags.js'
import { refreshLibraries, saveLinks } from '../library/lifecycle.js'
import { refreshSidebar } from '../sidebar/sidebar.js'
import { flushUi } from './preferences.js'
import { call, errorText } from '../platform/ipc.js'
import AppIcon from '../shared/AppIcon.vue'
import AppChip from '../shared/AppChip.vue'
import SettingsWindow from '../settings/SettingsWindow.vue'
import SmartFolderDialog from '../filters/SmartFolderDialog.vue'
import { iconNames } from './icons.js'

function pickChoice(v) {
  const d = S.dialogs.choice
  S.dialogs.choice = null
  d?.pick(v)
}
function cancelChoice() {
  const d = S.dialogs.choice
  S.dialogs.choice = null
  d?.cancel?.()
}
// a name asked for: { title, name, okLabel, done(name) }
function finishName() {
  const d = S.dialogs.name
  const name = d?.name.trim()
  if (!name) return
  S.dialogs.name = null
  d.done(name)
}
// a question whose answer can't be taken back starts on Cancel, so Enter alone doesn't confirm it
const dangerAsked = computed(() => (S.dialogs.confirm?.okSeverity || 'danger') === 'danger')
function confirmOk() {
  const d = S.dialogs.confirm
  S.dialogs.confirm = null
  d?.ok()
}

async function finishLibName() {
  const d = S.dialogs.libName
  const name = d?.name.trim()
  if (!name) return
  S.dialogs.libName = null
  if (d.mode === 'rename') {
    try {
      await call('library.rename', { library: d.path, name })
      refreshLibraries()
      toast(t('app.libName.renamed', { name }))
    } catch (e) {
      toast(errorText(e), 'error')
    }
  } else {
    await flushUi()
    if (await run('new_library', { name, choose: true })) location.reload()
  }
}

const iconSearch = ref('')
const icons = computed(() => {
  const q = iconSearch.value.trim().toLowerCase()
  const list = q ? iconNames.filter((n) => n.includes(q)) : ['folder', 'folder-star', 'photo', 'camera', 'movie', 'music', 'book', 'books', 'brand-windows', 'brand-ubuntu', 'device-desktop', 'device-gamepad-2', 'palette', 'brush', 'pencil', 'heart', 'star', 'bookmark', 'archive', 'box', 'cube', 'code', 'terminal-2', 'world', 'map', 'plane', 'car', 'bike', 'tools-kitchen-2', 'paw', 'leaf', 'flower', 'sun', 'moon', 'cloud', 'flame', 'bolt', 'diamond', 'crown', 'user', 'users', 'mood-smile', 'ghost', 'robot', 'rocket', 'school', 'briefcase', 'calendar', 'clock', 'lock', 'key', 'tag', 'layout-grid', 'printer-3d', 'printer', 'playlist', 'player-play', 'headphones', 'microphone', 'sparkles', 'wand', 'brand-github', 'file-text', 'file-zip', 'typography', 'square-letter-a']
  return list.slice(0, 400)
})
function saveAutoTag() {
  const d = S.dialogs.autoTag
  edit.folder('auto_tags', { id: d.id, value: d.tags.join(', ') })
  const f = S.sidebar.folders.find((x) => x.id === d.id)
  if (f && d.name.trim() && d.name.trim() !== f.name) edit.folder('rename', { id: d.id, name: d.name.trim() })
  S.dialogs.autoTag = null
}
function pickAutoTags(e) {
  const d = S.dialogs.autoTag
  openPicker('tags', e, {
    value: [...d.tags],
    canCreate: true,
    onToggle: (name) => {
      const i = d.tags.indexOf(name)
      if (i >= 0) d.tags.splice(i, 1)
      else d.tags.push(name)
    },
  })
}
function pickIcon(name) {
  const d = S.dialogs.icon
  if (d.kind === 'action') d.form.icon = name
  else if (d.kind === 'folder') edit.folder('icon', { id: d.id, value: name })
  else if (d.kind === 'tagGroup') edit.tag('group_look', { id: d.id, color: d.color ?? null, name })
  else if (d.kind === 'root') edit.locationLook(d.id, { icon: name })
  else if (d.kind === 'library') call('library.setLook', { library: d.path, icon: name }).then(refreshLibraries).catch((e) => toast(errorText(e), 'error'))
  else edit.smart('icon', { id: d.id, icon: name })
  S.dialogs.icon = null
}

// ---------------------------------------------------------------- misc dialogs
// Add web addresses: any text with addresses in it, such as a column pasted from a spreadsheet
const linkText = ref('')
const links = computed(() => [...new Set(linkText.value.match(/https?:\/\/[^\s,;"'<>]+/g) || [])])
function importLinks() {
  const folder = S.scope.kind === 'folder' && S.view === 'grid' ? S.scope.id : null
  saveLinks(links.value, folder)
  linkText.value = ''
  S.dialogs.url = false
}
// Create an example plugin: the kinds src/core/plugins/mod.rs can write
const EXAMPLE_KINDS = computed(() => [['window', 'app-window'], ['tab', 'layout-grid'], ['widget', 'layout-sidebar-right'], ['viewer', 'file-text'], ['program', 'terminal-2'], ['url', 'link']].map(([value, icon]) => ({ value, icon, label: t(`app.example.kinds.${value}.label`), hint: t(`app.example.kinds.${value}.hint`) })))
async function makeExample() {
  const msg = await run('plugin_op', { op: 'example', kind: S.dialogs.examplePlugin.kind }).catch(() => '')
  if (!msg) return
  S.dialogs.examplePlugin = null
  toast(msg)
  refreshSidebar()
}
</script>

<template>
  <!-- Settings -->
  <Dialog v-if="!S.prefsWindow" v-model:visible="S.dialogs.prefs" modal :showHeader="false" :style="{ width: 'min(1180px, 94vw)', height: 'min(820px, 92vh)' }" :contentStyle="{ padding: 0, height: '100%' }" dismissableMask class="prefs-dialog">
    <SettingsWindow v-if="S.dialogs.prefs" />
  </Dialog>

  <SmartFolderDialog />

  <!-- Icon picker (full Tabler set) -->
  <Dialog :visible="!!S.dialogs.icon" @update:visible="(v) => !v && (S.dialogs.icon = null)" :header="$t('app.icons.title')" modal :style="{ width: '44.76rem' }">
    <InputText v-model="iconSearch" :placeholder="$t('app.icons.search', { count: iconNames.length, n: iconNames.length.toLocaleString() })" fluid v-focus />
    <div class="icons">
      <button v-for="n in icons" :key="n" class="ic" :class="{ sel: S.dialogs.icon?.current === n }" v-tooltip="{ value: n, showDelay: 400 }" @click="pickIcon(n)"><AppIcon :name="n" :size="22" /></button>
    </div>
    <div class="row foot"><span class="faint small grow">{{ $t('app.icons.showing', { n: icons.length + (icons.length === 400 ? '+' : '') }) }}</span><Button :label="$t('app.icons.default')" text @click="pickIcon(null)" /></div>
  </Dialog>

  <!-- Tags for whatever lands in a folder -->
  <Dialog :visible="!!S.dialogs.autoTag" :header="$t('app.autoTag.title')" modal :style="{ width: '34.97rem' }" @update:visible="(v) => !v && (S.dialogs.autoTag = null)">
    <template v-if="S.dialogs.autoTag">
      <div class="label">{{ $t('app.autoTag.folder') }}</div>
      <InputText v-model="S.dialogs.autoTag.name" fluid />
      <div class="label auto-label">{{ $t('app.autoTag.tags') }}</div>
      <div class="tag-box" tabindex="0" role="button" @click="pickAutoTags" @keydown.enter.space.prevent="pickAutoTags">
        <AppChip v-for="t in S.dialogs.autoTag.tags" :key="t" class="tag-chip" :label="t" :style="tagTint(t)" removable @remove="S.dialogs.autoTag.tags.splice(S.dialogs.autoTag.tags.indexOf(t), 1)" />
        <span v-if="!S.dialogs.autoTag.tags.length" class="faint">{{ $t('app.autoTag.newTag') }}</span>
      </div>
      <p class="faint small">{{ $t('app.autoTag.about') }}</p>
      <div class="row foot"><span class="grow" /><Button :label="$t('common.saveChanges')" @click="saveAutoTag" /><Button :label="$t('common.cancel')" severity="secondary" @click="S.dialogs.autoTag = null" /></div>
    </template>
  </Dialog>

  <!-- Create an example plugin -->
  <Dialog :visible="!!S.dialogs.examplePlugin" :header="$t('app.example.title')" modal :style="{ width: '36.36rem' }" @update:visible="(v) => !v && (S.dialogs.examplePlugin = null)">
    <template v-if="S.dialogs.examplePlugin">
      <div class="label">{{ $t('app.example.kind') }}</div>
      <Select v-model="S.dialogs.examplePlugin.kind" :options="EXAMPLE_KINDS" optionLabel="label" optionValue="value" fluid>
        <template #option="{ option }"><span class="row"><AppIcon :name="option.icon" /> {{ option.label }}</span></template>
      </Select>
      <p class="faint small">{{ EXAMPLE_KINDS.find((k) => k.value === S.dialogs.examplePlugin.kind)?.hint }}</p>
      <p class="faint small">{{ $t('app.example.about') }}</p>
    </template>
    <template #footer>
      <Button :label="$t('app.example.choose')" @click="makeExample" />
      <Button :label="$t('common.cancel')" severity="secondary" @click="S.dialogs.examplePlugin = null" />
    </template>
  </Dialog>


  <Dialog v-model:visible="S.dialogs.url" :header="$t('app.links.title')" modal :style="{ width: '43.36rem' }">
    <Textarea v-model="linkText" v-focus class="field links" rows="10" spellcheck="false" :placeholder="$t('app.links.placeholder')" />
    <div class="row foot"><span class="grow faint small">{{ links.length ? $t('app.links.found', { count: links.length }) + ' ' : '' }}{{ $t('app.links.about') }}</span><Button :label="links.length ? $t('app.links.importCount', { count: links.length }) : $t('app.links.import')" :disabled="!links.length" @click="importLinks" /></div>
  </Dialog>

  <Dialog :visible="!!S.dialogs.libName" @update:visible="(v) => !v && (S.dialogs.libName = null)" modal :header="S.dialogs.libName?.mode === 'rename' ? $t('app.libName.rename') : $t('app.libName.new')" :style="{ width: '32.17rem' }">
    <template v-if="S.dialogs.libName">
      <div class="label">{{ $t('app.name.name') }}</div>
      <InputText v-model="S.dialogs.libName.name" v-focus fluid @keydown.enter="finishLibName" />
      <p class="faint small">{{ S.dialogs.libName.mode === 'rename' ? $t('app.libName.renameAbout', { name: S.dialogs.libName.name.trim() }) : $t('app.libName.newAbout', { name: S.dialogs.libName.name.trim() }) }}</p>
      <div class="row foot"><span class="grow" /><Button :label="S.dialogs.libName.mode === 'rename' ? $t('app.libName.renameButton') : $t('app.libName.chooseFolder')" :disabled="!S.dialogs.libName.name.trim()" @click="finishLibName" /><Button :label="$t('common.cancel')" severity="secondary" @click="S.dialogs.libName = null" /></div>
    </template>
  </Dialog>

  <Dialog :visible="!!S.dialogs.name" @update:visible="(v) => !v && (S.dialogs.name = null)" modal :header="S.dialogs.name?.title" :style="{ width: '29.37rem' }">
    <template v-if="S.dialogs.name">
      <div class="label">{{ $t('app.name.name') }}</div>
      <InputText v-model="S.dialogs.name.name" v-focus fluid spellcheck="false" maxlength="200" @keydown.enter="finishName" />
      <div class="row foot"><span class="grow" /><Button :label="S.dialogs.name.okLabel || $t('common.save')" :disabled="!S.dialogs.name.name.trim()" @click="finishName" /><Button :label="$t('common.cancel')" severity="secondary" @click="S.dialogs.name = null" /></div>
    </template>
  </Dialog>


  <Dialog :visible="!!S.dialogs.confirm" @update:visible="(v) => !v && (S.dialogs.confirm = null)" modal :header="S.dialogs.confirm?.title" :style="{ width: '33.57rem' }">
    <p>{{ S.dialogs.confirm?.body }}</p>
    <div class="row foot"><span class="grow" /><Button :label="S.dialogs.confirm?.okLabel || $t('common.continue')" :severity="S.dialogs.confirm?.okSeverity || 'danger'" :autofocus="!dangerAsked" @click="confirmOk" /><Button :label="$t('common.cancel')" severity="secondary" :autofocus="dangerAsked" @click="S.dialogs.confirm = null" /></div>
  </Dialog>
  <Dialog :visible="!!S.dialogs.choice" @update:visible="(v) => !v && cancelChoice()" modal :header="S.dialogs.choice?.title" :style="{ width: '36.36rem' }">
    <p>{{ S.dialogs.choice?.body }}</p>
    <div class="row foot"><span class="grow" /><Button v-for="(c, i) in S.dialogs.choice?.choices || []" :key="c.value" :label="c.label" :severity="c.severity" :autofocus="i === 0" @click="pickChoice(c.value)" /><Button :label="$t('common.cancel')" severity="secondary" @click="cancelChoice" /></div>
  </Dialog>
</template>

<style scoped>
.small { font-size: 0.85rem; }
.label { font-weight: 500; margin-bottom: calc(0.42rem * var(--g)); }
.auto-label { margin-top: calc(1.12rem * var(--g)); }
.tag-box { display: flex; flex-wrap: wrap; align-items: flex-start; gap: calc(0.42rem * var(--g)); min-height: round(5.31rem, 1px); padding: calc(0.56rem * var(--g)); border-radius: calc(6px * var(--r)); background: var(--input); border: 1px solid var(--input-border); cursor: pointer; }
.tag-box:focus { border-color: var(--accent); outline: 0; }
.links { font-family: var(--mono-font); font-size: 0.92rem; resize: vertical; }
.links::placeholder { color: var(--text-faint); opacity: 1; }
.foot { margin-top: calc(1.12rem * var(--g)); gap: calc(0.7rem * var(--g)); }
.icons { display: grid; grid-template-columns: repeat(auto-fill, minmax(3.08rem, 1fr)); gap: calc(0.28rem * var(--g)); max-height: 29.37rem; overflow-y: auto; margin-top: calc(0.7rem * var(--g)); }
.ic { height: round(3.08rem, 1px); border-radius: calc(8px * var(--r)); border: 1px solid transparent; background: transparent; color: var(--text); cursor: pointer; }
.ic:hover { background: var(--hover); }
.ic.sel { border-color: var(--accent); }
</style>

<style>
.prefs-dialog.p-dialog { background: var(--panel); }
.prefs-dialog .p-dialog-content { background: var(--panel); overflow: hidden; }
</style>

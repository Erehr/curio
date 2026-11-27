<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import Select from 'openvue/select'
import Button from 'openvue/button'
import Slider from 'openvue/slider'
import InputText from 'openvue/inputtext'
import InputGroup from 'openvue/inputgroup'
import Tag from 'openvue/tag'
import Message from 'openvue/message'
import { listedRoot, rootIcon } from '../sidebar/sidebar.js'
import { relocateRoot, importPaths } from '../library/lifecycle.js'
import { S } from '../app/state.js'
import { run, toast } from '../app/notify.js'
import { call, on } from '../platform/ipc.js'
import AppIcon from '../shared/AppIcon.vue'
import { useSettingsData } from './settingsData.js'
import { t } from '../shared/i18n.js'

const { libSettings, saveLib, shown } = useSettingsData()
const storeInfo = ref(null)
const fileOps = ref(null)
const orphans = ref(null)
const showOrphans = ref(false)
const cacheCustom = ref(false)
async function loadStore() {
  if (!(S.dialogs.prefs && shown('storage') && S.libOpen)) return
  storeInfo.value = JSON.parse(await call('maintenance', { op: 'paths' }).catch(() => '{}'))
  fileOps.value = await call('file_ops', { action: 'list', ids: [] }).catch(() => null)
  orphans.value = await call('managed_orphans', { action: 'list', paths: [] }).catch(() => null)
}
async function orphansDo(action) {
  await run('managed_orphans', { action, paths: orphans.value.items })
  showOrphans.value = false
  setTimeout(loadStore, 500)
}
async function fileOpsDo(action, ids) {
  await run('file_ops', { action, ids })
  loadStore()
}
watch(() => [S.dialogs.prefs, shown('storage'), S.startup.current], loadStore, { immediate: true })
const offLib = on('lib', (p) => { if (p?.kind === 'done' || p?.kind === 'error') loadStore() })
onBeforeUnmount(() => offLib.then((f) => f()))
async function addLocation() {
  const p = await call('pick', { kind: 'folder' })
  if (p) importPaths(p, false, null)
}
async function maint(op) {
  const m = await run('maintenance', { op })
  if (m) toast(m)
  if (op === 'cache-dir') cacheCustom.value = true
  else if (op === 'cache-default') cacheCustom.value = false
  loadStore()
}
async function copyPath(p) {
  if (!p) return
  await navigator.clipboard.writeText(p)
  toast(t('settings.about.pathCopied'))
}
function askClearThumbs() {
  S.dialogs.confirm = {
    title: t('settings.storage.cache.clearTitle'),
    body: t('settings.storage.cache.clearBody'),
    okLabel: t('settings.storage.cache.clear'),
    ok: () => maint('clear-thumbs'),
  }
}
// what the location's drive looks like; making it read-only stays the user's choice
const ROOT_HINTS = new Set(['network', 'removable', 'read_only_media'])
const ACCESS = computed(() => ['read_only', 'full', 'organize'].map((v) => ({ v, l: t(`settings.storage.access.${v}.label`) })))
const accessTip = (r) => t(`settings.storage.access.${['read_only', 'organize'].includes(r.access) ? r.access : 'full'}.tip`)
const roots = computed(() => S.sidebar.roots.filter(listedRoot))
</script>

<template>
  <section class="s-section">
    <div class="s-section-head">
      <span class="s-section-title">{{ $t('settings.storage.locations.title') }}</span>
      <div class="s-section-actions">
        <Button v-if="roots.length" :label="$t('settings.storage.locations.rescanAll')" severity="secondary" size="small" @click="maint('rescan')"><template #icon><AppIcon name="refresh" :size="15" /></template></Button>
        <Button :label="$t('settings.storage.locations.add')" severity="secondary" size="small" @click="addLocation"><template #icon><AppIcon name="plus" :size="15" /></template></Button>
      </div>
    </div>
    <div class="s-card">
      <div v-if="!roots.length" class="s-empty">{{ $t('settings.storage.locations.empty') }}</div>
      <div v-for="r in roots" :key="r.id" class="s-row">
        <AppIcon :name="rootIcon(r)" :size="18" class="loc-ico" :style="{ color: r.color || undefined }" />
        <div class="s-meta">
          <span class="s-label">
            <span class="ellipsis">{{ r.label }}</span>
            <Tag v-if="ROOT_HINTS.has(r.hint)" severity="warn" :value="$t(`settings.storage.hints.${r.hint}.label`)" v-tooltip.bottom="$t(`settings.storage.hints.${r.hint}.tip`)" />
            <Tag severity="secondary" :value="r.asset_count.toLocaleString()" />
          </span>
          <span class="s-desc s-mono ellipsis">{{ r.path }}</span>
        </div>
        <div class="s-ctrl">
          <Select class="access-select" :modelValue="r.access" :options="ACCESS" optionLabel="l" optionValue="v" v-tooltip="accessTip(r)" @update:modelValue="(v) => run('location.setAccess', { id: r.id, access: v })" />
          <Button severity="secondary" text size="small" v-tooltip="$t('settings.storage.locations.rescan')" :aria-label="$t('settings.storage.locations.rescan')" @click="run('location.rescan', { id: r.id })"><template #icon><AppIcon name="refresh" :size="16" /></template></Button>
          <Button severity="secondary" text size="small" v-tooltip="$t('settings.storage.locations.relocate')" :aria-label="$t('settings.storage.locations.relocate')" @click="relocateRoot(r.id)"><template #icon><AppIcon name="route" :size="16" /></template></Button>
        </div>
      </div>
    </div>
  </section>

  <section v-if="storeInfo" class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.storage.library.sectionTitle') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.storage.library.title') }} <Tag severity="secondary" :value="$t('settings.storage.library.schema', { n: storeInfo.schema })" /></span>
          <span class="s-desc s-mono ellipsis" :title="$t('settings.storage.library.ids')">{{ storeInfo.library_id }} · {{ storeInfo.installation_id }}</span>
        </div>
        <div class="s-ctrl w-lg">
          <InputGroup>
            <InputText class="s-mono" :modelValue="storeInfo.root" :title="storeInfo.root" readonly />
            <Button severity="secondary" size="small" :aria-label="$t('settings.api.tokens.copy')" @click="copyPath(storeInfo.root)"><template #icon><AppIcon name="copy" :size="15" /></template></Button>
          </InputGroup>
        </div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.storage.cache.title') }}</span></div>
        <div class="s-ctrl w-lg">
          <InputGroup>
            <InputText class="s-mono" :modelValue="storeInfo.thumbs" :title="storeInfo.thumbs" readonly />
            <Button :label="$t('settings.storage.cache.move')" severity="secondary" size="small" @click="maint('cache-dir')" />
            <Button v-if="cacheCustom || (storeInfo.root && !storeInfo.thumbs.startsWith(storeInfo.root))" :label="$t('settings.storage.defaultLocation')" size="small" severity="secondary" text @click="maint('cache-default')" />
          </InputGroup>
        </div>
      </div>
      <div v-if="libSettings" class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.storage.cache.limitLabel') }}</span></div>
        <div class="s-ctrl">
          <div class="s-slider-wrap">
            <Slider v-model="libSettings.cache_limit_mb" :min="256" :max="65536" :step="256" @slideend="saveLib" />
            <span class="s-slider-val">{{ $t('settings.storage.cache.mb', { size: libSettings.cache_limit_mb }) }}</span>
          </div>
        </div>
      </div>
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.storage.cache.clearLabel') }}</span>
          <span class="s-desc">{{ $t('settings.storage.cache.clearHint') }}</span>
        </div>
        <div class="s-ctrl">
          <Button :label="$t('settings.storage.cache.clear')" size="small" severity="secondary" @click="askClearThumbs" />
        </div>
      </div>
    </div>
  </section>

  <section v-if="fileOps?.pending || fileOps?.failed || orphans?.count" class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.storage.files.title') }}</span></div>
    <div class="s-card">
      <div v-if="orphans?.count" class="s-subblock">
        <span class="s-desc">{{ $t('settings.storage.files.orphans', { count: orphans.count.toLocaleString() }, orphans.count) }}</span>
        <div v-if="showOrphans" class="s-nested">
          <div v-for="p in orphans.items" :key="p" class="s-mono ellipsis">{{ p }}</div>
          <div v-if="orphans.count > orphans.items.length" class="s-desc">{{ $t('settings.storage.files.more', { count: (orphans.count - orphans.items.length).toLocaleString() }) }}</div>
        </div>
        <div class="s-actions">
          <Button :label="showOrphans ? $t('settings.storage.files.hide') : $t('settings.storage.files.show')" size="small" severity="secondary" text @click="showOrphans = !showOrphans" />
          <Button :label="$t('settings.storage.files.adopt')" size="small" severity="secondary" @click="orphansDo('adopt')" />
          <Button :label="$t('settings.storage.files.recycle')" size="small" severity="secondary" text @click="orphansDo('recycle')" />
        </div>
      </div>
      <div v-if="fileOps?.pending" class="s-subblock">
        <Message severity="info" :closable="false">{{ $t('settings.storage.files.pending', { count: fileOps.pending.toLocaleString() }, fileOps.pending) }}</Message>
      </div>
      <div v-if="fileOps?.failed" class="s-subblock">
        <Message severity="warn" :closable="false">{{ $t('settings.storage.files.failed', { count: fileOps.failed.toLocaleString() }, fileOps.failed) }}</Message>
        <div class="s-nested">
          <div v-for="f in fileOps.items" :key="f.id" class="row gap">
            <span class="grow ellipsis small"><span class="s-mono">{{ f.path }}</span><span class="faint"> · {{ f.error }}</span></span>
            <Button v-if="f.kind === 'recycle'" :label="$t('settings.storage.files.retry')" severity="secondary" size="small" text @click="fileOpsDo('retry', [f.id])" />
            <Button :label="$t('settings.storage.files.forget')" severity="secondary" size="small" text v-tooltip="$t('settings.storage.files.cancel')" @click="fileOpsDo('forget', [f.id])" />
          </div>
        </div>
        <div class="s-actions">
          <Button :label="$t('settings.storage.files.retryAll')" size="small" severity="secondary" @click="fileOpsDo('retry', fileOps.items.filter((f) => f.kind === 'recycle').map((f) => f.id))" />
          <Button :label="$t('settings.storage.files.forgetAll')" size="small" severity="secondary" text @click="fileOpsDo('forget', fileOps.items.map((f) => f.id))" />
        </div>
      </div>
    </div>
  </section>

  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.storage.import.title') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.storage.import.eagleLabel') }}</span>
        </div>
        <div class="s-ctrl s-actions">
          <Button :label="$t('settings.storage.backups.eagle')" size="small" severity="secondary" @click="run('import_dot_library', { copy: true })" />
          <Button :label="$t('settings.storage.backups.eagleLocation')" size="small" severity="secondary" @click="run('import_dot_library', { copy: false })" />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.loc-ico { flex: none; color: var(--text-muted); }
.access-select { width: round(11.2rem, 1px); flex: none; }
</style>

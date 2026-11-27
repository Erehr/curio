<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import Button from 'openvue/button'
import Slider from 'openvue/slider'
import SelectButton from 'openvue/selectbutton'
import InputText from 'openvue/inputtext'
import InputGroup from 'openvue/inputgroup'
import Tag from 'openvue/tag'
import Message from 'openvue/message'
import { S } from '../app/state.js'
import { run, toast } from '../app/notify.js'
import { call, on } from '../platform/ipc.js'
import AppIcon from '../shared/AppIcon.vue'
import { useSettingsData } from './settingsData.js'
import { t } from '../shared/i18n.js'
import { sizeText } from '../shared/numbers.js'

const { libSettings, loadLib, saveLib, shown } = useSettingsData()
const storeInfo = ref(null)
const backups = ref(null)
async function loadStore() {
  if (!(S.dialogs.prefs && shown('backup') && S.libOpen)) return
  storeInfo.value = JSON.parse(await call('maintenance', { op: 'paths' }).catch(() => '{}'))
}
watch(() => [S.dialogs.prefs, shown('backup'), S.startup.current], () => {
  backups.value = null
  loadStore()
}, { immediate: true })
// a backup reports back with done / error
const offLib = on('lib', (p) => { if (p?.kind === 'done' || p?.kind === 'error') loadStore() })
onBeforeUnmount(() => offLib.then((f) => f()))

async function maint(op) {
  const m = await run('maintenance', { op })
  if (m) toast(m)
  if (op.startsWith('backup-')) {
    // the folder was saved on the Rust side; refresh the snapshot so a later saveLib doesn't send the old one back
    await loadLib()
    loadStore()
  }
}
const when = (ms) => (ms ? new Date(ms).toLocaleString() : '')
const BACKUP_KINDS = new Set(['manual', 'scheduled', 'pre-migration', 'before-restore', 'unknown'])
const backupKind = (k) => (BACKUP_KINDS.has(k) ? t(`settings.storage.backups.kinds.${k}`) : k)
const backupMeta = (b) => [b.schema ? t('settings.storage.backups.schema', { n: b.schema }) : '', sizeText(b.size)].filter(Boolean).join(' · ')
async function listBackups() {
  backups.value = backups.value ? null : await run('backup_list')
}
async function restore(path, confirmed = false) {
  const r = await run('backup_restore', { path, confirmed })
  if (r?.confirm) {
    S.dialogs.confirm = { title: t('settings.storage.restore.title'), body: r.confirm, okLabel: t('settings.storage.restore.ok'), ok: () => restore(r.path, true) }
  } else if (r?.message) toast(r.message)
}
function askRestore(b) {
  S.dialogs.confirm = {
    title: t('settings.storage.restore.title'),
    body: b.other_library ? t('settings.storage.restore.bodyOther', { date: when(b.created_at) }) : t('settings.storage.restore.body', { date: when(b.created_at) }),
    okLabel: t('settings.storage.restore.ok'),
    ok: () => restore(b.path, true),
  }
}
const BACKUP_EVERY = computed(() => [[0, 'manual'], [24, 'daily'], [168, 'weekly']].map(([value, k]) => ({ value, label: t(`settings.storage.backups.every.${k}`) })))
</script>

<template>
  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.storage.backups.sectionTitle') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.storage.backups.snapshotLabel') }}</span>
          <span v-if="storeInfo?.backup?.last_ok_at" class="s-desc">{{ $t('settings.storage.backups.last', { date: when(storeInfo.backup.last_ok_at) }) }}</span>
          <span v-else-if="storeInfo" class="s-desc">{{ $t('settings.storage.backups.noneYet') }}</span>
        </div>
        <div class="s-ctrl s-actions">
          <Button :label="$t('settings.storage.backups.now')" size="small" @click="maint('backup')" />
          <Button :label="backups ? $t('settings.storage.backups.hide') : $t('settings.storage.backups.restore')" size="small" severity="secondary" @click="listBackups" />
        </div>
      </div>
      <div v-if="storeInfo?.backup?.last_error" class="s-subblock">
        <Message severity="warn" :closable="false">{{ $t('settings.storage.backups.failed', { date: when(storeInfo.backup.last_error_at), error: storeInfo.backup.last_error }) }}</Message>
      </div>
      <div v-if="backups" class="s-subblock">
        <div class="s-nested">
          <div v-if="!backups.length" class="s-desc">{{ $t('settings.storage.backups.none') }}</div>
          <div v-for="b in backups" :key="b.path" class="backup-row">
            <AppIcon :name="b.checked ? 'circle-check' : 'database'" :size="16" :class="b.checked ? 'ok' : 'faint'" />
            <div class="s-meta">
              <span class="s-label">
                <span>{{ when(b.created_at) }}</span>
                <Tag severity="secondary" :value="backupKind(b.kind)" />
                <Tag v-if="b.other_library" severity="warn" :value="$t('settings.storage.backups.otherLibrary')" />
              </span>
              <span v-if="backupMeta(b)" class="s-desc s-mono">{{ backupMeta(b) }}</span>
            </div>
            <Button :label="$t('settings.storage.restore.ok')" size="small" severity="secondary" text @click="askRestore(b)" />
          </div>
          <div class="s-actions">
            <Button :label="$t('settings.storage.backups.chooseFile')" size="small" severity="secondary" text @click="restore('')" />
          </div>
        </div>
      </div>
      <template v-if="libSettings">
        <div class="s-row">
          <div class="s-meta">
            <span class="s-label">{{ $t('settings.storage.backups.auto') }}</span>
            <span class="s-desc">{{ $t('settings.storage.backups.hint') }}</span>
          </div>
          <div class="s-ctrl"><SelectButton v-model="libSettings.backup_every_hours" :options="BACKUP_EVERY" optionLabel="label" optionValue="value" :allowEmpty="false" @change="saveLib" /></div>
        </div>
        <div class="s-row">
          <div class="s-meta"><span class="s-label">{{ $t('settings.storage.backups.keepLabel') }}</span></div>
          <div class="s-ctrl">
            <div class="s-slider-wrap">
              <Slider v-model="libSettings.backup_keep" :min="0" :max="50" @slideend="saveLib" />
              <span class="s-slider-val">{{ libSettings.backup_keep ? libSettings.backup_keep : $t('settings.storage.backups.keepAllShort') }}</span>
            </div>
          </div>
        </div>
      </template>
      <div v-if="storeInfo" class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.storage.backups.folderLabel') }}</span></div>
        <div class="s-ctrl w-lg">
          <InputGroup>
            <InputText class="s-mono" :modelValue="storeInfo.backups" :title="storeInfo.backups" readonly />
            <Button :label="$t('settings.storage.backups.change')" size="small" severity="secondary" @click="maint('backup-dir')" />
            <Button v-if="libSettings?.backup_dir" :label="$t('settings.storage.defaultLocation')" size="small" severity="secondary" text @click="maint('backup-default')" />
          </InputGroup>
        </div>
      </div>
    </div>
  </section>

  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.storage.maintenance.title') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.storage.maintenance.integrityLabel') }}</span>
          <span class="s-desc">{{ $t('settings.storage.maintenance.integrityHint') }}</span>
        </div>
        <div class="s-ctrl"><Button :label="$t('settings.storage.backups.integrity')" size="small" severity="secondary" @click="maint('integrity')" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.storage.maintenance.optimizeLabel') }}</span>
          <span class="s-desc">{{ $t('settings.storage.maintenance.optimizeHint') }}</span>
        </div>
        <div class="s-ctrl"><Button :label="$t('settings.storage.backups.optimize')" size="small" severity="secondary" @click="maint('optimize')" /></div>
      </div>
    </div>
  </section>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.backup-row { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); padding: calc(0.28rem * var(--g)) 0; }
</style>

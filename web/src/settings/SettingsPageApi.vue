<script setup>
import { ref, watch } from 'vue'
import InputText from 'openvue/inputtext'
import InputGroup from 'openvue/inputgroup'
import Button from 'openvue/button'
import Checkbox from 'openvue/checkbox'
import ToggleSwitch from 'openvue/toggleswitch'
import Tag from 'openvue/tag'
import Message from 'openvue/message'
import { S } from '../app/state.js'
import { toast } from '../app/notify.js'
import { call, errorText } from '../platform/ipc.js'
import AppIcon from '../shared/AppIcon.vue'
import { useSettingsData } from './settingsData.js'
import { t } from '../shared/i18n.js'

const { appCfg, setCfg, shown } = useSettingsData()
// a Mac shows a program kept running in its menu bar
const MAC = navigator.userAgent.includes('Mac')
// the permission ids a token can hold, as the API names them; what each allows is in the catalog under its id
const API_PERMS = ['library.read', 'metadata.read', 'metadata.write', 'organize', 'files.read', 'files.write', 'files.outside', 'library.manage', 'ui']
const API_PRESETS = [['readOnly', ['library.read', 'metadata.read']], ['readAdd', ['library.read', 'metadata.read', 'metadata.write', 'files.write']], ['all', API_PERMS]]
const apiLibs = ref([])
watch(() => [S.dialogs.prefs, shown('api')], async () => {
  if (S.dialogs.prefs && shown('api')) apiLibs.value = await call('library_list').catch(() => [])
}, { immediate: true })
const tokenForm = ref(null)
const newSecret = ref(null)
function openTokenForm(tok) {
  newSecret.value = null
  tokenForm.value = tok ? { id: tok.id, name: tok.name, perms: [...tok.perms], origins: tok.origins.join('\n'), libraries: [...tok.libraries] } : { id: null, name: '', perms: [...API_PRESETS[0][1]], origins: '', libraries: [] }
}
function toggleIn(list, v, on) {
  const i = list.indexOf(v)
  if (on && i < 0) list.push(v)
  if (!on && i >= 0) list.splice(i, 1)
}
async function saveToken() {
  const f = tokenForm.value
  try {
    const r = await call('api_token_op', { op: f.id ? 'update' : 'create', id: f.id, name: f.name, perms: f.perms, origins: f.origins.split(/\s+/).filter(Boolean), libraries: f.libraries })
    if (!f.id) newSecret.value = { name: r.token.name, secret: r.secret }
    tokenForm.value = null
    appCfg.value = await call('app_config', { set: null })
  } catch (e) {
    toast(errorText(e), 'error')
  }
}
function deleteToken(token) {
  S.dialogs.confirm = {
    title: t('settings.api.delete.title', { name: token.name }),
    body: t('settings.api.delete.body'),
    okLabel: t('settings.api.delete.ok'),
    ok: async () => {
      await call('api_token_op', { op: 'delete', id: token.id }).catch((e) => toast(errorText(e), 'error'))
      appCfg.value = await call('app_config', { set: null }).catch(() => appCfg.value)
    },
  }
}
function tokenReach(token) {
  const libs = token.libraries.length ? token.libraries.map((id) => apiLibs.value.find((l) => l.id === id)?.name || t('settings.api.reach.forgotten')).join(', ') : t('settings.api.reach.everyLibrary')
  return [libs, token.origins.length ? token.origins.join(', ') : t('settings.api.reach.anyOrigin'), token.created ? t('settings.api.reach.made', { date: new Date(token.created).toLocaleDateString() }) : ''].filter(Boolean).join(' · ')
}
async function copyText(text, msg) {
  await navigator.clipboard.writeText(text)
  toast(msg)
}
</script>

<template>
  <template v-if="appCfg">
    <section class="s-section">
      <div class="s-section-head"><span class="s-section-title">{{ $t('settings.api.service.title') }}</span></div>
      <div class="s-card">
        <div class="s-row">
          <div class="s-meta">
            <span class="s-label">{{ $t('settings.api.service.enable') }}</span>
            <span class="s-desc">{{ $t('settings.api.service.local') }}</span>
          </div>
          <div class="s-ctrl"><ToggleSwitch :modelValue="appCfg.apiEnabled" :aria-label="$t('settings.api.service.enable')" @update:modelValue="(v) => setCfg({ apiEnabled: v })" /></div>
        </div>
        <div v-if="appCfg.apiEnabled" class="s-row indented">
          <div class="s-meta">
            <span class="s-label">
              <span>{{ $t('settings.api.service.endpoint') }}</span>
              <Tag :severity="appCfg.apiServing ? 'success' : 'secondary'" :value="appCfg.apiServing ? $t('settings.api.service.servedHere') : $t('settings.api.service.servedElsewhere')" />
            </span>
          </div>
          <div class="s-ctrl w-lg">
            <InputGroup>
              <InputText class="s-mono" :modelValue="appCfg.apiUrl" :title="appCfg.apiUrl" readonly />
              <Button severity="secondary" size="small" :aria-label="$t('settings.api.tokens.copy')" @click="copyText(appCfg.apiUrl, $t('settings.api.urlCopied'))"><template #icon><AppIcon name="copy" :size="15" /></template></Button>
            </InputGroup>
          </div>
        </div>
        <div class="s-row">
          <div class="s-meta"><span class="s-label">{{ $t('settings.api.service.port') }}</span></div>
          <div class="s-ctrl w-sm"><InputText :modelValue="String(appCfg.apiPort)" class="s-mono" @change="(e) => setCfg({ apiPort: Number(e.target.value) || 41733 })" /></div>
        </div>
        <div class="s-row">
          <div class="s-meta">
            <span class="s-label">{{ $t('settings.api.service.browserStart') }}</span>
            <span class="s-desc">{{ appCfg.browserStartAllowed ? (MAC ? $t('settings.api.service.browserStartHintMac') : $t('settings.api.service.browserStartHint')) : $t('settings.api.service.browserStartOff') }}</span>
          </div>
          <div class="s-ctrl"><ToggleSwitch :modelValue="appCfg.browserStartAllowed && appCfg.browserStart" :disabled="!appCfg.browserStartAllowed" :aria-label="$t('settings.api.service.browserStart')" @update:modelValue="(v) => setCfg({ browserStart: v })" /></div>
        </div>
      </div>
    </section>

    <section class="s-section">
      <div class="s-section-head">
        <span class="s-section-title">{{ $t('settings.api.tokens.title') }} <Tag v-if="appCfg.apiTokens.length" severity="secondary" :value="String(appCfg.apiTokens.length)" /></span>
        <div class="s-section-actions">
          <Button :label="$t('settings.api.tokens.new')" size="small" severity="secondary" :disabled="!!tokenForm" @click="openTokenForm(null)"><template #icon><AppIcon name="plus" :size="15" /></template></Button>
        </div>
      </div>

      <Message v-if="newSecret" severity="success" :closable="false">
        <div class="secret-box">
          <i18n-t keypath="settings.api.tokens.secret" tag="div"><template #name><b>{{ newSecret.name }}</b></template></i18n-t>
          <InputGroup>
            <InputText class="s-mono" :modelValue="newSecret.secret" readonly />
            <Button :label="$t('settings.api.tokens.copy')" size="small" @click="copyText(newSecret.secret, $t('settings.api.copied'))" />
            <Button severity="secondary" text size="small" :aria-label="$t('settings.api.tokens.done')" @click="newSecret = null"><template #icon><AppIcon name="x" :size="15" /></template></Button>
          </InputGroup>
        </div>
      </Message>

      <div v-if="tokenForm" class="s-card">
        <div class="s-card-pad">
          <div class="s-form-grid">
            <div class="s-field">
              <label class="s-field-label">{{ $t('settings.api.tokens.name') }}</label>
              <InputText v-model="tokenForm.name" :placeholder="$t('settings.api.tokens.namePlaceholder')" />
            </div>
            <div class="s-field">
              <label class="s-field-label">{{ $t('settings.api.tokens.origins') }}</label>
              <InputText v-model="tokenForm.origins" class="s-mono" :placeholder="$t('settings.api.tokens.originsPlaceholder')" />
            </div>
          </div>
          <div class="s-field">
            <div class="s-toolbar">
              <span class="s-field-label">{{ $t('settings.api.tokens.allow') }}</span>
              <div class="s-actions">
                <Button v-for="[l, ps] in API_PRESETS" :key="l" :label="$t(`settings.api.presets.${l}`)" size="small" severity="secondary" @click="tokenForm.perms = [...ps]" />
              </div>
            </div>
            <div class="s-check-grid perm-list">
              <label v-for="p in API_PERMS" :key="p" class="opt top-opt"><Checkbox :modelValue="tokenForm.perms.includes(p)" binary @update:modelValue="(v) => toggleIn(tokenForm.perms, p, v)" /><span><span class="s-mono">{{ p }}</span><br /><span class="s-desc">{{ $t(`settings.api.perms.${p}`) }}</span></span></label>
            </div>
          </div>
          <div class="s-field">
            <span class="s-field-label">{{ $t('settings.api.tokens.libraries') }}</span>
            <label class="opt"><Checkbox :modelValue="!tokenForm.libraries.length" binary @update:modelValue="(v) => (tokenForm.libraries = v ? [] : apiLibs.filter((l) => l.id).map((l) => l.id))" /> {{ $t('settings.api.reach.everyLibrary') }}</label>
            <div v-if="tokenForm.libraries.length" class="s-check-grid">
              <label v-for="l in apiLibs.filter((x) => x.id)" :key="l.id" class="opt"><Checkbox :modelValue="tokenForm.libraries.includes(l.id)" binary @update:modelValue="(v) => toggleIn(tokenForm.libraries, l.id, v)" /> {{ l.name }}</label>
            </div>
            <span class="s-desc">{{ $t('settings.api.tokens.librariesHint') }}</span>
          </div>
          <div class="s-actions right-actions">
            <Button :label="tokenForm.id ? $t('common.save') : $t('settings.api.tokens.make')" size="small" :disabled="!tokenForm.perms.length" @click="saveToken" />
            <Button :label="$t('common.cancel')" size="small" severity="secondary" @click="tokenForm = null" />
          </div>
        </div>
      </div>

      <div class="s-card">
        <div v-if="!appCfg.apiTokens.length && !tokenForm" class="s-empty">{{ $t('settings.api.tokens.none') }}</div>
        <div v-for="tok in appCfg.apiTokens" :key="tok.id" class="s-row">
          <AppIcon name="key" :size="17" class="faint key-ico" />
          <div class="s-meta">
            <span class="s-label">
              <span>{{ tok.name }}</span>
              <Tag severity="secondary" :value="$t('settings.api.tokens.permCount', { count: tok.perms.length.toLocaleString() }, tok.perms.length)" v-tooltip.bottom="tok.perms.join(', ')" />
            </span>
            <span class="s-desc ellipsis">{{ tokenReach(tok) }}</span>
          </div>
          <div class="s-ctrl">
            <Button severity="secondary" text size="small" v-tooltip="$t('common.edit')" :aria-label="$t('common.edit')" @click="openTokenForm(tok)"><template #icon><AppIcon name="pencil" :size="15" /></template></Button>
            <Button severity="secondary" text size="small" v-tooltip="$t('settings.api.delete.ok')" :aria-label="$t('settings.api.delete.ok')" @click="deleteToken(tok)"><template #icon><AppIcon name="trash" :size="15" /></template></Button>
          </div>
        </div>
      </div>
    </section>
  </template>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.secret-box { display: flex; flex-direction: column; gap: calc(0.49rem * var(--g)); width: 100%; }
.top-opt { align-items: flex-start; }
.perm-list { padding-top: calc(0.28rem * var(--g)); }
.right-actions { justify-content: flex-end; }
.key-ico { flex: none; }
</style>

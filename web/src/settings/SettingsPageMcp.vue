<script setup>
import { computed, ref } from 'vue'
import Checkbox from 'openvue/checkbox'
import ToggleSwitch from 'openvue/toggleswitch'
import InputText from 'openvue/inputtext'
import InputGroup from 'openvue/inputgroup'
import Button from 'openvue/button'
import Tag from 'openvue/tag'
import AppIcon from '../shared/AppIcon.vue'
import { toast } from '../app/notify.js'
import { useSettingsData } from './settingsData.js'
import { t } from '../shared/i18n.js'

const { appCfg, setCfg } = useSettingsData()
const AUTH_HEADER = 'Authorization: Bearer <token>'
// the MCP tools, sorted into families
const MCP_FAMILIES = [['app', /^(app|library)_/], ['item', /^(item|selection)_/], ['folder', /^folder_/], ['smartFolder', /^smartFolder_/], ['tag', /^tag_/], ['tagGroup', /^tagGroup_/], ['field', /^field_/], ['location', /^location_/], ['query', /^query_/]]
const mcpOpen = ref(new Set())
const mcpFamilies = computed(() => {
  const all = appCfg.value?.mcpTools || []
  const off = new Set(appCfg.value?.mcpDisabled || [])
  const left = new Set(all)
  return MCP_FAMILIES.map(([name, re]) => {
    const tools = all.filter((x) => left.has(x) && re.test(x))
    tools.forEach((x) => left.delete(x))
    return { name, tools, on: tools.filter((x) => !off.has(x)).length }
  }).filter((f) => f.tools.length)
})
const mcpOnCount = computed(() => mcpFamilies.value.reduce((n, f) => n + f.on, 0))
const mcpToolCount = computed(() => mcpFamilies.value.reduce((n, f) => n + f.tools.length, 0))
function setMcpTools(names, on) {
  const off = new Set(appCfg.value?.mcpDisabled || [])
  for (const n of names) {
    if (on) off.delete(n)
    else off.add(n)
  }
  setCfg({ mcpDisabled: [...off] })
}
function toggleMcpFamily(name) {
  const open = new Set(mcpOpen.value)
  if (open.has(name)) open.delete(name)
  else open.add(name)
  mcpOpen.value = open
}
async function copyUrl() {
  await navigator.clipboard.writeText(appCfg.value.mcpUrl)
  toast(t('settings.api.urlCopied'))
}
</script>

<template>
  <template v-if="appCfg">
    <section class="s-section">
      <div class="s-section-head"><span class="s-section-title">{{ $t('settings.mcp.service.title') }}</span></div>
      <div class="s-card">
        <div class="s-row">
          <div class="s-meta">
            <span class="s-label">{{ $t('settings.mcp.service.enable') }}</span>
            <i18n-t keypath="settings.mcp.service.token" tag="span" class="s-desc"><template #header><span class="s-mono">{{ AUTH_HEADER }}</span></template></i18n-t>
          </div>
          <div class="s-ctrl"><ToggleSwitch :modelValue="appCfg.mcpEnabled" :aria-label="$t('settings.mcp.service.enable')" @update:modelValue="(v) => setCfg({ mcpEnabled: v })" /></div>
        </div>
        <div v-if="appCfg.mcpEnabled" class="s-row indented">
          <div class="s-meta">
            <span class="s-label">
              <span>{{ $t('settings.api.service.endpoint') }}</span>
              <Tag severity="success" :value="$t('settings.mcp.service.active')" />
            </span>
          </div>
          <div class="s-ctrl w-lg">
            <InputGroup>
              <InputText class="s-mono" :modelValue="appCfg.mcpUrl" :title="appCfg.mcpUrl" readonly />
              <Button severity="secondary" size="small" :aria-label="$t('settings.api.tokens.copy')" @click="copyUrl"><template #icon><AppIcon name="copy" :size="15" /></template></Button>
            </InputGroup>
          </div>
        </div>
      </div>
    </section>

    <section class="s-section">
      <div class="s-section-head">
        <span class="s-section-title">{{ $t('settings.mcp.tools.title') }}</span>
        <Tag severity="secondary" :value="`${mcpOnCount}/${mcpToolCount}`" />
      </div>
      <div class="s-exp-list">
        <div class="s-subblock"><span class="s-desc">{{ $t('settings.mcp.tools.hint') }}</span></div>
        <div v-for="fam in mcpFamilies" :key="fam.name" class="s-exp-item" :class="{ open: mcpOpen.has(fam.name) }">
          <div class="s-exp-head" @click="toggleMcpFamily(fam.name)">
            <AppIcon name="chevron-right" :size="15" class="s-exp-chev" />
            <div class="s-meta">
              <span class="s-label">
                <span>{{ $t(`settings.mcp.families.${fam.name}`) }}</span>
                <Tag severity="secondary" :value="`${fam.on}/${fam.tools.length}`" />
              </span>
            </div>
            <span @click.stop><ToggleSwitch :modelValue="fam.on > 0" :aria-label="$t(`settings.mcp.families.${fam.name}`)" @update:modelValue="(v) => setMcpTools(fam.tools, v)" /></span>
          </div>
          <div v-if="mcpOpen.has(fam.name)" class="s-exp-body">
            <div class="s-check-grid cols3">
              <label v-for="tool in fam.tools" :key="tool" class="opt s-mono"><Checkbox :modelValue="!(appCfg.mcpDisabled || []).includes(tool)" binary @update:modelValue="(v) => setMcpTools([tool], v)" /> {{ tool }}</label>
            </div>
          </div>
        </div>
      </div>
    </section>
  </template>
</template>

<style scoped src="./settings.css"></style>

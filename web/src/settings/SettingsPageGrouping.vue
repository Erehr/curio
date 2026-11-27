<script setup>
import { computed, ref } from 'vue'
import SelectButton from 'openvue/selectbutton'
import Button from 'openvue/button'
import Tag from 'openvue/tag'
import { S } from '../app/state.js'
import { run, toast } from '../app/notify.js'
import { call } from '../platform/ipc.js'
import { useSettingsData, onSettingsRefresh } from './settingsData.js'
import { t } from '../shared/i18n.js'

const { plugins, libSettings, saveLib } = useSettingsData()
// files grouped into one item: each rule in force with what it grouped, and what it would group among the files already here once checked
const RAW_PAIRS = computed(() => ['off', 'jpeg', 'raw'].map((v) => ({ v, l: t(`settings.grouping.rawPairs.${v}`) })))
const ROLE_NAMES = new Set(['motion', 'edits', 'metadata', 'preview', 'materials', 'buffers'])
const grouping = ref([])
const wouldGroup = ref(null)
const rawRule = computed(() => grouping.value.find((g) => g.id === 'library/raw-pairs') || null)
const otherRules = computed(() => grouping.value.filter((g) => g.id !== 'library/raw-pairs'))
const totalWouldGroup = computed(() => (wouldGroup.value ? Object.values(wouldGroup.value).reduce((s, n) => s + (n || 0), 0) : null))
async function loadGrouping() {
  grouping.value = (await call('library.grouping', {}).catch(() => null))?.rules || []
}
function ruleLabel(g) {
  if (g.id === 'user') return t('settings.grouping.byHand')
  if (g.id === 'library/raw-pairs') return t('settings.grouping.rawJpeg')
  const role = ROLE_NAMES.has(g.role) ? t(`settings.grouping.roles.${g.role}`) : g.role.charAt(0).toUpperCase() + g.role.slice(1)
  return [plugins.value.find((p) => p.id === g.owner)?.name || g.owner, role].join(' · ')
}
function formatTemplate(tpl) {
  return tpl.startsWith('{stem}') || tpl.startsWith('{name}') ? tpl.slice(6) : tpl
}
function listExts(list, limit) {
  const clean = [...new Set(list.map((e) => (e.startsWith('.') ? e : '.' + e)))]
  if (!limit || clean.length <= limit) return clean.join(', ')
  return `${clean.slice(0, limit).join(', ')} +${clean.length - limit}`
}
function ruleMatch(g, limit = 5) {
  if (!g?.exts?.length || !g?.files?.length) return ''
  return `${listExts(g.exts, limit)} → ${listExts(g.files.map(formatTemplate), limit)}`
}
async function setRawPairs(v) {
  libSettings.value.raw_pairs = v
  await saveLib().catch(() => {})
  wouldGroup.value = null
  loadGrouping()
}
async function surveyGrouping() {
  const r = await run('library.grouping.survey', {}).catch(() => null)
  if (r) wouldGroup.value = Object.fromEntries(r.rules.map((x) => [x.id, x.count]))
}
async function applyGrouping() {
  const r = await run('library.grouping.apply', {}).catch(() => null)
  if (!r) return
  toast(r.queued ? t('settings.grouping.groupingLater') : r.count ? t('settings.grouping.grouped', { count: r.count }, r.count) : t('settings.grouping.nothing'))
  wouldGroup.value = null
  loadGrouping()
}
function splitRule(g) {
  S.dialogs.confirm = {
    title: t('settings.grouping.split.title', { rule: ruleLabel(g) }),
    body: t('settings.grouping.split.body'),
    okLabel: t('settings.grouping.split.ok'),
    ok: async () => {
      const r = await run('library.grouping.split', { rule: g.id }).catch(() => null)
      if (r) toast(r.queued ? t('settings.grouping.splittingLater') : t('settings.grouping.splitDone', { count: r.count }, r.count))
      loadGrouping()
    },
  }
}
onSettingsRefresh(loadGrouping)
loadGrouping()
</script>

<template>
  <div v-if="!libSettings" class="s-card s-empty">{{ $t('settings.grouping.noLibrary') }}</div>
  <section v-else class="s-section">
    <div class="s-section-head">
      <span class="s-section-title">{{ $t('settings.grouping.sectionTitle') }}</span>
      <div class="s-section-actions">
        <Button :label="$t('settings.grouping.check')" severity="secondary" size="small" @click="surveyGrouping" />
        <Button :label="totalWouldGroup ? $t('settings.grouping.groupCount', { count: totalWouldGroup }, totalWouldGroup) : $t('settings.grouping.groupExisting')" :severity="totalWouldGroup ? undefined : 'secondary'" size="small" @click="applyGrouping" />
      </div>
    </div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">
            <span>{{ $t('settings.grouping.rawJpeg') }}</span>
            <Tag v-if="rawRule?.grouped" severity="secondary" :value="rawRule.grouped.toLocaleString()" />
            <Tag v-if="rawRule && wouldGroup?.[rawRule.id]" severity="info" :value="`+${wouldGroup[rawRule.id].toLocaleString()}`" />
          </span>
          <span v-if="ruleMatch(rawRule)" class="s-desc s-mono ellipsis" v-tooltip.bottom="{ value: ruleMatch(rawRule, 0), showDelay: 500 }">{{ ruleMatch(rawRule) }}</span>
        </div>
        <div class="s-ctrl">
          <SelectButton :modelValue="libSettings.raw_pairs || 'off'" :options="RAW_PAIRS" optionLabel="l" optionValue="v" :allowEmpty="false" @update:modelValue="setRawPairs" />
          <Button v-if="rawRule?.grouped" :label="$t('settings.grouping.splitAll')" severity="secondary" text size="small" @click="splitRule(rawRule)" />
        </div>
      </div>
      <div v-for="g in otherRules" :key="g.id" class="s-row">
        <div class="s-meta">
          <span class="s-label">
            <span>{{ ruleLabel(g) }}</span>
            <Tag severity="secondary" :value="g.grouped.toLocaleString()" />
            <Tag v-if="wouldGroup?.[g.id]" severity="info" :value="`+${wouldGroup[g.id].toLocaleString()}`" />
          </span>
          <span v-if="ruleMatch(g)" class="s-desc s-mono ellipsis" v-tooltip.bottom="{ value: ruleMatch(g, 0), showDelay: 500 }">{{ ruleMatch(g) }}</span>
        </div>
        <div class="s-ctrl">
          <Button :label="$t('settings.grouping.splitAll')" severity="secondary" text size="small" :class="{ unseen: !g.grouped }" :tabindex="g.grouped ? undefined : -1" @click="splitRule(g)" />
        </div>
      </div>
      <div v-if="totalWouldGroup === 0" class="s-subblock">
        <span class="s-desc">{{ $t('settings.grouping.nothing') }}</span>
      </div>
    </div>
  </section>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.unseen { visibility: hidden; pointer-events: none; }
</style>

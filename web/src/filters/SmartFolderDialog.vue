<script setup>
import { computed, ref, watch } from 'vue'
import Dialog from 'openvue/dialog'
import InputText from 'openvue/inputtext'
import Select from 'openvue/select'
import Button from 'openvue/button'
import Tag from 'openvue/tag'
import Message from 'openvue/message'
import { S } from '../app/state.js'
import { edit } from '../library/edits.js'
import { requestSearch } from '../library/results.js'
import { countOf } from '../library/lifecycle.js'
import SmartFolderDialogRules from './SmartFolderDialogRules.vue'
import { groupsToQuery, queryToGroups, newGroup, allProps, fieldGroups } from './smartRules.js'
import { ownerName } from './query.js'
import { t } from '../shared/i18n.js'
import { errorText } from '../platform/ipc.js'

const sf = ref(null)
const found = ref(null)
const sfError = ref(null)
watch(() => S.dialogs.smart, (d) => {
  found.value = null
  sfError.value = null
  if (!d) return (sf.value = null)
  const existing = S.sidebar.smart.find((x) => x.id === d.id)
  const parent = d.parent ?? existing?.parent_id ?? null
  const groupBy = d.group_by ?? existing?.group_by ?? ''
  const groups = d.query?.trim() ? queryToGroups(d.query, S.sidebar.fields) : d.id ? [] : [newGroup(allProps(S.sidebar.fields))]
  sf.value = { id: d.id, name: d.name, parent, groupBy, groups }
})
const groupByOptions = computed(() => {
  const builtin = [
    { value: '', label: t('filters.smart.groupNone') },
    { value: 'year:added', label: t('filters.smart.groups.yearAdded') },
    { value: 'month:added', label: t('filters.smart.groups.monthAdded') },
    { value: 'year:created', label: t('filters.smart.groups.yearCreated') },
    { value: 'month:created', label: t('filters.smart.groups.monthCreated') },
    { value: 'year:modified', label: t('filters.smart.groups.yearModified') },
    { value: 'month:modified', label: t('filters.smart.groups.monthModified') },
    { value: 'rating', label: t('filters.smart.groups.rating') },
    { value: 'kind', label: t('filters.smart.groups.kind') },
    { value: 'ext', label: t('filters.smart.groups.ext') },
    { value: 'tag', label: t('filters.smart.groups.tag') },
  ]
  const fields = fieldGroups(S.sidebar.fields, ownerName).map((g) => ({
    label: g.label,
    items: g.items.map((f) => ({ value: 'field:' + f.key, label: f.label })),
  }))
  return [{ label: t('filters.smart.groups.builtin'), items: builtin }, ...fields]
})
// a subfolder finds only what the folders above it find
const sfParents = computed(() => {
  const out = []
  let p = sf.value?.parent
  while (p != null && out.length < 64) {
    const x = S.sidebar.smart.find((s) => s.id === p)
    if (!x) break
    out.unshift(x)
    p = x.parent_id
  }
  return out
})
let vTimer = null
// one without rules finds what its subfolders find, which isn't counted here
const noRules = computed(() => !sf.value?.groups.length)
const sfQuery = computed(() => (!sf.value || noRules.value ? '' : groupsToQuery(sf.value.groups, S.sidebar.fields)))
// rows still to be filled in write no query, which is not the same as having none
const unfilled = computed(() => !noRules.value && !sfQuery.value.trim())
const sfCombined = computed(() => (noRules.value ? '' : [...sfParents.value.map((x) => x.query), sfQuery.value].filter((q) => q?.trim()).map((q) => `(${q})`).join(' AND ')))
watch([sfQuery, sfCombined], ([q, all]) => {
  clearTimeout(vTimer)
  vTimer = setTimeout(async () => {
    if (!all.trim()) {
      found.value = null
      sfError.value = null
      return
    }
    try {
      if (q.trim()) await countOf(q)
      found.value = await countOf(all)
      sfError.value = null
    } catch (e) {
      sfError.value = errorText(e)
      found.value = null
    }
  }, 250)
}, { immediate: true })
async function saveSmart() {
  const s = sf.value
  const q = sfQuery.value
  if (!s.name.trim() || unfilled.value || sfError.value) return
  if (s.id) await edit.smart('update', { id: s.id, name: s.name, query: q, groupBy: s.groupBy || '' })
  else await edit.smart('create', { name: s.name, query: q, parent: s.parent, groupBy: s.groupBy || '' })
  S.dialogs.smart = null
  if (s.id && (S.scope.kind === 'smart' || S.scope.kind === 'smartBucket') && S.scope.id === s.id) requestSearch()
}
</script>

<template>
  <Dialog :visible="!!S.dialogs.smart" @update:visible="(v) => !v && (S.dialogs.smart = null)" :header="$t('filters.smart.title')" modal :style="{ width: 'min(54rem, 94vw)' }">
    <div v-if="sf" class="sf-body">
      <div class="sf-head-row">
        <div class="sf-field grow">
          <label class="sf-lbl">{{ $t('filters.smart.name') }}</label>
          <InputText v-model="sf.name" v-focus fluid />
        </div>
        <div class="sf-field gb-col">
          <label class="sf-lbl">{{ $t('filters.smart.groupBy') }}</label>
          <Select v-model="sf.groupBy" :options="groupByOptions" optionLabel="label" optionValue="value" optionGroupLabel="label" optionGroupChildren="items" :placeholder="$t('filters.smart.groupNone')" filter autoFilterFocus fluid />
        </div>
      </div>

      <p v-if="!noRules" class="sf-hint">{{ sfParents.length ? $t('filters.smart.collectsWithin', { path: sfParents.map((x) => x.name).join(' › ') }) : $t('filters.smart.collects') }}</p>

      <SmartFolderDialogRules :groups="sf.groups" :emptyText="sf.groupBy ? $t('filters.smart.noRulesGrouped') : $t('filters.smart.noRules')" />

      <Message v-if="sfError" severity="error" :closable="false">{{ sfError }}</Message>
    </div>

    <template #footer>
      <div class="sf-foot">
        <div class="sf-foot-left">
          <Tag v-if="!sfError && found != null" severity="info" :value="$t('filters.smart.found', { count: found, n: found.toLocaleString() }, found)" />
        </div>
        <div class="sf-foot-right">
          <Button :label="$t('common.saveChanges')" @click="saveSmart" :disabled="!!sfError || unfilled" />
          <Button :label="$t('common.cancel')" severity="secondary" @click="S.dialogs.smart = null" />
        </div>
      </div>
    </template>
  </Dialog>
</template>

<style scoped>
.sf-body { display: flex; flex-direction: column; gap: calc(0.84rem * var(--g)); }
.sf-head-row { display: flex; gap: calc(0.84rem * var(--g)); align-items: flex-end; }
.sf-field { display: flex; flex-direction: column; gap: calc(0.28rem * var(--g)); min-width: 0; }
.sf-lbl { font-size: 0.82rem; font-weight: 500; color: var(--text-muted); }
.gb-col { width: round(16.1rem, 1px); flex: none; }
.sf-hint { margin: 0; font-size: 0.85rem; color: var(--text-muted); }
.sf-foot { display: flex; align-items: center; justify-content: space-between; gap: calc(0.7rem * var(--g)); width: 100%; }
.sf-foot-left { display: flex; align-items: center; }
.sf-foot-right { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); }
</style>

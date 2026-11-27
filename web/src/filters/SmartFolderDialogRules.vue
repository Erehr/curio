<script setup>
import { computed } from 'vue'
import Select from 'openvue/select'
import Button from 'openvue/button'
import { S } from '../app/state.js'
import { ownerName } from './query.js'
import AppIcon from '../shared/AppIcon.vue'
import SmartFolderDialogRuleValue from './SmartFolderDialogRuleValue.vue'
import { OPS, opLabel, propLabel, propsWith, allProps, fieldGroups, newGroup, newRule, ruleKey, ruleType, pairedField, fitValue, QUERY_PROP, FIELD_PROP } from './smartRules.js'

// Flat list of rule groups joined by AND; removing every group turns the smart folder into a container for its subfolders.
const props = defineProps({ groups: { type: Array, required: true }, emptyText: { type: String, default: '' } })
const PROPS = computed(() => propsWith(S.sidebar.fields))
const ALL_PROPS = computed(() => allProps(S.sidebar.fields))
const propsFor = (r) => (r.prop === QUERY_PROP[0] ? [QUERY_PROP, ...PROPS.value] : PROPS.value)
const typeOf = (r) => ruleType(r, ALL_PROPS.value)
const optionsOf = (r) => ALL_PROPS.value.find((p) => p[0] === ruleKey(r))?.[3] || []
const anyFields = computed(() => fieldGroups(S.sidebar.fields, ownerName))
const pairOf = (r) => (r.op === '=' ? pairedField(ruleKey(r), S.sidebar.fields) : null)
function removeRule(gi, ri) {
  const g = props.groups[gi]
  g.rules.splice(ri, 1)
  if (!g.rules.length) props.groups.splice(gi, 1)
}
function changeProp(r) {
  Object.assign(r, newRule(r.prop, ALL_PROPS.value))
}
function changeField(r) {
  Object.assign(r, newRule(r.prop, ALL_PROPS.value, r.field))
}
function changeOp(r) {
  fitValue(r)
  if (typeOf(r) === 'date') {
    r.value = ''
    r.value2 = ''
  }
}
</script>

<template>
  <div class="rules">
    <template v-if="!groups.length">
      <div class="empty-head">
        <span class="empty-title">{{ $t('filters.rules.groupsTitle') }}</span>
        <Button :label="$t('filters.rules.addGroup')" severity="secondary" size="small" @click="groups.push(newGroup(ALL_PROPS))"><template #icon><AppIcon name="plus" :size="15" /></template></Button>
      </div>
      <div class="empty-card">{{ emptyText }}</div>
    </template>
    <template v-for="(g, gi) in groups" :key="gi">
      <div v-if="gi > 0" class="and-div">
        <span class="and-pill">{{ $t('filters.rules.and') }}</span>
      </div>
      <div class="sf-group">
        <div class="sf-group-head">
          <i18n-t keypath="filters.rules.sentence" tag="span" class="sentence"><template #match><Select v-model="g.match" :options="['all', 'any']" :optionLabel="(o) => $t(`filters.rules.match.${o}`)" class="narrow" /></template><template #truth><Select v-model="g.truth" :options="['true', 'false']" :optionLabel="(o) => $t(`filters.rules.truth.${o}`)" class="narrow" /></template></i18n-t>
          <span class="grow" />
          <Button severity="secondary" text size="small" class="head-btn" v-tooltip="$t('filters.rules.removeGroup')" :aria-label="$t('filters.rules.removeGroup')" @click="groups.splice(gi, 1)"><template #icon><AppIcon name="trash" :size="15" /></template></Button>
        </div>
        <div class="sf-group-body">
          <div v-for="(r, ri) in g.rules" :key="ri" class="rule-row">
            <Select v-model="r.prop" :options="propsFor(r)" :optionLabel="propLabel" :optionValue="(p) => p[0]" filter autoFilterFocus class="pp" @change="changeProp(r)" />
            <Select v-if="r.prop === FIELD_PROP[0]" v-model="r.field" :options="anyFields" optionLabel="label" optionValue="key" optionGroupLabel="label" optionGroupChildren="items" filter autoFilterFocus :placeholder="$t('filters.rules.field')" class="pp" @change="changeField(r)" />
            <Select v-if="typeOf(r) !== 'query'" v-model="r.op" :options="OPS[typeOf(r)]" :optionLabel="(o) => opLabel(typeOf(r), o)" :optionValue="(o) => o[0]" class="op" @change="changeOp(r)" />
            <SmartFolderDialogRuleValue :rule="r" :type="typeOf(r)" :options="optionsOf(r)" :paired="pairOf(r)" />
            <Button severity="secondary" size="small" class="step-btn" :aria-label="$t('filters.rules.remove')" @click="removeRule(gi, ri)"><template #icon><AppIcon name="minus" :size="15" /></template></Button>
            <Button severity="secondary" size="small" class="step-btn" :aria-label="$t('filters.rules.addBelow')" @click="g.rules.splice(ri + 1, 0, newRule(r.prop, ALL_PROPS, r.field))"><template #icon><AppIcon name="plus" :size="15" /></template></Button>
          </div>
        </div>
      </div>
    </template>
    <div v-if="groups.length" class="add-group-row">
      <Button :label="$t('filters.rules.addGroup')" severity="secondary" size="small" @click="groups.push(newGroup(ALL_PROPS))"><template #icon><AppIcon name="plus" :size="15" /></template></Button>
    </div>
  </div>
</template>

<style scoped>
.rules { display: flex; flex-direction: column; gap: calc(0.63rem * var(--g)); }
.empty-head { display: flex; align-items: center; justify-content: space-between; gap: calc(0.56rem * var(--g)); }
.empty-title { font-size: 0.85rem; font-weight: 500; color: var(--text-muted); }
.empty-card { background: var(--surface-2); border-radius: calc(8px * var(--r)); padding: calc(0.91rem * var(--g)) calc(1.05rem * var(--g)); color: var(--text-muted); font-size: 0.92rem; }
.add-group-row { display: flex; justify-content: flex-start; }
.and-div { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); }
.and-div::before, .and-div::after { content: ''; flex: 1; border-top: 1px solid var(--border-subtle); }
.and-pill { font-size: 0.72rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted); background: var(--surface-3); padding: 0.14rem 0.56rem; border-radius: calc(999px * var(--r)); }
.sf-group { background: var(--surface-2); border-radius: calc(8px * var(--r)); overflow: hidden; }
.sf-group-head { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); padding: calc(0.49rem * var(--g)) calc(0.77rem * var(--g)); background: var(--surface-3); }
.sentence { display: inline-flex; align-items: center; gap: calc(0.49rem * var(--g)); color: var(--text-muted); font-size: 0.92rem; white-space: nowrap; }
.narrow { width: round(6.5rem, 1px); }
.sf-group-body { display: flex; flex-direction: column; gap: calc(0.49rem * var(--g)); padding: calc(0.77rem * var(--g)); }
.rule-row { display: flex; align-items: center; gap: calc(0.42rem * var(--g)); }
.pp { width: round(11.2rem, 1px); flex: none; }
.op { width: round(9.8rem, 1px); flex: none; }
.head-btn { width: var(--control-h); min-width: var(--control-h); height: var(--control-h); padding: 0; flex: none; }
.step-btn { width: var(--control-h); min-width: var(--control-h); height: var(--control-h); padding: 0; flex: none; }
</style>

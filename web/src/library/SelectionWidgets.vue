<script>
const lastSum = { key: '', view: '', sum: null }
</script>

<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { S } from '../app/state.js'
import { edit } from './edits.js'
import { filterQuery } from '../filters/query.js'
import { scopeSummary } from './selection.js'
import { scopeTitle } from './navigation.js'
import AppIcon from '../shared/AppIcon.vue'
import FieldEditor from './FieldEditor.vue'
import { fmtSize, selectedIds as ids } from './itemSelectors.js'
import { t } from '../shared/i18n.js'

// The current view when nothing is selected (All, Random, a tag, a folder or several, a filter: its name, items, size and kinds), and the tools for several selected items that have no widget of their own.
const props = defineProps({ many: Boolean })
// the totals wait for searches typed quickly to settle
const SUM_DELAY_MS = 300
const counts = computed(() => S.sidebar.counts || {})
const narrowed = computed(() => !!S.search.trim() || !!filterQuery())
const title = computed(() => (narrowed.value ? t('inspector.summary.filtered', { view: scopeTitle() }) : scopeTitle()))
const kindsText = computed(() => (viewSum.value?.kinds || []).map((k) => t(`inspector.kindCounts.${k.kind}`, { count: k.count })).join(', '))
const viewSum = ref(null)
let sumTimer = null
let sumSeq = 0
// what the view is, and the same with its results' version: the totals last read stay while an item is selected and this widget is put away
const viewOf = () => JSON.stringify([S.scope.kind, S.scope.id, String(S.scope.ids || ''), S.scope.path || '', S.search, filterQuery()])
watch(() => [`${S.resultsVer} ${viewOf()}`, S.selected.length === 0], ([key]) => {
  clearTimeout(sumTimer)
  if (props.many || S.selected.length) return
  if (lastSum.key === key) return (viewSum.value = lastSum.sum)
  // the same view searched again keeps its totals until the new ones come, rather than blinking
  const view = viewOf()
  viewSum.value = lastSum.view === view ? lastSum.sum : null
  const mine = ++sumSeq
  sumTimer = setTimeout(async () => {
    const r = await scopeSummary().catch(() => null)
    if (mine !== sumSeq) return
    viewSum.value = r
    Object.assign(lastSum, { key, view, sum: r })
  }, SUM_DELAY_MS)
}, { immediate: true })
onBeforeUnmount(() => clearTimeout(sumTimer))
</script>

<template>
  <div v-if="many" class="batch">
    <div class="label">{{ $t('inspector.batch.setField') }}</div>
    <FieldEditor batch :ids="ids()" />
    <template v-if="S.scope.kind === 'trash'">
      <button class="add-btn" @click="edit.trash(ids(), false)"><AppIcon name="restore" /> {{ $t('inspector.batch.restore') }}</button>
      <button class="add-btn danger" @click="edit.purge(ids())"><AppIcon name="trash-x" /> {{ $t('inspector.batch.purge') }}</button>
    </template>
    <button v-else class="add-btn danger" @click="edit.trash(ids(), true)"><AppIcon name="trash" /> {{ $t('inspector.batch.trash') }}</button>
  </div>

  <div v-else class="summary">
    <div class="folder-head ellipsis">{{ title }}</div>
    <div class="kv"><span>{{ $t('inspector.summary.items') }}</span><span class="mono">{{ S.results.ids.length.toLocaleString() }}</span></div>
    <div class="kv" v-if="viewSum"><span>{{ $t('inspector.props.size') }}</span><span class="mono">{{ fmtSize(viewSum.size) }}</span></div>
    <div class="kv" v-if="kindsText"><span>{{ $t('inspector.props.types') }}</span><span class="mono long">{{ kindsText }}</span></div>
    <div class="kv" v-if="S.scope.kind === 'all' && counts.missing"><span class="warn">{{ $t('inspector.summary.unavailable') }}</span><span class="mono warn">{{ counts.missing }}</span></div>
  </div>
</template>

<style scoped>
.summary, .batch { font-size: 1rem; padding: 0 0.14rem calc(0.84rem * var(--g)); display: flex; flex-direction: column; }
/* its heading sits level with the toolbar's first row, as a picture at the top does */
.summary { gap: 0.14rem; padding-top: calc(0.7rem * var(--g)); }
.batch { gap: calc(0.56rem * var(--g)); padding-top: calc(0.84rem * var(--g)); }
.count-head { font-size: 0.92rem; color: var(--text-faint); font-weight: 500; }
/* the same heading as the other widgets' */
.folder-head { padding: calc(0.56rem * var(--g)) 0 calc(0.42rem * var(--g)); font-size: 0.92rem; font-weight: 500; color: var(--text-faint); }
.stack { position: relative; height: round(11.89rem, 1px); margin: calc(0.42rem * var(--g)) 0 calc(0.98rem * var(--g)); flex: none; }
.stack-card { position: absolute; inset: 0 2.1rem; border-radius: calc(8px * var(--r)); overflow: hidden; background: var(--grid-item); }
.kv { display: flex; justify-content: space-between; align-items: center; gap: calc(1.12rem * var(--g)); min-height: round(2.1rem * var(--row), 1px); line-height: 1.3; font-size: 1rem; }
/* a label longer than the room wraps, in any language, and dates and sizes stay whole */
.kv > span:first-child { flex: 0 1 auto; min-width: 0; color: var(--text); overflow-wrap: break-word; }
.kv > span:last-child { min-width: 0; text-align: right; }
.kv .mono { flex-shrink: 0; color: var(--text-muted); font-size: 0.92rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
/* a list of types is the one value long enough to give way: it takes the room the label leaves and ends in an ellipsis */
.kv .mono.long { flex: 1 1 0; min-width: 0; }
.label { font-size: 0.92rem; font-weight: 500; color: var(--text-faint); margin-top: calc(0.84rem * var(--g)); padding-top: calc(0.84rem * var(--g)); }
.chips { display: flex; flex-wrap: wrap; gap: calc(0.56rem * var(--g)); align-items: center; }
.add-btn { display: inline-flex; align-items: center; justify-content: center; gap: calc(0.42rem * var(--g)); height: var(--control-h); padding: 0 calc(0.7rem * var(--g)); border-radius: calc(6px * var(--r)); border: 0; background: var(--button); color: var(--text); font: inherit; font-size: 0.92rem; cursor: pointer; }
.add-btn:hover { background: var(--button-hover); }
.add-btn.wide { width: 100%; }
.add-btn.danger { color: var(--danger); }
.rate { display: inline-flex; gap: 0.14rem; }
.rate .ico { color: color-mix(in srgb, var(--text) 22%, transparent); cursor: pointer; }
.rate .ico.on { color: var(--star); }
.warn { color: var(--warning); }
.small { font-size: 0.85rem; }
</style>

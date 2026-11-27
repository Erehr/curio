<script setup>
import { computed, ref, watch } from 'vue'
import Button from 'openvue/button'
import Column from 'openvue/column'
import DataTable from 'openvue/datatable'
import SelectButton from 'openvue/selectbutton'
import Tag from 'openvue/tag'
import { call, errorText } from '../platform/ipc.js'
import { previewUrl } from '../platform/fileUrls.js'
import { S } from '../app/state.js'
import { toast } from '../app/notify.js'
import { t } from '../shared/i18n.js'
import { sizeText } from '../shared/numbers.js'
import AppIcon from '../shared/AppIcon.vue'
import AppImageCompare from '../shared/AppImageCompare.vue'
import AppRating from '../shared/AppRating.vue'
import ToolPage from './ToolPage.vue'
import { loadRows, rows as gridRows } from './tools.js'

// Two pictures compared: a slider, side by side, one over the other or their difference, with their properties and ratings; files dropped on a half take its place.
const SIDES = ['A', 'B']
const PREVIEW = 2048
const date = (ms) => (ms ? new Date(ms).toLocaleString() : '')
const PROPS = [
  ['name', (i) => i.name],
  ['dimensions', (i) => (i.width ? `${i.width} × ${i.height}` : '')],
  ['megapixels', (i) => (i.width ? ((i.width * i.height) / 1e6).toLocaleString(undefined, { maximumFractionDigits: 1, minimumFractionDigits: 1 }) : '')],
  ['fileSize', (i) => sizeText(i.size || 0)],
  ['format', (i) => String(i.ext).toUpperCase()],
  ['rating', (i) => '★'.repeat(i.rating || 0) || t('tools.compare.noRating')],
  ['tags', (i) => i.tags.join(', ')],
  ['folders', (i) => i.folders.map((f) => f.name).join(', ')],
  ['added', (i) => date(i.imported)],
]
const MODES = ['slider', 'side', 'overlay', 'difference']

// the two sides, A and B; either may still be empty
const items = ref([null, null])
const mode = ref('slider')
const props = ref(false)
const modes = computed(() => MODES.map((value) => ({ value, label: t(`tools.compare.modes.${value}`) })))
const both = computed(() => !!(items.value[0] && items.value[1]))
const urlOf = (it) => {
  const r = it && gridRows.get(it.id)
  return r ? previewUrl(r.id, PREVIEW, r.thumb_ver) : ''
}
const urls = computed(() => items.value.map(urlOf))
async function load(ids) {
  const want = [ids[0] ?? null, ids[1] ?? null]
  const got = await call('item.getMany', { ids: want.filter(Boolean) }).catch(() => [])
  await loadRows(got.map((i) => i.id))
  items.value = want.map((id) => got.find((i) => i.id === id) || null)
}
const ids = () => items.value.map((i) => i?.id ?? null)
watch(() => S.toolWindow.seq, () => load((S.toolWindow.run?.items || []).slice(0, 2)), { immediate: true })

// files held over the window: the half they would land on
const stage = ref(null)
const sideAt = (x) => {
  const r = stage.value?.getBoundingClientRect()
  return Number.isFinite(x) && r ? (x < r.left + r.width / 2 ? 0 : 1) : null
}
const over = computed(() => (S.toolWindow.over ? sideAt(S.toolWindow.over.x) : null))
// two dropped at once fill both sides; one goes to the half it was dropped on, or else to the empty side
watch(() => S.toolWindow.drop?.seq, () => {
  const d = S.toolWindow.drop
  const got = d?.items || []
  if (got.length >= 2) return load(got.slice(0, 2))
  if (!got.length) return
  const now = ids()
  const side = sideAt(d.x) ?? (now[0] && !now[1] ? 1 : 0)
  now[side] = got[0]
  load(now)
})

const rows = computed(() => {
  const [a, b] = items.value
  if (!a || !b) return []
  return PROPS.map(([key, f]) => ({ label: t(`tools.compare.props.${key}`), a: f(a), b: f(b), differs: f(a) !== f(b) }))
})
const differences = computed(() => rows.value.filter((r) => r.differs).length)
const bigger = computed(() => {
  const [a, b] = items.value
  if (!a || !b) return null
  return a.width * a.height === b.width * b.height ? null : a.width * a.height > b.width * b.height ? 0 : 1
})
// only the rating changes: the pictures stay, with their zoom
async function rate(k, v) {
  try {
    await call('item.update', { id: items.value[k].id, rating: v ?? 0 })
    items.value = items.value.map((it, i) => (i === k ? { ...it, rating: v ?? 0 } : it))
  } catch (e) {
    toast(errorText(e), 'error')
  }
}
const labels = computed(() => items.value.map((it, k) => (it ? `${SIDES[k]} · ${it.name}` : '')))
</script>

<template>
  <ToolPage class="compare-tool">
    <div class="compare-head">
      <span class="grow" />
      <SelectButton v-model="mode" :options="modes" option-label="label" option-value="value" :allow-empty="false" />
    </div>
    <div class="compare-body">
      <div ref="stage" class="stage" :class="{ empty: !items[0] && !items[1] }">
        <template v-if="!items[0] && !items[1]">
          <AppIcon name="layout-columns" :size="40" />
          <span>{{ $t('tools.compare.empty') }}</span>
        </template>
        <div v-else-if="!both" class="pair">
          <template v-for="(it, k) in items" :key="k">
            <figure v-if="it"><img :src="urls[k]" :alt="it.name" draggable="false" /><figcaption>{{ labels[k] }}</figcaption></figure>
            <div v-else class="drop-here"><AppIcon name="photo-plus" :size="32" /><span>{{ $t('tools.compare.dropHere', { side: SIDES[k] }) }}</span></div>
          </template>
        </div>
        <AppImageCompare v-else :images="urls" :mode="mode" :labels="labels" toolbar />
        <div v-if="over != null" class="drop-mark" :class="over ? 'b' : 'a'"><AppIcon name="photo-down" :size="28" /><span>{{ $t('tools.compare.dropAs', { side: SIDES[over] }) }}</span></div>
      </div>
      <section v-if="both && props" class="props">
        <h3>{{ $t('tools.compare.properties') }}</h3>
        <p class="faint">{{ bigger == null ? $t('tools.compare.sameSize') : $t('tools.compare.morePixels', { side: SIDES[bigger] }) }}</p>
        <DataTable :value="rows" size="small" class="tool-table" :row-class="(r) => (r.differs ? 'diff' : '')">
          <Column field="label" header="" :style="{ width: '9.09rem' }" />
          <Column field="a" :header="SIDES[0]" />
          <Column field="b" :header="SIDES[1]" />
        </DataTable>
      </section>
    </div>
    <template #foot>
      <template v-if="both">
        <span class="faint">{{ SIDES[0] }}</span>
        <AppRating :model-value="items[0].rating || 0" :aria-label="$t('tools.compare.rating', { side: SIDES[0] })" @update:model-value="(v) => rate(0, v)" />
        <span class="faint">{{ SIDES[1] }}</span>
        <AppRating :model-value="items[1].rating || 0" :aria-label="$t('tools.compare.rating', { side: SIDES[1] })" @update:model-value="(v) => rate(1, v)" />
        <span class="grow" />
        <Tag :value="$t('tools.compare.differences', { count: differences.toLocaleString() }, differences)" severity="secondary" />
        <Button :label="$t('tools.compare.properties')" severity="secondary" :text="!props" :aria-pressed="String(props)" @click="props = !props" />
      </template>
      <span v-else class="faint grow">{{ $t('tools.compare.nothing') }}</span>
    </template>
  </ToolPage>
</template>

<style>
.compare-tool .compare-head { display: flex; align-items: center; gap: calc(0.56rem * var(--g)); padding: calc(0.56rem * var(--g)) calc(1.12rem * var(--g)); flex: none; }
.compare-tool .compare-body { flex: 1; min-height: 0; overflow: auto; padding: 0 calc(1.12rem * var(--g)) calc(1.12rem * var(--g)); display: flex; flex-direction: column; gap: calc(0.98rem * var(--g)); }
.compare-tool .stage { flex: 1 0 auto; min-height: round(22.38rem, 1px); display: flex; flex-direction: column; position: relative; }
.compare-tool .stage.empty, .compare-tool .stage:has(> .pair) { align-items: center; justify-content: center; background: var(--viewer-bg); border-radius: calc(10px * var(--r)); }
.compare-tool .stage.empty { gap: 0.7rem; color: var(--text-faint); text-align: center; padding: 0 calc(1.68rem * var(--g)); }
.compare-tool .stage > .curio-compare { flex: 1; }
.compare-tool .pair { display: grid; grid-template-columns: 1fr 1fr; gap: calc(0.7rem * var(--g)); width: 100%; height: 100%; padding: calc(0.7rem * var(--g)); box-sizing: border-box; }
.compare-tool .pair figure { margin: 0; display: flex; flex-direction: column; gap: 0.42rem; min-height: 0; }
.compare-tool .pair img { flex: 1; min-height: 0; width: 100%; object-fit: contain; }
.compare-tool .pair figcaption { text-align: center; font-size: 0.92rem; color: var(--text-muted); }
.compare-tool .p-datatable .diff > td { color: var(--warning); }
.compare-tool .drop-here { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 0.56rem; height: 100%; border: 1px dashed var(--border); border-radius: calc(8px * var(--r)); color: var(--text-faint); }
/* the half a drop would land on */
.compare-tool .drop-mark { position: absolute; top: 0; bottom: 0; width: 50%; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 0.56rem; border: 2px dashed var(--accent); border-radius: calc(10px * var(--r)); background: color-mix(in srgb, var(--accent) 16%, transparent); color: var(--text); font-weight: 500; pointer-events: none; z-index: 5; box-sizing: border-box; }
.compare-tool .drop-mark.a { left: 0; }
.compare-tool .drop-mark.b { right: 0; }
.compare-tool .props { display: flex; flex-direction: column; gap: calc(0.7rem * var(--g)); padding: calc(0.98rem * var(--g)) calc(1.12rem * var(--g)); border-radius: calc(10px * var(--r)); background: var(--surface); border: 1px solid var(--border); }
.compare-tool .props h3 { margin: 0; font-size: 1rem; font-weight: 500; }
.compare-tool .props p { margin: 0; }
</style>

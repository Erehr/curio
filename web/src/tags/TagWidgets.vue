<script setup>
import { t } from '../shared/i18n.js'
import { computed, ref, watch } from 'vue'
import { S } from '../app/state.js'
import { ensureItems, requestSearch } from '../library/results.js'
import { resetFilters, setScope } from '../library/navigation.js'
import { search } from '../platform/ipc.js'
import { KIND_ICON, thumbOf } from '../formats/index.js'
import AppIcon from '../shared/AppIcon.vue'

// On the Tags screen: the best-rated items carrying the selected tags, or the open group's.
const SHOWN = 12
const ids = ref([])
const busy = ref(false)
const names = computed(() => {
  const f = S.tagFocus
  if (!f) return []
  if (f.names.length) return f.names
  const tags = S.sidebar.tags.filter((t) => t.count > 0)
  if (f.place === 'starred') return tags.filter((t) => t.starred).map((t) => t.name)
  if (typeof f.place === 'number') return tags.filter((t) => t.group_ids?.includes(f.place)).map((t) => t.name)
  return []
})
const title = computed(() => {
  const f = S.tagFocus
  if (f?.names.length === 1) return f.names[0]
  if (f?.names.length) return t('tags.best.count', { count: f.names.length })
  if (f?.place === 'starred') return t('tags.best.starred')
  return S.sidebar.tagGroups.find((g) => g.id === f?.place)?.name || ''
})
let seq = 0
watch(names, async (list) => {
  const mine = ++seq
  if (!list.length) {
    ids.value = []
    return
  }
  busy.value = true
  const text = `tag IN (${list.map((n) => JSON.stringify(n)).join(', ')}) AND rating >= 1`
  const r = await search({ scope: 'all', scope_id: 0, scope_ids: [], scope_path: '', history: '', text, sort_key: 'rating', sort_field: '', descending: true, subfolders: false }).catch(() => null)
  if (mine !== seq) return
  busy.value = false
  ids.value = r ? Array.from(r.ids.subarray(0, SHOWN)) : []
  ensureItems(ids.value)
}, { immediate: true })
const items = computed(() => {
  void S.itemsVer
  return ids.value.map((id) => ({ id, item: S.items.get(id) }))
})
function showAll() {
  setScope('all')
  resetFilters()
  S.filters.tags.push(...names.value)
  S.filters.tagRule = 'OR'
  S.view = 'grid'
  requestSearch()
}
</script>

<template>
  <div class="tag-best">
    <div class="tb-head"><span class="ellipsis">{{ title ? $t('tags.best.titleOf', { name: title }) : $t('tags.best.title') }}</span></div>
    <div v-if="!names.length" class="faint tb-hint">{{ $t('tags.best.none') }}</div>
    <div v-else-if="!busy && !items.length" class="faint tb-hint">{{ $t('tags.best.nothing', { count: names.length }) }}</div>
    <div v-else class="tb-grid">
      <button v-for="x in items" :key="x.id" class="tb-cell" v-tooltip.bottom="x.item?.title" @click="showAll">
        <img v-if="thumbOf(x.item)" :src="thumbOf(x.item)" draggable="false" />
        <AppIcon v-else :name="KIND_ICON[x.item?.kind] || 'file'" :size="22" class="faint" />
        <span v-if="x.item?.rating" class="tb-stars"><AppIcon name="star-filled" :size="11" />{{ x.item.rating }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.tag-best { padding: 0 0.14rem calc(0.84rem * var(--g)); font-size: 1rem; }
.tb-head { padding: calc(0.56rem * var(--g)) 0 calc(0.42rem * var(--g)); font-size: 0.92rem; font-weight: 500; color: var(--text-faint); }
.tb-hint { font-size: 0.92rem; }
.tb-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: calc(0.42rem * var(--g)); }
.tb-cell { position: relative; aspect-ratio: 1; padding: 0; border: 0; border-radius: calc(6px * var(--r)); overflow: hidden; background: var(--grid-item); display: flex; align-items: center; justify-content: center; cursor: pointer; }
.tb-cell img { width: 100%; height: 100%; object-fit: cover; display: block; }
.tb-stars { position: absolute; right: 0.21rem; bottom: 0.21rem; display: inline-flex; align-items: center; gap: 0.14rem; padding: 0 0.28rem; border-radius: calc(4px * var(--r)); background: rgba(0, 0, 0, .55); color: var(--star); font-size: 0.75rem; line-height: round(1.12rem, 1px); }
</style>

<script setup>
import { language, t } from '../shared/i18n.js'
import { computed, nextTick, ref, watch } from 'vue'
import Dialog from 'openvue/dialog'
import InputText from 'openvue/inputtext'
import Button from 'openvue/button'
import { S } from './state.js'
import { openPicker } from '../tags/tags.js'
import { pointer } from './windowRequests.js'
import AppMenu from '../shared/AppMenu.vue'
import ColorPicker from 'openvue/colorpicker'
import RadioButton from 'openvue/radiobutton'
import Checkbox from 'openvue/checkbox'

// What a script or extension asks the person at this window (ui.confirm, ui.prompt, ui.pick*, ui.menu); the answer goes back to it.
const A = computed(() => S.apiAsk)
const text = ref('')
const filter = ref('')
const chosen = ref([])
const color = ref('#808080')
const menu = ref()
// each permission's words in `app.ask.perms`
const PERMS = { 'library.read': 'libraryRead', 'metadata.read': 'metadataRead', 'metadata.write': 'metadataWrite', organize: 'organize', 'files.write': 'filesWrite', 'files.outside': 'filesOutside' }
const everyLibrary = ref(true)
const allowed = ref([])

watch(A, async (a) => {
  if (!a) return
  text.value = a.kind === 'prompt' ? (a.value ?? '') : ''
  filter.value = ''
  chosen.value = [...(a.selected || [])]
  everyLibrary.value = true
  allowed.value = []
  color.value = a.kind === 'color' && /^#?[0-9a-f]{6}$/i.test(a.value || '') ? ('#' + a.value.replace('#', '')).toUpperCase() : '#808080'
  if (a.kind === 'menu') {
    await nextTick()
    menu.value?.openAt(pointer.x, pointer.y)
  }
  // tags and folders are chosen in the window's own pickers, which answer with what is ticked once they close
  if (a.kind === 'tags' || a.kind === 'folders') {
    const at = { currentTarget: { getBoundingClientRect: () => ({ left: pointer.x, right: pointer.x, top: pointer.y, bottom: pointer.y }) } }
    const picked = chosen.value
    openPicker(a.kind, at, {
      value: [...picked],
      byId: a.kind === 'folders',
      canCreate: a.kind === 'tags',
      onToggle: (v) => {
        const i = picked.indexOf(v)
        if (i >= 0) picked.splice(i, 1)
        else picked.push(v)
      },
      asked: a.call,
      onClose: () => S.apiAsk === a && answer([...picked]),
    })
  }
})
// a question the caller stopped waiting for takes its picker with it
watch(() => S.apiAsk, (now, before) => {
  if (!now && before && S.picker?.asked === before.call) S.picker = null
})

function answer(v) {
  const a = S.apiAsk
  if (!a) return
  S.apiAsk = null
  a.resolve(v)
}
const cancel = () => answer(null)

function ok() {
  const k = A.value?.kind
  if (k === 'confirm') answer(true)
  else if (k === 'prompt') answer(text.value)
  else if (k === 'color') answer(color.value)
  else if (k === 'pair') answer(everyLibrary.value ? [] : [...allowed.value])
}
const canAllow = computed(() => everyLibrary.value || allowed.value.length > 0)
const permLine = computed(() => new Intl.ListFormat(language(), { type: 'conjunction' }).format((A.value?.perms || []).map((p) => (PERMS[p] ? t(`app.ask.perms.${PERMS[p]}`) : p))))

const rows = computed(() => {
  const a = A.value
  if (!a) return []
  let list = []
  if (a.kind === 'field') list = S.sidebar.fields.map((f) => ({ key: f.key, label: f.label || f.key, hint: f.key }))
  const q = filter.value.trim().toLowerCase()
  return q ? list.filter((r) => r.label.toLowerCase().includes(q)) : list
})

const dialog = computed(() => !!A.value && !['menu', 'tags', 'folders'].includes(A.value.kind))
const listed = computed(() => A.value?.kind === 'field')

const menuModel = () => (A.value?.items || []).map((e) => entry(e))
function entry(e) {
  if (e.separator) return { separator: true }
  return { label: e.label, icon: e.icon || undefined, disabled: !!e.disabled, items: e.items?.length ? e.items.map(entry) : undefined, command: e.items?.length ? undefined : () => answer(e.id ?? null) }
}
// the menu closes before it runs the chosen entry
function menuHidden() {
  const a = S.apiAsk
  setTimeout(() => S.apiAsk === a && a?.kind === 'menu' && answer(null), 0)
}
</script>

<template>
  <Dialog :visible="dialog" @update:visible="(v) => !v && cancel()" modal :header="A?.title || (A?.kind ? $t(`app.ask.headers.${A.kind}`) : '')" :style="{ width: '33.57rem' }">
    <template v-if="A">
      <p v-if="A.from" class="faint small">{{ $t('app.ask.askedBy', { name: A.from }) }}</p>
      <p v-if="A.body" class="ask-body">{{ A.body }}</p>
      <InputText v-if="A.kind === 'prompt'" v-model="text" v-focus fluid :placeholder="A.placeholder || ''" @keydown.enter="ok" />
      <template v-else-if="listed">
        <InputText v-model="filter" v-focus fluid :placeholder="$t('app.ask.filter')" />
        <div class="ask-list">
          <div v-for="r in rows" :key="r.key" class="ask-row" tabindex="0" role="button" @click="answer(r.key)" @keydown.enter.space.prevent="answer(r.key)">
            <span class="grow">{{ r.label }}</span><span v-if="r.hint" class="faint small">{{ r.hint }}</span>
          </div>
          <p v-if="!rows.length" class="faint small">{{ $t('app.ask.nothing') }}</p>
        </div>
      </template>
      <ColorPicker v-else-if="A.kind === 'color'" inline :modelValue="color.replace('#', '')" @update:modelValue="(v) => (color = '#' + v.toUpperCase())" />
      <template v-else-if="A.kind === 'pair'">
        <i18n-t keypath="app.ask.pair.wants" tag="p" scope="global"><template #name><b>{{ A.name }}</b></template></i18n-t>
        <div class="pair-code">{{ A.code }}</div>
        <p class="faint small">{{ $t('app.ask.pair.code') }}</p>
        <p class="small">{{ $t('app.ask.pair.may', { perms: permLine }) }}</p>
        <label class="opt"><RadioButton v-model="everyLibrary" :value="true" /> {{ $t('app.ask.pair.all') }}</label>
        <label class="opt"><RadioButton v-model="everyLibrary" :value="false" /> {{ $t('app.ask.pair.some') }}</label>
        <div v-if="!everyLibrary" class="ask-list">
          <label v-for="l in A.libraries" :key="l.id" class="opt"><Checkbox v-model="allowed" :value="l.id" /> {{ l.name }}</label>
        </div>
        <p class="faint small mono">{{ A.origin }}</p>
      </template>
      <div class="row foot">
        <span class="grow" />
        <Button v-if="A.kind === 'pair'" :label="$t('app.ask.pair.allow')" :disabled="!canAllow" @click="ok" />
        <Button v-else-if="A.kind !== 'field'" :label="A.okLabel || $t('common.ok')" :autofocus="A.kind === 'confirm'" @click="ok" />
        <Button :label="A.kind === 'pair' ? $t('app.ask.pair.deny') : $t('common.cancel')" severity="secondary" @click="cancel" />
      </div>
    </template>
  </Dialog>
  <AppMenu ref="menu" :model="menuModel" @hide="menuHidden" />
</template>

<style scoped>
.ask-body { white-space: pre-wrap; }
.pair-code { font-size: 1.96rem; font-weight: 600; letter-spacing: .2em; text-align: center; margin: 0.56rem 0; font-variant-numeric: tabular-nums; }
.opt { display: flex; align-items: center; gap: 0.56rem; padding: 0.21rem 0; }
.ask-list { max-height: 22.38rem; overflow-y: auto; margin-top: 0.56rem; }
.ask-row { display: flex; align-items: center; gap: 0.56rem; padding: 0.35rem 0.42rem; border-radius: calc(6px * var(--r)); cursor: pointer; }
.ask-row:hover, .ask-row:focus-visible { background: color-mix(in srgb, var(--text) 7%, transparent); }
.ask-row:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
</style>

<script setup>
import { computed, ref, watch } from 'vue'
import Select from 'openvue/select'
import InputText from 'openvue/inputtext'
import Button from 'openvue/button'
import Slider from 'openvue/slider'
import SelectButton from 'openvue/selectbutton'
import Checkbox from 'openvue/checkbox'
import ToggleSwitch from 'openvue/toggleswitch'
import AppIcon from '../shared/AppIcon.vue'
import AppSegments from '../shared/AppSegments.vue'
import { S } from '../app/state.js'
import { panelsFor, currentAspect } from '../layout/arrangement.js'
import { saveUi, setLayoutPerLibrary } from '../app/preferences.js'
import { toast } from '../app/notify.js'
import LayoutThumbnail from '../layout/LayoutThumbnail.vue'
import { BUILTIN_LAYOUTS, DEFAULT_FOR, isBuiltinLayout, changedBuiltin, layoutLabel, layoutNames, layoutOf, copyName } from '../layout/layouts.js'
import { t } from '../shared/i18n.js'

// layouts are chosen for one window shape at a time, the one in use first
const slot = ref(currentAspect() === 'vertical' ? 'vertical' : 'horizontal')
const SLOTS = computed(() => [{ value: 'horizontal', label: t('settings.layout.slots.horizontal'), icon: 'device-desktop' }, { value: 'vertical', label: t('settings.layout.slots.vertical'), icon: 'device-mobile' }])
const pan = computed(() => panelsFor(slot.value))
const LIBRARY_PLACES = computed(() => ['left', 'right'].map((v) => ({ v, l: t(`settings.layout.libraryPlaces.${v}`) })))
const INSPECTOR_PLACES = computed(() => ['left', 'right', 'bottom', 'tabs', 'split'].map((v) => ({ v, l: t(`settings.layout.inspectorPlaces.${v}`) })))
const DOCK_LAYOUT_CHOICES = computed(() => ['column', 'grid', 'masonry', 'row'].map((v) => ({ v, l: t(`settings.layout.dockLayouts.${v}`) })))
const RAIL_OPEN = computed(() => ['hover', 'click'].map((v) => ({ v, l: t(`settings.layout.all.railOpen.${v}`) })))
// where the Inspector is, as one choice: a side, under the grid, or merged into the Library
const inspectorPlace = computed(() => (pan.value.merged ? (pan.value.mergeMode === 'split' ? 'split' : 'tabs') : pan.value.inspectorDock))
const besideGrid = computed(() => ['left', 'right'].includes(inspectorPlace.value))
function setInspectorPlace(v) {
  const l = pan.value
  l.merged = v === 'tabs' || v === 'split'
  if (l.merged) l.mergeMode = v
  else l.inspectorDock = v
  saveUi()
}
const allLayouts = computed(() => layoutNames(S.ui.layouts))
const chosenLayout = computed(() => S.ui.layoutFor[slot.value])
const editingLayout = ref(null)
watch(slot, () => (editingLayout.value = null))
function chooseLayout(n) {
  if (editingLayout.value && editingLayout.value.was !== n) editingLayout.value = null
  S.ui.layoutFor[slot.value] = n
  saveUi()
}
function editLayout() {
  editingLayout.value = { was: chosenLayout.value, name: chosenLayout.value }
}
function duplicateLayout() {
  const from = chosenLayout.value
  const name = copyName(from, S.ui.layouts)
  S.ui.layouts[name] = JSON.parse(JSON.stringify(layoutOf(S.ui.layouts, from)))
  chooseLayout(name)
  editLayout()
}
function restoreLayout() {
  const n = chosenLayout.value
  S.ui.layouts[n] = BUILTIN_LAYOUTS[n]()
  saveUi()
}
function deleteLayout() {
  const n = chosenLayout.value
  editingLayout.value = null
  delete S.ui.layouts[n]
  for (const k of ['horizontal', 'vertical']) if (S.ui.layoutFor[k] === n) S.ui.layoutFor[k] = DEFAULT_FOR[k]
  saveUi()
}
function finishLayout() {
  const e = editingLayout.value
  const name = e.name.trim()
  if (!isBuiltinLayout(e.was) && name !== e.was) {
    if (!name) return toast(t('settings.layout.edit.noName'), 'error')
    if (isBuiltinLayout(name) || S.ui.layouts[name]) return toast(t('settings.layout.edit.taken'), 'error')
    S.ui.layouts[name] = S.ui.layouts[e.was]
    delete S.ui.layouts[e.was]
    for (const k of ['horizontal', 'vertical']) if (S.ui.layoutFor[k] === e.was) S.ui.layoutFor[k] = name
  }
  editingLayout.value = null
  saveUi()
}
</script>

<template>
  <section class="s-section">
    <div class="s-section-head">
      <span class="s-section-title">{{ $t('settings.layout.title') }}</span>
      <div class="s-section-actions">
        <Button v-if="!editingLayout" :label="$t('settings.layout.editLayout', { name: layoutLabel(chosenLayout) })" severity="secondary" size="small" @click="editLayout" />
        <Button :label="$t('settings.layout.duplicate')" severity="secondary" size="small" @click="duplicateLayout" />
        <Button v-if="changedBuiltin(chosenLayout, S.ui.layouts[chosenLayout])" :label="$t('settings.layout.restore')" severity="secondary" text size="small" v-tooltip.bottom="$t('settings.layout.reset')" @click="restoreLayout" />
        <Button v-if="!isBuiltinLayout(chosenLayout)" :label="$t('settings.layout.delete')" text severity="danger" size="small" @click="deleteLayout" />
      </div>
    </div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.layout.windowShape') }}</span></div>
        <div class="s-ctrl"><AppSegments v-model="slot" :options="SLOTS" /></div>
      </div>
      <div class="s-subblock">
        <div class="lay-cards" :class="slot">
          <button v-for="n in allLayouts" :key="n" type="button" class="lcard" :class="{ sel: chosenLayout === n }" :aria-pressed="chosenLayout === n" @click="chooseLayout(n)">
            <span class="lpic"><LayoutThumbnail :layout="layoutOf(S.ui.layouts, n)" :aspect="slot" :edge="!!S.ui.edgeReveal" /></span>
            <span class="lcard-foot">
              <span class="lname">{{ layoutLabel(n) }}</span>
              <AppIcon v-if="chosenLayout === n" name="check" :size="14" class="lcard-check" />
            </span>
          </button>
        </div>
      </div>
    </div>
  </section>

  <section v-if="editingLayout" class="s-section">
    <div class="s-section-head">
      <span class="s-section-title">{{ $t('settings.layout.edit.title') }} · {{ layoutLabel(editingLayout.was) }}</span>
      <div class="s-section-actions">
        <Button :label="$t('settings.layout.edit.done')" size="small" @click="finishLayout" />
      </div>
    </div>
    <div class="s-card">
      <div v-if="!isBuiltinLayout(editingLayout.was)" class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.layout.edit.name') }}</span></div>
        <div class="s-ctrl w-md"><InputText v-model="editingLayout.name" :placeholder="$t('settings.layout.edit.name')" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.layout.edit.library') }}</span></div>
        <div class="s-ctrl lay-edit-ctrl">
          <Select v-model="pan.libraryDock" :options="LIBRARY_PLACES" optionLabel="l" optionValue="v" class="lsel" :disabled="!pan.library" @change="saveUi" />
          <label class="opt" :class="{ off: !pan.library }"><Checkbox v-model="pan.libraryCompact" binary :disabled="!pan.library" @change="saveUi" /> {{ $t('settings.layout.compact') }}</label>
          <ToggleSwitch v-model="pan.library" :aria-label="$t('settings.layout.edit.library')" @change="saveUi" />
        </div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.layout.edit.inspector') }}</span></div>
        <div class="s-ctrl lay-edit-ctrl">
          <Select :modelValue="inspectorPlace" :options="INSPECTOR_PLACES" optionLabel="l" optionValue="v" class="lsel" :disabled="!pan.inspector" @update:modelValue="setInspectorPlace" />
          <label class="opt" :class="{ off: !pan.inspector || !besideGrid }"><Checkbox v-model="pan.inspectorCompact" binary :disabled="!pan.inspector || !besideGrid" @change="saveUi" /> {{ $t('settings.layout.compact') }}</label>
          <ToggleSwitch v-model="pan.inspector" :aria-label="$t('settings.layout.edit.inspector')" @change="saveUi" />
        </div>
      </div>
      <div v-if="pan.inspector && inspectorPlace === 'bottom'" class="s-row indented">
        <div class="s-meta"><span class="s-label">{{ $t('settings.layout.edit.underGrid') }}</span></div>
        <div class="s-ctrl lay-edit-ctrl">
          <Select v-model="pan.dockLayout" :options="DOCK_LAYOUT_CHOICES" optionLabel="l" optionValue="v" class="lsel" @change="saveUi" />
          <div v-if="pan.dockLayout === 'row'" class="s-slider-wrap">
            <Slider v-model="pan.dockItemWidth" :min="180" :max="600" @slideend="saveUi" />
            <span class="s-slider-val">{{ $t('settings.layout.edit.itemWidth', { n: pan.dockItemWidth }) }}</span>
          </div>
          <div v-else-if="pan.dockLayout !== 'column'" class="s-slider-wrap">
            <Slider v-model="pan.dockColumns" :min="1" :max="8" @slideend="saveUi" />
            <span class="s-slider-val">{{ $t('settings.layout.edit.columns', { count: pan.dockColumns }) }}</span>
          </div>
        </div>
      </div>
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.layout.edit.filterBar') }}</span>
          <span class="s-desc">{{ $t('settings.layout.edit.widgetsHint') }}</span>
        </div>
        <div class="s-ctrl"><ToggleSwitch v-model="pan.filterbar" :aria-label="$t('settings.layout.edit.filterBar')" @change="saveUi" /></div>
      </div>
    </div>
  </section>

  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.layout.all.title') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.layout.compact') }}</span></div>
        <div class="s-ctrl"><SelectButton v-model="S.ui.railOpen" :options="RAIL_OPEN" optionLabel="l" optionValue="v" :allowEmpty="false" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.layout.all.edgeHover') }}</span></div>
        <div class="s-ctrl"><ToggleSwitch v-model="S.ui.edgeReveal" :aria-label="$t('settings.layout.all.edgeHover')" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.layout.all.perLibrary') }}</span>
          <span class="s-desc">{{ $t('settings.layout.all.perLibraryHint') }}</span>
        </div>
        <div class="s-ctrl"><ToggleSwitch :modelValue="!!S.ui.layoutPerLibrary" :aria-label="$t('settings.layout.all.perLibrary')" @update:modelValue="setLayoutPerLibrary" /></div>
      </div>
    </div>
  </section>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.lay-cards.vertical { grid-template-columns: repeat(auto-fill, minmax(7.69rem, 1fr)); }
.lay-cards.vertical .lpic { height: round(8.39rem, 1px); }
.lay-edit-ctrl { flex-wrap: wrap; gap: calc(0.7rem * var(--g)); }
.lsel { width: round(12.6rem, 1px); flex: none; }
.off { opacity: 0.5; }
@container (max-width: 44rem) {
  .s-row:has(> .lay-edit-ctrl) { flex-wrap: wrap; row-gap: calc(0.49rem * var(--g)); }
  .lay-edit-ctrl .s-slider-wrap { width: round(12.6rem, 1px); }
}
</style>

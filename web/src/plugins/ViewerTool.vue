<script setup>
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import Slider from 'openvue/slider'
import Popover from 'openvue/popover'
import Select from 'openvue/select'
import ToggleSwitch from 'openvue/toggleswitch'
import AppIcon from '../shared/AppIcon.vue'
import AppSegments from '../shared/AppSegments.vue'
import { numberText } from '../shared/numbers.js'
import { S } from '../app/state.js'
import { keyTip } from '../app/shortcuts.js'
import { toolClicked } from './bridge.js'
import { send, valueOf } from './viewerTools.js'
import { viewerKey } from './pluginActions.js'

// One of a viewer page's controls, by its id: the text, slider and button the page gave viewer.setTools under it, a button perhaps with a panel of controls.
// `tool`: as the plugin declares it, drawn as its icon in the layout editor while the page gives nothing under its id.
const props = defineProps({ id: String, tool: Object })
const T = computed(() => (S.viewerTools?.plugin === props.tool.plugin ? S.viewerTools : null))
const label = computed(() => T.value?.labels.find((l) => l.id === props.id))
const slider = computed(() => (T.value?.slider?.id === props.id ? T.value.slider : null))
const button = computed(() => T.value?.buttons.find((b) => b.id === props.id))
const stand = computed(() => S.customize && !label.value && !slider.value && !button.value)
// the button's key as it is bound now, which the page can't know when it names the button
const tip = computed(() => {
  const k = (S.contrib.viewerKeys || []).find((x) => x.plugin === props.tool.plugin && x.id === props.id)
  return keyTip(button.value.label, [k ? viewerKey(k) : ''])
})

// right to left, the slider's start is on the right
const flip = (v) => (slider.value.dir === 'rtl' ? slider.value.max + slider.value.min - v : v)
const sliderValue = computed({ get: () => flip(valueOf(slider.value)), set: (v) => send(slider.value.id, flip(v)) })

const pop = ref()
const open = ref(false)
const tab = ref(0)
const tabs = computed(() => button.value?.panel || [])
const controls = computed(() => tabs.value[Math.min(tab.value, tabs.value.length - 1)]?.controls || [])
function clicked(e) {
  if (!button.value.panel) return toolClicked(props.id)
  pop.value.toggle(e)
}
watch(() => !!button.value?.panel, (has) => !has && pop.value?.hide())
const hide = () => pop.value?.hide()
window.addEventListener('curio-frame-down', hide)
onBeforeUnmount(() => window.removeEventListener('curio-frame-down', hide))

const shown = (c, v) => `${numberText(v, { digits: Number.isInteger(c.step) ? 0 : String(c.step).split('.')[1]?.length || 1, fixed: true, grouping: false })}${c.unit || ''}`
</script>

<template>
  <span v-if="stand" class="icon-btn v-tool-stand" :data-tool="id" v-tooltip.bottom="tool.title"><AppIcon :name="tool.icon || 'puzzle'" :size="18" /></span>
  <span v-else-if="label || slider || button" class="v-tool">
    <span v-if="label" class="faint mono v-tool-text" :data-tool-text="id">{{ label.text }}</span>
    <Slider v-if="slider" v-model="sliderValue" :min="slider.min" :max="slider.max" :step="slider.step" class="v-tool-slider" :aria-label="slider.label" v-tooltip.bottom="slider.label" />
    <button v-if="button" class="icon-btn" :class="{ on: button.on || (button.panel && open) }" :data-tool="id" v-tooltip.bottom="tip" @click="clicked" @contextmenu.prevent.stop="!button.panel && toolClicked(id, $event)"><AppIcon v-if="button.icon" :name="button.icon" :size="18" /><span v-else>{{ button.label }}</span></button>
    <Popover v-if="button?.panel" ref="pop" class="tool-panel-pop" @show="open = true" @hide="open = false">
      <div class="tool-panel">
        <AppSegments v-if="tabs.length > 1" v-model="tab" class="tp-tabs" :options="tabs.map((t, i) => ({ value: i, label: t.label, icon: t.icon }))" :size="15" />
        <template v-for="(c, i) in controls" :key="c.id || 's' + i">
          <div v-if="c.type === 'section'" class="section-title tp-section">{{ c.label }}</div>
          <hr v-else-if="c.type === 'separator'" />
          <div v-else-if="c.type === 'slider'" class="tp-row">
            <span class="tp-lab">{{ c.label }}</span>
            <span class="tp-range"><Slider :modelValue="valueOf(c)" :min="c.min" :max="c.max" :step="c.step" class="tp-slider" :aria-label="c.label" @update:modelValue="(v) => send(c.id, v)" /><span class="faint mono tp-val">{{ shown(c, valueOf(c)) }}</span></span>
          </div>
          <label v-else-if="c.type === 'toggle'" class="tp-row">
            <span class="tp-lab grow">{{ c.label }}</span>
            <ToggleSwitch :modelValue="valueOf(c)" @update:modelValue="(v) => send(c.id, v)" />
          </label>
          <div v-else-if="c.type === 'choice'" class="tp-row">
            <span class="tp-lab">{{ c.label }}</span>
            <AppSegments :modelValue="valueOf(c)" :options="c.options" :iconOnly="c.options.every((o) => o.icon)" :size="15" @update:modelValue="(v) => v != null && send(c.id, v)" />
          </div>
          <div v-else-if="c.type === 'select'" class="tp-row">
            <span class="tp-lab">{{ c.label }}</span>
            <Select :modelValue="valueOf(c)" :options="c.options" optionLabel="label" optionValue="value" :filter="c.options.length > 12" autoFilterFocus size="small" class="tp-select" @update:modelValue="(v) => send(c.id, v)" />
          </div>
          <div v-else-if="c.type === 'swatches'" class="tp-row">
            <span class="tp-lab">{{ c.label }}</span>
            <span class="tp-swatches">
              <button v-for="o in c.options" :key="String(o.value)" class="tp-swatch" :class="{ on: valueOf(c) === o.value }" :style="{ background: o.background || undefined, color: o.foreground || undefined }" :aria-label="o.label" v-tooltip.bottom="o.label" @click="send(c.id, o.value)"><AppIcon v-if="o.icon" :name="o.icon" :size="13" /></button>
            </span>
          </div>
        </template>
      </div>
    </Popover>
  </span>
</template>

<style>
.v-tool { display: inline-flex; align-items: center; min-width: 0; }
.v-tool-text { margin: 0 calc(0.42rem * var(--g)); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 40ch; }
/* the handle stands half its width past either end of the track, at the first and last value; the margins keep it clear of the text beside it */
.v-tool-slider { width: round(9.79rem, 1px); margin: 0 calc(0.7rem * var(--g) + 0.77rem) 0 calc(0.7rem * var(--g) + 0.77rem); }
.v-tool-stand { opacity: .55; }
/* laid out as the layout popover is: a name on the left, its control on the right */
.tool-panel { width: round(25.17rem, 1px); display: flex; flex-direction: column; gap: calc(0.7rem * var(--g)); padding: calc(0.28rem * var(--g)); }
.tool-panel .tp-tabs { align-self: stretch; margin-bottom: calc(0.14rem * var(--g)); }
.tool-panel .tp-section { margin-top: calc(0.14rem * var(--g)); }
.tool-panel hr { width: 100%; margin: 0; border: 0; border-top: 1px solid var(--border); }
/* a label and control too long for the panel together, as in a longer language, put the control on a line of its own, still to the right */
.tool-panel .tp-row { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: calc(0.7rem * var(--g)); min-height: round(2.1rem, 1px); }
.tool-panel .tp-row > :last-child { margin-left: auto; }
.tool-panel .tp-lab { flex: none; }
.tool-panel .tp-lab.grow { flex: 1; }
.tool-panel .tp-range { display: inline-flex; align-items: center; width: round(14.69rem, 1px); }
.tool-panel .tp-slider { flex: 1; margin: 0 calc(0.56rem * var(--g)); }
.tool-panel .tp-val { flex: none; width: round(3.22rem, 1px); text-align: right; font-variant-numeric: tabular-nums; }
.tool-panel .tp-select { width: round(14.69rem, 1px); min-width: 0; }
.tool-panel .tp-swatches { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: calc(0.63rem * var(--g)); }
/* plain dots, as the folder colour menu has; a faint ring keeps one the popup's own colour visible */
.tool-panel .tp-swatch { width: round(1.4rem, 1px); height: round(1.4rem, 1px); display: grid; place-items: center; padding: 0; border: 0; border-radius: var(--round); background: var(--surface); color: var(--text-muted); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--text) 35%, transparent); cursor: pointer; }
.tool-panel .tp-swatch.on { outline: 2px solid var(--text); outline-offset: 2px; }
</style>

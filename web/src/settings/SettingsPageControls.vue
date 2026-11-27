<script setup>
import { computed } from 'vue'
import SelectButton from 'openvue/selectbutton'
import ToggleSwitch from 'openvue/toggleswitch'
import { S } from '../app/state.js'
import { saveUi } from '../app/preferences.js'
import { t } from '../shared/i18n.js'

const WHEEL = computed(() => [{ v: 'navigate', l: t('settings.controls.wheel.navigate') }, { v: 'zoom', l: t('settings.controls.wheel.zoom') }, { v: 'scroll', l: t('settings.controls.wheel.pan') }])
const SPACE = computed(() => [{ v: 'toggle', l: t('settings.controls.space.toggle') }, { v: 'hold', l: t('settings.controls.space.hold') }, { v: 'play', l: t('settings.controls.space.play') }, { v: 'scroll', l: t('settings.controls.space.scroll') }])
const DOUBLE_CLICK = computed(() => [{ v: 'here', l: t('settings.controls.doubleClick.here') }, { v: 'app', l: t('settings.controls.openApp') }])
const MIDDLE_CLICK = computed(() => [{ v: 'window', l: t('settings.controls.middleClick.window') }, { v: 'app', l: t('settings.controls.openApp') }, { v: 'none', l: t('settings.controls.middleClick.none') }])
const SIDE_BUTTONS = computed(() => [{ v: 'history', l: t('settings.controls.sideButtons.history') }, { v: 'navigate', l: t('settings.controls.sideButtons.navigate') }, { v: 'none', l: t('settings.controls.sideButtons.none') }])
const ANIMATED = computed(() => [{ v: false, l: t('settings.controls.animated.hover') }, { v: true, l: t('settings.controls.animated.always') }])
</script>

<template>
  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.controls.viewerSection') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.controls.wheel.title') }}</span>
          <span class="s-desc">{{ $t('settings.controls.wheel.ctrl') }}</span>
        </div>
        <div class="s-ctrl"><SelectButton v-model="S.ui.viewerWheel" :options="WHEEL" optionLabel="l" optionValue="v" :allowEmpty="false" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.controls.space.title') }}</span></div>
        <div class="s-ctrl"><SelectButton v-model="S.ui.spaceAction" :options="SPACE" optionLabel="l" optionValue="v" :allowEmpty="false" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.controls.rightClick.dragZoom') }}</span></div>
        <div class="s-ctrl"><ToggleSwitch v-model="S.ui.rightDragZoom" :aria-label="$t('settings.controls.rightClick.dragZoom')" @change="saveUi" /></div>
      </div>
    </div>
  </section>

  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.controls.gridSection') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.controls.doubleClick.title') }}</span></div>
        <div class="s-ctrl"><SelectButton v-model="S.ui.doubleClick" :options="DOUBLE_CLICK" optionLabel="l" optionValue="v" :allowEmpty="false" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.controls.middleClick.title') }}</span></div>
        <div class="s-ctrl"><SelectButton v-model="S.ui.middleClick" :options="MIDDLE_CLICK" optionLabel="l" optionValue="v" :allowEmpty="false" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.controls.sideButtons.title') }}</span></div>
        <div class="s-ctrl"><SelectButton v-model="S.ui.sideButtons" :options="SIDE_BUTTONS" optionLabel="l" optionValue="v" :allowEmpty="false" @change="saveUi" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.controls.animated.title') }}</span></div>
        <div class="s-ctrl"><SelectButton v-model="S.ui.gifAlways" :options="ANIMATED" optionLabel="l" optionValue="v" :allowEmpty="false" @change="saveUi" /></div>
      </div>
    </div>
  </section>
</template>

<style scoped src="./settings.css"></style>

<script setup>
import { computed, ref, watch } from 'vue'
import Select from 'openvue/select'
import InputText from 'openvue/inputtext'
import InputGroup from 'openvue/inputgroup'
import Button from 'openvue/button'
import SelectButton from 'openvue/selectbutton'
import Checkbox from 'openvue/checkbox'
import ToggleSwitch from 'openvue/toggleswitch'
import ColorPicker from 'openvue/colorpicker'
import Tag from 'openvue/tag'
import Textarea from 'openvue/textarea'
import AppIcon from '../shared/AppIcon.vue'
import AppSegments from '../shared/AppSegments.vue'
import { S } from '../app/state.js'
import { saveUi } from '../app/preferences.js'
import { refreshTheme, currentTheme, loadThemeFile, seeThrough } from '../app/appearance.js'
import { run, toast } from '../app/notify.js'
import { call } from '../platform/ipc.js'
import { BUILTIN, TOKENS, applyTheme, toCss, toHex, fromCss, themeLabel } from '../shared/theme.js'
import { onSettingsReset, setUi } from './settingsData.js'
import { t } from '../shared/i18n.js'

// appearance: themes are chosen for dark and for light, one tab at a time; without Follow system, the tab is the mode in use
const THEME_SLOTS = computed(() => [{ value: 'dark', label: t('settings.appearance.slots.dark'), icon: 'moon' }, { value: 'light', label: t('settings.appearance.slots.light'), icon: 'sun' }])
const themeSlot = ref(S.ui.themeMode === 'light' || (S.ui.themeMode === 'system' && !S.systemDark) ? 'light' : 'dark')
const followSystem = computed({
  get: () => S.ui.themeMode === 'system',
  set: (on) => setUi('themeMode', on ? 'system' : themeSlot.value),
})
function setThemeSlot(v) {
  const prevSlot = themeSlot.value
  themeSlot.value = v
  if (S.ui.themeMode !== 'system') {
    const prevName = prevSlot === 'light' ? S.ui.lightTheme : S.ui.darkTheme
    const family = BUILTIN[prevName]?.family
    if (family) {
      const sibling = Object.keys(BUILTIN).find((n) => BUILTIN[n].family === family && !!BUILTIN[n].dark === (v === 'dark'))
      if (sibling) S.ui[v === 'dark' ? 'darkTheme' : 'lightTheme'] = sibling
    }
    setUi('themeMode', v)
  }
}
const themeNames = computed(() => [...Object.keys(BUILTIN), ...Object.keys(S.ui.customThemes)].filter((n) => !!themeOf(n).dark === (themeSlot.value === 'dark')))
const editingTheme = ref(null)
async function exportTheme() {
  const th = currentTheme()
  await run('save_text', { name: `${th.name}.css`, content: toCss(th) })
}
async function importTheme() {
  const got = await run('pick_text_file')
  if (!got) return
  const th = fromCss(got.text)
  const name = th.name === 'Imported' ? got.name.replace(/\.css$/i, '') : th.name
  S.ui.customThemes[name] = got.text
  if (th.dark) S.ui.darkTheme = name
  else S.ui.lightTheme = name
  saveUi()
  refreshTheme()
}
const themeOf = (n) => BUILTIN[n] || fromCss(S.ui.customThemes[n])
// the stock Dark and Light share the family name Default, which a language translates
const cardName = (n) => (BUILTIN[n]?.family === 'Default' ? t('settings.appearance.theme.default') : themeLabel(n, themeOf(n)))
const chosenTheme = (n) => n === (themeSlot.value === 'light' ? S.ui.lightTheme : S.ui.darkTheme)
const chosenCustom = computed(() => Object.keys(S.ui.customThemes).find((n) => chosenTheme(n)) || null)
function chooseTheme(n) {
  // an edit in progress belongs to the theme it started from
  if (editingTheme.value && editingTheme.value.was !== n) editingTheme.value = null
  const dark = themeOf(n).dark
  S.ui[dark ? 'darkTheme' : 'lightTheme'] = n
  if (S.ui.themeMode !== 'system') S.ui.themeMode = dark ? 'dark' : 'light'
  saveUi()
  refreshTheme()
}
function previewPalette(n) {
  const p = themeOf(n).tokens
  return { bg: p.bg, panel: p.panel, s2: p['surface-2'], s3: p['surface-3'] || p['surface-2'], border: p.border, text: p.text, muted: p['text-muted'], accent: p.accent, star: p.star || '#eab308' }
}
const themeCards = computed(() => themeNames.value.map((n) => ({ id: n, label: cardName(n), pal: previewPalette(n) })))
/** A theme profile: its name and a colour for each token, shown live while editing. */
function freeName(base) {
  let name = base
  for (let n = 2; BUILTIN[name] || S.ui.customThemes[name]; n++) name = `${base} ${n}`
  return name
}
function newProfile() {
  const theme = currentTheme()
  editingTheme.value = { name: freeName(t('settings.appearance.copyName', { name: theme.name })), dark: !!theme.dark, tokens: { ...theme.tokens }, drafts: { ...theme.tokens }, was: null }
}
function editProfile(n) {
  const th = fromCss(S.ui.customThemes[n])
  editingTheme.value = { name: n, dark: !!th.dark, tokens: { ...th.tokens }, drafts: { ...th.tokens }, was: n }
  applyProfile()
}
function applyProfile() {
  const e = editingTheme.value
  applyTheme({ name: e.name, dark: e.dark, tokens: e.tokens }, { userCss: S.ui.userCss, safeMode: !!S.startup.safeMode, extraVars: S.contrib.themeVariables || {}, seeThrough: seeThrough() })
}
function parseColor(v) {
  const s = String(v || '').trim()
  if (!s) return null
  if (/^[0-9a-f]{6}$/i.test(s)) return '#' + s
  return CSS.supports('color', s) ? s : null
}
function setToken(k, v) {
  editingTheme.value.tokens[k] = v
  editingTheme.value.drafts[k] = v
  applyProfile()
}
function typeToken(k, v) {
  editingTheme.value.drafts[k] = v
  const color = parseColor(v)
  if (color) {
    editingTheme.value.tokens[k] = color
    applyProfile()
  }
}
function commitToken(k) {
  editingTheme.value.drafts[k] = editingTheme.value.tokens[k]
}
function saveEditedTheme() {
  const e = editingTheme.value
  const name = e.name.trim()
  if (!name) return toast(t('settings.appearance.edit.noName'), 'error')
  if (BUILTIN[name] || (name !== e.was && S.ui.customThemes[name])) return toast(t('settings.appearance.edit.taken'), 'error')
  if (e.was && e.was !== name) delete S.ui.customThemes[e.was]
  S.ui.customThemes[name] = toCss({ name, dark: e.dark, tokens: e.tokens })
  S.ui[e.dark ? 'darkTheme' : 'lightTheme'] = name
  if (S.ui.themeMode !== 'system') S.ui.themeMode = e.dark ? 'dark' : 'light'
  editingTheme.value = null
  saveUi()
  refreshTheme()
}
function cancelEditedTheme() {
  editingTheme.value = null
  refreshTheme()
}
function deleteProfile(n) {
  delete S.ui.customThemes[n]
  if (S.ui.darkTheme === n) S.ui.darkTheme = 'Dark'
  if (S.ui.lightTheme === n) S.ui.lightTheme = 'Light'
  saveUi()
  refreshTheme()
}
async function pickThemeFile() {
  const p = await call('pick_theme_file')
  if (!p) return
  S.ui.themeFile = p
  saveUi()
  loadThemeFile()
}
async function clearThemeFile() {
  await call('clear_theme_file').catch(() => {})
  S.ui.themeFile = ''
  saveUi()
  loadThemeFile()
}
const USER_CSS_WAIT_MS = 400
let userCssTimer = null
const userCssDraft = ref('')
watch(userCssDraft, (v) => {
  clearTimeout(userCssTimer)
  if (v !== S.ui.userCss) userCssTimer = setTimeout(() => setUi('userCss', v), USER_CSS_WAIT_MS)
})
const CORNER_CHOICES = computed(() => [['none', 'none'], ['small', 'small'], ['normal', 'normal']].map(([v, k]) => ({ v, l: t(`settings.appearance.corners.${k}`) })))
const SPACING_CHOICES = computed(() => [['compact', 'compact'], ['normal', 'normal'], ['expanded', 'expanded']].map(([v, k]) => ({ v, l: t(`settings.appearance.spacing.${k}`) })))
const SCROLLBAR_CHOICES = computed(() => [[true, 'overlay'], [false, 'beside']].map(([v, k]) => ({ v, l: t(`settings.appearance.scrollbars.${k}`) })))
// the fonts installed on this computer; an empty choice is the system's own interface font
const fontNames = ref([])
// a Select reads an empty value as no choice at all
const SYSTEM_FONT = '\u0000system'
const USER_CSS_EXAMPLE = ':root { --accent: #ff7a00; }'
const fontChoices = computed(() => {
  const names = S.ui.font && !fontNames.value.includes(S.ui.font) ? [S.ui.font, ...fontNames.value] : fontNames.value
  return [{ v: SYSTEM_FONT, l: t('settings.appearance.interface.systemFont') }, ...names.map((f) => ({ v: f, l: f }))]
})
async function listFonts() {
  if (!fontNames.value.length) fontNames.value = await call('system_fonts').catch(() => [])
}
listFonts()
watch(() => S.dialogs.prefs, () => (userCssDraft.value = S.ui.userCss), { immediate: true })
onSettingsReset(() => (userCssDraft.value = ''))
</script>

<template>
  <section class="s-section">
    <div class="s-section-head">
      <span class="s-section-title">{{ $t('settings.appearance.theme.title') }}</span>
      <div class="s-section-actions">
        <Button :label="$t('settings.appearance.files.export')" severity="secondary" size="small" @click="exportTheme" />
        <Button :label="$t('settings.appearance.files.import')" severity="secondary" size="small" @click="importTheme" />
      </div>
    </div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-ctrl"><AppSegments :modelValue="themeSlot" :options="THEME_SLOTS" @update:modelValue="setThemeSlot" /></div>
        <div class="s-ctrl">
          <span class="s-desc">{{ $t('settings.appearance.theme.followSystem') }}</span>
          <ToggleSwitch v-model="followSystem" :aria-label="$t('settings.appearance.theme.followSystem')" />
        </div>
      </div>
      <div class="s-subblock">
        <div class="theme-grid">
          <button v-for="c in themeCards" :key="c.id" type="button" class="theme-card" :class="{ active: chosenTheme(c.id) }" :aria-pressed="chosenTheme(c.id)" @click="chooseTheme(c.id)">
            <svg viewBox="0 0 120 74" class="theme-svg" aria-hidden="true">
              <rect width="120" height="74" rx="4" :fill="c.pal.bg" />
              <rect width="30" height="74" :fill="c.pal.panel" />
              <line x1="30" y1="0" x2="30" y2="74" :stroke="c.pal.border" stroke-width="1" />
              <rect x="6" y="8" width="18" height="3.5" rx="1.5" :fill="c.pal.text" opacity="0.65" />
              <rect x="4" y="16" width="22" height="7" rx="2" :fill="c.pal.accent" opacity="0.22" />
              <rect x="7" y="18.2" width="14" height="2.6" rx="1" :fill="c.pal.accent" />
              <rect x="6" y="27" width="16" height="2.6" rx="1" :fill="c.pal.muted" opacity="0.6" />
              <rect x="6" y="34" width="13" height="2.6" rx="1" :fill="c.pal.muted" opacity="0.5" />
              <rect x="6" y="41" width="17" height="2.6" rx="1" :fill="c.pal.muted" opacity="0.45" />
              <rect x="36" y="7" width="38" height="7" rx="2" :fill="c.pal.s2" />
              <rect x="96" y="7" width="18" height="7" rx="2" :fill="c.pal.accent" />
              <rect x="36" y="19" width="24" height="22" rx="3" :fill="c.pal.s2" />
              <rect x="63" y="19" width="24" height="22" rx="3" :fill="c.pal.s3" :stroke="c.pal.accent" stroke-width="1" />
              <rect x="90" y="19" width="24" height="22" rx="3" :fill="c.pal.s2" />
              <rect x="36" y="45" width="24" height="22" rx="3" :fill="c.pal.s2" />
              <rect x="63" y="45" width="24" height="22" rx="3" :fill="c.pal.s2" />
              <rect x="90" y="45" width="24" height="22" rx="3" :fill="c.pal.s2" />
            </svg>
            <div class="theme-foot">
              <span class="theme-name">{{ c.label }}</span>
              <AppIcon v-if="chosenTheme(c.id)" name="check" :size="14" class="theme-check" />
            </div>
          </button>
        </div>
        <div class="s-actions">
          <Button :label="$t('settings.appearance.theme.duplicate')" severity="secondary" size="small" @click="newProfile" />
          <template v-if="chosenCustom">
            <Button :label="$t('settings.appearance.editTheme', { name: chosenCustom })" severity="secondary" size="small" @click="editProfile(chosenCustom)" />
            <Button :label="$t('settings.appearance.deleteTheme', { name: chosenCustom })" severity="danger" text size="small" @click="deleteProfile(chosenCustom)" />
          </template>
        </div>
      </div>
    </div>
  </section>

  <section v-if="editingTheme" class="s-section">
    <div class="s-section-head">
      <span class="s-section-title">{{ $t('settings.appearance.edit.title') }}</span>
      <Tag severity="info" :value="$t('settings.appearance.edit.livePreview')" />
    </div>
    <div class="s-card">
      <div class="s-row theme-edit-row">
        <div class="s-actions grow">
          <InputText v-model="editingTheme.name" :placeholder="$t('settings.appearance.edit.name')" class="theme-name-in" />
          <label class="opt"><Checkbox v-model="editingTheme.dark" binary @change="applyProfile" /> {{ $t('settings.appearance.edit.dark') }}</label>
        </div>
        <div class="s-actions">
          <Button :label="$t('settings.appearance.edit.save')" size="small" @click="saveEditedTheme" />
          <Button :label="$t('common.cancel')" severity="secondary" size="small" @click="cancelEditedTheme" />
        </div>
      </div>
      <div class="s-subblock">
        <div class="token-grid">
          <div v-for="[k] in TOKENS" :key="k" class="token-row">
            <ColorPicker :modelValue="toHex(editingTheme.tokens[k]).slice(1)" format="hex" :ariaLabel="k" @update:modelValue="(v) => setToken(k, '#' + v)" />
            <span class="token-key">{{ k }}</span>
            <InputText class="s-mono token-value" :modelValue="editingTheme.drafts[k]" spellcheck="false" @update:modelValue="(v) => typeToken(k, v)" @blur="commitToken(k)" @keydown.enter="commitToken(k)" />
          </div>
        </div>
      </div>
    </div>
  </section>

  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.appearance.interface.title') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.appearance.interface.font') }}</span></div>
        <div class="s-ctrl w-md">
          <Select :modelValue="S.ui.fontSystem ? SYSTEM_FONT : S.ui.font || SYSTEM_FONT" :options="fontChoices" optionLabel="l" optionValue="v" filter autoFilterFocus :filterPlaceholder="$t('settings.appearance.interface.searchFonts')" :virtualScrollerOptions="{ itemSize: 32 }" overlayClass="font-list" @show="listFonts" @update:modelValue="(v) => { setUi('fontSystem', false); setUi('font', v === SYSTEM_FONT ? '' : v) }" />
        </div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.appearance.interface.corners') }}</span></div>
        <div class="s-ctrl"><SelectButton :modelValue="S.ui.corners" :options="CORNER_CHOICES" optionLabel="l" optionValue="v" :allowEmpty="false" @update:modelValue="(v) => setUi('corners', v)" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.appearance.interface.spacing') }}</span></div>
        <div class="s-ctrl"><SelectButton :modelValue="S.ui.spacing" :options="SPACING_CHOICES" optionLabel="l" optionValue="v" :allowEmpty="false" @update:modelValue="(v) => setUi('spacing', v)" /></div>
      </div>
      <div class="s-row">
        <div class="s-meta"><span class="s-label">{{ $t('settings.appearance.interface.scrollbars') }}</span></div>
        <div class="s-ctrl"><SelectButton :modelValue="S.ui.overlayScrollbars !== false" :options="SCROLLBAR_CHOICES" optionLabel="l" optionValue="v" :allowEmpty="false" @update:modelValue="(v) => setUi('overlayScrollbars', v)" /></div>
      </div>
    </div>
  </section>

  <section class="s-section">
    <div class="s-section-head"><span class="s-section-title">{{ $t('settings.appearance.userCss.sectionTitle') }}</span></div>
    <div class="s-card">
      <div class="s-row">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.appearance.follow.title') }}</span>
          <span class="s-desc">{{ $t('settings.appearance.follow.hint') }}</span>
        </div>
        <div class="s-ctrl w-lg">
          <InputGroup>
            <InputText class="s-mono" :modelValue="S.ui.themeFile" :title="S.ui.themeFile" :placeholder="$t('settings.appearance.follow.none')" readonly />
            <Button :label="$t('settings.appearance.follow.choose')" severity="secondary" size="small" @click="pickThemeFile" />
            <Button v-if="S.ui.themeFile" :label="$t('settings.appearance.follow.stop')" severity="secondary" text size="small" @click="clearThemeFile" />
          </InputGroup>
        </div>
      </div>
      <div class="s-subblock">
        <div class="s-meta">
          <span class="s-label">{{ $t('settings.appearance.userCss.title') }}</span>
          <i18n-t keypath="settings.appearance.userCss.hint" tag="span" class="s-desc"><template #flag><code>--safe-mode</code></template></i18n-t>
        </div>
        <Textarea v-model="userCssDraft" class="s-mono code-area" rows="5" spellcheck="false" :placeholder="USER_CSS_EXAMPLE" />
      </div>
    </div>
  </section>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.theme-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(11.4rem, 1fr)); gap: calc(0.63rem * var(--g)); }
.theme-card { display: flex; flex-direction: column; gap: calc(0.42rem * var(--g)); padding: calc(0.49rem * var(--g)); border-radius: calc(8px * var(--r)); border: 1.5px solid transparent; background: var(--surface-3); color: inherit; font: inherit; cursor: pointer; text-align: left; transition: background 0.12s, border-color 0.12s; }
.theme-card:hover { background: var(--hover); }
.theme-card.active { border-color: var(--accent); background: var(--accent-subtle); }
.theme-svg { width: 100%; height: auto; display: block; border-radius: calc(5px * var(--r)); }
.theme-foot { display: flex; align-items: center; justify-content: space-between; gap: calc(0.28rem * var(--g)); padding: 0 calc(0.14rem * var(--g)); }
.theme-name { font-size: 0.86rem; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.theme-check { color: var(--accent); flex: none; }
.theme-edit-row { flex-wrap: wrap; row-gap: calc(0.49rem * var(--g)); }
.theme-name-in { width: round(14rem, 1px); }
.token-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: calc(0.42rem * var(--g)) calc(1.26rem * var(--g)); }
.token-key { flex: 1; min-width: 0; font-family: var(--mono-font); font-size: 0.85rem; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.token-value { flex: 0 0 7.6rem; min-width: 0; }
.code-area { width: 100%; }
@container (max-width: 34rem) {
  .token-grid { grid-template-columns: 1fr; }
}
</style>

import { createApp } from 'vue'
import OpenVue from 'openvue/config'
import Tooltip from 'openvue/tooltip'
import ToastService from 'openvue/toastservice'
import { installOverlayScroll } from './shared/overlayScroll.js'
import './style.css'
import { THEME_OPTIONS } from './shared/preset.js'
import { openvueDefaults } from './shared/openvueDefaults.js'
import App from './App.vue'
import { boot } from './app/boot.js'
import { loadIcons } from './app/icons.js'
import { installWindowDrag } from './platform/windowFrame.js'
import { app as appText, openvue as openvueText } from 'virtual:curio-locales'
import { followLanguage, makeI18n } from './shared/i18n.js'
import { followSetting, startLanguage } from './app/language.js'

const app = createApp(App)
openvueDefaults()
app.use(OpenVue, THEME_OPTIONS)
app.use(makeI18n({ messages: appText, openvue: openvueText, warn: import.meta.env.CURIO_PSEUDO }))
followLanguage(app)
app.use(ToastService)
installOverlayScroll()
// tooltips open below what they describe unless a position is given
const POSITIONS = ['top', 'bottom', 'left', 'right']
const below = (b) => (b.arg || Object.keys(b.modifiers || {}).some((k) => POSITIONS.includes(k)) ? b : { ...b, modifiers: { ...b.modifiers, bottom: true } })
// a button with only an icon is named by its tooltip, for screen readers; a name given by hand stays
function nameByTip(el, b) {
  if (!(el.tagName === 'BUTTON' || el.getAttribute('role') === 'button') || el.textContent.trim() || (el.hasAttribute('aria-label') && !el.dataset.tipName)) return
  const v = typeof b.value === 'string' ? b.value : b.value?.escape === false ? b.value.value.replace(/<span class="tip-keys">.*$/s, '').replace(/<[^>]*>/g, '').replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'") : b.value?.value
  if (!v) return
  el.setAttribute('aria-label', v)
  el.dataset.tipName = '1'
}
app.directive('tooltip', {
  ...Tooltip,
  beforeMount: (el, b, v, p) => Tooltip.beforeMount(el, below(b), v, p),
  mounted: (el, b, v, p) => {
    Tooltip.mounted?.(el, below(b), v, p)
    nameByTip(el, b)
  },
  updated: (el, b, v, p) => {
    Tooltip.updated(el, below(b), v, p)
    nameByTip(el, b)
  },
})
// names being typed get the cursor straight away, with any existing text selected
app.directive('focus', {
  mounted(el) {
    const input = el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' ? el : el.querySelector('input,textarea')
    // a Dialog focuses its close button once it has opened, unless something in it is marked autofocus
    input?.setAttribute('autofocus', '')
    requestAnimationFrame(() => {
      input?.focus()
      input?.select?.()
      el.scrollIntoView?.({ block: 'nearest' })
    })
  },
})
// a window of Curio's is never a frame: a plugin's frame that navigates here gets nothing but this page, never a second window's state and calls
if (window.top === window) {
  Promise.all([startLanguage(), loadIcons()]).then(() => {
    app.mount('#app')
    installWindowDrag()
    followSetting()
    boot().catch((e) => console.error(e))
  })
}

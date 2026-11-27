// No system frame: empty parts of the toolbars move the window, and a double click maximises it.
// A Mac's window buttons, drawn by Curio too, sit at the top left in its own colours (`WindowControls.vue`).
import { getCurrentWindow } from '@tauri-apps/api/window'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { call } from './ipc.js'

// the rest of the window reaches Tauri through platform/ alone
export { getCurrentWindow, getCurrentWebview }

const INTERACTIVE = 'button, input, textarea, select, a, label, [role="button"], [role="combobox"], .p-select, .p-slider, .slot-over, [data-no-drag]'
export const MATERIALS = ['mica', 'acrylic']
export const MAC_BUTTONS = navigator.userAgent.includes('Mac')
// panel opacity for each Acrylic transparency; at exactly 80 % the panels pick up a pink cast
const ACRYLIC = { low: 90, medium: 79, high: 70 }

// how long the system takes to bring a window's material back when it comes to the front
const MATERIAL_FADE_MS = 260
// Settings › General: the material stays while the window is in the background
let keepMaterial = false
let refreshActive = () => {}

// A modal dialog already stops everything else; the window keeps still under it too, so no blurred surface is open while it moves.
let modal = false

export function installWindowDrag() {
  document.addEventListener('mousedown', (e) => {
    if (modal || e.button !== 0 || !e.target.closest?.('[data-window-drag]') || e.target.closest(INTERACTIVE)) return
    const win = getCurrentWindow()
    if (e.detail === 2) win.toggleMaximize().catch(() => {})
    else win.startDragging().catch(() => {})
  })
  // an inactive window gets a flat grey from the system in place of its material, which reads as brown beside the theme's tinted greys; the theme's own colours stand in for it.
  // Coming to the front, the system fades its grey back into the material: the panels stay solid until it has, so the grey doesn't flash through them and the change is one step.
  let timer = null
  const active = () => {
    clearTimeout(timer)
    const root = document.documentElement
    if (keepMaterial || document.hasFocus()) timer = setTimeout(() => root.classList.remove('fx-inactive'), keepMaterial || !root.classList.contains('fx-inactive') ? 0 : MATERIAL_FADE_MS)
    else root.classList.add('fx-inactive')
  }
  window.addEventListener('focus', active)
  window.addEventListener('blur', active)
  refreshActive = active
  active()
  new MutationObserver(() => {
    const open = !!document.querySelector('body > .p-dialog-mask')
    if (open === modal) return
    modal = open
    getCurrentWindow().setResizable(!open).catch(() => {})
  }).observe(document.body, { childList: true })
}

let zoom = 0
let look = ''

/** Settings → General: the zoom of the whole window, and the material behind see-through panels. */
export function applyWindowLook(ui, { dark, materials }) {
  const z = (ui.zoomLevel || 100) / 100
  if (z !== zoom) {
    zoom = z
    getCurrentWebview().setZoom(z).catch(() => {})
  }
  const material = materials.includes(ui.windowMaterial) ? ui.windowMaterial : 'off'
  const root = document.documentElement
  root.classList.toggle('fx-see-through', material !== 'off')
  for (const m of MATERIALS) root.classList.toggle(`fx-${m}`, m === material)
  // Mica stays fully see-through, as in the system's own apps; Acrylic turns grey rather than clearer below about 60 %
  const opacity = material === 'acrylic' ? ACRYLIC[ui.acrylicTransparency] || ACRYLIC.medium : 0
  root.style.setProperty('--fx-panel', `${opacity}%`)
  root.style.setProperty('--fx-side', `${Math.max(0, opacity - 16)}%`)
  keepMaterial = material !== 'off' && !!ui.keepMaterial
  refreshActive()
  const next = { material, dark, keep: keepMaterial }
  const key = JSON.stringify(next)
  if (key === look) return
  look = key
  call('window_look', { look: next }).catch(() => {})
}

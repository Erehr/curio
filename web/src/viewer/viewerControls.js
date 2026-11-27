// What is on show in an opened item, shared by the viewer and the toolbar widgets that drive it.
import { computed, reactive } from 'vue'
import { getCurrentWindow } from '../platform/windowFrame.js'
import { S } from '../app/state.js'
import { openItem } from '../library/lifecycle.js'
import { saveUi } from '../app/preferences.js'
import { stepViewer, viewerFile } from './session.js'
import { toast } from '../app/notify.js'
import { fitBox, kindOf } from '../formats/index.js'
import { MARKDOWN } from '../formats/markdown.js'
import { rotateItem } from '../actions/actions.js'
import { errorText } from '../platform/ipc.js'

// `live`: a Live Photo's clip plays over its still; `alt`: the other file of a RAW and JPEG pair is shown
export const V = reactive({ zoom: 1, pan: { x: 0, y: 0 }, rotation: 0, flip: false, stageW: 0, stageH: 0, live: false, alt: false })

export const viewerItem = computed(() => {
  void S.itemsVer
  return S.viewer ? S.items.get(S.viewer.id) : null
})
// the file on show, which may be one grouped into the item: its kind picks the viewer and the toolbar
const shownItem = computed(() => {
  void S.itemsVer
  return S.viewer ? S.items.get(viewerFile()) : null
})
// a Live Photo's clip and a RAW / JPEG pair's other file go with the item's own file
export const liveClip = computed(() => (S.viewer?.file == null ? viewerItem.value?.motion : null))
export const pairFile = computed(() => (S.viewer?.file == null ? viewerItem.value?.pair : null))
export const viewerKind = computed(() => kindOf(shownItem.value))
export const isPicture = computed(() => viewerKind.value === 'image')
export const isVideo = computed(() => viewerKind.value === 'video')
export const isPlayer = computed(() => ['video', 'audio'].includes(viewerKind.value))
export const isText = computed(() => viewerKind.value === 'text')
export const isMarkdown = computed(() => isText.value && MARKDOWN.includes(shownItem.value?.ext))

const dpr = () => window.devicePixelRatio || 1
const fitted = computed(() => fitBox(shownItem.value, V.stageW, V.stageH, null, shownItem.value?.rotation || 0))
/** One step of zoom = this share of the file's pixels. */
export const fitShare = computed(() => (fitted.value && shownItem.value?.width ? (fitted.value.w * dpr()) / shownItem.value.width : 1))
export const percent = computed(() => Math.round(isText.value ? textView.value.zoom * 100 : V.zoom * fitShare.value * 100))

// Zoom is felt in ratios, so everything here is logarithmic: every part of the slider, every pixel of
// the gesture and every bit of wheel multiplies the size by the same amount.
const PERCENT_MIN = 10
const PERCENT_MAX = 1000
const SLIDER_MAX = 100
const RANGE = Math.log(PERCENT_MAX / PERCENT_MIN)
export const toPercent = (v, lo = PERCENT_MIN, range = RANGE) => lo * Math.exp((v / SLIDER_MAX) * range)
export const toSlider = (d, lo = PERCENT_MIN, range = RANGE) => (Math.log(d / lo) / range) * SLIDER_MAX
// the slider catches on the doublings of 100 %
const SNAP_POINTS = [25, 50, 100, 200, 400]
const SNAP_WITHIN = 0.04
export function snapPercent(d) {
  return SNAP_POINTS.find((p) => Math.abs(d / p - 1) <= SNAP_WITHIN) ?? d
}
/** A picture already larger or smaller than the limits at fit size may stay at fit size. */
export const clampZoom = (z) => {
  const share = fitShare.value || 1
  return Math.min(Math.max(PERCENT_MAX / 100 / share, 1), Math.max(Math.min(PERCENT_MIN / 100 / share, 1), z))
}
export const zoomFromPercent = (d) => clampZoom(d / 100 / (fitShare.value || 1))
export const zoomSlider = computed({
  get: () => (isText.value ? toSlider(textView.value.zoom * 100, TEXT_MIN, TEXT_RANGE) : toSlider(V.zoom * fitShare.value * 100)),
  set: (v) => (isText.value ? setTextZoom(snapPercent(toPercent(v, TEXT_MIN, TEXT_RANGE)) / 100) : setZoomCentered(zoomFromPercent(snapPercent(toPercent(v))))),
})

// the right-button gesture: this many pixels up doubles the size
const GESTURE_DOUBLING_PX = 160
export function gesturePercent(startPercent, up) {
  const wanted = startPercent * 2 ** (up / GESTURE_DOUBLING_PX)
  return Math.min(Math.max(PERCENT_MAX, startPercent), Math.max(Math.min(PERCENT_MIN, startPercent), wanted))
}

// the wheel: this much of its movement doubles the size, however finely a touchpad reports it
const WHEEL_DOUBLING = 500
export const wheelFactor = (deltaY) => 2 ** (-deltaY / WHEEL_DOUBLING)

/** The furthest a picture of this zoom may sit from the middle: none while it fits on that side. */
function clampPan(pan, z) {
  const f = fitted.value
  if (!f) return pan
  const roomX = Math.max(0, (f.w * z - V.stageW) / 2)
  const roomY = Math.max(0, (f.h * z - V.stageH) / 2)
  return { x: Math.max(-roomX, Math.min(roomX, pan.x)), y: Math.max(-roomY, Math.min(roomY, pan.y)) }
}
/** Moves the picture, as far as its edges allow: every zoom and drag keeps to the same bounds, so the next one starts where this one left it. */
export function panTo(pan) {
  V.pan = clampPan(pan, V.zoom)
}

/** Zoom keeping the point of the picture under the pointer there, as far as its edges allow; (px, py) is measured from the middle of the stage. */
export function zoomAround(z, px, py) {
  const next = clampZoom(z)
  const k = next / V.zoom
  V.pan = clampPan({ x: px - (px - V.pan.x) * k, y: py - (py - V.pan.y) * k }, next)
  V.zoom = next
}
function setZoomCentered(z) {
  V.pan = clampPan({ x: V.pan.x * (z / V.zoom), y: V.pan.y * (z / V.zoom) }, z)
  V.zoom = z
}

export function oneToOne() {
  if (isText.value) return setTextZoom(1)
  V.zoom = zoomFromPercent(100)
  V.pan = { x: 0, y: 0 }
}
export function fit() {
  if (isText.value) return setTextZoom(1)
  V.zoom = 1
  V.pan = { x: 0, y: 0 }
}
// the zoom keys: a picture by this much a press, text through the sizes a browser steps through
const KEY_ZOOM = 1.25
const TEXT_STEPS = [50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400]
export function zoomStep(dir) {
  if (isText.value) {
    const now = Math.round(textView.value.zoom * 100)
    const next = dir > 0 ? TEXT_STEPS.find((p) => p > now) : TEXT_STEPS.findLast((p) => p < now)
    if (next) setTextZoom(next / 100)
  } else if (isPicture.value) setZoomCentered(clampZoom(V.zoom * KEY_ZOOM ** dir))
}

// Text and Markdown: one size, view and wrapping for every file, kept with the settings.
export const TEXT_MODES = ['rendered', 'source', 'split']
const TEXT_MIN = 50
const TEXT_MAX = 400
const TEXT_RANGE = Math.log(TEXT_MAX / TEXT_MIN)
export const textView = computed(() => S.ui.textView)
export function setTextView(key, value) {
  S.ui.textView = { ...S.ui.textView, [key]: value }
  saveUi()
}
export const setTextZoom = (z) => setTextView('zoom', Math.min(TEXT_MAX / 100, Math.max(TEXT_MIN / 100, z)))
export const textPercentFrom = (d) => Math.min(TEXT_MAX, Math.max(TEXT_MIN, d))
export function nextTextMode() {
  if (isMarkdown.value) setTextView('mode', TEXT_MODES[(TEXT_MODES.indexOf(textView.value.mode) + 1) % TEXT_MODES.length])
}
/** Whether the text on show is drawn as written, where wrapping and line numbers apply. */
export const textAsWritten = computed(() => isText.value && !(isMarkdown.value && textView.value.mode === 'rendered'))
export function toggleText(key) {
  if (textAsWritten.value) setTextView(key, !textView.value[key])
}
// the text's scrolling pane, for the keys that scroll it
let textPane = null
export const setTextPane = (el) => (textPane = el)
/** Scrolls the text on show: `top`, `bottom`, or a page up (-1) or down (1). False when no text is open. */
export function scrollText(how) {
  if (!isText.value || !textPane?.isConnected) return false
  const page = textPane.clientHeight * 0.9
  textPane.scrollTo({ top: how === 'top' ? 0 : how === 'bottom' ? textPane.scrollHeight : textPane.scrollTop + how * page, behavior: typeof how === 'number' ? 'smooth' : 'instant' })
  return true
}
// ItemViewer.vue is on screen, so R and H turn and mirror what it shows
let mounted = false
export const setViewerMounted = (on) => (mounted = on)
export const viewerMounted = () => mounted
export function rotate() {
  if (isVideo.value) V.rotation = (V.rotation + 90) % 360
  else if (isPicture.value) rotateItem(viewerFile()).catch((e) => toast(errorText(e), 'error'))
}
export function flip() {
  V.flip = !V.flip
}
export function toggleStrip() {
  S.ui.viewerStrip = !S.ui.viewerStrip
  saveUi()
}
export const step = (d) => stepViewer(d)
export function closeViewer() {
  if (S.fullscreen) S.fullscreen = false
  else if (S.itemWindow) getCurrentWindow().close().catch(() => {})
  else S.viewer = null
}
// the file shown: with RAW / JPEG switched, the item's pair
export const shownId = () => (V.alt && pairFile.value) || viewerFile()
export const revealItem = () => S.viewer && openItem(shownId(), 'reveal')
export const openExternally = () => S.viewer && openItem(shownId(), 'open')

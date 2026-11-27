// A viewer page's controls (viewer.setTools), each drawn by the widget its plugin declares for it (`viewer_tools`).
import { reactive } from 'vue'
import { S } from '../app/state.js'
import { toolChanged } from './bridge.js'

// a value being set here wins over the page's until the page has had time to answer, so a dragged slider never jumps back
const HOLD_MS = 600
const held = reactive({})
const timers = {}
let frame = 0
let next = null

export function send(id, value) {
  held[id] = value
  clearTimeout(timers[id])
  timers[id] = setTimeout(() => delete held[id], HOLD_MS)
  next = [id, value]
  if (!frame) frame = requestAnimationFrame(() => {
    frame = 0
    toolChanged(...next)
  })
}

export const valueOf = (c) => (c.id in held ? held[c.id] : c.value)

/** The ids the page showing now gave a text, slider or button, in that order. */
export function suppliedIds() {
  const T = S.viewerTools
  if (!T) return []
  return [...new Set([...T.labels.map((l) => l.id), ...(T.slider ? [T.slider.id] : []), ...T.buttons.map((b) => b.id)])]
}

/** Whether the page showing now gave anything under this declared tool's id. */
export function supplied(tool) {
  const T = S.viewerTools
  return !!T && T.plugin === tool.plugin && suppliedIds().includes(tool.id)
}

// Pictures decoded ahead of time. A decoded element can go straight onto the screen in the same frame,
// which is what makes stepping to the next item instant: the items on both sides, most recent kept.
import { ensureItems } from '../library/results.js'
import { S } from '../app/state.js'
import { fitBox, imageUrlFor, kindOf, neededPixels } from './index.js'

const KEEP = 8
const pool = new Map()

/** Load and decode a picture; asking again returns the same one. */
export function loadImage(url) {
  const known = pool.get(url)
  if (known) {
    // most recently used last
    pool.delete(url)
    pool.set(url, known)
    return known.ready
  }
  const img = new Image()
  img.draggable = false
  img.decoding = 'async'
  const entry = { img, done: false, ready: null }
  entry.ready = (async () => {
    img.src = url
    await img.decode()
    entry.done = true
    return img
  })()
  entry.ready.catch(() => pool.delete(url))
  pool.set(url, entry)
  while (pool.size > KEEP) pool.delete(pool.keys().next().value)
  return entry.ready
}

/** Already decoded? Then it can be shown without waiting. */
export function readyImage(url) {
  const e = pool.get(url)
  return e?.done ? e.img : null
}

// The next items matter most: people mostly go forward.
const AHEAD = [1, 2, -1]

/** Get the neighbours of an open item ready for an area of this size: in the results, or in another list, such as the files of an item. */
export function preloadAround(index, areaW, areaH, ids = S.results.ids) {
  const near = AHEAD.map((d) => ids[index + d]).filter((id) => id != null)
  ensureItems(near)
  for (const id of near) {
    const item = S.items.get(id)
    if (!item || kindOf(item) !== 'image') continue
    const box = fitBox(item, areaW, areaH)
    if (box) loadImage(imageUrlFor(item, neededPixels(box))).catch(() => {})
  }
}

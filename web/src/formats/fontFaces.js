// Fonts of the library loaded into the window for the list's font samples (a plugin's `columns` with `render: "font"`): each item's file once, kept for the rows last shown.
import { originalUrl } from '../platform/fileUrls.js'

const KEEP = 200
const AT_ONCE = 4
const faces = new Map()
const queue = []
let running = 0
let serial = 0

function pump() {
  while (running < AT_ONCE && queue.length) {
    const job = queue.shift()
    running++
    job().finally(() => {
      running--
      pump()
    })
  }
}

async function load(id, family) {
  const r = await fetch(originalUrl(id))
  if (!r.ok) return null
  const face = new FontFace(family, await r.arrayBuffer())
  await face.load()
  document.fonts.add(face)
  return face
}

/** The family name an item's font is loaded under, or null when the web view can't load it; `key` changes with the file. */
export function fontFamily(id, key) {
  const k = `${id}:${key}`
  let e = faces.get(k)
  if (e) {
    // the most recently used last, so the oldest go first
    faces.delete(k)
    faces.set(k, e)
    return e.promise
  }
  const family = `curio-item-${++serial}`
  e = { promise: null, face: null }
  const entry = e
  e.promise = new Promise((resolve) => {
    queue.push(() =>
      load(id, family).then(
        (f) => {
          entry.face = f
          resolve(f ? family : null)
        },
        () => resolve(null),
      ),
    )
  })
  faces.set(k, e)
  pump()
  for (const [old, x] of faces) {
    if (faces.size <= KEEP) break
    faces.delete(old)
    x.promise.then(() => x.face && document.fonts.delete(x.face))
  }
  return e.promise
}

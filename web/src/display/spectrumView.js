import { markRaw, reactive } from 'vue'

// What the Spectrum view shows and which part of it is on screen, shared by the view and its map in the Inspector.
// `x`, `y` are the middle of the screen and `w`, `h` its size, in the layout's units (a row's height); they outlive the view while an item is open in the viewer, so closing it returns to the same place.
// `order` and `posOf` hold the disc's row-by-row reading order (result indices) so the viewer steps through items in Spectrum's order.
// `missing` counts the items it leaves out for having no palette.
export const spectrumView = reactive({ layout: null, rgb: null, x: 0, y: 0, w: 0, h: 0, key: '', missing: 0, order: null, posOf: null, results: null, points: null, pointOf: null, indexOf: null, resultsVer: -1 })

/** The geometric middle of a laid-out disc, in layout units. */
export function spectrumCenter(A) {
  return A?.n || A?.folders?.count ? [(A.min[0] + A.max[0]) / 2, (A.min[1] + A.max[1]) / 2] : [0, 0]
}

function spectrumSequence(A, indexOf, results) {
  const n = A?.n || 0
  const posOf = new Int32Array(results?.ids?.length || 0).fill(-1)
  if (!n || !indexOf?.length || !results?.ids?.length) {
    return { order: new Int32Array(0), posOf, results: { ids: new Uint32Array(0), aspects: new Float32Array(0) } }
  }
  const order = new Int32Array(n)
  const ids = new Uint32Array(n)
  const aspects = new Float32Array(n)
  let k = 0
  for (let y = A.min[1]; y <= A.max[1]; y++) {
    const r = A.rows.get(y)
    if (!r) continue
    for (const j of r.members) {
      const i = indexOf[j]
      order[k] = i
      ids[k] = results.ids[i]
      aspects[k] = results.aspects[i]
      posOf[i] = k
      k++
    }
  }
  return { order: order.subarray(0, k), posOf, results: { ids: ids.subarray(0, k), aspects: aspects.subarray(0, k) } }
}

/** A new layout: the same search keeps its place while returning from the viewer; a new search or a switch into Spectrum starts in the middle. */
export function showSpectrum(layout, rgb, key, meta = null) {
  const hasAny = (A) => !!(A?.n || A?.folders?.count)
  const wasEmpty = !hasAny(spectrumView.layout)
  spectrumView.layout = markRaw(layout)
  spectrumView.rgb = markRaw(rgb)
  if (meta) {
    const seq = spectrumSequence(layout, meta.indexOf, meta.results)
    spectrumView.order = markRaw(seq.order)
    spectrumView.posOf = markRaw(seq.posOf)
    spectrumView.results = markRaw(seq.results)
    spectrumView.points = markRaw(meta.points)
    spectrumView.pointOf = markRaw(meta.pointOf)
    spectrumView.indexOf = markRaw(meta.indexOf)
    spectrumView.resultsVer = meta.resultsVer
  }
  if (key !== spectrumView.key || (wasEmpty && hasAny(layout))) {
    const [cx, cy] = spectrumCenter(layout)
    spectrumView.x = cx
    spectrumView.y = cy
  }
  spectrumView.key = key
}

/** Clears the saved place when leaving Spectrum for another layout. */
export function resetSpectrum() {
  spectrumView.layout = null
  spectrumView.rgb = null
  spectrumView.key = ''
  spectrumView.x = 0
  spectrumView.y = 0
  spectrumView.missing = 0
  spectrumView.order = null
  spectrumView.posOf = null
  spectrumView.results = null
  spectrumView.points = null
  spectrumView.pointOf = null
  spectrumView.indexOf = null
  spectrumView.resultsVer = -1
}

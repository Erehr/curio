// The Spectrum view: every picture on one disc, its direction its hue (red to the right, as on a colour wheel) and its distance how dark it is, the lightest in the middle.
// Hues with many pictures take a wider angle and those with few a narrower one (evenHues), in the wheel's order, so the disc comes out round rather than in arms.
// Each stretch of hue is a wedge whose area holds its pictures, filled from its lightest at the centre to its darkest at the rim: distance is rank, not lightness itself.
// Pictures keep their shape: they stand in rows one unit high, each as wide as its aspect, go to the free stretch of a row nearest their place, and each row then closes up.

const RECORD = 12
const SECTORS = 36
const WIDEST = 4
const NARROWEST = 0.25

/** The packed points of `spectrum_colors`, with the place of each in the results; points of items no longer listed are left out. */
export function readSpectrum(buf, indexOf) {
  const ab = buf instanceof ArrayBuffer ? buf : new Uint8Array(buf).buffer
  const view = new DataView(ab)
  const n = Math.floor(ab.byteLength / RECORD)
  const index = new Int32Array(n)
  const hue = new Float32Array(n)
  const chroma = new Float32Array(n)
  const light = new Float32Array(n)
  const rgb = new Uint8Array(n * 3)
  let m = 0
  for (let k = 0; k < n; k++) {
    const o = k * RECORD
    const i = indexOf(view.getUint32(o, true))
    if (i == null || i < 0) continue
    index[m] = i
    hue[m] = view.getUint16(o + 4, true) / 10
    chroma[m] = view.getUint8(o + 6)
    light[m] = view.getUint8(o + 7) / 2.55
    rgb.set([view.getUint8(o + 8), view.getUint8(o + 9), view.getUint8(o + 10)], m * 3)
    m++
  }
  return { n: m, index: index.subarray(0, m), hue: hue.subarray(0, m), chroma: chroma.subarray(0, m), light: light.subarray(0, m), rgb: rgb.subarray(0, m * 3) }
}

// the first of `starts` (sorted) at or after `x`
function after(starts, x) {
  let lo = 0
  let hi = starts.length
  while (lo < hi) {
    const mid = (lo + hi) >> 1
    if (starts[mid] < x) lo = mid + 1
    else hi = mid
  }
  return lo
}

// where in a row a stretch `len` long fits nearest to starting at `s`: [position, distance], or null when none is nearer than `limit`
function nearestFree(row, s, len, limit) {
  const { starts, ends } = row
  let best = null
  let bestD = limit
  const k = after(starts, s)
  // the gaps to the right of `s`, then to its left, each bounded by the stretches either side
  for (let g = k; g <= starts.length; g++) {
    const lo = g > 0 ? ends[g - 1] : -Infinity
    const hi = g < starts.length ? starts[g] : Infinity
    if (lo - s >= bestD) break
    if (hi - lo >= len) {
      const pos = Math.min(Math.max(s, lo), hi - len)
      const d = Math.abs(pos - s)
      if (d < bestD) ((bestD = d), (best = pos))
    }
  }
  for (let g = k - 1; g >= 0; g--) {
    const lo = g > 0 ? ends[g - 1] : -Infinity
    const hi = starts[g]
    if (s + len - hi >= bestD) break
    if (hi - lo >= len) {
      const pos = Math.min(Math.max(s, lo), hi - len)
      const d = Math.abs(pos - s)
      if (d < bestD) ((bestD = d), (best = pos))
    }
  }
  return best == null ? null : [best, bestD]
}

/**
 * The hues turned so that each stretch of the wheel holds about as many pictures: a hue with many takes a wider angle, one with few a narrower, in the wheel's order and with red still at 0.
 * `even` 0 leaves them as they are, 1 spreads them evenly; between, a share of each.
 */
export function evenHues(hue, even) {
  const n = hue.length
  if (!even || !n) return hue
  const order = Int32Array.from({ length: n }, (_, j) => j).sort((a, b) => hue[a] - hue[b])
  const out = new Float32Array(n)
  order.forEach((j, k) => (out[j] = (1 - even) * hue[j] + (even * 360 * (k + 0.5)) / n))
  return out
}

// a contiguous stretch of a row's pictures side by side, clamped to [minStart, maxEnd] so rows split by the center folder island never cross its keep-out zone
function closeSegment(r, from, to, len, x0, minStart = -Infinity, maxEnd = Infinity) {
  const count = to - from
  if (count <= 0) return
  let run = 0
  let sum = 0
  for (let k = from; k < to; k++) {
    sum += x0[k] - run
    run += len[r.members[k]]
  }
  let at = sum / count
  if (at + run > maxEnd) at = maxEnd - run
  if (at < minStart) at = minStart
  for (let k = from; k < to; k++) {
    r.starts[k] = at
    at += len[r.members[k]]
    r.ends[k] = at
  }
}

// a row's pictures side by side with no space between, preserving any reserved center blocker (member === -1)
function closeUp(r, len, x0) {
  const sIdx = r.members.indexOf(-1)
  if (sIdx < 0) return closeSegment(r, 0, r.members.length, len, x0)
  const bLo = r.starts[sIdx]
  const bHi = r.ends[sIdx]
  closeSegment(r, 0, sIdx, len, x0, -Infinity, bLo)
  closeSegment(r, sIdx + 1, r.members.length, len, x0, bHi, Infinity)
}

const FOLDER_ASPECT = 4 / 3

/** Centered grid of `count` 4:3 folder tiles around (0, 0), arranged by cover colour when given, plus keep-out bounds per affected row. */
function layoutCenterFolders(count, gap, margin, folderColors, dark) {
  if (!count) return null
  const fw = FOLDER_ASPECT * (1 - gap)
  const fstep = fw + gap
  const cols = Math.max(1, Math.ceil(Math.sqrt(count / fstep)))
  const rows = Math.ceil(count / cols)
  const rowStart = -Math.floor(rows / 2)
  const rowEnd = rowStart + rows - 1
  const cy = rowStart + (rows - 1) / 2
  const slotX = new Float32Array(count)
  const slotRow = new Int32Array(count)
  const fullW = Math.min(count, cols) * fstep - gap
  const halfW = fullW / 2
  for (let i = 0; i < count; i++) {
    const r = Math.floor(i / cols)
    const c = i % cols
    const inRow = r === rows - 1 ? count - r * cols : cols
    const rowW = inRow * fstep - gap
    slotX[i] = -rowW / 2 + c * fstep
    slotRow[i] = rowStart + r
  }
  let x = slotX
  let row = slotRow
  if (count > 1 && folderColors?.length === count) {
    // match each folder's hue and lightness to the same polar orientation as the surrounding spectrum disc
    const colored = []
    for (let i = 0; i < count; i++) {
      const col = folderColors[i]
      if (col && col[1] >= 6) colored.push(i)
    }
    colored.sort((a, b) => (dark ? folderColors[a][2] - folderColors[b][2] : folderColors[b][2] - folderColors[a][2]))
    const rankOf = new Map(colored.map((i, k) => [i, (k + 0.5) / colored.length]))
    const rx = Math.max(0.5, halfW)
    const ry = Math.max(0.5, rows / 2)
    const tx = new Float32Array(count)
    const ty = new Float32Array(count)
    for (let i = 0; i < count; i++) {
      const col = folderColors[i]
      if (col && col[1] >= 6) {
        const a = (col[0] * Math.PI) / 180
        const rad = 0.35 + 0.65 * (rankOf.get(i) ?? 0.5)
        tx[i] = rad * rx * Math.cos(a)
        ty[i] = cy - rad * ry * Math.sin(a)
      } else {
        tx[i] = 0
        ty[i] = cy
      }
    }
    const scx = new Float32Array(count)
    for (let s = 0; s < count; s++) scx[s] = slotX[s] + fw / 2
    const assign = Int32Array.from({ length: count }, (_, i) => i)
    const cost = (i, s) => (scx[s] - tx[i]) ** 2 + (slotRow[s] - ty[i]) ** 2
    for (let pass = 0; pass < 32; pass++) {
      let swapped = false
      for (let a = 0; a < count; a++) {
        for (let b = a + 1; b < count; b++) {
          const sa = assign[a]
          const sb = assign[b]
          if (cost(a, sb) + cost(b, sa) + 1e-6 < cost(a, sa) + cost(b, sb)) {
            assign[a] = sb
            assign[b] = sa
            swapped = true
          }
        }
      }
      if (!swapped) break
    }
    x = new Float32Array(count)
    row = new Int32Array(count)
    for (let i = 0; i < count; i++) {
      x[i] = slotX[assign[i]]
      row[i] = slotRow[assign[i]]
    }
  }
  const blocked = new Map()
  for (let y = rowStart; y <= rowEnd; y++) blocked.set(y, [-halfW - margin, halfW + margin])
  // rows directly above and below the folder cluster keep a rounded keep-out so items sit a clear gap away vertically too
  const cornerHalf = halfW + margin * 0.45
  blocked.set(rowStart - 1, [-cornerHalf, cornerHalf])
  blocked.set(rowEnd + 1, [-cornerHalf, cornerHalf])
  const innerR = Math.sqrt(((fullW + margin * 2) * (rows + 2)) / Math.PI)
  return { count, x, row, w: fw, rows, cols, rowStart, rowEnd, halfW, cy, blocked, innerR }
}

/**
 * The disc laid out: per point its left `x` and width `w` (units of a row's height) and its `row` (y, down), with the rows for finding what is where.
 * `aspect(j)` is a point's width over height; `gap` the space between pictures, as a share of a row.
 * `even` turns crowded hues wider (evenHues) so the disc comes out round; `snap` closes each row up, so pictures move a little off their place and leave no gaps; `dark` puts the darkest in the middle instead.
 * When `folders > 0`, places a compact centered grid of 4:3 folder tiles at the middle surrounded by a `folderGap`-unit breathing ring.
 */
export function placeSpectrum(points, aspect, { gap = 0.06, even = 1, snap = true, dark = false, folders = 0, folderColors = null, folderGap = 0.75 } = {}) {
  const n = points.n
  const island = layoutCenterFolders(folders, gap, folderGap, folderColors, dark)
  const hue = evenHues(points.hue, even)
  const len = new Float32Array(n)
  for (let j = 0; j < n; j++) len[j] = Math.min(WIDEST, Math.max(NARROWEST, aspect(j) || 1)) * (1 - gap) + gap
  const bySector = Array.from({ length: SECTORS }, () => [])
  for (let j = 0; j < n; j++) bySector[Math.floor(hue[j] / (360 / SECTORS)) % SECTORS].push(j)
  const wedge = (2 * Math.PI) / SECTORS
  const baseArea = island ? 0.5 * wedge * island.innerR * island.innerR : 0
  const cy = island ? island.cy : 0
  const tx = new Float32Array(n)
  const ty = new Float32Array(n)
  const tr = new Float32Array(n)
  for (const list of bySector) {
    list.sort((a, b) => (dark ? points.light[a] - points.light[b] : points.light[b] - points.light[a]))
    // the area before a picture puts it at the radius whose wedge holds that much; outside any center folder island
    let area = baseArea
    for (const j of list) {
      const r = Math.sqrt((2 * area) / wedge)
      const a = (hue[j] * Math.PI) / 180
      tx[j] = r * Math.cos(a)
      ty[j] = cy - r * Math.sin(a)
      tr[j] = r
      area += len[j]
    }
  }
  const order = Int32Array.from({ length: n }, (_, j) => j).sort((a, b) => tr[a] - tr[b])
  const x = new Float32Array(n)
  const w = new Float32Array(n)
  const row = new Int32Array(n)
  const rows = new Map()
  const rowOf = (y) => rows.get(y) || (rows.set(y, { starts: [], ends: [], members: [] }), rows.get(y))
  if (island) {
    for (const [y, [bLo, bHi]] of island.blocked) {
      const r = rowOf(y)
      r.starts.push(bLo)
      r.ends.push(bHi)
      r.members.push(-1)
    }
  }
  const min = island ? [-island.halfW, island.rowStart] : [0, 0]
  const max = island ? [island.halfW, island.rowEnd] : [0, 0]
  for (const j of order) {
    const s = tx[j] - len[j] / 2
    const y0 = Math.round(ty[j])
    let best = null
    // the rows above and below, while a row away costs less than the best stretch found
    for (let d = 0; best == null || d * d < best[1] ** 2 + 1e-9; d++) {
      for (const y of d ? [y0 + d, y0 - d] : [y0]) {
        const limit = best ? Math.sqrt(Math.max(0, best[1] ** 2 - d * d)) : Infinity
        const f = nearestFree(rowOf(y), s, len[j], limit)
        if (f && (!best || f[1] ** 2 + d * d < best[1] ** 2)) best = [y, Math.sqrt(f[1] ** 2 + d * d), f[0]]
      }
    }
    const [y, , pos] = best
    const r = rowOf(y)
    const k = after(r.starts, pos)
    r.starts.splice(k, 0, pos)
    r.ends.splice(k, 0, pos + len[j])
    r.members.splice(k, 0, j)
    row[j] = y
  }
  for (const r of rows.values()) {
    if (snap && r.members.some((m) => m >= 0)) closeUp(r, len, r.starts.slice())
    const sIdx = r.members.indexOf(-1)
    if (sIdx >= 0) {
      r.starts.splice(sIdx, 1)
      r.ends.splice(sIdx, 1)
      r.members.splice(sIdx, 1)
    }
    r.members.forEach((j, k) => {
      x[j] = r.starts[k] + gap / 2
      w[j] = len[j] - gap
      min[0] = Math.min(min[0], r.starts[k])
      max[0] = Math.max(max[0], r.ends[k])
      min[1] = Math.min(min[1], row[j])
      max[1] = Math.max(max[1], row[j])
    })
  }
  return { n, x, w, row, rows, gap, min, max, folders: island ? { count: island.count, x: island.x, row: island.row, w: island.w, rowStart: island.rowStart, rowEnd: island.rowEnd, halfW: island.halfW } : null }
}

/** Subfolder indices intersecting [x0, x1] × [y0, y1] (units). */
export function foldersIn(A, x0, y0, x1, y1) {
  const F = A?.folders
  if (!F || !F.count || x1 < -F.halfW || x0 > F.halfW || y1 < F.rowStart - 0.5 || y0 > F.rowEnd + 0.5) return []
  const out = []
  for (let i = 0; i < F.count; i++) {
    const ry = F.row[i]
    if (ry + 0.5 < y0 || ry - 0.5 > y1) continue
    if (F.x[i] <= x1 && F.x[i] + F.w >= x0) out.push(i)
  }
  return out
}

/** The points in the rectangle [x0, x1] × [y0, y1] (units, rows by their centre line). */
export function pointsIn(A, x0, y0, x1, y1) {
  const out = []
  for (let y = Math.ceil(y0 - 0.5); y <= Math.floor(y1 + 0.5); y++) {
    const r = A.rows.get(y)
    if (!r) continue
    // a picture is at most as wide as the widest, so one starting that far left may still reach in
    for (let k = Math.max(0, after(r.starts, x0 - WIDEST - 1)); k < r.starts.length && r.starts[k] <= x1; k++) {
      const j = r.members[k]
      if (A.x[j] <= x1 && A.x[j] + A.w[j] >= x0) out.push(j)
    }
  }
  return out
}

/** The point at (x, y), units. */
export function pointAt(A, x, y) {
  const r = A.rows.get(Math.round(y))
  if (!r || Math.abs(y - Math.round(y)) > (1 - A.gap) / 2) return null
  const k = after(r.starts, x + 1e-9) - 1
  if (k < 0) return null
  const j = r.members[k]
  return x >= A.x[j] && x <= A.x[j] + A.w[j] ? j : null
}

/** The point a step from `j` lands on: beside it in its row, or in the nearest row that way the one nearest its middle. */
export function stepSpectrum(A, j, dx, dy) {
  const r = A.rows.get(A.row[j])
  if (dx) {
    const k = r.members.indexOf(j) + dx
    return k >= 0 && k < r.members.length ? r.members[k] : null
  }
  const mid = A.x[j] + A.w[j] / 2
  for (let y = A.row[j] + dy; y >= A.min[1] && y <= A.max[1]; y += dy) {
    const o = A.rows.get(y)
    if (!o?.members.length) continue
    let best = null
    let bestD = Infinity
    for (const m of o.members) {
      const d = Math.abs(A.x[m] + A.w[m] / 2 - mid)
      if (d < bestD) ((bestD = d), (best = m))
    }
    return best
  }
  return null
}

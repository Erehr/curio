// Grid layout engine: positions come from aspect ratios only, are computed once per
// width/size/result change, and visible cells are found by binary search, so only
// on-screen items are rendered even for a million results.

// space between thumbnails before Settings › Appearance › Spacing scales it
export const GRID_GAP = 8

// subfolder tiles keep a 4:3 cover face across Grid, Masonry and Justified
export const FOLDER_ASPECT = 4 / 3

const clampAspect = (a) => (Number.isFinite(a) && a > 0.05 ? Math.min(5, Math.max(0.2, a)) : 1)

/** Virtualised layout for the top Subfolders section: 4:3 tiles scaling with `size` in tile views, or rows of `rowH` in List view. */
export function computeFolderSection(mode, count, width, size, gap, rowH, captionH, headH, collapsed, sectionGap) {
  width = Math.max(50, width)
  if (!count) return { headerY: 0, headerH: 0, bodyY: 0, endY: 0, height: 0, cols: 1, stride: 0, count: 0, collapsed: !!collapsed, x: new Float32Array(0), y: new Float32Array(0), w: new Float32Array(0), h: new Float32Array(0), faceH: 0 }
  const headerY = 0
  const bodyY = headH
  if (collapsed) return { headerY, headerH: headH, bodyY, endY: headH, height: headH + sectionGap, cols: 1, stride: 0, count, collapsed: true, x: new Float32Array(0), y: new Float32Array(0), w: new Float32Array(0), h: new Float32Array(0), faceH: 0 }
  const x = new Float32Array(count)
  const y = new Float32Array(count)
  const w = new Float32Array(count)
  const h = new Float32Array(count)
  if (mode === 'list') {
    for (let i = 0; i < count; i++) {
      x[i] = 0
      y[i] = bodyY + i * rowH
      w[i] = width
      h[i] = rowH
    }
    const endY = bodyY + count * rowH
    return { headerY, headerH: headH, bodyY, endY, height: endY + sectionGap, cols: 1, stride: rowH, count, collapsed: false, x, y, w, h, faceH: 0 }
  }
  const cols = Math.max(1, Math.floor((width + gap) / (size + gap)))
  const colW = (width - gap * (cols - 1)) / cols
  const faceH = Math.round(colW / FOLDER_ASPECT)
  const cardH = faceH + captionH
  const stride = cardH + gap
  for (let i = 0; i < count; i++) {
    const r = Math.floor(i / cols)
    const c = i % cols
    x[i] = c * (colW + gap)
    y[i] = bodyY + r * stride
    w[i] = colW
    h[i] = cardH
  }
  const rows = Math.ceil(count / cols)
  const endY = bodyY + rows * cardH + Math.max(0, rows - 1) * gap
  return { headerY, headerH: headH, bodyY, endY, height: endY + sectionGap, cols, stride, count, collapsed: false, x, y, w, h, faceH }
}

/** O(1) visible indices inside a uniform Subfolders section. */
export function visibleFolderIndices(F, lo, hi) {
  if (!F || !F.count || F.collapsed || F.stride <= 0 || hi < F.bodyY || lo > F.endY) return []
  const maxRow = Math.ceil(F.count / F.cols) - 1
  const r0 = Math.max(0, Math.floor((lo - F.bodyY) / F.stride))
  const r1 = Math.min(maxRow, Math.floor((hi - F.bodyY) / F.stride))
  if (r1 < r0) return []
  const from = r0 * F.cols
  const to = Math.min(F.count - 1, (r1 + 1) * F.cols - 1)
  const out = []
  for (let i = from; i <= to; i++) out.push(i)
  return out
}

// `noted` (one byte per item, or null) adds `noteH` to the cards with a note, and to every card of their row where a row keeps one height; `below` is what each card has under its picture.
// When `secOpts` (`{ groups, collapsed, headerH, sectionGap, collapsedGap }`) is given, items are laid out per collapsible section with Masonry columns and Justified rows resetting cleanly at each header.
export function computeLayout(mode, aspects, width, size, gap, caption, noted = null, noteH = 0, secOpts = null) {
  width = Math.max(50, width)
  const n = aspects.length
  const x = new Float32Array(n)
  const y = new Float32Array(n)
  const w = new Float32Array(n)
  const h = new Float32Array(n)
  let height = 0
  let cols = 1
  let columns = null
  const below = new Float32Array(n)
  const note = (i) => (noted?.[i] ? noteH : 0)
  const rowNote = (from, to) => {
    for (let j = from; j <= to; j++) if (noted?.[j]) return noteH
    return 0
  }
  if (secOpts?.groups) {
    const { groups, collapsed, headerH = 34, sectionGap = 16, collapsedGap = 4 } = secOpts
    const sections = []
    const rowH = mode === 'list' ? Math.max(30, size * 0.22) : 0
    if (mode === 'grid' || mode === 'masonry') cols = Math.max(1, Math.floor((width + gap) / (size + gap)))
    const colW = mode === 'grid' || mode === 'masonry' ? (width - gap * (cols - 1)) / cols : width
    let yy = 0
    for (let sIdx = 0; sIdx < groups.length; sIdx++) {
      const g = groups[sIdx]
      const isCollapsed = !!collapsed?.has(g.key)
      const headerY = yy
      const bodyY = yy + headerH
      let endY = headerY + headerH
      let secCols = null
      const from = g.start
      const to = Math.min(n, g.start + g.count)
      if (isCollapsed || from >= to) {
        yy = endY + collapsedGap
      } else if (mode === 'grid') {
        let sy = bodyY
        for (let start = from; start < to; start += cols) {
          const end = Math.min(to, start + cols) - 1
          const rh = colW + caption + rowNote(start, end)
          for (let i = start; i <= end; i++) {
            x[i] = (i - start) * (colW + gap)
            y[i] = sy
            w[i] = colW
            h[i] = rh
            below[i] = rh - colW
          }
          sy += rh + gap
        }
        endY = sy - gap
        yy = endY + sectionGap
      } else if (mode === 'masonry') {
        const ys = new Float64Array(cols).fill(bodyY)
        const raw = Array.from({ length: cols }, () => [])
        for (let i = from; i < to; i++) {
          let c = 0
          for (let j = 1; j < cols; j++) if (ys[j] < ys[c]) c = j
          const hh = colW / clampAspect(aspects[i]) + caption + note(i)
          x[i] = c * (colW + gap)
          y[i] = ys[c]
          w[i] = colW
          h[i] = hh
          below[i] = caption + note(i)
          raw[c].push(i)
          ys[c] += hh + gap
        }
        let maxY = bodyY
        for (let j = 0; j < cols; j++) if (ys[j] > maxY) maxY = ys[j]
        endY = maxY > bodyY ? maxY - gap : bodyY
        secCols = raw.map((c) => Uint32Array.from(c))
        yy = endY + sectionGap
      } else if (mode === 'justified') {
        let start = from
        let sy = bodyY
        let sum = 0
        for (let i = from; i < to; i++) {
          sum += clampAspect(aspects[i])
          const count = i - start + 1
          const rowW = sum * size + gap * (count - 1)
          if (rowW >= width || i === to - 1) {
            const full = rowW >= width
            const hh = full ? (width - gap * (count - 1)) / sum : size
            const under = caption + rowNote(start, i)
            let xx = 0
            for (let j = start; j <= i; j++) {
              const ww = clampAspect(aspects[j]) * hh
              x[j] = xx
              y[j] = sy
              w[j] = ww
              h[j] = hh + under
              below[j] = under
              xx += ww + gap
            }
            sy += hh + under + gap
            start = i + 1
            sum = 0
          }
        }
        endY = sy - gap
        yy = endY + sectionGap
      } else if (mode === 'list') {
        for (let i = from; i < to; i++) {
          x[i] = 0
          y[i] = bodyY + (i - from) * rowH
          w[i] = width
          h[i] = rowH
        }
        endY = bodyY + (to - from) * rowH
        yy = endY + sectionGap
      }
      sections.push({ index: sIdx, key: g.key, kind: g.kind, val: g.val, start: g.start, count: g.count, collapsed: isCollapsed, headerY, headerH, bodyY, endY, columns: secCols })
    }
    return { mode, x, y, w, h, below, n, height: yy, width, cols, columns: null, sections }
  }
  if (mode === 'grid') {
    cols = Math.max(1, Math.floor((width + gap) / (size + gap)))
    const cell = (width - gap * (cols - 1)) / cols
    let yy = 0
    for (let start = 0; start < n; start += cols) {
      const end = Math.min(n, start + cols) - 1
      const rowH = cell + caption + rowNote(start, end)
      for (let i = start; i <= end; i++) {
        x[i] = (i - start) * (cell + gap)
        y[i] = yy
        w[i] = cell
        h[i] = rowH
        below[i] = rowH - cell
      }
      yy += rowH + gap
    }
    height = yy
  } else if (mode === 'masonry') {
    cols = Math.max(1, Math.floor((width + gap) / (size + gap)))
    const colW = (width - gap * (cols - 1)) / cols
    const ys = new Float64Array(cols)
    columns = Array.from({ length: cols }, () => [])
    for (let i = 0; i < n; i++) {
      let c = 0
      for (let j = 1; j < cols; j++) if (ys[j] < ys[c]) c = j
      const hh = colW / clampAspect(aspects[i]) + caption + note(i)
      x[i] = c * (colW + gap)
      y[i] = ys[c]
      w[i] = colW
      h[i] = hh
      below[i] = caption + note(i)
      columns[c].push(i)
      ys[c] += hh + gap
    }
    for (let j = 0; j < cols; j++) height = Math.max(height, ys[j])
    columns = columns.map((c) => Uint32Array.from(c))
  } else if (mode === 'justified') {
    let start = 0
    let yy = 0
    let sum = 0
    for (let i = 0; i < n; i++) {
      sum += clampAspect(aspects[i])
      const count = i - start + 1
      const rowW = sum * size + gap * (count - 1)
      if (rowW >= width || i === n - 1) {
        const full = rowW >= width
        const hh = full ? (width - gap * (count - 1)) / sum : size
        const under = caption + rowNote(start, i)
        let xx = 0
        for (let j = start; j <= i; j++) {
          const ww = clampAspect(aspects[j]) * hh
          x[j] = xx
          y[j] = yy
          w[j] = ww
          h[j] = hh + under
          below[j] = under
          xx += ww + gap
        }
        yy += hh + under + gap
        start = i + 1
        sum = 0
      }
    }
    height = yy
  } else if (mode === 'list') {
    const rowH = Math.max(30, size * 0.22)
    for (let i = 0; i < n; i++) {
      x[i] = 0
      y[i] = i * rowH
      w[i] = width
      h[i] = rowH
    }
    height = n * rowH
  } else if (mode === 'filmstrip') {
    const hh = Math.min(200, Math.max(60, size))
    let xx = 0
    for (let i = 0; i < n; i++) {
      const ww = Math.min(hh * 2, Math.max(hh * 0.5, hh * clampAspect(aspects[i])))
      x[i] = xx
      y[i] = 0
      w[i] = ww
      h[i] = hh
      xx += ww + gap
    }
    return { mode, x, y, w, h, n, height: hh, width: xx, cols: 1, columns: null, sections: null }
  } else if (mode === 'column') {
    // the filmstrip stood on end: a fixed width, each item as tall as its shape allows
    const ww = Math.max(40, size)
    let yy = 0
    for (let i = 0; i < n; i++) {
      const hh = Math.min(ww * 1.5, Math.max(ww * 0.5, ww / clampAspect(aspects[i])))
      x[i] = 0
      y[i] = yy
      w[i] = ww
      h[i] = hh
      yy += hh + gap
    }
    return { mode, x, y, w, h, n, height: yy, width: ww, cols: 1, columns: null, sections: null }
  }
  return { mode, x, y, w, h, below, n, height, width, cols, columns, sections: null }
}

function lowerBound(n, pred) {
  let lo = 0
  let hi = n
  while (lo < hi) {
    const mid = (lo + hi) >> 1
    if (pred(mid)) lo = mid + 1
    else hi = mid
  }
  return lo
}

/** The section holding result index `i`, or null when ungrouped. */
export function sectionOfIndex(L, i) {
  if (!L?.sections?.length || i == null) return null
  const k = lowerBound(L.sections.length, (m) => L.sections[m].start + L.sections[m].count <= i)
  return k < L.sections.length && i >= L.sections[k].start ? L.sections[k] : null
}

/** Sections whose headers intersect [lo, hi]. */
export function visibleSections(L, lo, hi) {
  const out = []
  if (!L?.sections?.length) return out
  const s = lowerBound(L.sections.length, (k) => L.sections[k].headerY + L.sections[k].headerH < lo)
  for (let k = s; k < L.sections.length; k++) {
    const sec = L.sections[k]
    if (sec.headerY > hi) break
    out.push(sec)
  }
  return out
}

/** Indices of items intersecting [lo, hi] (x range for the filmstrip, y otherwise). Collapsed sections emit nothing. */
export function visibleRange(L, lo, hi) {
  const out = []
  if (!L || !L.n) return out
  if (L.sections) {
    const s0 = lowerBound(L.sections.length, (k) => L.sections[k].endY < lo)
    for (let k = s0; k < L.sections.length; k++) {
      const sec = L.sections[k]
      if (sec.headerY > hi) break
      if (sec.collapsed || !sec.count || sec.endY < lo || sec.bodyY > hi) continue
      if (L.mode === 'masonry') {
        const sub = []
        for (const col of sec.columns) {
          const s = lowerBound(col.length, (m) => L.y[col[m]] + L.h[col[m]] < lo)
          for (let m = s; m < col.length; m++) {
            const i = col[m]
            if (L.y[i] > hi) break
            sub.push(i)
          }
        }
        sub.sort((a, b) => a - b)
        for (let m = 0; m < sub.length; m++) out.push(sub[m])
      } else {
        const s = lowerBound(sec.count, (m) => L.y[sec.start + m] + L.h[sec.start + m] < lo)
        for (let m = s; m < sec.count; m++) {
          const i = sec.start + m
          if (L.y[i] > hi) break
          out.push(i)
        }
      }
    }
    return out
  }
  if (L.mode === 'masonry') {
    for (const col of L.columns) {
      const s = lowerBound(col.length, (k) => L.y[col[k]] + L.h[col[k]] < lo)
      for (let k = s; k < col.length; k++) {
        const i = col[k]
        if (L.y[i] > hi) break
        out.push(i)
      }
    }
    out.sort((a, b) => a - b)
    return out
  }
  const pos = L.mode === 'filmstrip' ? L.x : L.y
  const len = L.mode === 'filmstrip' ? L.w : L.h
  const s = lowerBound(L.n, (i) => pos[i] + len[i] < lo)
  for (let i = s; i < L.n; i++) {
    if (pos[i] > hi) break
    out.push(i)
  }
  return out
}

/** The layout of a result list, with the ids it places: an index from it always names an item of the list it was made for. */
export function layoutFor(results, mode, width, size, gap, caption, noteH = 0, secOpts = null) {
  const noted = noteH && results.notes?.size ? Uint8Array.from(results.ids, (id) => +results.notes.has(id)) : null
  const L = computeLayout(mode, results.aspects, width, size, gap, caption, noted, noteH, secOpts)
  L.ids = results.ids
  return L
}

/** The items intersecting [lo, hi], as `{ i, id }`. */
export function visibleItems(L, lo, hi) {
  return visibleRange(L, lo, hi).map((i) => ({ i, id: L.ids[i] }))
}

function firstOpenSection(sections, from, step) {
  for (let k = from; k >= 0 && k < sections.length; k += step) if (!sections[k].collapsed && sections[k].count > 0) return sections[k]
  return null
}

/** Keyboard navigation target; skips collapsed sections and keeps column alignment across Grid section boundaries. */
export function neighbor(L, from, dx, dy) {
  if (!L || !L.n) return null
  if (L.mode === 'spectrum') return L.step(from, dx, dy)
  if (L.sections) {
    if (from == null || from >= L.n || L.h[from] === 0) return firstOpenSection(L.sections, 0, 1)?.start ?? null
    const sec = sectionOfIndex(L, from)
    if (!sec) return firstOpenSection(L.sections, 0, 1)?.start ?? null
    if (dy === 0 || L.mode === 'list') {
      const step = dy || dx
      const next = from + step
      if (next >= sec.start && next < sec.start + sec.count) return next
      if (step > 0) return firstOpenSection(L.sections, sec.index + 1, 1)?.start ?? null
      const prev = firstOpenSection(L.sections, sec.index - 1, -1)
      return prev ? prev.start + prev.count - 1 : null
    }
    if (L.mode === 'grid') {
      const off = from - sec.start
      const col = off % L.cols
      const row = Math.floor(off / L.cols)
      const totalRows = Math.ceil(sec.count / L.cols)
      if (dy > 0) {
        if (row + 1 < totalRows) return sec.start + Math.min(sec.count - 1, (row + 1) * L.cols + col)
        const ns = firstOpenSection(L.sections, sec.index + 1, 1)
        return ns ? ns.start + Math.min(ns.count - 1, col) : null
      }
      if (row > 0) return sec.start + (row - 1) * L.cols + col
      const ps = firstOpenSection(L.sections, sec.index - 1, -1)
      if (!ps) return null
      const lastRowStart = Math.floor((ps.count - 1) / L.cols) * L.cols
      return ps.start + Math.min(ps.count - 1, lastRowStart + col)
    }
    const cx = L.x[from] + L.w[from] / 2
    const probe = dy > 0 ? L.y[from] + L.h[from] + 1 : L.y[from] - 1
    const span = Math.max(2 * L.h[from], 240)
    const cand = visibleRange(L, Math.max(sec.bodyY, probe - span), Math.min(sec.endY, probe + span))
    let best = null
    let bestD = Infinity
    for (const i of cand) {
      if (i === from || i < sec.start || i >= sec.start + sec.count) continue
      if (dy > 0 ? L.y[i] <= L.y[from] : L.y[i] >= L.y[from]) continue
      const d = Math.abs(L.x[i] + L.w[i] / 2 - cx) + Math.abs(L.y[i] + L.h[i] / 2 - probe)
      if (d < bestD) {
        bestD = d
        best = i
      }
    }
    if (best != null) return best
    const target = firstOpenSection(L.sections, sec.index + (dy > 0 ? 1 : -1), dy > 0 ? 1 : -1)
    if (!target) return null
    const edgeY = dy > 0 ? target.bodyY : target.endY
    const edgeCand = visibleRange(L, Math.max(target.bodyY, edgeY - span), Math.min(target.endY, edgeY + span))
    for (const i of edgeCand) {
      if (i < target.start || i >= target.start + target.count) continue
      const d = Math.abs(L.x[i] + L.w[i] / 2 - cx) + Math.abs((dy > 0 ? L.y[i] : L.y[i] + L.h[i]) - edgeY)
      if (d < bestD) {
        bestD = d
        best = i
      }
    }
    return best ?? (dy > 0 ? target.start : target.start + target.count - 1)
  }
  if (from == null || from >= L.n) return 0
  if (dy === 0 || L.mode === 'list' || L.mode === 'filmstrip' || L.mode === 'column') {
    const next = from + (dy || dx)
    return next >= 0 && next < L.n ? next : null
  }
  if (L.mode === 'grid') {
    const next = from + dy * L.cols
    return next >= 0 && next < L.n ? next : null
  }
  const cx = L.x[from] + L.w[from] / 2
  const probe = dy > 0 ? L.y[from] + L.h[from] + 1 : L.y[from] - 1
  const cand = visibleRange(L, probe - 2 * L.h[from], probe + 2 * L.h[from])
  let best = null
  let bestD = Infinity
  for (const i of cand) {
    if (i === from) continue
    if (dy > 0 ? L.y[i] <= L.y[from] : L.y[i] >= L.y[from]) continue
    const d = Math.abs(L.x[i] + L.w[i] / 2 - cx) + Math.abs(L.y[i] + L.h[i] / 2 - probe)
    if (d < bestD) {
      bestD = d
      best = i
    }
  }
  return best
}

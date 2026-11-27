// Pointer-based drag and drop inside the window: items onto folders, tags, the Trash and plugin pages, and the lists reordered by dragging (folders, libraries, filters, columns, widgets).
// Native HTML5 drag events are reserved by the web view for dropping files from the desktop, which Tauri hands to us with real paths.
import { call } from '../platform/ipc.js'
import { liftCopy, liftLabel, dropSlot, shift, unshiftAll } from './dragLook.js'

// How long the pointer rests on a closed folder before it opens.
const OPEN_ON_HOVER_MS = 600
// The part of a row's height at each end that means "before" or "after" rather than "into".
const EDGE_ZONE = 0.28
// one entry of a list ("folder:12", "lib:3"), as opposed to a whole area
const ROW_TARGET = /^[a-z]+:-?\d+$/
// how often a drag that may leave the window asks whether another window stands over it under the pointer
const COVER_MS = 80
// how far outside a list the pointer may stray and still be placing in it
const LIST_MARGIN = 24
// how long a list keeps the new order on show while the library answers, before it shows what it has
const SETTLE_MS = 1500
// the room made in a list for something from elsewhere: as big as it was, within these
const ROOM_MIN = 24
const ROOM_MAX = 160

let handler = () => {}
let hoverHandler = () => {}
export function onDrop(fn) {
  handler = fn
}
export function onDragHover(fn) {
  hoverHandler = fn
}

/** Where an entry of a list dragged from index `from` ends, from the target its drop names: before that entry, or after it for the list's last. */
export function finalIndex(from, target, zone) {
  const j = Number(target.split(':').at(-1))
  const s = zone === 'after' ? j + 1 : j
  return s > from ? s - 1 : s
}

// the element an entry moves as: itself, or the wrappers it is alone in (a menu's row in its list item)
function itemOf(el, box) {
  let it = el
  while (it.parentElement && it.parentElement !== box && it.parentElement.children.length === 1) it = it.parentElement
  return it
}
// an entry and the rows after it that go with it (`data-drag-with`: a location's open folders)
function blockOf(item) {
  const block = [item]
  for (let n = item.nextElementSibling; n?.hasAttribute('data-drag-with'); n = n.nextElementSibling) block.push(n)
  return block
}
function union(els) {
  let r = null
  for (const el of els) {
    const b = el.getBoundingClientRect()
    if (!b.width && !b.height) continue
    r = r ? { left: Math.min(r.left, b.left), top: Math.min(r.top, b.top), right: Math.max(r.right, b.right), bottom: Math.max(r.bottom, b.bottom) } : { left: b.left, top: b.top, right: b.right, bottom: b.bottom }
  }
  r ||= { left: 0, top: 0, right: 0, bottom: 0 }
  return { ...r, width: r.right - r.left, height: r.bottom - r.top }
}
// a column when each entry starts below the one before, a row when each starts to its right; entries may differ in size and inset
function axisOf(box, rects) {
  if (rects.length < 2) return getComputedStyle(box).flexDirection.startsWith('row') ? 'x' : 'y'
  const follows = (s, e) => rects.every((r, i) => !i || r[s] >= rects[i - 1][e] - 2)
  if (follows('top', 'bottom')) return 'y'
  if (follows('left', 'right')) return 'x'
  return 'wrap'
}
const START = { x: 'left', y: 'top' }
const END = { x: 'right', y: 'bottom' }
const SIZE = { x: 'width', y: 'height' }
const mid = (r, axis) => r[START[axis]] + r[SIZE[axis]] / 2
// how a list that wraps sets its entries: in lines from its first entry's place, each line as tall as its tallest entry, which the others are centred on
function flowOf(items, rects) {
  const box = items[0].parentElement
  const cs = getComputedStyle(box)
  const b = box.getBoundingClientRect()
  const first = rects.filter((r) => r.top < rects[0].bottom)
  return {
    x0: rects[0].left,
    y0: Math.min(...first.map((r) => r.top)),
    left: b.left + parseFloat(cs.borderLeftWidth) + parseFloat(cs.paddingLeft),
    right: b.right - parseFloat(cs.borderRightWidth) - parseFloat(cs.paddingRight),
    gapX: parseFloat(cs.columnGap) || 0,
    gapY: parseFloat(cs.rowGap) || 0,
  }
}
// where entries of these sizes stand when set in that order
function flowPlace(f, sizes) {
  const out = []
  let line = []
  let x = f.x0
  let y = f.y0
  const close = () => {
    const h = Math.max(...line.map((i) => sizes[i].height))
    for (const i of line) out[i].top = y + (h - sizes[i].height) / 2
    y += h + f.gapY
    line = []
  }
  sizes.forEach((s, i) => {
    if (line.length && x + s.width > f.right + 1) {
      close()
      x = f.left
    }
    out[i] = { left: x, top: 0, width: s.width, height: s.height }
    line.push(i)
    x += s.width + f.gapX
  })
  if (line.length) close()
  return out
}

/**
 * Begin tracking a potential drag from a pointerdown event. `payload` is any object; `label` names what is dragged (text, or a function giving it) where nothing is lifted.
 * Drop targets are elements with `data-drop="kind:id"`, taken where `opts.accept(target)` says.
 * The element pressed (`opts.lift` names another, or null for none) is lifted and follows the pointer, with `opts.count` on it when several go.
 * `opts.sort` (true, or a test of the target) makes the targets lists that are reordered: their entries make room where the drop would land, and the drop names the entry it lands before (`zone` 'before'), or the last with 'after'.
 * A list is the targets of one kind (the part before the first colon) under one `[data-drop-list]`, else one parent; one that wraps onto several lines is set again around the room, as it would stand with the entry there.
 * `opts.zones: true` makes them a tree, each row's depth in `data-depth`: near a row's top or bottom edge the drop lands before it or after it and what it holds, with room made there; over its middle, into it.
 * Any other target takes the drop whole (`zone` 'inside'). `opts.describe(target, mods)` says under the lifted element what dropping on the target (or on nothing) will do; `mods.shift` and `mods.ctrl` say whether Shift or Ctrl is held, and the drop handler gets the same, with the pointer's `x` and `y`. Escape puts it back.
 */
export function beginDrag(ev, payload, label, opts = {}) {
  if (ev.button !== 0) return
  const sx = ev.clientX
  const sy = ev.clientY
  const from = opts.lift !== undefined ? opts.lift : ev.currentTarget
  const accept = opts.accept || (() => true)
  const tree = opts.zones === true
  const inList = (t) => (typeof opts.sort === 'function' ? opts.sort(t) : (!!opts.sort || tree) && ROW_TARGET.test(t))
  const own = from?.dataset?.drop && inList(from.dataset.drop) ? from : null
  const count = () => (typeof opts.count === 'function' ? opts.count() : opts.count || 1)
  let started = false
  let ended = false
  let shiftKey = !!ev.shiftKey
  let ctrl = !!(ev.ctrlKey || ev.metaKey)
  let px = sx
  let py = sy
  let look = null
  let slot = null
  let lists = []
  let at = null
  let lit = null
  let hoverTimer = null
  let asking = false
  let askedAt = 0
  let watch = null
  let again = 0
  let gone = []
  const mods = (e) => ({ shift: !!e?.shiftKey, ctrl: !!(e?.ctrlKey || e?.metaKey), x: e?.clientX, y: e?.clientY })

  const say = () => {
    if (opts.describe) look?.say(opts.describe(at?.el.dataset.drop || null, { shift: shiftKey, ctrl }))
  }
  // true when the held modifiers changed
  const track = (e) => {
    const c = !!(e.ctrlKey || e.metaKey)
    if (e.shiftKey === shiftKey && c === ctrl) return false
    shiftKey = e.shiftKey
    ctrl = c
    return true
  }

  // ---------------------------------------------------------------- the lists on show
  function measure(box, els) {
    const items = els.map((el) => itemOf(el, box))
    const blocks = items.map(blockOf)
    const rects = blocks.map(union)
    const axis = axisOf(box, rects)
    const self = els.indexOf(own)
    const gap = rects.length > 1 && axis !== 'wrap' ? Math.max(0, rects[1][START[axis]] - rects[0][END[axis]]) : 0
    const tight = union(blocks.flat())
    const bounds = { left: tight.left - LIST_MARGIN, right: tight.right + LIST_MARGIN, top: tight.top - LIST_MARGIN, bottom: tight.bottom + LIST_MARGIN }
    const depth = els.map((el) => Number(el.dataset.depth || 0))
    const pad = els.map((el) => parseFloat(getComputedStyle(el).paddingLeft) || 0)
    const l = { box, els, items, blocks, rects, axis, self, gap, tight, bounds, depth, pad, ok: els.map((el) => el !== own && accept(el.dataset.drop)) }
    l.tree = tree && axis === 'y'
    // a tree's rows not in a column (a folder's subfolder cards) are each a drop into it
    l.into = tree && axis !== 'y'
    l.others = els.map((_, k) => k).filter((k) => k !== self)
    if (axis === 'wrap' && !l.into) l.flow = flowOf(items, rects)
    if (l.tree && self >= 0) {
      l.ownEnd = subtreeEnd(l, self)
      l.ownSize = (l.ownEnd + 1 < els.length ? rects[l.ownEnd + 1].top : rects[l.ownEnd].bottom + gap) - rects[self].top
    }
    return l
  }
  function subtreeEnd(l, k) {
    let j = k + 1
    while (j < l.els.length && l.depth[j] > l.depth[k]) j++
    return j - 1
  }
  function scan() {
    unshiftAll(false)
    // a list is the targets of one kind under one box: a zone holds folders, locations and libraries apart
    const groups = new Map()
    for (const el of document.querySelectorAll('[data-drop]')) {
      if (el !== own && !inList(el.dataset.drop)) continue
      if (!el.getClientRects().length) continue
      const box = el.closest('[data-drop-list]') || el.parentElement
      const kind = el.dataset.drop.split(':')[0]
      if (!groups.has(box)) groups.set(box, new Map())
      const kinds = groups.get(box)
      if (!kinds.has(kind)) kinds.set(kind, [])
      kinds.get(kind).push(el)
    }
    lists = [...groups].flatMap(([box, kinds]) => [...kinds.values()].map((els) => [box, els])).filter(([, els]) => els.some((el) => el === own || accept(el.dataset.drop))).map(([box, els]) => measure(box, els))
    // a dragged branch leaves its place with all it holds
    for (const el of gone) el.classList.remove('drag-source')
    gone = []
    const mine = lists.find((l) => l.self >= 0)
    if (mine?.tree) for (let k = mine.self; k <= mine.ownEnd; k++) gone.push(mine.items[k])
    for (const el of gone) el.classList.add('drag-source')
    watch?.disconnect()
    watch = new MutationObserver(scanSoon)
    for (const l of lists) watch.observe(l.box, { childList: true, subtree: true })
  }
  // after a folder opened under the pointer or the list scrolled
  function scanSoon() {
    if (again || ended) return
    again = requestAnimationFrame(() => {
      again = 0
      if (ended) return
      scan()
      at = pick(px, py)
      render()
    })
  }

  // ---------------------------------------------------------------- where a drop lands
  const contains = (r, x, y) => x >= r.left && x <= r.right && y >= r.top && y <= r.bottom
  // a list held inside another's entry (a row of widgets in a zone) takes the drop over its holder
  const innermost = (ls) => ls.find((a) => !ls.some((b) => b.box !== a.box && a.box.contains(b.box)))
  function pick(x, y) {
    const hit = document.elementFromPoint(x, y)?.closest('[data-drop]')
    if (hit && !inList(hit.dataset.drop)) return accept(hit.dataset.drop) ? { el: hit, zone: 'inside' } : null
    const l = innermost(lists.filter((x2) => contains(x2.tight, x, y))) || innermost(lists.filter((x2) => contains(x2.bounds, x, y)))
    if (!l) return null
    if (l.into) {
      const k = l.rects.findIndex((r) => contains(r, x, y))
      return k >= 0 && l.ok[k] ? { el: l.els[k], zone: 'inside' } : null
    }
    if (l.tree) return pickTree(l, y)
    if (l.axis === 'wrap') return pickWrap(l, x, y)
    return pickLine(l, x, y)
  }
  function pickLine(l, x, y) {
    const p = l.axis === 'y' ? y : x
    const others = l.els.map((_, k) => k).filter((k) => k !== l.self)
    if (!others.length) return null
    let b = 0
    for (const k of others) if (p > mid(l.rects[k], l.axis)) b++
    const k = others[Math.min(b, others.length - 1)]
    if (!l.ok[k]) return null
    return { l, el: l.els[k], zone: b < others.length ? 'before' : 'after', b, others, noop: l.self >= 0 && b === l.self }
  }
  function pickWrap(l, x, y) {
    const others = l.others
    if (!others.length) return null
    // against the list as it stands without the dragged entry, so the room it makes doesn't move what is under the pointer
    l.base ||= flowPlace(l.flow, others.map((k) => l.rects[k]))
    let j = 0
    let dist = Infinity
    l.base.forEach((r, i) => {
      const d = Math.hypot(Math.max(r.left - x, 0, x - r.left - r.width), Math.max(r.top - y, 0, y - r.top - r.height))
      if (d < dist) [j, dist] = [i, d]
    })
    const b = j + (x > l.base[j].left + l.base[j].width / 2 ? 1 : 0)
    const k = others[Math.min(b, others.length - 1)]
    if (!l.ok[k]) return null
    return { l, el: l.els[k], zone: b < others.length ? 'before' : 'after', b, others, noop: l.self >= 0 && b === l.self }
  }
  // the rows of a tree as they stand while its dragged branch is away
  const inBranch = (l, k) => l.self >= 0 && k >= l.self && k <= l.ownEnd
  const shownTop = (l, k) => l.rects[k].top - (l.self >= 0 && k > l.ownEnd ? l.ownSize : 0)
  function pickTree(l, y) {
    const rows = l.els.map((_, k) => k).filter((k) => !inBranch(l, k))
    if (!rows.length) return null
    let k = rows.find((j) => y >= shownTop(l, j) && y < shownTop(l, j) + l.rects[j].height)
    if (k === undefined) k = y < shownTop(l, rows[0]) ? rows[0] : rows.at(-1)
    if (!l.ok[k]) return null
    const f = (y - shownTop(l, k)) / l.rects[k].height
    const zone = f < EDGE_ZONE ? 'before' : f > 1 - EDGE_ZONE ? 'after' : 'inside'
    // after a row is after all it holds, as its next sibling's place
    const ins = zone === 'after' ? subtreeEnd(l, k) + 1 : k
    const same = l.self >= 0 && l.depth[k] === l.depth[l.self] && (zone === 'before' ? k === l.ownEnd + 1 : zone === 'after' && ins === l.self)
    return { l, el: l.els[k], zone, k, ins, noop: same }
  }

  // ---------------------------------------------------------------- how it looks there
  // its own size in its own list; elsewhere the size the list's entries mostly have
  const sizeOf = (l) => {
    const dim = SIZE[l.axis === 'x' ? 'x' : 'y']
    if (l.self >= 0) return l.rects[l.self][dim] + l.gap
    const sizes = l.rects.map((r) => r[dim]).sort((a, b) => a - b)
    return Math.min(ROOM_MAX, Math.max(ROOM_MIN, sizes[sizes.length >> 1] || 30)) + l.gap
  }
  // the room in a list that wraps: its own size, or the size its entries mostly have
  function flowRoom(l) {
    if (l.self >= 0) return l.rects[l.self]
    const middle = (dim) => l.rects.map((r) => r[dim]).sort((a, b) => a - b)[l.rects.length >> 1]
    return { width: Math.min(ROOM_MAX, Math.max(ROOM_MIN, middle('width'))), height: middle('height') }
  }
  // where each entry of a list that wraps stands with `t` over it, the room at -1; nothing moves while the drag is over nothing
  function flowNow(l, t) {
    let order = null
    if (t?.l === l) order = [...l.others.slice(0, t.b), -1, ...l.others.slice(t.b)]
    else if (t && l.self >= 0) order = l.others
    if (!order) return null
    const room = flowRoom(l)
    const pos = flowPlace(l.flow, order.map((k) => (k < 0 ? room : l.rects[k])))
    return new Map(order.map((k, j) => [k, pos[j]]))
  }
  function moveOf(l, k, t, room) {
    if (l.flow || l.into) return 0
    if (l.tree) {
      if (inBranch(l, k)) return 0
      // over nothing it keeps its place; anywhere else the rows below it close up
      const close = t && l.self >= 0 && k > l.ownEnd ? -l.ownSize : 0
      return close + (t?.l === l && t.zone !== 'inside' && k >= t.ins ? room : 0)
    }
    if (t?.l === l) {
      if (l.self < 0) return k >= t.b ? room : 0
      if (k === l.self) return 0
      const pos = k < l.self ? k : k - 1
      if (k > l.self && pos < t.b) return -room
      if (k < l.self && pos >= t.b) return room
      return 0
    }
    // dragged out to somewhere else: its own list closes up
    if (t && l.self >= 0 && k > l.self) return -room
    return 0
  }
  function render() {
    const t = at
    for (const l of lists) {
      if (l.flow) {
        const spots = flowNow(l, t)
        l.blocks.forEach((block, k) => {
          const p = spots?.get(k)
          for (const el of block) shift(el, p ? p.left - l.rects[k].left : 0, p ? p.top - l.rects[k].top : 0)
        })
        continue
      }
      const room = sizeOf(l)
      l.blocks.forEach((block, k) => {
        const d = moveOf(l, k, t, room)
        for (const el of block) shift(el, l.axis === 'x' ? d : 0, l.axis === 'x' ? 0 : d)
      })
    }
    const s = slotFor(t)
    if (s) slot?.show(s)
    else slot?.hide()
    const light = t && t.zone === 'inside' ? t.el : null
    if (light !== lit) {
      lit?.classList.remove('drop-hover')
      light?.classList.add('drop-hover')
      lit = light
    }
  }
  // the room the drop would take, or where it came from while it is over nothing
  function slotFor(t) {
    if (!t) {
      const l = lists.find((x) => x.self >= 0)
      if (!l) return null
      return l.tree ? union(l.items.slice(l.self, l.ownEnd + 1)) : l.rects[l.self]
    }
    if (!t.l || t.zone === 'inside') return null
    const l = t.l
    if (l.flow) return flowNow(l, t).get(-1)
    const room = sizeOf(l)
    if (l.tree) {
      const n = l.els.length
      const last = l.els.map((_, k) => k).filter((k) => !inBranch(l, k)).at(-1)
      const top = t.ins < n ? shownTop(l, t.ins) : shownTop(l, last) + l.rects[last].height + l.gap
      const r = l.rects[t.k]
      return { left: r.left + l.pad[t.k], top, width: r.width - l.pad[t.k], height: room - l.gap }
    }
    const a = l.axis
    const c = a === 'x' ? 'y' : 'x'
    let start
    if (t.b === 0) start = l.rects[0][START[a]]
    else {
      const prev = t.others[t.b - 1]
      start = l.rects[prev][END[a]] + moveOf(l, prev, t, room) + l.gap
    }
    // as wide as what it lands beside, which in a zone may be inset or full
    const across = l.self >= 0 ? l.rects[l.self] : l.rects[l.els.indexOf(t.el)]
    const out = { [START[a]]: start, [SIZE[a]]: room - l.gap, [START[c]]: across[START[c]], [SIZE[c]]: across[SIZE[c]] }
    return { left: out.left, top: out.top, width: out.width, height: out.height }
  }

  // ---------------------------------------------------------------- start, move, end
  function start() {
    started = true
    const text = opts.describe ? opts.describe(null, { shift: shiftKey, ctrl }) : ''
    const name = () => (typeof label === 'function' ? label() : label)
    // only a drag that just reorders its list carries its copy in place; one that can land inside a target carries it aside
    look = from?.isConnected ? liftCopy(from, sx, sy, { count: count(), text, near: !own || tree }) : liftLabel(text || name(), sx, sy)
    slot = dropSlot(look.radius)
    // keep receiving moves when the pointer leaves the window (drag-out hand-off)
    try { ev.target.setPointerCapture?.(ev.pointerId) } catch { /* not capturable */ }
    document.body.classList.add('dragging')
    scan()
    if (own && !tree) {
      const mine = lists.find((l) => l.self >= 0)
      for (const el of mine ? mine.blocks[mine.self] : [own]) el.classList.add('drag-source')
    }
    window.addEventListener('scroll', scanSoon, true)
    render()
    opts.onStart?.()
  }
  const same = (a, b) => a === b || (!!a && !!b && a.el === b.el && a.zone === b.zone && a.b === b.b && a.ins === b.ins)
  const move = (e) => {
    px = e.clientX
    py = e.clientY
    if (!started) {
      if (Math.abs(px - sx) + Math.abs(py - sy) < 6) return
      start()
    }
    look.move(px, py)
    if (track(e)) say()
    // leaving the window hands the drag to the operating system (files to Explorer etc.)
    if (opts.onLeaveWindow && (px <= 1 || py <= 1 || px >= window.innerWidth - 1 || py >= window.innerHeight - 1)) return leave()
    // so does reaching another window that stands over this one, though the pointer is still within this one's bounds
    if (opts.onLeaveWindow && !asking && performance.now() - askedAt > COVER_MS) {
      asking = true
      askedAt = performance.now()
      call('pointer_over_other_window').then((other) => {
        asking = false
        if (other && !ended && opts.onLeaveWindow) leave()
      }).catch(() => (asking = false))
    }
    const t = pick(px, py)
    if (same(t, at)) return
    at = t
    render()
    say()
    clearTimeout(hoverTimer)
    if (t?.zone === 'inside') hoverTimer = setTimeout(() => hoverHandler(t.el.dataset.drop), OPEN_ON_HOVER_MS)
  }
  const key = (e) => {
    if (e.type === 'keydown' && e.key === 'Escape' && started && !ended) {
      e.preventDefault()
      e.stopPropagation()
      return cancel()
    }
    if (started && track(e)) say()
  }
  const detach = () => {
    ended = true
    clearTimeout(hoverTimer)
    cancelAnimationFrame(again)
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', up)
    window.removeEventListener('keydown', key, true)
    window.removeEventListener('keyup', key, true)
    window.removeEventListener('scroll', scanSoon, true)
    watch?.disconnect()
    document.body.classList.remove('dragging')
  }
  // everything back as it was before the drag
  const finish = () => {
    unshiftAll()
    for (const el of document.querySelectorAll('.drag-source')) el.classList.remove('drag-source')
    lit?.classList.remove('drop-hover')
    slot?.remove()
    look?.remove()
  }
  // back where it came from
  const cancel = () => {
    detach()
    finish()
    opts.onEnd?.()
  }
  const leave = () => {
    const cb = opts.onLeaveWindow
    opts.onLeaveWindow = null
    detach()
    finish()
    opts.onEnd?.()
    cb()
  }
  // the new order stays on show until the list draws it, or a while if it never does
  const settle = (boxes) => {
    let done = false
    const end = () => {
      if (done) return
      done = true
      obs.disconnect()
      clearTimeout(timer)
      finish()
    }
    const obs = new MutationObserver(end)
    for (const b of boxes) if (b?.isConnected) obs.observe(b, { childList: true, subtree: true, characterData: true })
    const timer = setTimeout(end, SETTLE_MS)
  }
  const up = (e) => {
    if (!started) {
      detach()
      return
    }
    detach()
    const t = at
    // swallow the click that follows a drag
    const swallow = (x) => x.stopPropagation()
    window.addEventListener('click', swallow, true)
    setTimeout(() => window.removeEventListener('click', swallow, true), 60)
    if (!t || t.noop) {
      finish()
      opts.onEnd?.()
      return
    }
    const room = slotFor(t)
    const home = lists.find((l) => l.self >= 0)
    const into = !t.l || t.zone === 'inside'
    // watched from before the drop, so a list that redraws at once is caught; until then it shows the new order, or its dragged entry gone
    if (!into || home) settle([t.l?.box, home?.box])
    handler(t.el.dataset.drop, payload, t.el, t.zone, mods(e))
    opts.onEnd?.()
    // the copy stands where the entry lands until the list draws it there
    if (!into && room) look.put(room)
    else look.remove()
    if (into && !home) finish()
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', up)
  window.addEventListener('keydown', key, true)
  window.addEventListener('keyup', key, true)
}

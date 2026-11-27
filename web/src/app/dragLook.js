// How a drag looks: a copy of what is dragged lifted under the pointer, with a count when several go and what the drop will do under it; the entries of a list moved aside to make room where it would land, and that room marked. Nothing slides: every change shows at once.
import { rem } from '../shared/rem.js'

// a bigger element is lifted smaller, so it doesn't hide what it is dragged over
const MAX_W = 320
const MAX_H = 240
// a copy carried onto targets sits this far beside the pointer, so the target under it stays in sight; a row keeps its size (cut to its start there), anything taller fits a box
const NEAR = 14
const NEAR_W = 160
const NEAR_H = 120
const ROW_H = 3.36
const ROW_W = 260
// past this many elements only the outer one keeps its look, so the drag starts at once
const MAX_NODES = 400
// what a copy keeps of each element's computed style: it leaves the ancestors whose rules gave it its look
const KEEP = [
  'display', 'flex-direction', 'flex-wrap', 'flex-grow', 'flex-shrink', 'flex-basis', 'align-items', 'justify-content', 'gap', 'box-sizing', 'width', 'height', 'min-width',
  'margin-top', 'margin-right', 'margin-bottom', 'margin-left', 'padding-top', 'padding-right', 'padding-bottom', 'padding-left',
  'border-top-width', 'border-right-width', 'border-bottom-width', 'border-left-width', 'border-top-style', 'border-right-style', 'border-bottom-style', 'border-left-style',
  'border-top-color', 'border-right-color', 'border-bottom-color', 'border-left-color', 'border-top-left-radius', 'border-top-right-radius', 'border-bottom-left-radius', 'border-bottom-right-radius',
  'background-color', 'background-image', 'background-size', 'background-position', 'color', 'opacity', 'visibility',
  'font-family', 'font-size', 'font-weight', 'font-style', 'line-height', 'letter-spacing', 'text-transform', 'text-align', 'white-space', 'overflow', 'text-overflow', 'vertical-align',
  'object-fit', 'position', 'top', 'right', 'bottom', 'left', 'fill', 'stroke',
]
// what would load or play again in a copy: a plugin's page, a video
const LIVE = 'iframe, video, canvas, object, embed'

function freeze(from, to) {
  const cs = getComputedStyle(from)
  for (const p of KEEP) to.style.setProperty(p, cs.getPropertyValue(p))
  to.style.transition = 'none'
}
function copyOf(src) {
  const copy = src.cloneNode(true)
  const a = [src, ...src.querySelectorAll('*')]
  const b = [copy, ...copy.querySelectorAll('*')]
  if (a.length <= MAX_NODES) a.forEach((el, i) => freeze(el, b[i]))
  else freeze(src, copy)
  for (const el of b) {
    el.removeAttribute('id')
    el.removeAttribute('data-drop')
    el.removeAttribute('data-drop-list')
    el.classList.remove('drop-hover', 'drag-source')
  }
  for (const el of copy.querySelectorAll(LIVE)) {
    const box = document.createElement('div')
    box.style.cssText = el.style.cssText
    box.style.background = 'var(--surface-2)'
    el.replaceWith(box)
  }
  return copy
}
const clear = (color) => !color || color === 'transparent' || /rgba\(.*,\s*0\)$/.test(color)

/** A copy of `src` lifted where it is: held at the point (`x`, `y`) it was pressed, or carried beside the pointer when `near`. */
export function liftCopy(src, x, y, { count = 1, text = '', near = false } = {}) {
  const r = src.getBoundingClientRect()
  const cs = getComputedStyle(src)
  const node = copyOf(src)
  const row = r.height <= rem(ROW_H)
  const w = row && near ? Math.min(r.width, ROW_W) : r.width
  const k = row ? 1 : near ? Math.min(1, NEAR_W / r.width, NEAR_H / r.height) : Math.min(1, MAX_W / r.width, MAX_H / r.height)
  const ox = x - r.left
  const oy = y - r.top
  // where the pointer is on the copy as drawn
  const ax = near ? -NEAR : ox * k
  const ay = near ? -NEAR : oy * k
  node.classList.add('drag-lift')
  Object.assign(node.style, { position: 'fixed', left: r.left + 'px', top: r.top + 'px', width: w + 'px', height: r.height + 'px', margin: '0', transformOrigin: '0 0', visibility: 'visible', opacity: '0.94' })
  if (clear(cs.backgroundColor)) node.style.backgroundColor = 'var(--surface-2)'
  if (!parseFloat(cs.borderTopLeftRadius)) node.style.borderRadius = 'calc(6px * var(--r))'
  if (count > 1) {
    const badge = document.createElement('span')
    badge.className = 'drag-count'
    badge.textContent = count
    // drawn at its own size on a copy drawn smaller
    Object.assign(badge.style, { transformOrigin: 'top right', transform: `scale(${1 / k})` })
    node.append(badge)
  }
  document.body.append(node)
  const cap = text ? document.createElement('div') : null
  if (cap) {
    cap.className = 'drag-ghost'
    cap.textContent = text
    document.body.append(cap)
  }
  let dx = 0
  let dy = 0
  const place = () => {
    const tx = dx + ox - ax
    const ty = dy + oy - ay
    node.style.transform = `translate(${tx}px, ${ty}px) scale(${k})`
    if (cap) {
      cap.style.left = Math.max(4, Math.min(r.left + tx, innerWidth - cap.offsetWidth - 4)) + 'px'
      cap.style.top = r.top + ty + r.height * k + 6 + 'px'
    }
  }
  place()
  return {
    rect: r,
    radius: node.style.borderRadius || cs.borderTopLeftRadius,
    move(px, py) {
      dx = px - x
      dy = py - y
      place()
    },
    say(t) {
      if (cap) cap.textContent = t
    },
    // where it was dropped, at its own size, until the list draws it there
    put(to) {
      cap?.remove()
      node.style.opacity = '1'
      node.style.transform = `translate(${to.left - r.left}px, ${to.top - r.top}px)`
    },
    remove() {
      node.remove()
      cap?.remove()
    },
  }
}

/** What is dragged named beside the pointer, when there is no element to lift. */
export function liftLabel(text, x, y) {
  const cap = document.createElement('div')
  cap.className = 'drag-ghost'
  cap.textContent = text
  document.body.append(cap)
  const move = (px, py) => {
    cap.style.left = px + 14 + 'px'
    cap.style.top = py + 12 + 'px'
  }
  move(x, y)
  return {
    rect: null,
    radius: '',
    move,
    say(t) {
      cap.textContent = t
    },
    put() {
      cap.remove()
    },
    remove() {
      cap.remove()
    },
  }
}

/** The room a drop would land in. */
export function dropSlot(radius) {
  const el = document.createElement('div')
  el.className = 'drag-slot'
  if (radius) el.style.borderRadius = radius
  el.style.display = 'none'
  document.body.append(el)
  return {
    show(r) {
      Object.assign(el.style, { display: 'block', left: r.left + 'px', top: r.top + 'px', width: r.width + 'px', height: r.height + 'px' })
    },
    hide() {
      el.style.display = 'none'
    },
    remove() {
      el.remove()
    },
  }
}

const moved = new Map()
/** Moves an entry of a list aside by (`dx`, `dy`), at once. */
export function shift(el, dx, dy) {
  const was = moved.get(el)
  if ((was?.dx || 0) === dx && (was?.dy || 0) === dy) return
  const keep = was || { transform: el.style.transform, transition: el.style.transition }
  el.style.transition = 'none'
  if (dx || dy) {
    el.style.transform = `translate(${dx}px, ${dy}px)`
    moved.set(el, { ...keep, dx, dy })
  } else {
    el.style.transform = keep.transform
    moved.delete(el)
    // its own transition comes back once it stands where it was, so it doesn't slide there
    requestAnimationFrame(() => !moved.has(el) && (el.style.transition = keep.transition))
  }
}
/** Every entry back in its place. */
export function unshiftAll() {
  for (const el of [...moved.keys()]) shift(el, 0, 0)
}

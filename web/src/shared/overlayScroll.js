// Scrollbars that take no room: the system's own are hidden (html.overlay-scroll) and one pair of thumbs floats over whatever is being scrolled or pointed at, fading out after a moment. Dragging a thumb scrolls.
const IDLE_MS = 900
const MIN_THUMB = 24
const INSET = 2
const SIZE = 6

let bars = null
let target = null
let hideTimer = null
let drag = null
let layer = 0

const scrollsY = (el) => el.scrollHeight > el.clientHeight + 1 && /(auto|scroll)/.test(getComputedStyle(el).overflowY)
const scrollsX = (el) => el.scrollWidth > el.clientWidth + 1 && /(auto|scroll)/.test(getComputedStyle(el).overflowX)

function scrollerOf(el) {
  for (let e = el; e && e !== document.body && e !== document.documentElement; e = e.parentElement) {
    if (e.closest?.('[data-no-scrollbar]')) return null
    if (scrollsY(e) || scrollsX(e)) return e
  }
  return null
}

function make(axis) {
  const t = document.createElement('div')
  t.className = `ov-thumb ov-${axis}`
  t.addEventListener('pointerdown', (e) => {
    if (!target) return
    e.preventDefault()
    e.stopPropagation()
    t.setPointerCapture(e.pointerId)
    drag = { axis, el: target, start: axis === 'y' ? e.clientY : e.clientX, from: axis === 'y' ? target.scrollTop : target.scrollLeft }
    t.classList.add('drag')
  })
  t.addEventListener('pointermove', (e) => {
    if (!drag || drag.axis !== axis) return
    const el = drag.el
    if (axis === 'y') el.scrollTop = drag.from + (e.clientY - drag.start) * (el.scrollHeight / el.clientHeight)
    else el.scrollLeft = drag.from + (e.clientX - drag.start) * (el.scrollWidth / el.clientWidth)
  })
  const end = () => {
    drag = null
    t.classList.remove('drag')
    later()
  }
  t.addEventListener('pointerup', end)
  t.addEventListener('lostpointercapture', end)
  t.addEventListener('pointerenter', () => clearTimeout(hideTimer))
  t.addEventListener('pointerleave', later)
  document.body.appendChild(t)
  return t
}

// just above what is scrolled and its own sticky parts, so popups and menus over it stay over its thumbs too
const BASE_Z = 60
function layerOf(el) {
  let z = BASE_Z
  for (let e = el; e && e !== document.body; e = e.parentElement) {
    const s = getComputedStyle(e)
    if (s.position !== 'static' && s.zIndex !== 'auto') z = Math.max(z, Number(s.zIndex) || 0)
  }
  return z + 1
}

function place() {
  const el = target
  if (!bars || !el || !el.isConnected) return vanish()
  const r = el.getBoundingClientRect()
  const y = scrollsY(el)
  const x = scrollsX(el)
  const ty = bars.y.style
  const tx = bars.x.style
  ty.zIndex = tx.zIndex = layer
  if (y) {
    const h = Math.max(MIN_THUMB, (el.clientHeight * el.clientHeight) / el.scrollHeight)
    const top = r.top + (el.scrollTop / (el.scrollHeight - el.clientHeight || 1)) * (el.clientHeight - h)
    Object.assign(ty, { display: 'block', left: r.left + el.clientWidth - SIZE - INSET + 'px', top: top + 'px', height: h + 'px' })
  } else ty.display = 'none'
  if (x) {
    const w = Math.max(MIN_THUMB, (el.clientWidth * el.clientWidth) / el.scrollWidth)
    const left = r.left + (el.scrollLeft / (el.scrollWidth - el.clientWidth || 1)) * (el.clientWidth - w)
    Object.assign(tx, { display: 'block', top: r.top + el.clientHeight - SIZE - INSET + 'px', left: left + 'px', width: w + 'px' })
  } else tx.display = 'none'
  bars.y.classList.add('show')
  bars.x.classList.add('show')
}

function hide() {
  if (drag || !bars) return
  bars.y.classList.remove('show')
  bars.x.classList.remove('show')
}
// what was scrolled has gone (a popup closed): its thumbs go with it at once rather than fading after it
function vanish() {
  if (!bars) return
  drag = null
  target = null
  clearTimeout(hideTimer)
  for (const t of [bars.y, bars.x]) {
    t.classList.remove('show', 'drag')
    t.style.display = 'none'
  }
}
function later() {
  clearTimeout(hideTimer)
  hideTimer = setTimeout(hide, IDLE_MS)
}

function show(el) {
  if (!el) return
  if (el !== target) layer = layerOf(el)
  target = el
  place()
  later()
}

let raf = 0
export function installOverlayScroll() {
  bars = { y: make('y'), x: make('x') }
  document.addEventListener('scroll', (e) => {
    if (!document.documentElement.classList.contains('overlay-scroll')) return
    const el = e.target === document ? null : e.target
    if (el && !el.closest?.('[data-no-scrollbar]')) show(el)
  }, true)
  document.addEventListener('pointermove', (e) => {
    if (drag || raf || !document.documentElement.classList.contains('overlay-scroll')) return
    raf = requestAnimationFrame(() => {
      raf = 0
      const el = scrollerOf(e.target)
      if (el && el !== target) show(el)
      else if (el) later()
    })
  }, { passive: true })
  window.addEventListener('resize', () => target && place())
  new MutationObserver(() => target && !target.isConnected && vanish()).observe(document.body, { childList: true, subtree: true })
}

// A font's thumbnail, drawn in the window: three lines of it, dark on paper as type specimens are, the same in every theme.
import { describeFont, faceBytes, loadFace } from './fontFile.js'
import { RTL, thumbLines } from './fontSamples.js'

const PAPER = '#f7f6f3'
const INK = '#1d1d1f'
const LINE = 1.3

/** The thumbnail of font item `id` as a PNG data address, `size` pixels square, or null when it has nothing to show. */
export async function drawFontThumb(id, size) {
  const info = await describeFont(id, 0)
  const { family, unload } = await loadFace(await faceBytes(id, info, 0))
  try {
    const lines = thumbLines(info)
    if (!lines.length) return null
    const canvas = document.createElement('canvas')
    canvas.width = canvas.height = size
    const ctx = canvas.getContext('2d')
    ctx.fillStyle = PAPER
    ctx.fillRect(0, 0, size, size)
    const rtl = RTL.has(info.primary)
    const pad = size * 0.1
    ctx.font = `100px "${family}"`
    const widest = Math.max(1, ...lines.map((l) => ctx.measureText(l).width))
    const px = Math.min((100 * (size - 2 * pad)) / widest, (size - 2 * pad) / (lines.length * LINE), size * 0.3)
    ctx.font = `${px}px "${family}"`
    ctx.fillStyle = INK
    ctx.textBaseline = 'middle'
    ctx.direction = rtl ? 'rtl' : 'ltr'
    ctx.textAlign = rtl ? 'right' : 'left'
    const top = size / 2 - (px * LINE * (lines.length - 1)) / 2
    lines.forEach((l, i) => ctx.fillText(l, rtl ? size - pad : pad, top + i * px * LINE))
    return canvas.toDataURL('image/png')
  } finally {
    unload()
  }
}

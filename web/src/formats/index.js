// How each file type is shown. One viewer per family lives next to this file; to show a new format,
// add its extension here and, on the engine side, to src/core/formats/.
import { originalUrl, previewUrl, thumbUrl } from '../platform/fileUrls.js'

export const VIDEO = ['3gp', 'avi', 'av1', 'f4v', 'flv', 'hevc', 'm4v', 'mkv', 'mov', 'mp4', 'mpeg', 'mpg', 'ogv', 'ogm', 'rm', 'rmvb', 'ts', 'vp8', 'vp9', 'webm', 'wmv', 'mts', 'm2ts']
export const AUDIO = ['mp3', 'wav', 'ogg', 'oga', 'm4a', 'flac', 'aac', 'opus', 'wma', 'aiff', 'aif']
export const TEXT = ['txt', 'md', 'markdown', 'json', 'csv', 'tsv', 'log', 'xml', 'yaml', 'yml', 'ini', 'toml', 'js', 'py', 'rs', 'css', 'scss', 'sh', 'bat', 'ps1', 'c', 'cpp', 'h', 'java', 'go', 'lua', 'sql', 'srt', 'vtt']
// the web view draws these itself, so the original file can be shown as it is
export const WEB_IMAGE = ['jpg', 'jpeg', 'jfif', 'jpe', 'png', 'apng', 'gif', 'webp', 'avif', 'svg', 'bmp', 'ico']

export const KIND_ICON = { image: 'photo', video: 'movie', audio: 'music', document: 'file-text', code: 'code', link: 'link', archive: 'file-zip', font: 'typography', model: 'cube', other: 'file' }

// A media element taken out of the page keeps decoding until it is collected; on Linux its sound output then repeats its last buffer.
export function releaseMedia(m) {
  if (!m) return
  m.pause()
  m.removeAttribute('src')
  m.load()
}

// the extensions of switched-off kinds: their files show as plain files, without a player or a preview of their own
let formatsOff = new Set()
export const setFormatsOff = (exts) => (formatsOff = new Set(exts || []))
export const formatOff = (item) => !!item && formatsOff.has(item.ext)
// the extensions a plugin's page draws thumbnails of
let pagedExts = new Set()
export const setPagedExts = (exts) => (pagedExts = new Set(exts || []))

export function kindOf(item) {
  if (!item) return 'none'
  const e = item.ext
  if (formatsOff.has(e)) return 'other'
  if (e === 'url' || e === 'webloc') return 'link'
  if (VIDEO.includes(e)) return 'video'
  if (AUDIO.includes(e)) return 'audio'
  if (TEXT.includes(e)) return 'text'
  if (e === 'pdf') return 'pdf'
  if (item.kind === 'font') return 'font'
  // a picture a plugin's page draws is the thumbnail alone; a larger one would be the system's
  if (item.kind !== 'image' && pagedExts.has(e)) return 'other'
  if (item.kind === 'image' || item.thumb_state === 1) return 'image'
  return 'other'
}

export const isAnimated = (item) => !!item && (item.animated || item.ext === 'gif')

// Previews come in steps of this many pixels, so a slightly resized window reuses them.
const STEP = 256
const MAX_PREVIEW = 8192

/**
 * The cheapest picture that is sharp at this many pixels on the long edge: the original when it is
 * about that size anyway (or can't be scaled, like an animation or a drawing), otherwise a preview
 * made to measure. Never more than the display can show.
 */
export function imageUrlFor(item, needPx) {
  const native = WEB_IMAGE.includes(item.ext)
  const long = Math.max(item.width || 0, item.height || 0)
  // the version makes a file that was turned or replaced load afresh
  const ver = item.thumb_ver
  if (native && (isAnimated(item) || item.ext === 'svg' || item.ext === 'ico' || item.ext === 'avif')) return originalUrl(item.id, ver)
  // size unknown yet: ask for what the screen needs
  const step = Math.min(MAX_PREVIEW, Math.ceil(needPx / STEP) * STEP)
  if (long && step >= long) return native ? originalUrl(item.id, ver) : previewUrl(item.id, Math.min(MAX_PREVIEW, Math.ceil(long / STEP) * STEP), ver)
  return previewUrl(item.id, step, ver)
}

/** The picture's box when fitted into an area; small pictures keep their own size. A picture turned on
 *  its side fits the other way round, though its box keeps its unturned shape. */
export function fitBox(item, areaW, areaH, natural = null, rotation = 0) {
  const w = natural?.w || item?.width || 0
  const h = natural?.h || item?.height || 0
  if (!w || !h || !areaW || !areaH) return null
  const turned = rotation % 180 !== 0
  const s = Math.min(1, areaW / (turned ? h : w), areaH / (turned ? w : h))
  return { w: Math.max(1, Math.round(w * s)), h: Math.max(1, Math.round(h * s)) }
}

/** What a box needs in device pixels on its long edge. */
export const neededPixels = (box, zoom = 1) => Math.ceil(Math.max(box.w, box.h) * (window.devicePixelRatio || 1) * Math.max(1, zoom))

export const fmtTime = (t) => {
  if (!Number.isFinite(t) || t < 0) t = 0
  const h = Math.floor(t / 3600)
  const m = Math.floor((t % 3600) / 60)
  const s = Math.floor(t % 60)
  return h ? `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}` : `${m}:${String(s).padStart(2, '0')}`
}

/** The item's thumbnail, also while one is being remade (the old file stays until the new one replaces it), as for a missing file. */
// a thumbnail being made again (state 3) is still the one before it: the new one's address answers nothing until it is done, and a picture that failed once isn't tried again
export const thumbOf = (it) => (!it ? null : it.thumb_state === 1 ? thumbUrl(it.uid, it.thumb_ver) : it.thumb_state === 3 && it.thumb_ver > 0 ? thumbUrl(it.uid, it.thumb_ver - 1) : null)

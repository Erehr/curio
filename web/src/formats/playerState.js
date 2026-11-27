// What belongs to the video or song open right now: its element, the part being repeated, subtitles.
import { reactive } from 'vue'
import { toast } from '../app/notify.js'
import { viewerFile } from '../viewer/session.js'
import { call, errorText } from '../platform/ipc.js'
import { frameOp } from './mediaEngine.js'
import { t } from '../shared/i18n.js'

export const P = reactive({
  el: null,
  loopIn: 0,
  loopOut: 1,
  subtitles: [],
  subtitle: null,
  // the last one ended and the next is to play, whatever Settings say about starting videos
  goOn: false,
})

export function resetPlayer() {
  P.loopIn = 0
  P.loopOut = 1
  for (const s of P.subtitles) URL.revokeObjectURL(s.url)
  P.subtitles = []
  P.subtitle = null
}

/** The speeds on offer. */
export const VIDEO_SPEEDS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 3, 5]
export const AUDIO_SPEEDS = [0.5, 0.75, 1, 1.25, 1.5, 2]

/** SubRip to WebVTT: a header, full stops in the times, and styling braces made into tags. */
export function srtToVtt(text) {
  const body = text.replace(/^\uFEFF/, '')
  if (/^WEBVTT/.test(body)) return body
  const cues = body
    .replace(/\r\n?/g, '\n')
    .replace(/(\d{2}:\d{2}:\d{2}),(\d{3})/g, '$1.$2')
    .replace(/\{\\([ibu])0?\}|\{\/([ibu])\}/g, (_, a, b) => `</${a || b}>`)
    .replace(/\{\\?([ibu])1?\}/g, '<$1>')
  return `WEBVTT\n\n${cues.trim()}\n`
}

function addTrack(name, text) {
  const url = URL.createObjectURL(new Blob([srtToVtt(text)], { type: 'text/vtt' }))
  P.subtitles.push({ name, url })
  P.subtitle = name
}

/** Whatever lies next to the video under its name shows by itself. */
export async function loadSiblingSubtitles(id) {
  const tracks = await call('subtitle_tracks', { id }).catch(() => [])
  if (viewerFile() !== id || P.subtitles.length) return
  for (const t of tracks) addTrack(t.name, t.text)
  // the first one found is the one on show
  if (tracks.length) P.subtitle = tracks[0].name
}

export async function loadSubtitles() {
  const id = viewerFile()
  const picked = await call('pick', { kind: 'subtitles' })
  if (!picked?.length || id == null) return
  try {
    const track = await call('subtitle_load', { id, path: picked[0] })
    if (viewerFile() !== id) return
    const old = P.subtitles.findIndex((x) => x.name === track.name)
    if (old >= 0) {
      URL.revokeObjectURL(P.subtitles[old].url)
      P.subtitles.splice(old, 1)
    }
    addTrack(track.name, track.text)
    if (track.readOnly) toast(track.why ? t('formats.subtitles.notSavedWhy', { why: track.why }) : t('formats.subtitles.readOnly'))
    else if (!track.saved) toast(t('formats.subtitles.taken'))
  } catch (e) {
    toast(errorText(e), 'error')
  }
}

/** Show one subtitle track, or none. */
export function showSubtitle(name) {
  P.subtitle = name
  const tracks = P.el?.textTracks
  if (!tracks) return
  for (const t of tracks) t.mode = t.label === name ? 'showing' : 'disabled'
}

const COPIED_TOAST_MS = 1000
function frame(op) {
  const id = viewerFile()
  if (id == null || !P.el) return Promise.resolve()
  return frameOp(id, P.el.currentTime, op).then((m) => toast(m, 'info', op === 'copy' ? COPIED_TOAST_MS : undefined)).catch((e) => toast(errorText(e), 'error'))
}
export const setFrameAsThumbnail = () => frame('thumb')
export const copyFrame = () => frame('copy')
export const saveFrame = () => frame('save')

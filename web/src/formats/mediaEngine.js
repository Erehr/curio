// FFmpeg reads video and sound on the program's side; this file tells it what the window's player reads, and fetches what it measured.
import { call } from '../platform/ipc.js'
import { BASE } from '../platform/fileUrls.js'

// What to ask the player.
const VIDEO_CODECS = [['H.264', 'video/mp4; codecs="avc1.42E01E"'], ['H.265', 'video/mp4; codecs="hvc1.1.6.L93.B0"'], ['AV1', 'video/mp4; codecs="av01.0.04M.08"'], ['VP9', 'video/webm; codecs="vp09.00.10.08"'], ['VP8', 'video/webm; codecs="vp8"'], ['Theora', 'video/ogg; codecs="theora"']]
const AUDIO_CODECS = [['AAC', 'audio/mp4; codecs="mp4a.40.2"'], ['MP3', 'audio/mpeg'], ['Opus', 'audio/ogg; codecs="opus"'], ['Vorbis', 'audio/ogg; codecs="vorbis"'], ['FLAC', 'audio/flac'], ['WAV', 'audio/wav'], ['AC-3', 'audio/mp4; codecs="ac-3"'], ['E-AC-3', 'audio/mp4; codecs="ec-3"']]

// a Mac's WebKit says it reads WebM's VP8 and VP9 and then plays nothing, on Intel Macs at least (WebKit bug 216652), so there they are converted
const CLAIMED_ONLY = navigator.userAgent.includes('Mac') ? ['VP8', 'VP9'] : []

/** What this computer's player can read: { plays: [names], not: [names] }. */
export function playerCodecs() {
  const v = document.createElement('video')
  const out = { plays: [], not: [] }
  for (const [name, type] of [...VIDEO_CODECS, ...AUDIO_CODECS]) (v.canPlayType(type) && !CLAIMED_ONLY.includes(name) ? out.plays : out.not).push(name)
  return out
}
// the program serves what the player reads as it is and converts the rest
export const reportCodecs = () => call('media_codecs_found', { plays: playerCodecs().plays }).catch(() => {})

/** A sound's loudness peaks, measured once by FFmpeg and then kept. */
export async function loadWave(id) {
  const r = await fetch(`${BASE}/w/${id}`).catch(() => null)
  return r?.ok ? new Uint8Array(await r.arrayBuffer()) : new Uint8Array(0)
}

/** Copy, save or use a frame, which FFmpeg takes from the file. */
export const frameOp = (id, time, op) => call('video_frame_op', { id, time, op })

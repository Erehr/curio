// Notices, up to three stacked beside any errors, and commands whose failures become one.
import { call, errorText } from '../platform/ipc.js'
import { S } from './state.js'
import notifySoundUrl from './notify.wav'

// ------------------------------------------------------------------ notifications

const MAX_NOTICES = 3
const SOUND_GAP_MS = 80
let toastId = 0
let soundEl = null
let lastSoundAt = 0

export function playNotifySound(what = 'other') {
  const force = what === true
  if (!force && (S.ui.notifySound === false || !S.ui.notifySoundEvents?.[what])) return
  const now = performance.now()
  if (!force && now - lastSoundAt < SOUND_GAP_MS) return
  lastSoundAt = now
  try {
    soundEl ||= new Audio(notifySoundUrl)
    soundEl.currentTime = 0
    soundEl.play().catch(() => {})
  } catch {}
}

export function closeToast(id) {
  const i = S.toasts.findIndex((t) => t.id === id)
  if (i >= 0) S.toasts.splice(i, 1)
}
export function toast(text, kind = 'info', ms = undefined, undo = null, sound = null) {
  const id = ++toastId
  if (kind !== 'error') {
    const notices = S.toasts.filter((t) => t.kind !== 'error')
    while (notices.length >= MAX_NOTICES) {
      const oldest = notices.shift()
      const i = S.toasts.indexOf(oldest)
      if (i >= 0) S.toasts.splice(i, 1)
    }
  }
  S.toasts.push({ id, text, kind, undo, ms })
  playNotifySound(sound ?? (kind === 'error' ? 'error' : 'other'))
}

export async function run(cmd, args, okMsg) {
  try {
    const r = await call(cmd, args)
    if (okMsg) toast(okMsg)
    return r
  } catch (e) {
    toast(errorText(e), 'error')
    throw e
  }
}

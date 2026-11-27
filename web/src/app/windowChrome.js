// The window's own controls that no feature owns.
import { getCurrentWindow } from '../platform/windowFrame.js'
import { S } from './state.js'

/** Keep this window above the others, or stop. */
export async function toggleOnTop() {
  S.onTop = !S.onTop
  await getCurrentWindow().setAlwaysOnTop(S.onTop).catch(() => (S.onTop = false))
}

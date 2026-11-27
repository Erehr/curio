// A font of the library as the window reads it: what it says of itself, its glyph names, and the bytes the web view loads it from.
import { call } from '../platform/ipc.js'
import { originalUrl } from '../platform/fileUrls.js'

/** Everything about one face: names, style, category, scripts, axes, features and the characters as ranges, with a collection's face names. */
export const describeFont = (id, face = 0) => call('font_describe', { id, face })

/** The glyph names of these code points, null where the font names none. */
export const glyphNames = (id, face, chars) => call('font_glyph_names', { id, face, chars })

/** The bytes `FontFace` takes: the file as it is, or one face of a collection or dfont rebuilt as a font of its own. */
export async function faceBytes(id, info, face = 0) {
  if (info.container === 'collection' || info.container === 'dfont') return call('font_face_bytes', { id, face })
  const r = await fetch(originalUrl(id))
  if (!r.ok) throw new Error(String(r.status))
  return r.arrayBuffer()
}

let serial = 0

/** Loads the bytes as a font of the window under a name of its own; answers the name and a function that unloads it. */
export async function loadFace(bytes) {
  const family = `curio-font-view-${++serial}`
  const face = new FontFace(family, bytes)
  await face.load()
  document.fonts.add(face)
  return { family, unload: () => document.fonts.delete(face) }
}

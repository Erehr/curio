// Thin wrapper over the Tauri commands exposed by src/desktop/.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { hasText, t } from '../shared/i18n.js'

/** A dotted name such as `item.update` is an operation of the API (src/core/api), shared with scripts and MCP; other names are the window's own commands. */
export const call = (cmd, args = {}) => (cmd.includes('.') ? invoke('api', { op: cmd, args }) : invoke(cmd, args)).catch((e) => { throw new Failure(e) })

/** What Rust said (`src/core/said.rs`) in the window's language: each sentence by its message id when the catalog has one, else as Rust wrote it. */
export function saidText(said, text = '') {
  if (!said?.length) return text
  return said.map((p) => (p.id && hasText(p.id) ? t(p.id, shownValues(p.args), typeof p.args?.count === 'number' ? p.args.count : undefined) : p.text)).join(' ')
}
// numbers as the system writes them
const shownValues = (args) => Object.fromEntries(Object.entries(args || {}).map(([k, v]) => [k, typeof v === 'number' ? v.toLocaleString() : v]))

/** A command's failure: `message` is Rust's English, as programs and plugins are told; shown (`String(e)`), it reads in the window's language. */
class Failure extends Error {
  constructor(e) {
    super(typeof e === 'string' ? e : (e?.message ?? String(e)))
    this.code = e?.code
    this.said = e?.said
  }

  toString() {
    return saidText(this.said, this.message)
  }
}

/** Any error as the window shows it. */
export const errorText = (e) => (e instanceof Failure ? String(e) : typeof e === 'string' ? e : (e?.message ?? String(e)))
export const on = (event, fn) => listen(event, (e) => fn(e.payload))
export { emit } from '@tauri-apps/api/event'
// for plugin pages' own calls (plugins/bridge.js, plugins/pageJobs.js): call() would send a dotted name through the trusted api command
export { invoke }

/** search() returns an 8-byte header (u32 count, u32 groups_json_len), then packed (u32 id, f32 aspect) pairs, optional note bytes, and trailing UTF-8 JSON for groups. */
const utf8 = new TextDecoder()
export async function search(req) {
  const buf = await invoke('search', { req })
  const ab = buf instanceof ArrayBuffer ? buf : new Uint8Array(buf).buffer
  if (ab.byteLength < 8) return { ids: new Uint32Array(0), aspects: new Float32Array(0), notes: new Set(), groups: null }
  const view = new DataView(ab)
  const n = view.getUint32(0, true)
  const metaLen = view.getUint32(4, true)
  const ids = new Uint32Array(n)
  const aspects = new Float32Array(n)
  for (let i = 0; i < n; i++) {
    const off = 8 + i * 8
    ids[i] = view.getUint32(off, true)
    aspects[i] = view.getFloat32(off + 4, true)
  }
  const notes = new Set()
  const notesOff = 8 + n * 8
  if (req.notes) for (let i = 0; i < n; i++) if (view.getUint8(notesOff + i)) notes.add(ids[i])
  const groups = metaLen > 0 ? JSON.parse(utf8.decode(new Uint8Array(ab, ab.byteLength - metaLen, metaLen))) : null
  return { ids, aspects, notes, groups }
}

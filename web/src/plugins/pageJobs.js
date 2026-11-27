// Work plugin pages do for the library in hidden frames: thumbnails (`formats[].thumbnail_page`) and fields read from files (`extractors[].page`). The main window runs each such page and hands it the items the library left for it, one at a time.
import { nextTick, reactive } from 'vue'
import { call, invoke } from '../platform/ipc.js'
import { S } from '../app/state.js'
import { drawFontThumb } from '../formats/fontThumb.js'

/** The pages working now, for PageJobFrames.vue. */
export const PJ = reactive({ frames: [] })
const refs = new Map()
const waiting = new Map()
let request = 0

const IDLE_MS = 1500
// a file read stays queued until the library applies the answer: it isn't handed out again meanwhile
const ANSWERED_MS = 15000
const JOB_LIMIT_MS = 60000
// a page left without work is closed, freeing what it holds (a WebGL context, a PDF engine)
const CLOSE_AFTER_MS = 30000

export const setFrameRef = (key, c) => (c ? refs.set(key, c) : refs.delete(key))

/** A page's answer, from the bridge. */
export function pageAnswered(a) {
  const resolve = waiting.get(a?.request)
  if (!resolve) return
  waiting.delete(a.request)
  resolve(a)
}

async function frameFor(role, plugin, page) {
  const key = `${role}|${plugin}|${page}`
  let f = PJ.frames.find((x) => x.key === key)
  if (!f) PJ.frames.push((f = { key, role, plugin, page, used: Date.now() }))
  f.used = Date.now()
  for (let i = 0; i < 20 && !refs.get(key); i++) await nextTick()
  return refs.get(key)
}

const fileUrl = (plugin, uid) => invoke('plugin_api', { plugin, op: 'item.fileUrl', args: { id: uid } }).then((r) => r.ok?.url ?? null)

/** Tells the page `name` and waits for its answer: null when it gives none in time. */
function ask(frame, name, data) {
  const id = ++request
  return new Promise((resolve) => {
    waiting.set(id, resolve)
    frame.tell(name, { request: id, ...data })
    setTimeout(() => waiting.delete(id) && resolve(null), JOB_LIMIT_MS)
  })
}

async function thumbnail(job) {
  let answer = null
  try {
    // the Fonts entry's thumbnails are drawn here, by no page
    if (job.plugin === 'builtin.format.font') answer = { data: await drawFontThumb(job.id, job.size) }
    else {
      const frame = await frameFor('thumbnail', job.plugin, job.page)
      const url = await fileUrl(job.plugin, job.uid)
      if (frame && url) answer = await ask(frame, 'plugin.thumbnail', { item: job.uid, ext: job.ext, size: job.size, fileUrl: url })
    }
  } catch {
    /* reported as no picture: the system's thumbnailer then gets a try */
  }
  await call('page_job_done', { id: job.id, image: answer?.data ?? null, width: answer?.width ?? null, height: answer?.height ?? null })
}

// a frame or an address that can't be had leaves the file queued, for this window's next start
async function read(job) {
  const frame = await frameFor('extractor', job.plugin, job.page)
  const url = await fileUrl(job.plugin, job.uid)
  if (!frame || !url) throw new Error('no page')
  const parts = await Promise.all(job.parts.map(async (p) => ({ role: p.role, name: p.name, item: p.uid, size: p.size, fileUrl: await fileUrl(job.plugin, p.uid).catch(() => null) })))
  const answer = await ask(frame, 'plugin.extract', { reader: job.reader, item: job.uid, ext: job.ext, name: job.name, size: job.size, fileUrl: url, parts })
  await call('page_read_done', { id: job.id, reader: job.reader, mode: job.mode, values: answer?.values ?? null, error: answer?.error ?? null, timedOut: !answer })
}

/** Works through the items waiting for a plugin page, in the main window only. */
export function startPageJobs() {
  const gaveUpThumbs = new Set()
  const gaveUpReads = new Map()
  const answered = new Map()
  let lib = S.startup?.current ?? null
  let contrib = S.contrib
  const tick = async () => {
    let busy = false
    try {
      const curLib = S.startup?.current ?? null
      if (curLib !== lib) {
        lib = curLib
        gaveUpThumbs.clear()
        gaveUpReads.clear()
        answered.clear()
        PJ.frames = []
      }
      if (S.contrib !== contrib) {
        contrib = S.contrib
        gaveUpThumbs.clear()
        gaveUpReads.clear()
      }
      if (S.libOpen) {
        const thumbs = (await call('page_jobs', { limit: 8, skip: [...gaveUpThumbs] })).filter((j) => !gaveUpThumbs.has(j.id))
        const now = Date.now()
        for (const [key, entry] of answered) if (now - entry.at > ANSWERED_MS) answered.delete(key)
        const skipReads = [...gaveUpReads.values(), ...[...answered.values()].map((e) => [e.id, e.reader])]
        const reads = (await call('page_reads', { limit: 8, skip: skipReads })).filter((j) => !gaveUpReads.has(`${j.id}|${j.reader}`) && !answered.has(`${j.id}|${j.reader}`))
        busy = thumbs.length > 0 || reads.length > 0
        for (const j of thumbs) await thumbnail(j).catch(() => gaveUpThumbs.add(j.id))
        for (const j of reads) {
          const key = `${j.id}|${j.reader}`
          await read(j).then(() => answered.set(key, { id: j.id, reader: j.reader, at: Date.now() }), () => gaveUpReads.set(key, [j.id, j.reader]))
        }
      }
    } catch {
      /* the library closed meanwhile */
    }
    const now = Date.now()
    PJ.frames = PJ.frames.filter((f) => now - f.used < CLOSE_AFTER_MS)
    setTimeout(tick, busy ? 0 : IDLE_MS)
  }
  tick()
}

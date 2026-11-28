// Work plugin pages do for the library in hidden frames: thumbnails (`formats[].thumbnail_page`) and fields read from files (`extractors[].page`). The main window runs each such page and hands it the items the library left for it, a few at a time.
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
// a file read or a thumbnail stays queued until the library applies the answer: it isn't handed out again meanwhile
const ANSWERED_MS = 15000
const JOB_LIMIT_MS = 60000
// a page left without work is closed, freeing what it holds (a WebGL context, a PDF engine)
const CLOSE_AFTER_MS = 30000
// files each loop works on at once: one at a time leaves the computer mostly waiting on each answer; a few more would hold that many book or PDF engines in a page
const AT_ONCE = 3
// jobs asked for at once: one query of the library's queue for many files, not one each
const BATCH = 32

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

/** Runs `work` on the jobs `next(under)` hands out, `AT_ONCE` at a time, starting the next as each ends, so a slow file holds up only its own place; `under` are the jobs under way. Jobs are asked for `BATCH` at a time, again once those are started. Answers whether there was any. */
async function drain(next, keyOf, work) {
  const running = new Map()
  let waiting = []
  let any = false
  for (;;) {
    if (running.size < AT_ONCE) {
      if (!waiting.length) waiting = (await next([...running.values()].map((r) => r.job))).filter((j) => !running.has(keyOf(j)))
      const job = waiting.shift()
      if (job) {
        any = true
        const key = keyOf(job)
        running.set(key, { job, done: work(job).finally(() => running.delete(key)) })
        continue
      }
    }
    if (!running.size) return any
    await Promise.race([...running.values()].map((r) => r.done))
  }
}

/** Works through the items waiting for a plugin page, in the main window only: thumbnails and file reads in loops of their own, so a page slow to make a picture never holds up the reading. */
export function startPageJobs() {
  const gaveUpThumbs = new Set()
  const gaveUpReads = new Map()
  const answered = new Map()
  const madeThumbs = new Map()
  let lib = S.startup?.current ?? null
  let contrib = S.contrib
  // another library, or other plugins, start over
  const follow = () => {
    const curLib = S.startup?.current ?? null
    if (curLib !== lib) {
      lib = curLib
      gaveUpThumbs.clear()
      gaveUpReads.clear()
      answered.clear()
      madeThumbs.clear()
      PJ.frames = []
    }
    if (S.contrib !== contrib) {
      contrib = S.contrib
      gaveUpThumbs.clear()
      gaveUpReads.clear()
    }
  }
  const loop = (work) => {
    const tick = async () => {
      let busy = false
      try {
        follow()
        if (S.libOpen) busy = await work()
      } catch {
        /* the library closed meanwhile */
      }
      const now = Date.now()
      PJ.frames = PJ.frames.filter((f) => now - f.used < CLOSE_AFTER_MS)
      setTimeout(tick, busy ? 0 : IDLE_MS)
    }
    tick()
  }
  loop(() =>
    drain(
      async (under) => {
        const now = Date.now()
        for (const [id, at] of madeThumbs) if (now - at > ANSWERED_MS) madeThumbs.delete(id)
        // the library reads only so many: those under way and the latest made first
        const skip = [...under.map((j) => j.id), ...[...madeThumbs.keys()].reverse(), ...gaveUpThumbs]
        return (await call('page_jobs', { limit: BATCH, skip })).filter((j) => !gaveUpThumbs.has(j.id) && !madeThumbs.has(j.id))
      },
      (j) => j.id,
      (j) => thumbnail(j).then(() => madeThumbs.set(j.id, Date.now()), () => gaveUpThumbs.add(j.id)),
    ),
  )
  const readKey = (j) => `${j.id}|${j.reader}`
  loop(() =>
    drain(
      async (under) => {
        const now = Date.now()
        for (const [key, entry] of answered) if (now - entry.at > ANSWERED_MS) answered.delete(key)
        const skip = [...under.map((j) => [j.id, j.reader]), ...[...answered.values()].reverse().map((e) => [e.id, e.reader]), ...gaveUpReads.values()]
        return (await call('page_reads', { limit: BATCH, skip })).filter((j) => !gaveUpReads.has(readKey(j)) && !answered.has(readKey(j)))
      },
      readKey,
      (j) => read(j).then(() => answered.set(readKey(j), { id: j.id, reader: j.reader, at: Date.now() }), () => gaveUpReads.set(readKey(j), [j.id, j.reader])),
    ),
  )
}

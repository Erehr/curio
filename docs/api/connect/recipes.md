# Recipes

Runnable examples for common automation, extension, plugin and AI assistant tasks.

## Query active

=== "HTTP"

    ```bash
    # List libraries
    curl -s -X POST http://127.0.0.1:41733/api/v1/library/list \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" -d '{}'

    # Query a specific library by UUID
    curl -s -X POST "http://127.0.0.1:41733/api/v1/item/query?library=c4b8f1d2-..." \
      -H "Authorization: Bearer $CURIO_TOKEN" \
      -H "Content-Type: application/json" \
      -d '{"query":"rating >= 4","sort":"imported","descending":true,"limit":10}'
    ```

=== "JavaScript"

    ```javascript
    const Curio = require('./curio.js') // Downloaded from http://127.0.0.1:41733/api/v1/curio.js
    const curio = Curio.connect({ token: process.env.CURIO_TOKEN })

    const libraries = await curio.library.list()
    const open = libraries.find((l) => l.state === 'window') || libraries.find((l) => l.default)
    if (!open) throw new Error('No library configured')

    const lib = curio.forLibrary(open.id)
    const info = await lib.library.info()
    const top = await lib.item.query({ query: 'rating >= 4', sort: 'imported', descending: true, limit: 10 })
    console.log(`${info.name}: ${top.total} highly rated items`)
    ```

## Paginate

`item.query` returns compact `Item` records (max `limit: 1000`). Combine it with `item.getMany` to load full custom fields in batches:

```javascript
async function* iterateWithFields(curio, query, batchSize = 200) {
  let offset = 0
  while (true) {
    const page = await curio.item.query({ query, offset, limit: batchSize })
    if (!page.items.length) break
    const detailed = await curio.item.getMany({ ids: page.items.map((i) => i.id) })
    for (const item of detailed) yield item
    offset += page.items.length
    if (offset >= page.total) break
  }
}

for await (const item of iterateWithFields(curio, 'ext = png AND software IS NOT EMPTY')) {
  console.log(item.name, item.fields.model, item.fields.steps)
}
```

## Upload & track

Pass `requestId` for safe retries and listen to `items.added` so slow queued imports resolve to their created item IDs:

```javascript
const requestId = `import-${Date.now()}-${Math.random().toString(16).slice(2)}`

const addedPromise = new Promise((resolve) => {
  const sub = curio.on('items.added', (ev) => {
    if (ev.data.requestId === requestId) {
      sub.close()
      resolve(ev.data.ids)
    }
  })
})

const reply = await curio.item.add.fromUrls({
  urls: ['https://example.com/artwork.png'],
  tags: ['reference', 'lighting'],
  rating: 4,
  requestId,
  wait: false,
})

const ids = reply.queued ? await addedPromise : reply.ids
console.log('Imported item IDs:', ids)
```

## Pair & save

Run inside a browser extension (`chrome-extension://` or `moz-extension://`) with `curio.js` bundled:

```javascript
// 1. Generate a 4-8 digit verification code, show it to the user and send { name, code }
const code = String(Math.floor(1000 + Math.random() * 9000))
showCodeToUser(code)

const pairRes = await fetch('http://127.0.0.1:41733/api/v1/pair', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ name: 'My Browser Helper', code }),
})
const pairBody = await pairRes.json()
if (!pairRes.ok) throw new Curio.Error(pairBody.error.code, pairBody.error.message, pairRes.status)

// 2. Connect with the paired token and save an image URL
const curio = Curio.connect({ token: pairBody.token })
await curio.item.add.fromUrls({
  urls: ['https://example.com/photo.jpg'],
  sourceUrl: 'https://example.com/gallery/42',
  headers: { Referer: 'https://example.com/gallery/42' },
  onDuplicate: 'skip',
  wait: false,
})
```

## Selection hook

```javascript
const Curio = require('./curio.js') // Downloaded from http://127.0.0.1:41733/api/v1/curio.js
const curio = Curio.connect({ token: process.env.CURIO_TOKEN })

const sub = curio.on('selection.changed', async (ev) => {
  if (ev.data.count === 1) {
    const [item] = await curio.item.getMany({ ids: ev.data.ids })
    console.log('Selected:', item.name, item.tags)
  }
})

// Later, open a specific item in the active window's viewer
await curio.viewer.open({ id: '91c0f8b2-...' })
```

## Track job

Start a duplicate search with `duplicates.find` and stream progress until `job.finished` delivers the full group list:

```javascript
const { job } = await curio.duplicates.find({
  ways: ['content', 'looks'],
  thresholds: { looks: 0.92 },
})

const done = await new Promise((resolve, reject) => {
  const sub = curio.subscribe({ events: ['job.*'] }, (ev) => {
    if (ev.name === 'job.progress' && ev.data.job === job) {
      console.log(`${ev.data.done}/${ev.data.total}: ${ev.data.text}`)
    }
    if (ev.name === 'job.finished' && ev.data.id === job) {
      sub.close()
      if (ev.data.state === 'done') resolve(ev.data.result)
      else reject(new Error(ev.data.error || ev.data.state))
    }
  })
})

console.log('Duplicate groups found:', done.groups.length)
```

## MCP query

```bash
curl -s -X POST http://127.0.0.1:41733/mcp \
  -H "Authorization: Bearer $CURIO_READ_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "item_query",
      "arguments": {
        "query": "kind = image AND width >= 1920 AND favorite = true",
        "limit": 10
      }
    }
  }'
```

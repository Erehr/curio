# Browser markup

Websites and userscripts can annotate DOM elements for the [Send to Curio](../../features/import/browser.md) browser extension. Annotations supply full-resolution URLs, titles, notes, tags, ratings, custom fields and suggested destinations.

## Resolution

Send to Curio resolves each attribute by walking upward from the captured element through its DOM ancestors (and across shadow roots). Page-level `<meta name="curio:<name>" content="...">` tags provide the outermost fallback.

The nearest attribute wins for each property independently. A gallery card or `<article>` can set shared tags or metadata while each `<img>` supplies its own full-resolution `data-curio-src`.

```html
<head>
  <meta name="curio:tags" content='["portfolio","2026"]'>
</head>
<article data-curio-library="Inspiration" data-curio-annotation="Concept art study">
  <img
    src="/thumbs/forest_400.webp"
    data-curio-src="/originals/forest_4k.png"
    data-curio-title="Whispering Forest"
    data-curio-tags='["environment","lighting"]'
    data-curio-rating="5"
  >
</article>
```

## Core attributes

| Attribute | Accepted value | Limits & behavior |
|---|---|---|
| `data-curio-src` | URL (resolved relative to the page) | Replaces the element's `src` as the file or URL to save. Must resolve to `http:` or `https:`. |
| `data-curio-link` | URL (`http:` or `https:`) | Sets the item's source URL (`url`) instead of the current tab address. |
| `data-curio-title` | Plain text | Sets the imported item's name (truncated at **200 characters**). |
| `data-curio-annotation` | Plain text | Sets the imported item's notes (truncated at **10,000 characters**). |

## Metadata

| Attribute | Accepted value | Limits & behavior |
|---|---|---|
| `data-curio-tags` | JSON array (`'["a","b"]'`) or comma-separated string (`"a, b"`) | Up to **50 tags**, merged with destination and site-rule tags. |
| `data-curio-rating` | Integer string `0` to `5` | Sets the item's star rating. |
| `data-curio-fields` | JSON object (`'{"artist":"M. Chen"}'`) | Populates custom fields that already exist in the target library; Send to Curio ignores unknown field keys. |

## Destinations

!!! note "Page destinations require a matching Site rule on quick-save"
    Send to Curio ignores `data-curio-library` and `data-curio-folders` during quick-save unless a matching Site rule enables **Use page destination**. Without that rule, Send to Curio shows them only as suggestions in the capture panel.

| Attribute | Accepted value | Behavior |
|---|---|---|
| `data-curio-library` | Library UUID, library name or alias from **Extension Options › Library aliases** | Suggests the target library. |
| `data-curio-folders` | JSON array or single value: numeric folder IDs (`[12]`) or `/`-separated paths from the root (`["Art/Environments"]`) | Suggests target folders. Missing folders are created only when **Let pages create folders** is enabled in Extension Options and the token holds `organize`. |

## Type & referer

| Attribute | Accepted value | Behavior |
|---|---|---|
| `data-curio-type` | `file` or `link` | Setting `link` saves the address as a `.url` bookmark regardless of its MIME type. |
| `data-curio-referer` | URL on the page's own origin | Overrides the `Referer` header sent when fetching `data-curio-src`. |
| `data-curio-collectable` | Boolean attribute (present) | Marks a non-image element (such as a `<div>` card or link wrapper) as capturable alongside `data-curio-src`. |

## Shorthand & Eagle

- **`data-curio` JSON shorthand**: Supplies multiple properties in one JSON object (`data-curio='{"src":"...","title":"...","tags":["a"]}'`). Any explicit attribute (such as `data-curio-title`) on the same element overrides the shorthand key.
- **`curio-<name>` alias**: Attributes without the `data-` prefix (`curio-src`, `curio-tags`) work the same as `data-curio-<name>`.
- **Eagle compatibility**: Enabled by default under **Extension Options › Read `eagle-*` attributes**. Where no Curio attribute exists, Send to Curio reads `eagle-src`, `eagle-title`, `eagle-link`, `eagle-annotation`, `eagle-tags` (comma-separated) and `eagle-src-loading` (treated as `data-curio-pending`).

## Async collect

Before reading attributes, Send to Curio dispatches a bubbling, composed (`composed: true`) **`curio:collect`** `CustomEvent` on the target element. Site code or userscripts can populate attributes on demand inside this listener.

If the listener needs async work (such as fetching an original file URL), set **`data-curio-pending`** synchronously and remove it once done. Send to Curio waits up to **3 seconds** for `data-curio-pending` to clear:

```javascript
document.addEventListener('curio:collect', async (event) => {
  const img = event.target.closest('img[data-item-id]')
  if (!img || img.dataset.curioSrc) return
  img.setAttribute('data-curio-pending', '')
  try {
    const meta = await fetch(`/api/items/${img.dataset.itemId}`).then((r) => r.json())
    img.dataset.curioSrc = meta.originalUrl
    img.dataset.curioTitle = meta.title
    img.dataset.curioTags = JSON.stringify(meta.tags)
  } finally {
    img.removeAttribute('data-curio-pending')
  }
})
```

## Security

Send to Curio treats every DOM attribute as untrusted page input, whether written by the site or a userscript:

- **Trusted gestures**: Captures run only from real user actions (`event.isTrusted === true`).
- **Local-network blocking**: Pages cannot force downloads from loopback or local-network addresses (`127.0.0.1`, `localhost`, LAN IPs) or read files on disk.
- **Cross-host cookie stripping**: When `data-curio-src` points to another host, Send to Curio strips browser cookies even if **Send the site's cookies with a download** is enabled.
- **Token isolation**: Page attributes cannot read the API token, invoke arbitrary operations or write outside libraries allowed by the extension's token.

## Example userscript

Download or install [`curio-attributes.user.js`](../../assets/userscripts/curio-attributes.user.js) as a starting template for annotating a site with `data-curio-*` attributes and `curio:collect`:

```javascript
// ==UserScript==
// @name         Curio attributes (example)
// @namespace    curio
// @version      1.0.0
// @description  Annotates article images with full-size URLs, tags and a suggested library for Send to Curio.
// @match        https://example.com/*
// @grant        none
// @run-at       document-idle
// ==/UserScript==

// Send to Curio follows data-curio-library on quick-save only when a Site rule enables Use page destination.
;(function () {
  'use strict'

  function mark(root) {
    for (const img of root.querySelectorAll('article img:not([data-curio-src])')) {
      img.dataset.curioSrc = img.src.replace('/thumb/', '/full/')
      img.dataset.curioTags = JSON.stringify(['from the web', img.closest('article')?.dataset.author].filter(Boolean))
      img.dataset.curioLibrary = 'Inspiration'
    }
  }

  mark(document)
  new MutationObserver(() => mark(document)).observe(document.body, { childList: true, subtree: true })

  document.addEventListener('curio:collect', (e) => {
    const img = e.target
    if (!(img instanceof HTMLImageElement) || img.dataset.curioTitle) return
    img.dataset.curioPending = ''
    fetch(img.closest('a')?.href || location.href)
      .then((r) => r.text())
      .then((html) => {
        const title = new DOMParser().parseFromString(html, 'text/html').querySelector('h1')?.textContent?.trim()
        if (title) img.dataset.curioTitle = title
      })
      .finally(() => delete img.dataset.curioPending)
  })
})()
```

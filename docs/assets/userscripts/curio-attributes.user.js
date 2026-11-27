// ==UserScript==
// @name         Curio attributes (example)
// @namespace    curio
// @version      1.0.0
// @description  An example for the Send to Curio extension: marks pictures in articles with the full-size address, tags and a library, which the extension reads when one is saved. Change the selector, the address rewrite and the names for the site it runs on.
// @match        https://example.com/*
// @grant        none
// @run-at       document-idle
// ==/UserScript==

// The extension reads data-curio-* attributes from the picture and its ancestors when it is saved; see Page attributes in its README.
// A library named here is followed only where one of the extension's site rules has Use page destination; elsewhere its panel shows it, to use for that save.
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
  // pages that load more as they scroll
  new MutationObserver(() => mark(document)).observe(document.body, { childList: true, subtree: true })

  // a value that needs a request: say so with data-curio-pending, and take it away once the attribute is set
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

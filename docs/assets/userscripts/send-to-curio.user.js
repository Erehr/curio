// ==UserScript==
// @name         Send to Curio
// @namespace    curio
// @version      1.1.0
// @description  A Curio button on pictures: one click sends the picture, with the page it was on, to a Curio library. The script's menu sends the page as a link.
// @match        *://*/*
// @require      http://127.0.0.1:41733/api/v1/curio.js
// @grant        GM_xmlhttpRequest
// @grant        GM_getValue
// @grant        GM_setValue
// @grant        GM_registerMenuCommand
// @connect      127.0.0.1
// @connect      *
// @noframes
// ==/UserScript==

/* global Curio */
;(function () {
  'use strict'

  const MIN_SIZE = 120
  const HIDE_MS = 400
  let curio = null
  let button = null
  let target = null
  let hideTimer = null

  function client() {
    const token = GM_getValue('token', '')
    if (!token) return null
    if (!curio) curio = Curio.connect({ token, port: GM_getValue('port', 41733), library: GM_getValue('library', '') || undefined })
    return curio
  }

  function toast(text, error = false) {
    const t = document.createElement('div')
    t.textContent = text
    t.style.cssText = `position:fixed;z-index:2147483647;right:16px;bottom:16px;max-width:360px;padding:9px 14px;border-radius:8px;font:13px/1.4 system-ui,sans-serif;color:#fff;background:${error ? '#b33a3a' : '#0e7c8f'};box-shadow:0 4px 16px rgba(0,0,0,.3)`
    document.documentElement.append(t)
    setTimeout(() => t.remove(), error ? 6000 : 2500)
  }

  function askToken() {
    const token = prompt('A token from Curio, Settings › API, with library.read and files.write:', GM_getValue('token', ''))
    if (token === null) return false
    GM_setValue('token', token.trim())
    curio = null
    return !!token.trim()
  }

  async function chooseLibrary() {
    const c = client() || (askToken() && client())
    if (!c) return
    try {
      const list = (await c.library.list()).filter((l) => l.state !== 'offline')
      const lines = list.map((l, i) => `${i + 1}. ${l.name}${l.default ? ' (default)' : ''}`).join('\n')
      const n = prompt(`Library to send to (empty for default):\n\n${lines}`, '')
      if (n === null) return
      const pick = list[Number(n) - 1]
      GM_setValue('library', pick ? pick.id || pick.name : '')
      curio = null
      toast(pick ? `Sending to ${pick.name}` : 'Sending to the default library')
    } catch (e) {
      toast(e.message, true)
    }
  }

  // the picture as this browser sees it, with the site's sign-ins and from addresses Curio won't download from
  function fetchBytes(url) {
    return new Promise((resolve) => {
      GM_xmlhttpRequest({
        method: 'GET', url, responseType: 'blob',
        onload: (r) => resolve(r.status >= 200 && r.status < 300 && r.response && r.response.size ? r.response : null),
        onerror: () => resolve(null),
        ontimeout: () => resolve(null),
      })
    })
  }

  function dataUrl(blob) {
    return new Promise((resolve, reject) => {
      const r = new FileReader()
      r.onload = () => resolve(r.result)
      r.onerror = () => reject(r.error)
      r.readAsDataURL(blob)
    })
  }

  function fileName(url, type) {
    let base = 'picture'
    try {
      base = decodeURIComponent(new URL(url).pathname.split('/').filter(Boolean).pop() || base)
    } catch (_) {}
    const ext = (type.split('/')[1] || '').replace('jpeg', 'jpg').replace('svg+xml', 'svg')
    return /\.[a-z0-9]{2,5}$/i.test(base) || !ext ? base : `${base}.${ext}`
  }

  async function send(what) {
    const c = client() || (askToken() && client())
    if (!c) return
    try {
      const blob = what.image ? await fetchBytes(what.image) : null
      // the page says where a picture was found
      const r = blob
        ? await c.item.add.fromBytes({ data: await dataUrl(blob), name: fileName(what.image, blob.type || ''), url: what.page })
        : what.image
          ? await c.item.add.fromUrls({ urls: [what.image], sourceUrl: what.page, referer: what.page, wait: false })
          : await c.item.add.bookmark({ url: what.page, name: document.title || undefined })
      if (r.errors && r.errors.length) throw new Error(r.errors.join('; '))
      // a download is Curio's to tell, in a notification of its own
      toast(r.queued ? 'Sent to Curio' : what.image ? 'Picture sent to Curio' : 'Page sent to Curio')
    } catch (e) {
      if (e.code === 'unauthorized' && askToken()) return send(what)
      toast(e.message, true)
    }
  }

  function place() {
    const r = target.getBoundingClientRect()
    button.style.top = Math.max(4, r.top + 6) + 'px'
    button.style.left = Math.max(4, r.right - button.offsetWidth - 6) + 'px'
  }

  function show(img) {
    clearTimeout(hideTimer)
    target = img
    if (!button) {
      button = document.createElement('button')
      button.textContent = 'Curio'
      button.title = 'Send this picture to Curio'
      button.style.cssText = 'position:fixed;z-index:2147483647;padding:4px 10px;border:0;border-radius:6px;font:600 12px/1.4 system-ui,sans-serif;color:#fff;background:#0e9fb5;box-shadow:0 2px 8px rgba(0,0,0,.35);cursor:pointer'
      button.addEventListener('mouseenter', () => clearTimeout(hideTimer))
      button.addEventListener('mouseleave', hide)
      button.addEventListener('click', (e) => {
        e.preventDefault()
        e.stopPropagation()
        const src = target && (target.currentSrc || target.src)
        if (src) send({ image: new URL(src, location.href).href, page: location.href })
      })
      document.documentElement.append(button)
    }
    button.hidden = false
    place()
  }

  function hide() {
    clearTimeout(hideTimer)
    hideTimer = setTimeout(() => button && (button.hidden = true), HIDE_MS)
  }

  document.addEventListener('mouseover', (e) => {
    const img = e.target.closest && e.target.closest('img')
    if (img && img.naturalWidth >= MIN_SIZE && img.naturalHeight >= MIN_SIZE && !img.src.startsWith('data:')) show(img)
  }, true)
  document.addEventListener('mouseout', (e) => {
    if (e.target === target) hide()
  }, true)
  window.addEventListener('scroll', () => button && !button.hidden && target && place(), { passive: true })

  GM_registerMenuCommand('Send page to Curio as link', () => send({ page: location.href }))
  GM_registerMenuCommand('Choose the library', chooseLibrary)
  GM_registerMenuCommand('Set the token', askToken)
})()

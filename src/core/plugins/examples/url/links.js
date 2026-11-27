// A URL handler's window hears `plugin.run` with the addresses and where the person wanted them (`place`); it adds them itself.
const curio = Curio.connect()
const $ = (id) => document.getElementById(id)
let urls = []
let place = {}

curio.on('plugin.run', (e) => {
  urls = e.data.urls || []
  place = e.data.place || {}
  $('urls').replaceChildren(...urls.map((u) => Object.assign(document.createElement('li'), { textContent: u })))
  $('hint').hidden = urls.length > 0
  $('add').disabled = !urls.length
})
$('add').addEventListener('click', async () => {
  // a plugin's own addresses never come back to it, so these are added rather than handed over again
  const note = $('note').value.trim()
  const r = await curio.item.add.fromUrls({ urls, ...place, annotation: note || place.annotation })
  $('status').textContent = `Added ${r.ids?.length ?? urls.length}`
  curio.ui.notify({ text: `${urls.length} example link(s) added` })
})

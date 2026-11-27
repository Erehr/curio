// A widget hears `plugin.selection` whenever the selection changes, and writes through the API.
const curio = Curio.connect()
const $ = (id) => document.getElementById(id)
let ids = []
let rating = 0

function drawStars() {
  $('stars').replaceChildren(...[1, 2, 3, 4, 5].map((n) => {
    const b = document.createElement('button')
    b.type = 'button'
    b.className = 'star' + (n <= rating ? ' on' : '')
    b.textContent = '★'
    b.onclick = () => {
      if (!ids.length) return
      rating = rating === n ? 0 : n
      drawStars()
      curio.item.update({ ids, rating })
    }
    return b
  }))
}

curio.on('plugin.selection', async (e) => {
  ids = e.data.ids || []
  const items = ids.length ? await curio.item.getMany({ ids }) : []
  const same = items.every((i) => i.rating === items[0]?.rating)
  $('what').textContent = items.length === 1 ? items[0].name : `${items.length} items`
  rating = same ? items[0]?.rating || 0 : 0
  drawStars()
  const bytes = items.reduce((n, i) => n + (i.size || 0), 0)
  $('size').textContent = items.length ? `${(bytes / 1048576).toFixed(1)} MB` : ''
})
drawStars()

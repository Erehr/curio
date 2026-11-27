// The complete Tabler icon set, fetched once per window from tabler.json so the 2.1 MB set stays out of the JS bundle.
import { addCollection } from '@iconify/vue'

export let iconNames = []

export async function loadIcons() {
  const set = await fetch('./tabler.json').then((r) => r.json())
  addCollection(set)
  iconNames = Object.keys(set.icons || {})
}

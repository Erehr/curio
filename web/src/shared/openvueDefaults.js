import Select from 'openvue/select'
import MultiSelect from 'openvue/multiselect'
import Listbox from 'openvue/listbox'
import { t } from './i18n.js'

/** Gives an OpenVue list component Curio's filter defaults (focused search box and localized placeholder). */
export function applyFilterDefaults(c, autoFocus = true) {
  if (autoFocus) c.extends.props.autoFilterFocus.default = true
  c.extends.props.filterPlaceholder = { type: String, default: () => t('shared.search') }
}

/** Defaults Curio gives OpenVue's components: a list with a search has it focused as it opens, so typing searches at once, and says what it is. */
export function openvueDefaults() {
  for (const c of [Select, MultiSelect]) applyFilterDefaults(c, true)
  applyFilterDefaults(Listbox, false)
}

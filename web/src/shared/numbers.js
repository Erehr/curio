// Numbers, sizes and percentages as the system writes them: the web view's own locale, which is the system's display language, whatever language the text is in.
import { t } from './i18n.js'

/** A number with at most `digits` decimals (exactly that many when `fixed`), grouped in thousands unless `grouping` is false. */
export const numberText = (n, { digits = 0, fixed = false, grouping = true } = {}) => Number(n).toLocaleString(undefined, { maximumFractionDigits: digits, minimumFractionDigits: fixed ? digits : 0, useGrouping: grouping })

/** A fraction (0.5) as a percentage ("50%"). */
export const percentText = (fraction, digits = 0) => Number(fraction).toLocaleString(undefined, { style: 'percent', maximumFractionDigits: digits, minimumFractionDigits: digits })

const UNITS = ['bytes', 'kb', 'mb', 'gb', 'tb']

/** A size in bytes, in steps of 1024, with `digits` decimals from kilobytes up. */
export function sizeText(bytes, digits = 1) {
  let v = Number(bytes) || 0
  let i = 0
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024
    i++
  }
  return t(`shared.sizes.${UNITS[i]}`, { n: numberText(v, { digits: i ? digits : 0, fixed: true }) })
}

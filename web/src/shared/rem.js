// The root font's size, for the sizes script works out: rows, indents and widths that hold text follow it as the CSS's rem do.
import { ref } from 'vue'

/** The px in one rem, kept by whoever sets the root font. */
export const remPx = ref(13)

/** `n` rem in px, read reactively. */
export const rem = (n) => n * remPx.value

// Icon sizes are numbers in px at this root, the one the window was drawn at, and scale with the root from there.
const ICON_ROOT = 14.3

/** An icon's numeric size as a CSS length that follows the root, in whole px so its strokes stay sharp; a string is a length already. */
export const iconSize = (size) => (typeof size === 'number' || /^\d+(\.\d+)?$/.test(size) ? `round(${+(Number(size) / ICON_ROOT).toFixed(3)}rem, 1px)` : size)

import Aura from '@openvue/themes/aura'
import { definePreset } from '@openvue/themes'

// Curio's look as OpenVue design tokens over Aura: every colour is one of Curio's theme variables (theme.js), so a theme restyles the components too; the window and the UI kit share it.
// Curio's variables already differ between dark and light themes, so a token Aura sets per colour scheme gets the same value in both.
const both = (tokens) => ({ light: tokens, dark: tokens })
const g = (n) => `calc(${n}rem * var(--g))`
// the search icon of a list's head, in whole px so its outline stays sharp
const SEARCH_SIZE = 'round(1.19rem, 1px)'

const COLOURS = {
  primary: { color: 'var(--accent)', contrastColor: 'var(--accent-text)', hoverColor: 'color-mix(in srgb, var(--accent) 85%, white)', activeColor: 'color-mix(in srgb, var(--accent) 80%, black)' },
  highlight: { background: 'var(--selection)', focusBackground: 'var(--selection)', color: 'var(--text)', focusColor: 'var(--text)' },
  mask: { background: 'rgba(0, 0, 0, 0.45)', color: 'var(--text)' },
  formField: {
    background: 'var(--input)', disabledBackground: 'var(--button)', filledBackground: 'var(--input)', filledHoverBackground: 'var(--input)', filledFocusBackground: 'var(--input)',
    borderColor: 'var(--input-border)', hoverBorderColor: 'var(--text-faint)', focusBorderColor: 'var(--accent)',
    color: 'var(--text)', disabledColor: 'var(--text-faint)', placeholderColor: 'var(--text-faint)', iconColor: 'var(--text-muted)', shadow: 'none',
  },
  text: { color: 'var(--text)', hoverColor: 'var(--text)', mutedColor: 'var(--text-muted)', hoverMutedColor: 'var(--text)' },
  content: { background: 'var(--surface)', hoverBackground: 'var(--hover)', borderColor: 'var(--border)', color: 'var(--text)', hoverColor: 'var(--text)' },
  overlay: {
    select: { background: 'var(--surface)', borderColor: 'var(--float-border)', color: 'var(--text)' },
    popover: { background: 'var(--surface)', borderColor: 'var(--float-border)', color: 'var(--text)' },
    modal: { background: 'var(--surface)', borderColor: 'var(--float-border)', color: 'var(--text)' },
  },
  list: {
    option: { focusBackground: 'var(--hover)', selectedBackground: 'var(--selection)', selectedFocusBackground: 'var(--selection)', color: 'var(--text)', focusColor: 'var(--text)', selectedColor: 'var(--text)', selectedFocusColor: 'var(--text)' },
    optionGroup: { background: 'transparent', color: 'var(--text-faint)' },
  },
  navigation: {
    item: { focusBackground: 'var(--hover)', activeBackground: 'var(--active)', color: 'var(--text)', focusColor: 'var(--text)', icon: { color: 'var(--text-muted)', focusColor: 'var(--text)' } },
    submenuIcon: { color: 'var(--text-faint)' },
  },
}

// a box a field's colours: the check box's and the radio button's edge
const BOX = { background: 'transparent', borderColor: 'color-mix(in srgb, var(--text) 45%, transparent)', hoverBorderColor: 'var(--text-muted)' }
// what is otherwise fully round follows Settings › Appearance › Corners too
const ROUND = 'var(--round)'
const ROUND_PART = { borderRadius: ROUND }
// every control is --control-h tall, so a row of them lines up
const TALL = 'height: var(--control-h);'
// menus have the edge and corners of the popovers and lists
const MENU = { root: { borderColor: 'var(--float-border)', borderRadius: '{overlay.popover.border.radius}' } }
// Tabler's search icon, drawn by OpenVue's lists in place of their own so every list's search looks the same
const SEARCH_ICON = `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Cpath fill='none' stroke='black' stroke-linecap='round' stroke-linejoin='round' stroke-width='2' d='M3 10a7 7 0 1 0 14 0a7 7 0 1 0-14 0m18 11l-6-6'/%3E%3C/svg%3E")`
// The search at the top of a list that floats (a Select's, a MultiSelect's, a Listbox's in a popover, AppListSearch): flush with its edges, a line under it, the icon at its left and its own controls at its right, as the menus' search row (AppMenu) draws it.
// Each list's sheet and InputText's carry it, so it is there whichever loads first; doubled classes outrank InputText's and IconField's own rules in any order.
const HEAD = ':is(.p-select-header, .p-multiselect-header, .p-listbox-header, .list-search)'
const LIST_SEARCH = `${HEAD} { display: flex; align-items: center; gap: ${g(0.56)}; height: round(2.66rem, 1px); box-sizing: border-box; flex: none; padding: 0 ${g(0.7)} 0 ${g(0.98)}; border-bottom: 1px solid var(--border); margin: var(--list-search-margin, 0); }
${HEAD} .p-iconfield { flex: 1; min-width: 0; height: 100%; display: flex; align-items: center; }
${HEAD} .p-inputtext.p-inputtext.p-inputtext { flex: 1; min-width: 0; width: 100%; height: 100%; padding: 0; border: 0; border-radius: 0; background: transparent; box-shadow: none; outline: 0; font: inherit; color: var(--text); }
${HEAD} .p-iconfield .p-inputtext.p-inputtext.p-inputtext { padding-inline-start: calc(${SEARCH_SIZE} + ${g(0.56)}); }
${HEAD} .p-iconfield .p-inputicon.p-inputicon { inset-inline: 0 auto; width: ${SEARCH_SIZE}; height: ${SEARCH_SIZE}; margin-top: calc(${SEARCH_SIZE} / -2); background: var(--text-faint); mask: ${SEARCH_ICON} center / ${SEARCH_SIZE} no-repeat; }
${HEAD} .p-inputicon > * { visibility: hidden; }
.list-search .list-search-icon { flex: none; color: var(--text-faint); }
.p-multiselect-header > .p-checkbox { order: 2; margin-inline-start: auto; }`

export const Preset = definePreset(Aura, {
  primitive: {
    borderRadius: { xs: 'calc(2px * var(--r))', sm: 'calc(4px * var(--r))', md: 'calc(6px * var(--r))', lg: 'calc(8px * var(--r))', xl: 'calc(12px * var(--r))' },
  },
  semantic: {
    primary: {
      50: 'color-mix(in srgb, var(--accent) 10%, white)', 100: 'color-mix(in srgb, var(--accent) 22%, white)', 200: 'color-mix(in srgb, var(--accent) 40%, white)',
      300: 'color-mix(in srgb, var(--accent) 60%, white)', 400: 'color-mix(in srgb, var(--accent) 80%, white)', 500: 'var(--accent)',
      600: 'color-mix(in srgb, var(--accent) 85%, black)', 700: 'color-mix(in srgb, var(--accent) 70%, black)', 800: 'color-mix(in srgb, var(--accent) 55%, black)',
      900: 'color-mix(in srgb, var(--accent) 40%, black)', 950: 'color-mix(in srgb, var(--accent) 25%, black)',
    },
    focusRing: { color: 'var(--accent)' },
    formField: { paddingX: 'calc(0.65rem * var(--g))', paddingY: 'calc(0.35rem * var(--g))' },
    // a list's search box is as wide as its options
    list: { header: { padding: '0.25rem 0.25rem 0' } },
    // every floating layer has the same corners, following Settings › Appearance › Corners as all radii do
    overlay: { select: { borderRadius: '{border.radius.lg}' }, popover: { borderRadius: '{border.radius.lg}' }, modal: { borderRadius: '{border.radius.lg}' } },
    colorScheme: {
      light: COLOURS,
      dark: {
        ...COLOURS,
        surface: {
          0: '#ffffff', 50: '#f4f5f6', 100: '#e7e8ea', 200: '#d0d1d4', 300: '#b0b1b5', 400: '#8a8b90', 500: '#65666b',
          600: '#4a4b50', 700: '#36373b', 800: '#2a2b30', 900: '#1f2024', 950: '#141518',
        },
      },
    },
  },
  components: {
    button: {
      root: { roundedBorderRadius: ROUND },
      colorScheme: both({ root: { secondary: { background: 'var(--button)', hoverBackground: 'var(--button-hover)', activeBackground: 'var(--button-hover)', borderColor: 'var(--button)', hoverBorderColor: 'var(--button-hover)', activeBorderColor: 'var(--button-hover)', color: 'var(--text)', hoverColor: 'var(--text)', activeColor: 'var(--text)' } } }),
      css: () => `.p-button:not(.p-button-sm):not(.p-button-lg):not(.p-button-text):not(.p-button-link) { ${TALL} } .p-button-icon-only.p-button-rounded { border-radius: ${ROUND}; }`,
    },
    checkbox: { root: BOX },
    radiobutton: { root: BOX, icon: { size: '0.56rem' }, css: () => `.p-radiobutton-input, .p-radiobutton-box, .p-radiobutton-icon { border-radius: ${ROUND}; }` },
    // a button group shares the field background, border and height of selects and inputs
    togglebutton: {
      root: { padding: '0' },
      content: { padding: `0 ${g(0.63)}`, checkedShadow: 'none' },
      colorScheme: both({
        root: { background: 'transparent', checkedBackground: 'transparent', hoverBackground: 'transparent', borderColor: 'transparent', checkedBorderColor: 'transparent', color: 'var(--text-muted)', hoverColor: 'var(--text)', checkedColor: 'var(--text)', disabledBackground: 'transparent', disabledBorderColor: 'transparent', disabledColor: 'var(--text-faint)' },
        content: { checkedBackground: 'var(--button)' },
      }),
    },
    selectbutton: {
      css: () => `.p-selectbutton { ${TALL} padding: 0.14rem; gap: 0.14rem; background: var(--input); border: 1px solid var(--input-border); border-radius: calc(6px * var(--r)); } .p-selectbutton .p-togglebutton.p-togglebutton { height: 100%; border: 0; padding: 0; outline: none; border-radius: calc(4px * var(--r)); } .p-selectbutton .p-togglebutton-content { height: 100%; border-radius: calc(4px * var(--r)); }`,
    },
    // a see-through window lays its own track colours over the card (theme.js sets --switch-track)
    toggleswitch: {
      colorScheme: {
        light: { root: { background: 'var(--switch-track, {surface.300})', hoverBackground: 'var(--switch-track-hover, {surface.400})' }, handle: { checkedBackground: '#fff', checkedHoverBackground: '#fff' } },
        dark: { root: { background: 'var(--switch-track, {surface.700})', hoverBackground: 'var(--switch-track-hover, {surface.600})' }, handle: { checkedBackground: '#fff', checkedHoverBackground: '#fff' } },
      },
      root: ROUND_PART,
      handle: ROUND_PART,
    },
    slider: {
      track: { background: 'color-mix(in srgb, var(--text) 18%, transparent)' },
      handle: { background: 'var(--text)', hoverBackground: 'var(--text)', borderRadius: ROUND, content: { hoverBackground: 'var(--text)', borderRadius: ROUND } },
      colorScheme: both({ handle: { content: { background: 'var(--text)' } } }),
      // a click anywhere along the line, a little above or below it too, moves the handle there
      css: () => `.p-slider-horizontal::before { content: ''; position: absolute; left: 0; right: 0; top: -9px; bottom: -9px; } .p-slider:not(.p-disabled) { cursor: pointer; }`,
    },
    // set into a menu, a dialog or a page, the picker is its square and strip without a frame of their own
    colorpicker: { css: () => `.p-colorpicker .p-colorpicker-panel.p-colorpicker-panel-inline { background: transparent; border: 0; } .p-colorpicker-color-handle { border-radius: ${ROUND}; }` },
    // drawn as the popovers, menus and messages are: their background and border, no arrow
    tooltip: {
      root: { padding: `${g(0.35)} ${g(0.56)}`, shadow: 'none' },
      colorScheme: both({ root: { background: 'var(--surface)', color: 'var(--text)' } }),
      // a name and its keys stay on one line; a long tooltip without keys still wraps
      css: () => `.p-tooltip-text { font-size: 0.85rem; border: 1px solid var(--float-border); } .p-tooltip .p-tooltip-arrow { display: none; } .p-tooltip:has(.tip-keys) { max-width: 28rem; }`,
    },
    // no arrow: a popover opens as close under its button as a menu or a list does; one holding a list with a search at its top (`list-pop`) leaves the edges to them
    popover: { root: { gutter: '2px' }, css: () => `.p-popover::before, .p-popover::after { display: none; } .p-popover.list-pop .p-popover-content { padding: 0; }` },
    contextmenu: MENU,
    menu: MENU,
    tieredmenu: MENU,
    // the month and year are buttons, which take the browser's smaller font unless told otherwise
    datepicker: { panel: { borderColor: 'var(--float-border)', borderRadius: '{overlay.popover.border.radius}' }, date: ROUND_PART, css: () => `.p-datepicker-select-month, .p-datepicker-select-year { font-family: inherit; font-size: inherit; line-height: inherit; }` },
    chip: { root: ROUND_PART, css: () => `.p-chip-image, .p-chip-remove-icon { border-radius: ${ROUND}; }` },
    avatar: { css: () => `.p-avatar-circle, .p-avatar-circle img { border-radius: ${ROUND}; }` },
    badge: { css: () => `.p-badge-dot, .p-badge-circle { border-radius: ${ROUND}; }` },
    rating: { css: () => `.p-rating-option { border-radius: ${ROUND}; }` },
    skeleton: { css: () => `.p-skeleton-circle { border-radius: ${ROUND}; }` },
    metergroup: { css: () => `.p-metergroup-label-marker { border-radius: ${ROUND}; }` },
    splitbutton: { root: { roundedBorderRadius: ROUND } },
    datatable: { rowToggleButton: ROUND_PART },
    galleria: { navButton: { prev: ROUND_PART, next: ROUND_PART }, indicatorButton: ROUND_PART, closeButton: ROUND_PART },
    image: { toolbar: ROUND_PART, action: ROUND_PART },
    imagecompare: { handle: ROUND_PART },
    megamenu: { mobileButton: ROUND_PART },
    menubar: { mobileButton: ROUND_PART },
    message: {
      root: { borderWidth: '0', borderRadius: '{border.radius.lg}' },
      content: { padding: `${g(0.65)} ${g(0.84)}`, gap: g(0.65) },
      text: { fontSize: '0.92rem', fontWeight: '400' },
      closeButton: ROUND_PART,
      colorScheme: both({
        info: { background: 'var(--info-subtle)', borderColor: 'transparent', color: 'var(--text)', shadow: 'none', closeButton: { hoverBackground: 'var(--hover)' } },
        success: { background: 'var(--success-subtle)', borderColor: 'transparent', color: 'var(--text)', shadow: 'none', closeButton: { hoverBackground: 'var(--hover)' } },
        warn: { background: 'var(--warning-subtle)', borderColor: 'transparent', color: 'var(--text)', shadow: 'none', closeButton: { hoverBackground: 'var(--hover)' } },
        error: { background: 'var(--danger-subtle)', borderColor: 'transparent', color: 'var(--text)', shadow: 'none', closeButton: { hoverBackground: 'var(--hover)' } },
        secondary: { background: 'var(--surface-3)', borderColor: 'transparent', color: 'var(--text)', shadow: 'none', closeButton: { hoverBackground: 'var(--hover)' } },
      }),
      css: () => `.p-message-info .p-message-icon { color: var(--info); } .p-message-success .p-message-icon { color: var(--success); } .p-message-warn .p-message-icon { color: var(--warning); } .p-message-error .p-message-icon { color: var(--danger); }`,
    },
    tag: {
      root: { fontSize: '0.85rem', fontWeight: '500', padding: `0.18rem ${g(0.5)}`, gap: g(0.3), borderRadius: '{border.radius.md}' },
      colorScheme: both({
        primary: { background: 'var(--accent-subtle)', color: 'var(--accent)' },
        secondary: { background: 'var(--surface-3)', color: 'var(--text-muted)' },
        info: { background: 'var(--info-subtle)', color: 'var(--info)' },
        success: { background: 'var(--success-subtle)', color: 'var(--success)' },
        warn: { background: 'var(--warning-subtle)', color: 'var(--warning)' },
        danger: { background: 'var(--danger-subtle)', color: 'var(--danger)' },
      }),
    },
    organizationchart: { nodeToggleButton: ROUND_PART },
    paginator: { navButton: ROUND_PART },
    stepper: { stepNumber: ROUND_PART },
    steps: { itemNumber: ROUND_PART },
    timeline: { eventMarker: { borderRadius: ROUND, content: ROUND_PART } },
    toast: { closeButton: ROUND_PART },
    tree: { nodeToggleButton: ROUND_PART },
    treetable: { nodeToggleButton: ROUND_PART },
    inputtext: { css: () => `.p-inputtext:not(textarea) { ${TALL} } ${LIST_SEARCH}` },
    inputnumber: { css: () => `.p-inputnumber { ${TALL} }` },
    // an addon holding text, as a unit beside a number, lines up with its field
    // an addon stretches to its field, which a feature may make taller than a control
    inputgroup: { css: () => `.p-inputgroupaddon { min-height: var(--control-h); }` },
    select: { css: () => `.p-select { ${TALL} align-items: center; } .p-select-overlay .p-virtualscroller { overflow-x: hidden; } ${LIST_SEARCH}` },
    multiselect: { css: () => `.p-multiselect { ${TALL} align-items: center; } ${LIST_SEARCH}` },
    listbox: { css: () => LIST_SEARCH },
  },
})

// overlays and menus open above Curio's own floating layers (the edit window, the tag picker, menus), modals above the viewer
export const THEME_OPTIONS = { theme: { preset: Preset, options: { darkModeSelector: '.app-dark', cssLayer: false } }, pt: { popover: { transition: { css: false } } }, zIndex: { modal: 1100, overlay: 3500, menu: 3500, tooltip: 4000 } }

// A message's plural forms are written in this order, keeping only the categories its language has: English `one | other`, Polish `one | few | many | other`.
export const PLURAL_ORDER = ['zero', 'one', 'two', 'few', 'many', 'other']

const categories = new Map()

/** The plural categories a language has, in the order its forms are written. */
export function pluralCategories(code) {
  if (!categories.has(code)) {
    const has = new Intl.PluralRules(code).resolvedOptions().pluralCategories
    categories.set(code, PLURAL_ORDER.filter((c) => has.includes(c)))
  }
  return categories.get(code)
}

/** vue-i18n's rule for a language: which of a message's forms a count takes, by the language's own plural rules. */
export function pluralRule(code) {
  const order = pluralCategories(code)
  const rules = new Intl.PluralRules(code)
  return (choice, length) => Math.min(Math.max(order.indexOf(rules.select(Math.abs(choice))), 0), length - 1)
}

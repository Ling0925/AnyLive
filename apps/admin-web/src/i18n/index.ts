import { computed, ref, type Ref } from 'vue'
import {
  LOCALE_STORAGE_KEY,
  messages,
  type Locale,
  type MessageTree,
} from './messages'

export type { Locale, MessageTree }
export { LOCALE_STORAGE_KEY, messages }

function readStoredLocale(): Locale {
  try {
    if (typeof localStorage === 'undefined') return 'zh'
    const raw = localStorage.getItem(LOCALE_STORAGE_KEY)
    if (raw === 'en' || raw === 'zh') return raw
  } catch {
    // private mode
  }
  return 'zh'
}

const locale: Ref<Locale> = ref(readStoredLocale())

export function getLocale(): Locale {
  return locale.value
}

export function setLocale(next: Locale) {
  locale.value = next
  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(LOCALE_STORAGE_KEY, next)
    }
  } catch {
    // ignore
  }
  if (typeof document !== 'undefined') {
    document.documentElement.lang = next === 'zh' ? 'zh-CN' : 'en'
  }
}

/** Resolve dotted key against nested message tree. */
function lookup(tree: unknown, path: string): string | undefined {
  const parts = path.split('.')
  let cur: unknown = tree
  for (const p of parts) {
    if (cur == null || typeof cur !== 'object') return undefined
    cur = (cur as Record<string, unknown>)[p]
  }
  return typeof cur === 'string' ? cur : undefined
}

export type TranslateParams = Record<string, string | number | boolean | null | undefined>

/** Replace `{name}` placeholders. */
export function formatMessage(template: string, params?: TranslateParams): string {
  if (!params) return template
  return template.replace(/\{(\w+)\}/g, (_, key: string) => {
    const v = params[key]
    return v == null ? '' : String(v)
  })
}

/**
 * Translate a key for the active locale.
 * Falls back to zh, then to the key itself.
 */
export function t(key: string, params?: TranslateParams, loc: Locale = locale.value): string {
  const primary = lookup(messages[loc], key)
  const fallback = loc === 'zh' ? undefined : lookup(messages.zh, key)
  const template = primary ?? fallback ?? key
  return formatMessage(template, params)
}

/** Reactive i18n for Vue components. */
export function useI18n() {
  const current = computed(() => locale.value)
  const isZh = computed(() => locale.value === 'zh')

  function translate(key: string, params?: TranslateParams): string {
    // Depend on locale so templates re-render.
    void locale.value
    return t(key, params, locale.value)
  }

  function toggleLocale() {
    setLocale(locale.value === 'zh' ? 'en' : 'zh')
  }

  return {
    locale: current,
    isZh,
    t: translate,
    setLocale,
    toggleLocale,
  }
}

// Apply html lang on module load (browser).
if (typeof document !== 'undefined') {
  document.documentElement.lang = locale.value === 'zh' ? 'zh-CN' : 'en'
}

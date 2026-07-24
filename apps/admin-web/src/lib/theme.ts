/** Light / dark theme for AnyLive admin (Claude warm palette). */

export type Theme = 'dark' | 'light'

export const THEME_STORAGE_KEY = 'anylive_admin_theme_v1'

export function isTheme(value: unknown): value is Theme {
  return value === 'dark' || value === 'light'
}

export function readStoredTheme(
  storage: Pick<Storage, 'getItem'> | null | undefined = typeof localStorage !== 'undefined'
    ? localStorage
    : null,
): Theme | null {
  if (!storage) return null
  try {
    const raw = storage.getItem(THEME_STORAGE_KEY)
    return isTheme(raw) ? raw : null
  } catch {
    return null
  }
}

/** Prefer stored theme; otherwise follow OS preference. */
export function resolveInitialTheme(
  storage: Pick<Storage, 'getItem'> | null | undefined = typeof localStorage !== 'undefined'
    ? localStorage
    : null,
  prefersDark: boolean | null = typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function'
    ? window.matchMedia('(prefers-color-scheme: dark)').matches
    : null,
): Theme {
  const stored = readStoredTheme(storage)
  if (stored) return stored
  if (prefersDark === false) return 'light'
  return 'dark'
}

export function persistTheme(
  theme: Theme,
  storage: Pick<Storage, 'setItem'> | null | undefined = typeof localStorage !== 'undefined'
    ? localStorage
    : null,
): void {
  if (!storage) return
  try {
    storage.setItem(THEME_STORAGE_KEY, theme)
  } catch {
    // private mode / quota
  }
}

/** Apply theme to <html data-theme> + color-scheme. */
export function applyTheme(theme: Theme, root: HTMLElement | null = typeof document !== 'undefined' ? document.documentElement : null): void {
  if (!root) return
  root.setAttribute('data-theme', theme)
  root.style.colorScheme = theme
}

export function toggleTheme(current: Theme): Theme {
  return current === 'dark' ? 'light' : 'dark'
}

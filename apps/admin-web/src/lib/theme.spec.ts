import { describe, expect, it } from 'vitest'
import {
  THEME_STORAGE_KEY,
  applyTheme,
  isTheme,
  persistTheme,
  readStoredTheme,
  resolveInitialTheme,
  toggleTheme,
} from './theme'

function memoryStorage(seed: Record<string, string> = {}) {
  const map = new Map(Object.entries(seed))
  return {
    getItem: (k: string) => (map.has(k) ? map.get(k)! : null),
    setItem: (k: string, v: string) => {
      map.set(k, v)
    },
    removeItem: (k: string) => {
      map.delete(k)
    },
  }
}

describe('theme helpers', () => {
  it('validates theme values', () => {
    expect(isTheme('dark')).toBe(true)
    expect(isTheme('light')).toBe(true)
    expect(isTheme('system')).toBe(false)
  })

  it('toggles dark ↔ light', () => {
    expect(toggleTheme('dark')).toBe('light')
    expect(toggleTheme('light')).toBe('dark')
  })

  it('reads and persists storage', () => {
    const s = memoryStorage()
    expect(readStoredTheme(s)).toBeNull()
    persistTheme('light', s)
    expect(readStoredTheme(s)).toBe('light')
    expect(s.getItem(THEME_STORAGE_KEY)).toBe('light')
  })

  it('prefers stored over OS', () => {
    const s = memoryStorage({ [THEME_STORAGE_KEY]: 'light' })
    expect(resolveInitialTheme(s, true)).toBe('light')
  })

  it('falls back to OS when unset', () => {
    const s = memoryStorage()
    expect(resolveInitialTheme(s, true)).toBe('dark')
    expect(resolveInitialTheme(s, false)).toBe('light')
  })

  it('applies data-theme on root', () => {
    const el = document.createElement('html')
    applyTheme('light', el)
    expect(el.getAttribute('data-theme')).toBe('light')
    expect(el.style.colorScheme).toBe('light')
    applyTheme('dark', el)
    expect(el.getAttribute('data-theme')).toBe('dark')
  })
})

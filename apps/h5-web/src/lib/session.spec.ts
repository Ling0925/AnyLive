import { describe, expect, it } from 'vitest'
import { isLoggedIn, normalizeEmail, parseAuthSession } from './session'

describe('normalizeEmail', () => {
  it('trims and lowercases', () => {
    expect(normalizeEmail('  Foo@Example.COM ')).toBe('foo@example.com')
  })

  it('handles empty', () => {
    expect(normalizeEmail('')).toBe('')
    expect(normalizeEmail('   ')).toBe('')
  })
})

describe('isLoggedIn', () => {
  it('true for non-empty token', () => {
    expect(isLoggedIn('abc')).toBe(true)
    expect(isLoggedIn('  tok  ')).toBe(true)
  })

  it('false for empty / missing', () => {
    expect(isLoggedIn('')).toBe(false)
    expect(isLoggedIn('   ')).toBe(false)
    expect(isLoggedIn(null)).toBe(false)
    expect(isLoggedIn(undefined)).toBe(false)
  })
})

describe('parseAuthSession', () => {
  it('parses verify response', () => {
    const s = parseAuthSession({
      user: {
        id: 'u1',
        display_name: 'Alice',
        email: 'a@b.com',
      },
      access_token: 'at',
      refresh_token: 'rt',
      expires_in: 3600,
    })
    expect(s).toEqual({
      userId: 'u1',
      displayName: 'Alice',
      email: 'a@b.com',
      accessToken: 'at',
      refreshToken: 'rt',
      expiresIn: 3600,
    })
  })

  it('allows null email', () => {
    const s = parseAuthSession({
      user: { id: 'u1', display_name: 'Bob' },
      access_token: 'at',
      refresh_token: 'rt',
      expires_in: 1,
    })
    expect(s?.email).toBeNull()
  })

  it('returns null on missing token or user', () => {
    expect(parseAuthSession(null)).toBeNull()
    expect(parseAuthSession({})).toBeNull()
    expect(parseAuthSession({ user: { id: 'u' } })).toBeNull()
    expect(parseAuthSession({ access_token: 'x' })).toBeNull()
  })
})

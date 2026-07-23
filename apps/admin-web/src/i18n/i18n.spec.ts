import { describe, expect, it } from 'vitest'
import { formatMessage, getLocale, setLocale, t } from './index'
import { messages } from './messages'

describe('i18n', () => {
  it('defaults to zh messages for known keys', () => {
    setLocale('zh')
    expect(getLocale()).toBe('zh')
    expect(t('nav.dashboard')).toBe(messages.zh.nav.dashboard)
    expect(t('login.headline')).toBe('欢迎回来')
  })

  it('switches to en', () => {
    setLocale('en')
    expect(t('nav.dashboard')).toBe('Dashboard')
    expect(t('login.headline')).toBe('Welcome back')
    setLocale('zh')
  })

  it('formats placeholders', () => {
    expect(formatMessage('hi {name}', { name: 'ops' })).toBe('hi ops')
    setLocale('en')
    expect(t('rooms.forceClosed', { id: 'abc…' })).toContain('abc…')
    setLocale('zh')
  })

  it('falls back to key when missing', () => {
    expect(t('definitely.missing.key')).toBe('definitely.missing.key')
  })
})

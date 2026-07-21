import { describe, expect, it } from 'vitest'
import { buildShareUrl, isRoomEnded, readRoomFromQuery } from './share'

describe('readRoomFromQuery', () => {
  it('reads room from ?room=', () => {
    expect(readRoomFromQuery('?room=abc-123')).toBe('abc-123')
  })

  it('reads room without leading ?', () => {
    expect(readRoomFromQuery('room=abc-123&x=1')).toBe('abc-123')
  })

  it('returns empty when missing', () => {
    expect(readRoomFromQuery('')).toBe('')
    expect(readRoomFromQuery('?foo=bar')).toBe('')
  })

  it('trims whitespace', () => {
    expect(readRoomFromQuery('?room=%20uuid%20')).toBe('uuid')
  })
})

describe('buildShareUrl', () => {
  it('sets room on absolute url', () => {
    expect(buildShareUrl('https://watch.example/h5', 'r1')).toBe(
      'https://watch.example/h5?room=r1',
    )
  })

  it('replaces existing room param', () => {
    expect(buildShareUrl('https://watch.example/h5?room=old&x=1', 'new')).toBe(
      'https://watch.example/h5?room=new&x=1',
    )
  })

  it('works with relative paths', () => {
    expect(buildShareUrl('/watch', 'uuid-1')).toBe('/watch?room=uuid-1')
    expect(buildShareUrl('/watch?foo=1', 'uuid-1')).toBe('/watch?foo=1&room=uuid-1')
  })

  it('preserves hash on relative paths', () => {
    expect(buildShareUrl('/watch#player', 'r')).toBe('/watch?room=r#player')
  })
})

describe('isRoomEnded', () => {
  it('treats closed and idle as ended', () => {
    expect(isRoomEnded('closed')).toBe(true)
    expect(isRoomEnded('idle')).toBe(true)
    expect(isRoomEnded('live')).toBe(false)
    expect(isRoomEnded('')).toBe(false)
  })
})

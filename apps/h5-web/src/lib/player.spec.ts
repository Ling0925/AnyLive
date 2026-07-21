import { describe, expect, it } from 'vitest'
import { buildPlayUrl, isLiveStatus } from './player'

describe('player helpers', () => {
  it('builds hls url', () => {
    expect(buildPlayUrl('http://localhost:8080/live', 'room1')).toBe(
      'http://localhost:8080/live/room1.m3u8',
    )
  })

  it('detects live status', () => {
    expect(isLiveStatus('live')).toBe(true)
    expect(isLiveStatus('idle')).toBe(false)
  })
})

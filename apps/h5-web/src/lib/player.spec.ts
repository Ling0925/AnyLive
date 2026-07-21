import { describe, expect, it, vi } from 'vitest'
import { attachHls } from './hlsAttach'
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

describe('attachHls', () => {
  it('uses hls.js when supported', () => {
    const destroy = vi.fn()
    const loadSource = vi.fn()
    const attachMedia = vi.fn()
    class FakeHls {
      static isSupported() {
        return true
      }
      loadSource = loadSource
      attachMedia = attachMedia
      destroy = destroy
    }
    const video = document.createElement('video')
    const handle = attachHls(video, 'http://x/a.m3u8', FakeHls as unknown as typeof import('hls.js').default)
    expect(handle.mode).toBe('hls.js')
    expect(loadSource).toHaveBeenCalledWith('http://x/a.m3u8')
    expect(attachMedia).toHaveBeenCalled()
    handle.destroy()
    expect(destroy).toHaveBeenCalled()
  })

  it('falls back to native when hls.js unsupported but canPlayType ok', () => {
    class FakeHls {
      static isSupported() {
        return false
      }
    }
    const video = document.createElement('video')
    vi.spyOn(video, 'canPlayType').mockReturnValue('maybe')
    const handle = attachHls(video, 'http://x/a.m3u8', FakeHls as unknown as typeof import('hls.js').default)
    expect(handle.mode).toBe('native')
    expect(video.src).toContain('http://x/a.m3u8')
    handle.destroy()
  })
})

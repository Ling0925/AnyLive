import { describe, expect, it } from 'vitest'
import {
  ADMIN_NAV,
  adminGiftsPath,
  adminTitle,
  apiUrl,
  auditPath,
  banUserPath,
  buildHls,
  canAccessModule,
  countByStatus,
  forceCloseRoomPath,
  giftsListPath,
  grantAdminPath,
  mePath,
  muteUserPath,
  navLabel,
  normalizeApiBase,
  openReportCount,
  otpSendPath,
  otpVerifyPath,
  reportResolvePath,
  reportsListPath,
  roomPlayPath,
  roomStatusTone,
  roomsPath,
  shortId,
  unmuteUserPath,
  API_PATHS,
} from './admin'

describe('admin helpers', () => {
  it('titles non-prod with env', () => {
    expect(adminTitle('local')).toBe('AnyLive Admin (local)')
    expect(adminTitle('prod')).toBe('AnyLive Admin')
  })

  it('enforces simple RBAC matrix', () => {
    expect(canAccessModule('admin', 'wallet')).toBe(true)
    expect(canAccessModule('moderator', 'rooms')).toBe(true)
    expect(canAccessModule('moderator', 'wallet')).toBe(false)
    expect(canAccessModule('viewer', 'rooms')).toBe(false)
  })

  it('exposes sidebar nav items', () => {
    expect(ADMIN_NAV.map((n) => n.key)).toEqual([
      'dashboard',
      'rooms',
      'reports',
      'gifts',
      'moderation',
      'audit',
    ])
    expect(navLabel('rooms')).toBe('直播间')
  })
})

describe('api path helpers', () => {
  it('normalizes trailing slash on base URL', () => {
    expect(normalizeApiBase('http://localhost:8088/')).toBe('http://localhost:8088')
    expect(normalizeApiBase('http://localhost:8088')).toBe('http://localhost:8088')
  })

  it('joins base and path without double slashes', () => {
    expect(apiUrl('http://localhost:8088/', '/api/v1/gifts')).toBe(
      'http://localhost:8088/api/v1/gifts',
    )
    expect(apiUrl('http://localhost:8088', 'api/v1/gifts')).toBe(
      'http://localhost:8088/api/v1/gifts',
    )
  })

  it('exposes OTP auth paths', () => {
    expect(otpSendPath()).toBe('/api/v1/auth/otp/send')
    expect(otpVerifyPath()).toBe('/api/v1/auth/otp/verify')
    expect(mePath()).toBe('/api/v1/me')
    expect(API_PATHS.otpSend).toBe(otpSendPath())
  })

  it('exposes admin action paths', () => {
    expect(banUserPath()).toBe('/api/v1/admin/ban')
    expect(muteUserPath()).toBe('/api/v1/admin/mute')
    expect(unmuteUserPath()).toBe('/api/v1/admin/unmute')
    expect(forceCloseRoomPath()).toBe('/api/v1/admin/rooms/force-close')
    expect(auditPath()).toBe('/api/v1/admin/audit')
    expect(grantAdminPath()).toBe('/api/v1/admin/grant')
    expect(roomsPath()).toBe('/api/v1/rooms')
  })

  it('selects gifts path based on auth', () => {
    expect(giftsListPath(false)).toBe('/api/v1/gifts')
    expect(giftsListPath(true)).toBe('/api/v1/admin/gifts')
    expect(adminGiftsPath()).toBe(API_PATHS.adminGifts)
  })

  it('exposes reports list and resolve paths', () => {
    expect(reportsListPath()).toBe('/api/v1/admin/reports')
    expect(reportResolvePath('abc-123')).toBe('/api/v1/admin/reports/abc-123')
    expect(reportResolvePath('/abc-123/')).toBe('/api/v1/admin/reports/abc-123')
  })

  it('exposes room play path', () => {
    expect(roomPlayPath('room-1')).toBe('/api/v1/rooms/room-1/media/play')
    expect(roomPlayPath('/room-1/')).toBe('/api/v1/rooms/room-1/media/play')
  })

  it('builds hls url from play response or cdn fallback', () => {
    expect(buildHls({ hls: 'https://cdn.example/live/r1.m3u8' }, 'r1')).toBe(
      'https://cdn.example/live/r1.m3u8',
    )
    expect(buildHls({}, 'room-99')).toBe('http://localhost:8080/live/room-99.m3u8')
  })
})

describe('display helpers', () => {
  it('maps room status tone', () => {
    expect(roomStatusTone('live')).toBe('live')
    expect(roomStatusTone('idle')).toBe('idle')
    expect(roomStatusTone('closed')).toBe('closed')
    expect(roomStatusTone('weird')).toBe('unknown')
  })

  it('shortens ids', () => {
    expect(shortId('')).toBe('—')
    expect(shortId('abcdef12-3456-7890')).toBe('abcdef12…')
    expect(shortId('short')).toBe('short')
  })

  it('counts rooms and open reports for dashboard', () => {
    expect(
      countByStatus(
        [{ status: 'live' }, { status: 'live' }, { status: 'idle' }],
        'live',
      ),
    ).toBe(2)
    expect(openReportCount([{ status: 'open' }, { status: 'resolved' }, {}])).toBe(2)
  })
})

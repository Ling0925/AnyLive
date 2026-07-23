import { describe, expect, it } from 'vitest'
import {
  ADMIN_NAV,
  adminGiftsPath,
  adminGateMessage,
  adminTitle,
  apiUrl,
  auditPath,
  banUserPath,
  buildHls,
  canAccessModule,
  classifyAdminGrant,
  countByStatus,
  createRoomPath,
  forceCloseRoomPath,
  giftsListPath,
  grantAdminPath,
  isAdminForbidden,
  mePath,
  muteUserPath,
  navLabel,
  normalizeApiBase,
  obsServerFromPushUrl,
  openReportCount,
  otpSendPath,
  otpVerifyPath,
  parsePublishInfo,
  reportResolvePath,
  reportsListPath,
  roomPlayPath,
  roomPublishPath,
  roomStartPath,
  roomStatusTone,
  roomStopPath,
  roomsPath,
  shortId,
  unmuteUserPath,
  walletReconcilePath,
  payExpireOrdersPath,
  metricsPath,
  analyticsSummaryPath,
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

  it('classifies grant bootstrap outcomes', () => {
    expect(classifyAdminGrant(204)).toBe('granted')
    expect(classifyAdminGrant(200)).toBe('granted')
    expect(classifyAdminGrant(403, '{"message":"admin only"}')).toBe('bootstrap_closed')
    expect(classifyAdminGrant(409, 'admin bootstrap already claimed')).toBe('conflict')
    expect(classifyAdminGrant(500, 'boom')).toBe('error')
    expect(isAdminForbidden(403)).toBe(true)
    expect(isAdminForbidden(401)).toBe(false)
  })

  it('explains admin gate for non-ops logins', () => {
    const msg = adminGateMessage({
      apiBase: 'http://localhost:8088',
      email: 'viewer@example.com',
    })
    expect(msg).toContain('viewer@example.com')
    expect(msg).toContain('seed-admin-local.sh')
    expect(msg).toContain('admin_users')
  })

  it('exposes sidebar nav items', () => {
    expect(ADMIN_NAV.map((n) => n.key)).toEqual([
      'dashboard',
      'golive',
      'rooms',
      'reports',
      'gifts',
      'moderation',
      'audit',
    ])
    expect(navLabel('rooms')).toBe('直播间')
    expect(navLabel('golive')).toBe('开播')
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
    expect(walletReconcilePath()).toBe('/api/v1/admin/wallet/reconcile')
    expect(payExpireOrdersPath()).toBe('/api/v1/admin/pay/expire-orders')
    expect(analyticsSummaryPath()).toBe('/api/v1/admin/analytics/summary')
    expect(metricsPath()).toBe('/metrics')
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

  it('exposes go-live room lifecycle paths', () => {
    expect(createRoomPath()).toBe('/api/v1/rooms')
    expect(roomStartPath('r1')).toBe('/api/v1/rooms/r1/start')
    expect(roomStopPath('/r1/')).toBe('/api/v1/rooms/r1/stop')
    expect(roomPublishPath('r1')).toBe('/api/v1/rooms/r1/media/publish')
  })

  it('derives OBS server from push_url and stream_key', () => {
    const key = '11111111-1111-1111-1111-111111111111?exp=99&sig=abc'
    expect(
      obsServerFromPushUrl(`rtmp://localhost:1935/live/${key}`, key),
    ).toBe('rtmp://localhost:1935/live')
    expect(obsServerFromPushUrl('rtmp://host/live/foo', 'bar')).toBe(
      'rtmp://host/live',
    )
    expect(obsServerFromPushUrl('', 'x')).toBe('')
  })

  it('parses publish response for OBS fields', () => {
    const key = '11111111-1111-1111-1111-111111111111?exp=99&sig=abc'
    const info = parsePublishInfo({
      push_url: `rtmp://localhost:1935/live/${key}`,
      stream_key: key,
      expires_at: '2099-01-01T00:00:00Z',
    })
    expect(info).toEqual({
      pushUrl: `rtmp://localhost:1935/live/${key}`,
      streamKey: key,
      expiresAt: '2099-01-01T00:00:00Z',
      server: 'rtmp://localhost:1935/live',
    })
    expect(parsePublishInfo({})).toBeNull()
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

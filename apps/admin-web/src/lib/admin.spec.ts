import { describe, expect, it } from 'vitest'
import {
  adminGiftsPath,
  adminTitle,
  apiUrl,
  banUserPath,
  canAccessModule,
  forceCloseRoomPath,
  giftsListPath,
  muteUserPath,
  normalizeApiBase,
  otpSendPath,
  otpVerifyPath,
  reportResolvePath,
  reportsListPath,
  roomsPath,
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
    expect(API_PATHS.otpSend).toBe(otpSendPath())
    expect(API_PATHS.otpVerify).toBe(otpVerifyPath())
  })

  it('exposes admin action paths', () => {
    expect(banUserPath()).toBe('/api/v1/admin/ban')
    expect(muteUserPath()).toBe('/api/v1/admin/mute')
    expect(forceCloseRoomPath()).toBe('/api/v1/admin/rooms/force-close')
    expect(roomsPath()).toBe('/api/v1/rooms')
  })

  it('selects gifts path based on auth', () => {
    expect(giftsListPath(false)).toBe('/api/v1/gifts')
    expect(giftsListPath(true)).toBe('/api/v1/admin/gifts')
    expect(giftsListPath(true)).toBe(API_PATHS.adminGifts)
    expect(giftsListPath(false)).toBe(API_PATHS.publicGifts)
  })

  it('exposes admin gifts POST path', () => {
    expect(adminGiftsPath()).toBe('/api/v1/admin/gifts')
    expect(adminGiftsPath()).toBe(API_PATHS.adminGifts)
  })

  it('exposes reports list and resolve paths', () => {
    expect(reportsListPath()).toBe('/api/v1/admin/reports')
    expect(reportsListPath()).toBe(API_PATHS.adminReports)
    expect(reportResolvePath('abc-123')).toBe('/api/v1/admin/reports/abc-123')
    expect(reportResolvePath('/abc-123/')).toBe('/api/v1/admin/reports/abc-123')
  })
})

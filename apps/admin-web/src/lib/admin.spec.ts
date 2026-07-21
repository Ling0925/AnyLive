import { describe, expect, it } from 'vitest'
import { adminTitle, canAccessModule } from './admin'

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

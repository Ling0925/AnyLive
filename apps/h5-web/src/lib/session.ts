/**
 * Pure session helpers for optional H5 login.
 */

export type AuthSession = {
  userId: string
  displayName: string
  email: string | null
  accessToken: string
  refreshToken: string
  expiresIn: number
}

/** Trim + lowercase email for OTP send/verify. */
export function normalizeEmail(email: string): string {
  return email.trim().toLowerCase()
}

/** True when a non-empty access token is present. */
export function isLoggedIn(token: string | null | undefined): boolean {
  return typeof token === 'string' && token.trim().length > 0
}

/**
 * Parse OTP verify response JSON into a session.
 * Expected shape: `{ user: { id, display_name, email? }, access_token, refresh_token, expires_in }`.
 */
export function parseAuthSession(json: unknown): AuthSession | null {
  if (!json || typeof json !== 'object') return null
  const o = json as Record<string, unknown>
  const user = o.user
  if (!user || typeof user !== 'object') return null
  const u = user as Record<string, unknown>
  const accessToken = typeof o.access_token === 'string' ? o.access_token : ''
  if (!accessToken) return null
  return {
    userId: typeof u.id === 'string' ? u.id : '',
    displayName: typeof u.display_name === 'string' ? u.display_name : '',
    email: typeof u.email === 'string' ? u.email : null,
    accessToken,
    refreshToken: typeof o.refresh_token === 'string' ? o.refresh_token : '',
    expiresIn: typeof o.expires_in === 'number' ? o.expires_in : 0,
  }
}

export function adminTitle(env: string): string {
  return env === 'prod' ? 'AnyLive Admin' : `AnyLive Admin (${env})`
}

export function canAccessModule(role: string, module: string): boolean {
  if (role === 'admin') return true
  if (role === 'moderator') {
    return ['rooms', 'reports', 'users'].includes(module)
  }
  return false
}

/** Normalize API base URL (strip trailing slash). */
export function normalizeApiBase(base: string): string {
  return base.endsWith('/') ? base.slice(0, -1) : base
}

/** Join base URL with an absolute API path. */
export function apiUrl(base: string, path: string): string {
  const root = normalizeApiBase(base)
  const p = path.startsWith('/') ? path : `/${path}`
  return `${root}${p}`
}

export const API_PATHS = {
  otpSend: '/api/v1/auth/otp/send',
  otpVerify: '/api/v1/auth/otp/verify',
  rooms: '/api/v1/rooms',
  publicGifts: '/api/v1/gifts',
  adminGifts: '/api/v1/admin/gifts',
  adminBan: '/api/v1/admin/ban',
  adminMute: '/api/v1/admin/mute',
  adminForceClose: '/api/v1/admin/rooms/force-close',
  adminReports: '/api/v1/admin/reports',
  adminReportResolve: '/api/v1/admin/reports/resolve',
} as const

/** Gifts list path: admin catalog when authed, else public catalog. */
export function giftsListPath(authed: boolean): string {
  return authed ? API_PATHS.adminGifts : API_PATHS.publicGifts
}

/** Admin gift upsert (POST) path. */
export function adminGiftsPath(): string {
  return API_PATHS.adminGifts
}

export function otpSendPath(): string {
  return API_PATHS.otpSend
}

export function otpVerifyPath(): string {
  return API_PATHS.otpVerify
}

export function banUserPath(): string {
  return API_PATHS.adminBan
}

export function muteUserPath(): string {
  return API_PATHS.adminMute
}

export function forceCloseRoomPath(): string {
  return API_PATHS.adminForceClose
}

export function roomsPath(): string {
  return API_PATHS.rooms
}

export function reportsListPath(): string {
  return API_PATHS.adminReports
}

export function reportResolvePath(): string {
  return API_PATHS.adminReportResolve
}

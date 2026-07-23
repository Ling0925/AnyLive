export function adminTitle(env: string): string {
  return env === 'prod' ? 'AnyLive Admin' : `AnyLive Admin (${env})`
}

export function canAccessModule(role: string, module: string): boolean {
  if (role === 'admin') return true
  if (role === 'moderator') {
    return [
      'dashboard',
      'golive',
      'rooms',
      'reports',
      'users',
      'moderation',
      'audit',
    ].includes(module)
  }
  return false
}

/**
 * Classify POST /admin/grant response for first-boot bootstrap UX.
 * - 2xx: granted (bootstrap or existing admin granting)
 * - 403 with "admin only": bootstrap closed and caller is not admin
 * - 409: concurrent bootstrap already claimed
 * - other: transport/API error
 */
export type AdminGrantOutcome =
  | 'granted'
  | 'bootstrap_closed'
  | 'conflict'
  | 'error'

export function classifyAdminGrant(status: number, bodyText = ''): AdminGrantOutcome {
  if (status >= 200 && status < 300) return 'granted'
  if (status === 409) return 'conflict'
  if (status === 403) {
    const t = bodyText.toLowerCase()
    if (t.includes('admin only') || t.includes('forbidden')) return 'bootstrap_closed'
    return 'bootstrap_closed'
  }
  return 'error'
}

/** True when an admin-only route returned 403 (caller lacks ops privilege). */
export function isAdminForbidden(status: number): boolean {
  return status === 403
}

/**
 * Operator-facing copy when login succeeded but admin actions are locked.
 * Bootstrap only works while admin_users is empty.
 */
export function adminGateMessage(opts?: {
  apiBase?: string
  email?: string
}): string {
  const base = opts?.apiBase?.trim() || 'http://localhost:8088'
  const email = opts?.email?.trim()
  const emailHint = email
    ? `当前账号 ${email} 不是管理员。`
    : '当前账号不是管理员。'
  return (
    `${emailHint}自助 bootstrap 仅在 admin_users 为空时可用。` +
    `本地可：① 用已有管理员邮箱登录（如 dogfood 种子 / DOGFOOD_ADMIN_EMAIL）；` +
    `② 运行 scripts/seed-admin-local.sh ${email ? email : '<email>'}；` +
    `③ docker exec 向 admin_users 插入 user_id。API ${base}`
  )
}

/** Sidebar nav keys used by the ops shell. */
export type AdminNavKey =
  | 'dashboard'
  | 'golive'
  | 'rooms'
  | 'reports'
  | 'gifts'
  | 'moderation'
  | 'audit'

export type AdminNavItem = {
  key: AdminNavKey
  label: string
  /** Optional short description for dashboard cards. */
  blurb?: string
}

export const ADMIN_NAV: AdminNavItem[] = [
  { key: 'dashboard', label: '总览', blurb: '直播与运营概览' },
  { key: 'golive', label: '开播', blurb: '网页开播 / OBS 推流凭证' },
  { key: 'rooms', label: '直播间', blurb: '房间列表 / 预览 / 强关' },
  { key: 'reports', label: '举报队列', blurb: '用户举报处理' },
  { key: 'gifts', label: '礼物配置', blurb: '礼物目录管理' },
  { key: 'moderation', label: '处置中心', blurb: '封禁 / 禁言' },
  { key: 'audit', label: '审计日志', blurb: '运营写操作记录' },
]

export function navLabel(key: AdminNavKey): string {
  return ADMIN_NAV.find((n) => n.key === key)?.label ?? key
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
  me: '/api/v1/me',
  rooms: '/api/v1/rooms',
  publicGifts: '/api/v1/gifts',
  adminGifts: '/api/v1/admin/gifts',
  adminBan: '/api/v1/admin/ban',
  adminMute: '/api/v1/admin/mute',
  adminUnmute: '/api/v1/admin/unmute',
  adminForceClose: '/api/v1/admin/rooms/force-close',
  adminReports: '/api/v1/admin/reports',
  adminAudit: '/api/v1/admin/audit',
  adminGrant: '/api/v1/admin/grant',
  adminWalletReconcile: '/api/v1/admin/wallet/reconcile',
  adminPayExpireOrders: '/api/v1/admin/pay/expire-orders',
  adminAnalyticsSummary: '/api/v1/admin/analytics/summary',
  metrics: '/metrics',
} as const

/** Gifts list path: admin catalog when authed, else public catalog. */
export function giftsListPath(authed: boolean): string {
  return authed ? API_PATHS.adminGifts : API_PATHS.publicGifts
}

export function adminGiftsPath(): string {
  return API_PATHS.adminGifts
}

export function otpSendPath(): string {
  return API_PATHS.otpSend
}

export function otpVerifyPath(): string {
  return API_PATHS.otpVerify
}

export function mePath(): string {
  return API_PATHS.me
}

export function banUserPath(): string {
  return API_PATHS.adminBan
}

export function muteUserPath(): string {
  return API_PATHS.adminMute
}

export function unmuteUserPath(): string {
  return API_PATHS.adminUnmute
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

export function auditPath(): string {
  return API_PATHS.adminAudit
}

export function grantAdminPath(): string {
  return API_PATHS.adminGrant
}

export function walletReconcilePath(): string {
  return API_PATHS.adminWalletReconcile
}

export function payExpireOrdersPath(): string {
  return API_PATHS.adminPayExpireOrders
}

export function analyticsSummaryPath(): string {
  return API_PATHS.adminAnalyticsSummary
}

export function metricsPath(): string {
  return API_PATHS.metrics
}

/** PATCH resolve path for a report id: `/api/v1/admin/reports/{id}`. */
export function reportResolvePath(id: string): string {
  const clean = id.replace(/^\/+|\/+$/g, '')
  return `${API_PATHS.adminReports}/${encodeURIComponent(clean)}`
}

/** Public play URLs path: `/api/v1/rooms/{id}/media/play`. */
export function roomPlayPath(id: string): string {
  const clean = id.replace(/^\/+|\/+$/g, '')
  return `${API_PATHS.rooms}/${encodeURIComponent(clean)}/media/play`
}

/** Owner create room: `POST /api/v1/rooms`. */
export function createRoomPath(): string {
  return API_PATHS.rooms
}

/** Owner start live: `POST /api/v1/rooms/{id}/start`. */
export function roomStartPath(id: string): string {
  const clean = id.replace(/^\/+|\/+$/g, '')
  return `${API_PATHS.rooms}/${encodeURIComponent(clean)}/start`
}

/** Owner stop live: `POST /api/v1/rooms/{id}/stop`. */
export function roomStopPath(id: string): string {
  const clean = id.replace(/^\/+|\/+$/g, '')
  return `${API_PATHS.rooms}/${encodeURIComponent(clean)}/stop`
}

/** Owner publish credentials: `POST /api/v1/rooms/{id}/media/publish`. */
export function roomPublishPath(id: string): string {
  const clean = id.replace(/^\/+|\/+$/g, '')
  return `${API_PATHS.rooms}/${encodeURIComponent(clean)}/media/publish`
}

/**
 * OBS Server field from full RTMP push URL + stream key.
 * push_url is rtmp://host/app/stream — OBS wants Server=rtmp://host/app.
 */
export function obsServerFromPushUrl(pushUrl: string, streamKey: string): string {
  const push = (pushUrl || '').trim()
  if (!push) return ''
  const key = (streamKey || '').trim()
  if (key) {
    const suffix = `/${key}`
    if (push.endsWith(suffix)) return push.slice(0, -suffix.length)
  }
  const schemeSep = '://'
  const schemeI = push.indexOf(schemeSep)
  const minI = schemeI >= 0 ? schemeI + schemeSep.length : 0
  const i = push.lastIndexOf('/')
  if (i > minI) return push.slice(0, i)
  return push
}

export type PublishInfo = {
  pushUrl: string
  streamKey: string
  expiresAt: string
  server: string
}

/** Parse media/publish JSON into OBS-ready fields. */
export function parsePublishInfo(json: unknown): PublishInfo | null {
  if (!json || typeof json !== 'object') return null
  const o = json as Record<string, unknown>
  const pushUrl = typeof o.push_url === 'string' ? o.push_url.trim() : ''
  const streamKey = typeof o.stream_key === 'string' ? o.stream_key.trim() : ''
  if (!pushUrl || !streamKey) return null
  return {
    pushUrl,
    streamKey,
    expiresAt: typeof o.expires_at === 'string' ? o.expires_at : '',
    server: obsServerFromPushUrl(pushUrl, streamKey),
  }
}

/**
 * Build an HLS playlist URL from a play-API response body, or fall back to
 * `{cdnBase}/{roomId}.m3u8` when the response has no `hls` field.
 */
export function buildHls(
  play: { hls?: string | null } | null | undefined,
  roomId: string,
  cdnBase = 'http://localhost:8080/live',
): string {
  if (play?.hls) return play.hls
  const b = cdnBase.replace(/\/$/, '')
  const s = roomId.replace(/^\/+|\/+$/g, '')
  return `${b}/${s}.m3u8`
}

/** Room status badge class helper. */
export function roomStatusTone(status: string): 'live' | 'idle' | 'closed' | 'unknown' {
  switch (status) {
    case 'live':
      return 'live'
    case 'idle':
      return 'idle'
    case 'closed':
      return 'closed'
    default:
      return 'unknown'
  }
}

/** Shorten UUID for table display. */
export function shortId(id: string, head = 8): string {
  if (!id) return '—'
  if (id.length <= head + 4) return id
  return `${id.slice(0, head)}…`
}

/** Dashboard KPI helpers (pure, unit-tested). */
export function countByStatus(
  rooms: Array<{ status: string }>,
  status: string,
): number {
  return rooms.filter((r) => r.status === status).length
}

export function openReportCount(
  reports: Array<{ status?: string }>,
): number {
  // Backend list is newest-first; unresolved items typically have status open or omit resolved.
  return reports.filter((r) => !r.status || r.status === 'open').length
}

import { t, getLocale, type Locale } from '../i18n'

export function adminTitle(env: string, locale: Locale = getLocale()): string {
  if (env === 'prod') return t('app.titleProd', undefined, locale)
  return t('app.titleLocal', undefined, locale)
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
    const text = bodyText.toLowerCase()
    if (text.includes('admin only') || text.includes('forbidden')) return 'bootstrap_closed'
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
 * When bootstrap_closed (grant 403 after OTP), keep the same actionable paths.
 */
export function adminGateMessage(
  opts?: {
    apiBase?: string
    email?: string
    /** When true, prefix notes that first-boot grant was rejected (admin_users non-empty). */
    bootstrapClosed?: boolean
  },
  locale: Locale = getLocale(),
): string {
  const base = opts?.apiBase?.trim() || 'http://localhost:8088'
  const email = opts?.email?.trim()
  const emailHint = email
    ? t('gate.notAdmin', { email }, locale)
    : t('gate.notAdminGeneric', undefined, locale)
  const closedHint = opts?.bootstrapClosed
    ? t('gate.bootstrapClosed', undefined, locale)
    : t('gate.bootstrapOpen', undefined, locale)
  const seedEmail = email || '<email>'
  return (
    `${emailHint}${closedHint}` +
    t('gate.paths', { email: seedEmail, base }, locale)
  )
}

/**
 * Compact dashboard preflight hints for dogfood demos.
 * Informational only — never claims the 15min walkthrough is complete.
 */
export type DemoPrepHints = {
  adminOk: boolean
  giftCount: number
  giftSeedCmd: string
  runbookPath: string
  lines: string[]
}

export function demoPrepHints(
  opts: {
    isAdmin: boolean | null
    giftCount: number
  },
  locale: Locale = getLocale(),
): DemoPrepHints {
  const giftSeedCmd = './scripts/dogfood-gift-seed.sh'
  const runbookPath = 'docs/runbooks/admin-ops-15min-demo.md'
  const adminOk = opts.isAdmin === true
  const giftCount = Math.max(0, Number(opts.giftCount) || 0)
  const lines: string[] = []
  if (adminOk) {
    lines.push(t('prep.adminOk', undefined, locale))
  } else if (opts.isAdmin === false) {
    lines.push(t('prep.adminDenied', undefined, locale))
  } else {
    lines.push(t('prep.adminChecking', undefined, locale))
  }
  lines.push(
    giftCount > 0
      ? t('prep.giftsOk', { n: giftCount }, locale)
      : t('prep.giftsEmpty', { cmd: giftSeedCmd }, locale),
  )
  lines.push(t('prep.walkthrough', { path: runbookPath }, locale))
  return { adminOk, giftCount, giftSeedCmd, runbookPath, lines }
}

/** Sidebar nav keys used by the ops shell. */
export type AdminNavKey =
  | 'dashboard'
  | 'golive'
  | 'rooms'
  | 'reports'
  | 'gifts'
  | 'users'
  | 'moderation'
  | 'audit'

export type AdminNavItem = {
  key: AdminNavKey
  /** i18n key under nav.* */
  labelKey: string
  /** i18n key under navBlurb.* */
  blurbKey: string
}

export const ADMIN_NAV: AdminNavItem[] = [
  { key: 'dashboard', labelKey: 'nav.dashboard', blurbKey: 'navBlurb.dashboard' },
  { key: 'golive', labelKey: 'nav.golive', blurbKey: 'navBlurb.golive' },
  { key: 'rooms', labelKey: 'nav.rooms', blurbKey: 'navBlurb.rooms' },
  { key: 'reports', labelKey: 'nav.reports', blurbKey: 'navBlurb.reports' },
  { key: 'gifts', labelKey: 'nav.gifts', blurbKey: 'navBlurb.gifts' },
  { key: 'users', labelKey: 'nav.users', blurbKey: 'navBlurb.users' },
  { key: 'moderation', labelKey: 'nav.moderation', blurbKey: 'navBlurb.moderation' },
  { key: 'audit', labelKey: 'nav.audit', blurbKey: 'navBlurb.audit' },
]

export function navLabel(key: AdminNavKey, locale: Locale = getLocale()): string {
  return t(`nav.${key}`, undefined, locale)
}

export function navBlurb(key: AdminNavKey, locale: Locale = getLocale()): string {
  return t(`navBlurb.${key}`, undefined, locale)
}

/** localStorage key for admin session persistence (access + refresh). */
export const ADMIN_SESSION_KEY = 'anylive_admin_session_v1'

export type AdminSessionSnapshot = {
  accessToken: string
  refreshToken: string
  displayName: string
  userId: string
  email: string
}

/** Read persisted admin session from localStorage (browser only). */
export function loadAdminSession(
  storage: Pick<Storage, 'getItem'> | null | undefined = typeof localStorage !== 'undefined'
    ? localStorage
    : null,
): AdminSessionSnapshot | null {
  if (!storage) return null
  try {
    const raw = storage.getItem(ADMIN_SESSION_KEY)
    if (!raw) return null
    const o = JSON.parse(raw) as Partial<AdminSessionSnapshot>
    const accessToken = typeof o.accessToken === 'string' ? o.accessToken.trim() : ''
    if (!accessToken) return null
    return {
      accessToken,
      refreshToken: typeof o.refreshToken === 'string' ? o.refreshToken : '',
      displayName: typeof o.displayName === 'string' ? o.displayName : '',
      userId: typeof o.userId === 'string' ? o.userId : '',
      email: typeof o.email === 'string' ? o.email : '',
    }
  } catch {
    return null
  }
}

/** Persist admin session; no-op when access is empty. */
export function saveAdminSession(
  snap: AdminSessionSnapshot,
  storage: Pick<Storage, 'setItem'> | null | undefined = typeof localStorage !== 'undefined'
    ? localStorage
    : null,
): void {
  if (!storage) return
  const access = (snap.accessToken || '').trim()
  if (!access) return
  storage.setItem(
    ADMIN_SESSION_KEY,
    JSON.stringify({
      accessToken: access,
      refreshToken: snap.refreshToken || '',
      displayName: snap.displayName || '',
      userId: snap.userId || '',
      email: snap.email || '',
    }),
  )
}

/** Clear persisted admin session. */
export function clearAdminSession(
  storage: Pick<Storage, 'removeItem'> | null | undefined = typeof localStorage !== 'undefined'
    ? localStorage
    : null,
): void {
  if (!storage) return
  try {
    storage.removeItem(ADMIN_SESSION_KEY)
  } catch {
    // ignore quota / private mode
  }
}

/** Human-readable OTP / auth API errors for the login screen. */
export function authErrorMessage(
  status: number,
  bodyText = '',
  locale: Locale = getLocale(),
): string {
  const text = (bodyText || '').trim()
  try {
    const j = JSON.parse(text) as { message?: string; code?: string }
    if (j.message && j.message.trim()) return j.message.trim()
    if (j.code && j.code.trim()) return `${j.code} (HTTP ${status})`
  } catch {
    // fall through
  }
  if (status === 401) return t('auth.badOtp', undefined, locale)
  if (status === 404) return t('auth.needSendFirst', undefined, locale)
  if (status === 429) return t('auth.rateLimit', undefined, locale)
  if (status === 400) return t('auth.badRequest', undefined, locale)
  if (text && text.length < 160) return text
  return t('auth.httpFail', { status }, locale)
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
  tokenRefresh: '/api/v1/auth/token/refresh',
  logout: '/api/v1/auth/logout',
  me: '/api/v1/me',
  rooms: '/api/v1/rooms',
  publicGifts: '/api/v1/gifts',
  adminGifts: '/api/v1/admin/gifts',
  adminBan: '/api/v1/admin/ban',
  adminUnban: '/api/v1/admin/unban',
  adminMute: '/api/v1/admin/mute',
  adminUnmute: '/api/v1/admin/unmute',
  adminForceClose: '/api/v1/admin/rooms/force-close',
  adminUsersBanned: '/api/v1/admin/users/banned',
  adminUsersMuted: '/api/v1/admin/users/muted',
  adminUserModeration: '/api/v1/admin/users',
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

export function tokenRefreshPath(): string {
  return API_PATHS.tokenRefresh
}

export function logoutPath(): string {
  return API_PATHS.logout
}

export function banUserPath(): string {
  return API_PATHS.adminBan
}

export function unbanUserPath(): string {
  return API_PATHS.adminUnban
}

export function muteUserPath(): string {
  return API_PATHS.adminMute
}

export function unmuteUserPath(): string {
  return API_PATHS.adminUnmute
}

export function bannedUsersPath(): string {
  return API_PATHS.adminUsersBanned
}

export function mutedUsersPath(): string {
  return API_PATHS.adminUsersMuted
}

/** Lookup path: `/api/v1/admin/users/{id}/moderation`. */
export function userModerationPath(id: string): string {
  const clean = id.replace(/^\/+|\/+$/g, '')
  return `${API_PATHS.adminUserModeration}/${encodeURIComponent(clean)}/moderation`
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

/** Local filter for room table. */
export function filterRooms<T extends { title: string; id: string; status: string }>(
  rooms: T[],
  query: string,
  statusFilter: string,
): T[] {
  const q = query.trim().toLowerCase()
  return rooms.filter((r) => {
    if (statusFilter && statusFilter !== 'all' && r.status !== statusFilter) return false
    if (!q) return true
    return r.title.toLowerCase().includes(q) || r.id.toLowerCase().includes(q)
  })
}

/** Local filter for audit table. */
export function filterAudit<
  T extends { action: string; target: string; detail: string; actor_id: string },
>(items: T[], query: string): T[] {
  const q = query.trim().toLowerCase()
  if (!q) return items
  return items.filter(
    (a) =>
      a.action.toLowerCase().includes(q) ||
      a.target.toLowerCase().includes(q) ||
      a.detail.toLowerCase().includes(q) ||
      a.actor_id.toLowerCase().includes(q),
  )
}

/** Format ISO / opaque timestamps for tables. */
export function formatTs(raw: string, locale: Locale = getLocale()): string {
  if (!raw) return '—'
  const d = new Date(raw)
  if (Number.isNaN(d.getTime())) return raw
  try {
    return new Intl.DateTimeFormat(locale === 'zh' ? 'zh-CN' : 'en-US', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    }).format(d)
  } catch {
    return raw
  }
}

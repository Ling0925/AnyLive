/** Admin ops console state + API actions. */
import Hls from 'hls.js'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from '../i18n'
import {
  ADMIN_NAV,
  adminGiftsPath,
  adminGateMessage,
  adminTitle,
  apiUrl,
  auditPath,
  authErrorMessage,
  banUserPath,
  unbanUserPath,
  adminUsersPath,
  adminResetPasswordPath,
  adminRevokeSessionsPath,
  passwordLoginPath,
  buildHls,
  classifyAdminGrant,
  clearAdminSession,
  countByStatus,
  createRoomPath,
  demoPrepHints,
  filterAudit,
  filterRooms,
  forceCloseRoomPath,
  formatTs,
  giftsListPath,
  grantAdminPath,
  isAdminForbidden,
  loadAdminSession,
  logoutPath,
  muteUserPath,
  navBlurb,
  navLabel,
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
  saveAdminSession,
  shortId,
  tokenRefreshPath,
  unmuteUserPath,
  walletReconcilePath,
  payExpireOrdersPath,
  metricsPath,
  analyticsSummaryPath,
  bannedUsersPath,
  mutedUsersPath,
  userModerationPath,
  type AdminNavKey,
  type PublishInfo,
} from '../lib/admin'
import {
  applyTheme,
  persistTheme,
  resolveInitialTheme,
  toggleTheme as flipTheme,
  type Theme,
} from '../lib/theme'

export function useAdminApp() {


const { t, locale, setLocale } = useI18n()

const apiBase = import.meta.env.VITE_API_BASE ?? 'http://localhost:8088'
const envLabel = import.meta.env.MODE === 'production' ? 'prod' : 'local'
const title = computed(() => adminTitle(envLabel, locale.value))

/** Claude-warm light/dark; persisted under anylive_admin_theme_v1. */
const theme = ref<Theme>(resolveInitialTheme())
applyTheme(theme.value)

function setTheme(next: Theme) {
  theme.value = next
  applyTheme(next)
  persistTheme(next)
}

function onToggleTheme() {
  setTheme(flipTheme(theme.value))
}

const themeToggleLabel = computed(() =>
  theme.value === 'dark' ? t('theme.toLight') : t('theme.toDark'),
)

const NAV_ICONS: Record<AdminNavKey, string> = {
  dashboard:
    'M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-4 0h4',
  golive:
    'M15 10l4.553-2.069A1 1 0 0121 8.82v6.36a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z',
  rooms: 'M4 6a2 2 0 012-2h12a2 2 0 012 2v12a2 2 0 01-2 2H6a2 2 0 01-2-2V6zm4 4h8M8 14h5',
  reports:
    'M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z',
  gifts:
    'M12 8v13m0-13V6a2 2 0 112 2h-2zm0 0V5.5A2.5 2.5 0 109.5 8H12zm-7 4h14M5 12a2 2 0 110-4h14a2 2 0 110 4M5 12v7a2 2 0 002 2h10a2 2 0 002-2v-7',
  users:
    'M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2M9 11a4 4 0 100-8 4 4 0 000 8zm12 10v-2a4 4 0 00-3-3.87M16 3.13a4 4 0 010 7.75',
  moderation:
    'M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z',
  audit:
    'M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4',
}

const accessToken = ref<string | null>(null)
const refreshToken = ref('')
const displayName = ref('')
const userId = ref('')
const nav = ref<AdminNavKey>('dashboard')
/** null = unknown (pre-check), true = can call admin APIs, false = logged-in but not ops. */
const isAdmin = ref<boolean | null>(null)
const sessionRestoring = ref(true)

const email = ref('')
const password = ref('')
const otpCode = ref('')
const loginBusy = ref(false)
const otpSent = ref(false)
const useOtpLogin = ref(false)
const resendCooldown = ref(0)
let resendTimer: ReturnType<typeof setInterval> | null = null

const roomIdInput = ref('')
const userIdInput = ref('')
const muteUserIdInput = ref('')
const unmuteUserIdInput = ref('')
const actionReason = ref('')
const actionBusy = ref(false)

const giftName = ref('')
const giftPrice = ref('')
const giftBusy = ref(false)
/** When set, gift form upserts this catalog id instead of creating a new item. */
const giftEditId = ref<string | null>(null)
const giftEditActive = ref(true)

const rooms = ref<Array<{ id: string; title: string; status: string; owner_id?: string }>>([])
const gifts = ref<Array<{ id: string; name: string; price: number; active?: boolean }>>([])
const reports = ref<
  Array<{
    id: string
    reporter_id: string
    target_type: string
    target_id: string
    reason: string
    status?: string
    created_at: string
  }>
>([])
const audit = ref<
  Array<{
    id: string
    actor_id: string
    action: string
    target: string
    detail: string
    created_at: string
  }>
>([])

const roomQuery = ref('')
const roomStatusFilter = ref('all')
/** Client-side page for rooms table (API returns full list). */
const roomPage = ref(1)
const roomPageSize = ref(20)
const auditQuery = ref('')

const error = ref('')
const notice = ref('')
const listBusy = ref(false)

const previewRoomId = ref<string | null>(null)
const previewHlsUrl = ref('')
const previewBusy = ref(false)
const previewError = ref('')
const previewVideoEl = ref<HTMLVideoElement | null>(null)
let previewDetach: (() => void) | null = null

// --- go-live ---
const goLiveTitle = ref('')
const goLiveBusy = ref(false)
const goLiveRoomId = ref('')
const goLiveRoomStatus = ref('')
const goLivePublish = ref<PublishInfo | null>(null)
const goLiveHls = ref('')
const goLiveCopyHint = ref('')

const reconcileBusy = ref(false)
const reconcileHint = ref('')
const reconcileBalanced = ref<boolean | null>(null)
const reconcileChecked = ref(0)
const reconcileImbalance = ref(0)
const expireBusy = ref(false)
const expireHint = ref('')
const metricsBusy = ref(false)
const metricsHint = ref('')
const metricsText = ref('')
const metricsLines = ref(0)
const analyticsBusy = ref(false)
const analyticsHint = ref('')
const analyticsRetained = ref(0)
const analyticsUsers = ref(0)
const analyticsByName = ref<Array<{ name: string; count: number }>>([])
const analyticsRecent = ref<
  Array<{ id: string; user_id: string; name: string; occurred_at: string }>
>([])

// --- users admin ---
type AdminUserRow = {
  id: string
  display_name: string
  email?: string | null
  username?: string | null
  status: string
  created_at: string
  banned: boolean
  muted: boolean
  admin_role?: string | null
  must_change_password: boolean
}
const usersList = ref<AdminUserRow[]>([])
const usersTotal = ref(0)
const usersQuery = ref('')
const usersBusy = ref(false)
const createDisplayName = ref('')
const createUsername = ref('')
const createEmail = ref('')
const createPassword = ref('')
const createBusy = ref(false)
const tempPasswordNotice = ref('')
const unbanUserIdInput = ref('')
const lookupUserId = ref('')
const lookupBusy = ref(false)
const lookupStatus = ref<{
  user_id: string
  banned: boolean
  muted: boolean
  ban_reason: string | null
  mute_reason: string | null
  banned_at: string | null
  muted_at: string | null
} | null>(null)
const bannedUsers = ref<Array<{ user_id: string; reason: string; created_at: string }>>([])
const mutedUsers = ref<Array<{ user_id: string; reason: string; created_at: string }>>([])

const isAuthed = computed(() => Boolean(accessToken.value))
const liveCount = computed(() => countByStatus(rooms.value, 'live'))
const idleCount = computed(() => countByStatus(rooms.value, 'idle'))
const closedCount = computed(() => countByStatus(rooms.value, 'closed'))
const reportOpen = computed(() => openReportCount(reports.value))
const pageTitle = computed(() => navLabel(nav.value, locale.value))
const pageBlurb = computed(() => navBlurb(nav.value, locale.value))
const avatarLetter = computed(() => (displayName.value || 'A').slice(0, 1).toUpperCase())
/** Set when POST /admin/grant returns bootstrap_closed so gate copy is more specific. */
const bootstrapClosed = ref(false)

const filteredRooms = computed(() =>
  filterRooms(rooms.value, roomQuery.value, roomStatusFilter.value),
)
const roomTotalPages = computed(() =>
  Math.max(1, Math.ceil(filteredRooms.value.length / roomPageSize.value)),
)
const pagedRooms = computed(() => {
  const size = roomPageSize.value
  const page = Math.min(Math.max(1, roomPage.value), roomTotalPages.value)
  const start = (page - 1) * size
  return filteredRooms.value.slice(start, start + size)
})
const roomListMeta = computed(() => {
  const total = filteredRooms.value.length
  const pages = roomTotalPages.value
  const page = Math.min(Math.max(1, roomPage.value), pages)
  const size = roomPageSize.value
  return {
    total,
    page,
    pages,
    size,
    from: total === 0 ? 0 : (page - 1) * size + 1,
    to: Math.min(page * size, total),
  }
})

watch([roomQuery, roomStatusFilter, roomPageSize], () => {
  roomPage.value = 1
})
watch(roomTotalPages, (pages) => {
  if (roomPage.value > pages) roomPage.value = pages
})

function setRoomPage(next: number) {
  roomPage.value = Math.min(Math.max(1, next), roomTotalPages.value)
}

const filteredAudit = computed(() => filterAudit(audit.value, auditQuery.value))

const adminGateHint = computed(() =>
  isAdmin.value === false
    ? adminGateMessage(
        {
          apiBase,
          email: email.value || displayName.value,
          bootstrapClosed: bootstrapClosed.value,
        },
        locale.value,
      )
    : '',
)
const sessionRoleLabel = computed(() => {
  if (isAdmin.value === true) return t('topbar.roleAdmin')
  if (isAdmin.value === false) return t('topbar.roleNonAdmin')
  return isAuthed.value ? t('topbar.roleChecking') : t('topbar.roleNone')
})
const prepHints = computed(() =>
  demoPrepHints({ isAdmin: isAdmin.value, giftCount: gifts.value.length }, locale.value),
)
const giftSeedCmd = './scripts/dogfood-gift-seed.sh'
const giftSeedCopyHint = ref('')

const loginStep = computed(() => {
  if (useOtpLogin.value) {
    if (otpSent.value || otpCode.value.trim()) return 2
    return 1
  }
  // Password mode: email filled → step 2 (ready to submit)
  if (email.value.trim() && password.value) return 2
  return 1
})

function statusLabel(status: string): string {
  const key = `status.${status}` as const
  const mapped = t(key)
  return mapped === key ? status : mapped
}

async function copyGiftSeedCmd() {
  giftSeedCopyHint.value = ''
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(giftSeedCmd)
      giftSeedCopyHint.value = t('dashboard.seedCopied')
      return
    }
  } catch {
    // fall through
  }
  window.prompt(t('dashboard.copySeed'), giftSeedCmd)
  giftSeedCopyHint.value = t('dashboard.seedReady')
}

function startResendCooldown(sec = 60) {
  resendCooldown.value = sec
  if (resendTimer) clearInterval(resendTimer)
  resendTimer = setInterval(() => {
    resendCooldown.value -= 1
    if (resendCooldown.value <= 0 && resendTimer) {
      clearInterval(resendTimer)
      resendTimer = null
    }
  }, 1000)
}

function authHeaders(json = true): HeadersInit {
  const h: Record<string, string> = {}
  if (json) h['Content-Type'] = 'application/json'
  if (accessToken.value) h.Authorization = `Bearer ${accessToken.value}`
  return h
}

function persistSession() {
  if (!accessToken.value) {
    clearAdminSession()
    return
  }
  saveAdminSession({
    accessToken: accessToken.value,
    refreshToken: refreshToken.value,
    displayName: displayName.value,
    userId: userId.value,
    email: email.value,
  })
}

/** Rotate access via refresh token. Returns true when a new access token is stored. */
async function tryRefreshAccess(): Promise<boolean> {
  const rt = refreshToken.value.trim()
  if (!rt) return false
  try {
    const res = await fetch(apiUrl(apiBase, tokenRefreshPath()), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token: rt }),
    })
    if (!res.ok) return false
    const data = (await res.json()) as {
      access_token?: string
      refresh_token?: string
    }
    const next = (data.access_token || '').trim()
    if (!next) return false
    accessToken.value = next
    if (data.refresh_token) refreshToken.value = data.refresh_token
    persistSession()
    return true
  } catch {
    return false
  }
}

/** fetch with one automatic refresh retry on 401. */
async function apiFetch(input: string, init?: RequestInit): Promise<Response> {
  const res = await fetch(input, init)
  if (res.status !== 401 || !refreshToken.value) return res
  const ok = await tryRefreshAccess()
  if (!ok) return res
  const headers = new Headers(init?.headers)
  if (accessToken.value) headers.set('Authorization', `Bearer ${accessToken.value}`)
  return fetch(input, { ...init, headers })
}

function teardownPreviewPlayer() {
  previewDetach?.()
  previewDetach = null
}

function attachPreviewHls(video: HTMLVideoElement, src: string) {
  teardownPreviewPlayer()
  if (Hls.isSupported()) {
    const hls = new Hls()
    hls.loadSource(src)
    hls.attachMedia(video)
    previewDetach = () => hls.destroy()
    return
  }
  if (video.canPlayType('application/vnd.apple.mpegurl')) {
    video.src = src
    previewDetach = () => {
      video.removeAttribute('src')
      video.load()
    }
    return
  }
  previewError.value = t('rooms.hlsUnsupported')
}

watch([previewVideoEl, previewHlsUrl], ([el, url]) => {
  if (el && url) attachPreviewHls(el, url)
  else teardownPreviewPlayer()
})

onBeforeUnmount(() => {
  teardownPreviewPlayer()
  if (resendTimer) clearInterval(resendTimer)
})

function go(key: AdminNavKey) {
  nav.value = key
  notice.value = ''
  error.value = ''
  if (key === 'users' && accessToken.value) {
    void loadUsers()
    void loadModerationLists()
  }
}

async function previewRoom(room: { id: string; status: string }) {
  previewError.value = ''
  if (room.status !== 'live') {
    previewError.value = t('rooms.notLive')
    previewRoomId.value = room.id
    previewHlsUrl.value = ''
    await nextTick()
    document.querySelector('[data-testid="room-preview-panel"]')?.scrollIntoView({
      behavior: 'smooth',
      block: 'nearest',
    })
    return
  }
  previewBusy.value = true
  previewRoomId.value = room.id
  previewHlsUrl.value = ''
  try {
    const res = await apiFetch(apiUrl(apiBase, roomPlayPath(room.id)))
    if (!res.ok) throw new Error(`play ${res.status}`)
    const play = await res.json()
    previewHlsUrl.value = buildHls(play, room.id)
    await nextTick()
    document.querySelector('[data-testid="room-preview-panel"]')?.scrollIntoView({
      behavior: 'smooth',
      block: 'nearest',
    })
  } catch (e) {
    previewError.value = String(e)
  } finally {
    previewBusy.value = false
  }
}

function closePreview() {
  teardownPreviewPlayer()
  previewRoomId.value = null
  previewHlsUrl.value = ''
  previewError.value = ''
}

function useRoomId(id: string) {
  roomIdInput.value = id
  nav.value = 'moderation'
}

function useUserId(id: string) {
  userIdInput.value = id
  muteUserIdInput.value = id
  nav.value = 'moderation'
}

async function loadRooms() {
  const res = await apiFetch(apiUrl(apiBase, roomsPath()))
  if (!res.ok) throw new Error(`rooms ${res.status}`)
  const data = await res.json()
  rooms.value = data.items ?? []
}

async function loadGifts() {
  const path = giftsListPath(isAuthed.value)
  const res = await apiFetch(apiUrl(apiBase, path), { headers: authHeaders(false) })
  if (!res.ok) throw new Error(`gifts ${res.status}`)
  const data = await res.json()
  gifts.value = data.items ?? []
}

async function loadReports() {
  if (!accessToken.value) {
    reports.value = []
    return
  }
  const res = await apiFetch(apiUrl(apiBase, reportsListPath()), { headers: authHeaders(false) })
  if (isAdminForbidden(res.status)) {
    isAdmin.value = false
    reports.value = []
    return
  }
  if (!res.ok) throw new Error(`reports ${res.status}`)
  const data = await res.json()
  reports.value = data.items ?? []
  if (isAdmin.value !== true) isAdmin.value = true
}

async function loadAudit() {
  if (!accessToken.value) {
    audit.value = []
    return
  }
  const res = await apiFetch(apiUrl(apiBase, auditPath()), { headers: authHeaders(false) })
  if (isAdminForbidden(res.status)) {
    isAdmin.value = false
    audit.value = []
    return
  }
  if (!res.ok) throw new Error(`audit ${res.status}`)
  const data = await res.json()
  audit.value = data.items ?? []
  if (isAdmin.value !== true) isAdmin.value = true
}

async function refreshLists() {
  listBusy.value = true
  // Do not clear a sticky admin-gate error while refreshing rooms.
  if (isAdmin.value !== false) error.value = ''
  try {
    const tasks: Promise<void>[] = [loadRooms(), loadGifts()]
    if (isAuthed.value) {
      tasks.push(loadReports(), loadAudit(), loadUsers(), loadModerationLists())
    } else {
      reports.value = []
      audit.value = []
      usersList.value = []
      bannedUsers.value = []
      mutedUsers.value = []
    }
    await Promise.all(tasks)
    if (isAdmin.value === true) {
      bootstrapClosed.value = false
    }
    if (isAdmin.value === false && !error.value) {
      error.value = adminGateHint.value
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    listBusy.value = false
  }
}

async function runWalletReconcile() {
  if (!isAuthed.value) return
  reconcileBusy.value = true
  reconcileHint.value = ''
  try {
    const res = await apiFetch(apiUrl(apiBase, walletReconcilePath()), {
      headers: authHeaders(false),
    })
    if (!res.ok) throw new Error(`reconcile ${res.status}`)
    const data = await res.json()
    reconcileChecked.value = Number(data.checked_users ?? 0)
    reconcileImbalance.value = Number(data.imbalance_count ?? 0)
    reconcileBalanced.value = Boolean(data.balanced)
    reconcileHint.value = data.balanced
      ? t('dashboard.reconcileBalanced', { n: reconcileChecked.value })
      : t('dashboard.reconcileImbalance', {
          n: reconcileImbalance.value,
          m: reconcileChecked.value,
        })
  } catch (e) {
    reconcileHint.value = String(e)
    reconcileBalanced.value = null
  } finally {
    reconcileBusy.value = false
  }
}

async function runExpirePayOrders() {
  if (!isAuthed.value) return
  expireBusy.value = true
  expireHint.value = ''
  try {
    const res = await apiFetch(apiUrl(apiBase, payExpireOrdersPath()), {
      method: 'POST',
      headers: authHeaders(),
    })
    if (!res.ok) throw new Error(`expire-orders ${res.status}`)
    const data = await res.json()
    expireHint.value = t('dashboard.expireDone', { n: Number(data.expired_count ?? 0) })
  } catch (e) {
    expireHint.value = String(e)
  } finally {
    expireBusy.value = false
  }
}

async function runMetricsScrape() {
  metricsBusy.value = true
  metricsHint.value = ''
  try {
    const res = await apiFetch(apiUrl(apiBase, metricsPath()))
    if (!res.ok) throw new Error(`metrics ${res.status}`)
    const text = await res.text()
    metricsText.value = text
    metricsLines.value = text.split('\n').filter((l) => l.trim() && !l.startsWith('#')).length
    metricsHint.value = t('dashboard.metricsOk', { n: metricsLines.value })
  } catch (e) {
    metricsHint.value = String(e)
    metricsText.value = ''
    metricsLines.value = 0
  } finally {
    metricsBusy.value = false
  }
}

async function runAnalyticsSummary() {
  if (!isAuthed.value) return
  analyticsBusy.value = true
  analyticsHint.value = ''
  try {
    const res = await apiFetch(apiUrl(apiBase, analyticsSummaryPath()), {
      headers: authHeaders(false),
    })
    if (!res.ok) throw new Error(`analytics ${res.status}`)
    const data = await res.json()
    analyticsRetained.value = Number(data.retained_events ?? 0)
    analyticsUsers.value = Number(data.distinct_users ?? 0)
    analyticsByName.value = Array.isArray(data.by_name)
      ? data.by_name.map((r: { name?: string; count?: number }) => ({
          name: String(r.name ?? ''),
          count: Number(r.count ?? 0),
        }))
      : []
    analyticsRecent.value = Array.isArray(data.recent)
      ? data.recent.map(
          (r: {
            id?: string
            user_id?: string
            name?: string
            occurred_at?: string
          }) => ({
            id: String(r.id ?? ''),
            user_id: String(r.user_id ?? ''),
            name: String(r.name ?? ''),
            occurred_at: String(r.occurred_at ?? ''),
          }),
        )
      : []
    analyticsHint.value = t('dashboard.analyticsHint', {
      n: analyticsRetained.value,
      m: analyticsUsers.value,
    })
  } catch (e) {
    analyticsHint.value = String(e)
    analyticsByName.value = []
    analyticsRecent.value = []
  } finally {
    analyticsBusy.value = false
  }
}

async function sendOtp() {
  notice.value = ''
  error.value = ''
  const em = email.value.trim()
  if (!em || !em.includes('@')) {
    error.value = t('flash.needEmail')
    return
  }
  loginBusy.value = true
  try {
    const res = await apiFetch(apiUrl(apiBase, otpSendPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({ email: em }),
    })
    if (res.status !== 204) {
      const bodyText = await res.text().catch(() => '')
      throw new Error(authErrorMessage(res.status, bodyText, locale.value))
    }
    otpSent.value = true
    notice.value = t('flash.otpSent')
    startResendCooldown(60)
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loginBusy.value = false
  }
}

async function applySessionFromAuth(data: {
  access_token?: string
  refresh_token?: string
  user?: { display_name?: string; email?: string; id?: string; username?: string }
}) {
  accessToken.value = data.access_token ?? null
  refreshToken.value = data.refresh_token ?? ''
  displayName.value =
    data.user?.display_name ?? data.user?.username ?? data.user?.email ?? email.value
  userId.value = data.user?.id ?? ''
  isAdmin.value = null
  persistSession()
  notice.value = t('flash.loginOk')
  nav.value = 'dashboard'
  if (userId.value) {
    await tryBootstrapAdmin(userId.value)
  }
  await refreshLists()
}

async function passwordLogin() {
  notice.value = ''
  error.value = ''
  const id = email.value.trim()
  if (!id || !password.value) {
    error.value = t('flash.needEmail')
    return
  }
  loginBusy.value = true
  try {
    const res = await apiFetch(apiUrl(apiBase, passwordLoginPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        identifier: id,
        password: password.value,
      }),
    })
    if (!res.ok) {
      const bodyText = await res.text().catch(() => '')
      throw new Error(authErrorMessage(res.status, bodyText, locale.value))
    }
    const data = await res.json()
    await applySessionFromAuth(data)
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loginBusy.value = false
  }
}

async function verifyOtp() {
  notice.value = ''
  error.value = ''
  loginBusy.value = true
  try {
    const res = await apiFetch(apiUrl(apiBase, otpVerifyPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        email: email.value.trim(),
        code: otpCode.value.trim(),
      }),
    })
    if (!res.ok) {
      const bodyText = await res.text().catch(() => '')
      throw new Error(authErrorMessage(res.status, bodyText, locale.value))
    }
    const data = await res.json()
    await applySessionFromAuth(data)
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loginBusy.value = false
  }
}

async function tryBootstrapAdmin(id: string) {
  try {
    const res = await apiFetch(apiUrl(apiBase, grantAdminPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({ user_id: id }),
    })
    const bodyText = await res.text().catch(() => '')
    const outcome = classifyAdminGrant(res.status, bodyText)
    if (outcome === 'granted') {
      isAdmin.value = true
      bootstrapClosed.value = false
      notice.value = t('flash.bootstrapOk')
      return
    }
    if (outcome === 'bootstrap_closed' || outcome === 'conflict') {
      // Privilege still unknown until audit/reports probe in refreshLists.
      // Flag closed bootstrap so admin-gate copy is actionable when probe fails.
      bootstrapClosed.value = true
      isAdmin.value = null
      notice.value =
        outcome === 'conflict' ? t('flash.bootstrapConflict') : t('flash.bootstrapClosed')
      return
    }
    // network-ish error: leave isAdmin unknown
  } catch {
    // ignore transport errors; refreshLists will probe
  }
}

async function logout() {
  const rt = refreshToken.value.trim()
  const at = accessToken.value
  // Best-effort server revoke; always clear local state.
  if (at || rt) {
    try {
      await fetch(apiUrl(apiBase, logoutPath()), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...(at ? { Authorization: `Bearer ${at}` } : {}),
        },
        body: JSON.stringify(rt ? { refresh_token: rt } : {}),
      })
    } catch {
      // ignore
    }
  }
  accessToken.value = null
  refreshToken.value = ''
  displayName.value = ''
  userId.value = ''
  isAdmin.value = null
  bootstrapClosed.value = false
  otpSent.value = false
  otpCode.value = ''
  clearAdminSession()
  notice.value = t('flash.logoutOk')
  cancelEditGift()
  closePreview()
  void refreshLists()
}

async function restoreSession() {
  sessionRestoring.value = true
  try {
    const snap = loadAdminSession()
    if (!snap) return
    accessToken.value = snap.accessToken
    refreshToken.value = snap.refreshToken
    displayName.value = snap.displayName
    userId.value = snap.userId
    if (snap.email) email.value = snap.email
    // Validate access; rotate on 401.
    try {
      const meRes = await fetch(apiUrl(apiBase, '/api/v1/me'), {
        headers: authHeaders(false),
      })
      if (meRes.status === 401) {
        const ok = await tryRefreshAccess()
        if (!ok) {
          accessToken.value = null
          refreshToken.value = ''
          clearAdminSession()
          notice.value = t('flash.sessionExpired')
          return
        }
      } else if (!meRes.ok) {
        // Keep token; lists may still work or surface errors.
      } else {
        const me = (await meRes.json().catch(() => null)) as {
          display_name?: string
          email?: string
          id?: string
        } | null
        if (me?.display_name) displayName.value = me.display_name
        if (me?.email) {
          displayName.value = displayName.value || me.email
          email.value = email.value || me.email
        }
        if (me?.id) userId.value = me.id
      }
    } catch {
      // offline — keep local session for offline shell
    }
    notice.value = t('flash.sessionRestored')
    await refreshLists()
  } finally {
    sessionRestoring.value = false
  }
}

async function forceCloseRoom(id?: string) {
  const roomId = (id ?? roomIdInput.value).trim()
  if (!accessToken.value) {
    error.value = t('flash.needLogin')
    return
  }
  if (!roomId) {
    error.value = t('rooms.needRoomId')
    return
  }
  notice.value = ''
  error.value = ''
  actionBusy.value = true
  try {
    const res = await apiFetch(apiUrl(apiBase, forceCloseRoomPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        room_id: roomId,
        reason: actionReason.value.trim() || undefined,
      }),
    })
    if (isAdminForbidden(res.status)) {
      isAdmin.value = false
      error.value =
        adminGateHint.value || adminGateMessage({ apiBase, email: email.value }, locale.value)
      return
    }
    if (!res.ok) throw new Error(`force-close ${res.status}`)
    notice.value = t('rooms.forceClosed', { id: shortId(roomId) })
    roomIdInput.value = ''
    if (goLiveRoomId.value === roomId) {
      goLiveRoomStatus.value = 'closed'
      goLivePublish.value = null
      goLiveHls.value = ''
    }
    await Promise.all([loadRooms(), loadAudit()])
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

async function copyText(label: string, text: string) {
  goLiveCopyHint.value = ''
  if (!text) return
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text)
      goLiveCopyHint.value = t('golive.copyLabel', { label })
      return
    }
  } catch {
    // fall through
  }
  window.prompt(t('common.copy') + ' ' + label, text)
  goLiveCopyHint.value = t('golive.copyReady', { label })
}

/** One-click go-live: create → start → publish creds → HLS. */
async function goLiveStart() {
  if (!accessToken.value) {
    error.value = t('flash.needLogin')
    return
  }
  const titleText = goLiveTitle.value.trim() || t('golive.liveTitlePlaceholder')
  notice.value = ''
  error.value = ''
  goLiveCopyHint.value = ''
  goLiveBusy.value = true
  try {
    const createRes = await apiFetch(apiUrl(apiBase, createRoomPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({ title: titleText }),
    })
    if (!createRes.ok) throw new Error(`create room ${createRes.status}`)
    const room = await createRes.json()
    const rid = String(room.id || '')
    if (!rid) throw new Error('create room missing id')
    goLiveRoomId.value = rid

    const startRes = await apiFetch(apiUrl(apiBase, roomStartPath(rid)), {
      method: 'POST',
      headers: authHeaders(),
    })
    if (!startRes.ok) throw new Error(`start ${startRes.status}`)
    const started = await startRes.json()
    goLiveRoomStatus.value = String(started.status || 'live')

    const pubRes = await apiFetch(apiUrl(apiBase, roomPublishPath(rid)), {
      method: 'POST',
      headers: authHeaders(),
    })
    if (!pubRes.ok) throw new Error(`publish ${pubRes.status}`)
    const pubJson = await pubRes.json()
    const info = parsePublishInfo(pubJson)
    if (!info) throw new Error('publish parse failed')
    goLivePublish.value = info

    try {
      const playRes = await apiFetch(apiUrl(apiBase, roomPlayPath(rid)))
      if (playRes.ok) {
        const play = await playRes.json()
        goLiveHls.value = buildHls(play, rid)
      } else {
        goLiveHls.value = buildHls(null, rid)
      }
    } catch {
      goLiveHls.value = buildHls(null, rid)
    }

    notice.value = t('golive.started', { title: titleText, id: shortId(rid) })
    await loadRooms()
  } catch (e) {
    error.value = String(e)
  } finally {
    goLiveBusy.value = false
  }
}

/** Refresh publish creds for current go-live room. */
async function goLiveRefreshPublish() {
  const rid = goLiveRoomId.value.trim()
  if (!accessToken.value || !rid) {
    error.value = t('golive.needRoom')
    return
  }
  goLiveBusy.value = true
  error.value = ''
  try {
    const pubRes = await apiFetch(apiUrl(apiBase, roomPublishPath(rid)), {
      method: 'POST',
      headers: authHeaders(),
    })
    if (!pubRes.ok) throw new Error(`publish refresh ${pubRes.status}`)
    const info = parsePublishInfo(await pubRes.json())
    if (!info) throw new Error('publish parse failed')
    goLivePublish.value = info
    const playRes = await apiFetch(apiUrl(apiBase, roomPlayPath(rid)))
    if (playRes.ok) {
      goLiveHls.value = buildHls(await playRes.json(), rid)
    }
    notice.value = t('golive.refreshed')
  } catch (e) {
    error.value = String(e)
  } finally {
    goLiveBusy.value = false
  }
}

async function goLiveStop() {
  const rid = goLiveRoomId.value.trim()
  if (!accessToken.value || !rid) {
    error.value = t('golive.noRoom')
    return
  }
  goLiveBusy.value = true
  error.value = ''
  try {
    const res = await apiFetch(apiUrl(apiBase, roomStopPath(rid)), {
      method: 'POST',
      headers: authHeaders(),
    })
    if (!res.ok) throw new Error(`stop ${res.status}`)
    const room = await res.json()
    goLiveRoomStatus.value = String(room.status || 'idle')
    goLivePublish.value = null
    goLiveHls.value = ''
    notice.value = t('golive.stopped', { id: shortId(rid) })
    await loadRooms()
  } catch (e) {
    error.value = String(e)
  } finally {
    goLiveBusy.value = false
  }
}

/** Re-issue publish credentials for an existing room. */
async function loadPublishForRoom(id: string) {
  if (!accessToken.value) {
    error.value = t('flash.needLogin')
    return
  }
  const rid = id.trim()
  if (!rid) return
  goLiveBusy.value = true
  error.value = ''
  nav.value = 'golive'
  try {
    goLiveRoomId.value = rid
    const room = rooms.value.find((r) => r.id === rid)
    goLiveRoomStatus.value = room?.status || ''
    if (goLiveRoomStatus.value === 'idle') {
      const startRes = await apiFetch(apiUrl(apiBase, roomStartPath(rid)), {
        method: 'POST',
        headers: authHeaders(),
      })
      if (!startRes.ok) throw new Error(`start ${startRes.status}`)
      const started = await startRes.json()
      goLiveRoomStatus.value = String(started.status || 'live')
    }
    const pubRes = await apiFetch(apiUrl(apiBase, roomPublishPath(rid)), {
      method: 'POST',
      headers: authHeaders(),
    })
    if (!pubRes.ok) throw new Error(`publish ${pubRes.status}`)
    const info = parsePublishInfo(await pubRes.json())
    if (!info) throw new Error('publish parse failed')
    goLivePublish.value = info
    const playRes = await apiFetch(apiUrl(apiBase, roomPlayPath(rid)))
    goLiveHls.value = playRes.ok ? buildHls(await playRes.json(), rid) : buildHls(null, rid)
    notice.value = t('golive.loaded', { id: shortId(rid) })
    await loadRooms()
  } catch (e) {
    error.value = String(e)
  } finally {
    goLiveBusy.value = false
  }
}

async function banUser() {
  if (!accessToken.value) {
    error.value = t('flash.needLogin')
    return
  }
  const id = userIdInput.value.trim()
  if (!id) {
    error.value = t('moderation.needUserId')
    return
  }
  notice.value = ''
  error.value = ''
  actionBusy.value = true
  try {
    const res = await apiFetch(apiUrl(apiBase, banUserPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        user_id: id,
        reason: actionReason.value.trim() || undefined,
      }),
    })
    if (res.status !== 204) throw new Error(`ban ${res.status}`)
    notice.value = t('moderation.banned', { id: shortId(id) })
    userIdInput.value = ''
    await Promise.all([loadAudit(), loadUsers(), loadModerationLists()])
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

async function loadBannedUsers() {
  if (!accessToken.value) {
    bannedUsers.value = []
    return
  }
  const res = await apiFetch(apiUrl(apiBase, bannedUsersPath()), { headers: authHeaders(false) })
  if (isAdminForbidden(res.status)) {
    isAdmin.value = false
    bannedUsers.value = []
    return
  }
  if (!res.ok) throw new Error(`banned users ${res.status}`)
  const data = await res.json()
  bannedUsers.value = data.items ?? []
  if (isAdmin.value !== true) isAdmin.value = true
}

async function loadMutedUsers() {
  if (!accessToken.value) {
    mutedUsers.value = []
    return
  }
  const res = await apiFetch(apiUrl(apiBase, mutedUsersPath()), { headers: authHeaders(false) })
  if (isAdminForbidden(res.status)) {
    isAdmin.value = false
    mutedUsers.value = []
    return
  }
  if (!res.ok) throw new Error(`muted users ${res.status}`)
  const data = await res.json()
  mutedUsers.value = data.items ?? []
  if (isAdmin.value !== true) isAdmin.value = true
}

async function loadModerationLists() {
  await Promise.all([loadBannedUsers(), loadMutedUsers()])
}

async function lookupUserModeration() {
  if (!accessToken.value) {
    error.value = t('flash.needLogin')
    return
  }
  const id = lookupUserId.value.trim()
  if (!id) {
    error.value = t('users.needUserId')
    return
  }
  notice.value = ''
  error.value = ''
  lookupBusy.value = true
  try {
    const res = await apiFetch(apiUrl(apiBase, userModerationPath(id)), {
      headers: authHeaders(false),
    })
    if (isAdminForbidden(res.status)) {
      isAdmin.value = false
      throw new Error(`moderation status ${res.status}`)
    }
    if (!res.ok) throw new Error(`moderation status ${res.status}`)
    lookupStatus.value = await res.json()
    notice.value = t('users.lookedUp', { id: shortId(id) })
  } catch (e) {
    error.value = String(e)
  } finally {
    lookupBusy.value = false
  }
}

function fillBanFromLookup() {
  const id = lookupStatus.value?.user_id || lookupUserId.value.trim()
  if (!id) return
  userIdInput.value = id
  unbanUserIdInput.value = id
  nav.value = 'moderation'
}

function fillMuteFromLookup() {
  const id = lookupStatus.value?.user_id || lookupUserId.value.trim()
  if (!id) return
  muteUserIdInput.value = id
  unmuteUserIdInput.value = id
  nav.value = 'moderation'
}

async function loadUsers() {
  if (!accessToken.value) return
  usersBusy.value = true
  try {
    const q = usersQuery.value.trim()
    const path = q
      ? `${adminUsersPath()}?q=${encodeURIComponent(q)}&limit=50`
      : `${adminUsersPath()}?limit=50`
    const res = await apiFetch(apiUrl(apiBase, path), { headers: authHeaders() })
    if (!res.ok) {
      if (isAdminForbidden(res.status)) {
        isAdmin.value = false
      }
      throw new Error(`users ${res.status}`)
    }
    const data = await res.json()
    usersList.value = Array.isArray(data.items) ? data.items : []
    usersTotal.value = Number(data.total ?? usersList.value.length)
  } catch (e) {
    error.value = String(e)
  } finally {
    usersBusy.value = false
  }
}

async function createUser() {
  if (!accessToken.value) {
    error.value = t('flash.needLogin')
    return
  }
  if (!createDisplayName.value.trim()) {
    error.value = t('users.displayName')
    return
  }
  createBusy.value = true
  notice.value = ''
  error.value = ''
  tempPasswordNotice.value = ''
  try {
    const body: Record<string, unknown> = {
      display_name: createDisplayName.value.trim(),
    }
    if (createUsername.value.trim()) body.username = createUsername.value.trim()
    if (createEmail.value.trim()) body.email = createEmail.value.trim()
    if (createPassword.value) body.password = createPassword.value
    const res = await apiFetch(apiUrl(apiBase, adminUsersPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify(body),
    })
    if (res.status !== 201) {
      const text = await res.text().catch(() => '')
      throw new Error(`create ${res.status} ${text}`)
    }
    const data = await res.json()
    notice.value = t('users.created')
    if (data.temporary_password) {
      tempPasswordNotice.value = `${t('users.tempPassword')}: ${data.temporary_password}`
    }
    createDisplayName.value = ''
    createUsername.value = ''
    createEmail.value = ''
    createPassword.value = ''
    await loadUsers()
  } catch (e) {
    error.value = String(e)
  } finally {
    createBusy.value = false
  }
}

async function resetUserPassword(id: string) {
  if (!accessToken.value) return
  actionBusy.value = true
  tempPasswordNotice.value = ''
  try {
    const res = await apiFetch(apiUrl(apiBase, adminResetPasswordPath(id)), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({ must_change_password: true }),
    })
    if (!res.ok) throw new Error(`reset ${res.status}`)
    const data = await res.json()
    if (data.temporary_password) {
      tempPasswordNotice.value = `${t('users.tempPassword')}: ${data.temporary_password}`
    }
    notice.value = t('users.resetPassword')
    await loadUsers()
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

async function revokeUserSessions(id: string) {
  if (!accessToken.value) return
  actionBusy.value = true
  try {
    const res = await apiFetch(apiUrl(apiBase, adminRevokeSessionsPath(id)), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({}),
    })
    if (!res.ok) throw new Error(`revoke ${res.status}`)
    notice.value = t('users.revokeSessions')
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

async function unbanUser(targetId?: string) {
  if (!accessToken.value) {
    error.value = t('flash.needLogin')
    return
  }
  const id = (targetId ?? unbanUserIdInput.value).trim()
  if (!id) {
    error.value = t('moderation.needUserId')
    return
  }
  notice.value = ''
  error.value = ''
  actionBusy.value = true
  try {
    const res = await apiFetch(apiUrl(apiBase, unbanUserPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        user_id: id,
        reason: actionReason.value.trim() || undefined,
      }),
    })
    if (res.status !== 204) throw new Error(`unban ${res.status}`)
    notice.value = t('moderation.unbanned', { id: shortId(id) })
    if (!targetId) unbanUserIdInput.value = ''
    if (lookupStatus.value?.user_id === id) {
      lookupStatus.value = {
        ...lookupStatus.value,
        banned: false,
        ban_reason: null,
        banned_at: null,
      }
    }
    await Promise.all([loadAudit(), loadModerationLists(), loadUsers()])
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

async function unbanUserId(id: string) {
  await unbanUser(id)
}

async function banUserId(id: string) {
  userIdInput.value = id
  await banUser()
}

async function setUserStatus(id: string, status: string) {
  if (!accessToken.value) return
  actionBusy.value = true
  try {
    const res = await apiFetch(apiUrl(apiBase, `${adminUsersPath()}/${encodeURIComponent(id)}`), {
      method: 'PATCH',
      headers: authHeaders(),
      body: JSON.stringify({ status }),
    })
    if (!res.ok) throw new Error(`patch ${res.status}`)
    notice.value = status === 'disabled' ? t('users.disable') : t('users.enable')
    await loadUsers()
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

async function muteUser() {
  if (!accessToken.value) {
    error.value = t('flash.needLogin')
    return
  }
  const id = muteUserIdInput.value.trim()
  if (!id) {
    error.value = t('moderation.needUserId')
    return
  }
  notice.value = ''
  error.value = ''
  actionBusy.value = true
  try {
    const res = await apiFetch(apiUrl(apiBase, muteUserPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        user_id: id,
        reason: actionReason.value.trim() || undefined,
      }),
    })
    if (res.status !== 204) throw new Error(`mute ${res.status}`)
    notice.value = t('moderation.muted', { id: shortId(id) })
    muteUserIdInput.value = ''
    await Promise.all([loadAudit(), loadModerationLists()])
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

async function unmuteUser(targetId?: string) {
  if (!accessToken.value) {
    error.value = t('flash.needLogin')
    return
  }
  const id = (targetId ?? unmuteUserIdInput.value).trim()
  if (!id) {
    error.value = t('moderation.needUserId')
    return
  }
  notice.value = ''
  error.value = ''
  actionBusy.value = true
  try {
    const res = await apiFetch(apiUrl(apiBase, unmuteUserPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        user_id: id,
        reason: actionReason.value.trim() || undefined,
      }),
    })
    if (res.status !== 204) throw new Error(`unmute ${res.status}`)
    notice.value = t('moderation.unmuted', { id: shortId(id) })
    if (!targetId) unmuteUserIdInput.value = ''
    if (lookupStatus.value?.user_id === id) {
      lookupStatus.value = {
        ...lookupStatus.value,
        muted: false,
        mute_reason: null,
        muted_at: null,
      }
    }
    await Promise.all([loadAudit(), loadModerationLists(), loadUsers()])
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

function beginEditGift(g: { id: string; name: string; price: number; active?: boolean }) {
  giftEditId.value = g.id
  giftName.value = g.name
  giftPrice.value = String(g.price)
  giftEditActive.value = g.active !== false
  notice.value = t('gifts.editing', { id: shortId(g.id) })
  error.value = ''
}

function cancelEditGift() {
  giftEditId.value = null
  giftName.value = ''
  giftPrice.value = ''
  giftEditActive.value = true
}

/**
 * Create or update gift via POST /api/v1/admin/gifts.
 * Optional body.id targets an existing catalog row; active toggles visibility.
 */
async function upsertGift(opts?: {
  id?: string
  name?: string
  price?: number
  active?: boolean
  successLabel?: string
}) {
  if (!accessToken.value) {
    error.value = t('flash.needLogin')
    return
  }
  const id = opts?.id
  const name = (opts?.name ?? giftName.value).trim()
  const price = opts?.price ?? Number(giftPrice.value)
  const active = opts?.active ?? (id ? giftEditActive.value : true)
  if (!name || !Number.isFinite(price) || price <= 0) {
    error.value = t('gifts.needFields')
    return
  }
  notice.value = ''
  error.value = ''
  giftBusy.value = true
  try {
    const body: { name: string; price: number; active: boolean; id?: string } = {
      name,
      price,
      active,
    }
    if (id) body.id = id
    const res = await apiFetch(apiUrl(apiBase, adminGiftsPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify(body),
    })
    if (isAdminForbidden(res.status)) {
      isAdmin.value = false
      error.value =
        adminGateHint.value || adminGateMessage({ apiBase, email: email.value }, locale.value)
      return
    }
    if (res.status !== 201 && !res.ok) throw new Error(`upsert gift ${res.status}`)
    const label =
      opts?.successLabel ??
      (id ? t('gifts.updated', { name }) : t('gifts.created', { name }))
    notice.value = label
    cancelEditGift()
    await loadGifts()
  } catch (e) {
    error.value = String(e)
  } finally {
    giftBusy.value = false
  }
}

async function createGift() {
  await upsertGift()
}

async function saveGiftEdit() {
  if (!giftEditId.value) {
    error.value = t('gifts.noEdit')
    return
  }
  await upsertGift({ id: giftEditId.value })
}

/** Toggle active flag on a gift row without leaving the list. */
async function toggleGiftActive(g: {
  id: string
  name: string
  price: number
  active?: boolean
}) {
  const next = g.active === false
  await upsertGift({
    id: g.id,
    name: g.name,
    price: g.price,
    active: next,
    successLabel: next ? t('gifts.enabled', { name: g.name }) : t('gifts.disabled', { name: g.name }),
  })
}

async function resolveReport(reportId: string) {
  if (!accessToken.value) {
    error.value = t('flash.needLogin')
    return
  }
  notice.value = ''
  error.value = ''
  actionBusy.value = true
  try {
    const res = await apiFetch(apiUrl(apiBase, reportResolvePath(reportId)), {
      method: 'PATCH',
      headers: authHeaders(),
      body: JSON.stringify({
        status: 'resolved',
        note: actionReason.value.trim() || undefined,
      }),
    })
    if (!res.ok) throw new Error(`resolve report ${res.status}`)
    notice.value = t('reports.resolved', { id: shortId(reportId) })
    await Promise.all([loadReports(), loadAudit()])
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

onMounted(() => {
  void restoreSession().finally(() => {
    if (!accessToken.value) void refreshLists()
  })
})

  return {
    setTheme,
    onToggleTheme,
    statusLabel,
    copyGiftSeedCmd,
    startResendCooldown,
    authHeaders,
    persistSession,
    tryRefreshAccess,
    apiFetch,
    teardownPreviewPlayer,
    attachPreviewHls,
    go,
    previewRoom,
    closePreview,
    useRoomId,
    useUserId,
    loadRooms,
    loadGifts,
    loadReports,
    loadAudit,
    refreshLists,
    runWalletReconcile,
    runExpirePayOrders,
    runMetricsScrape,
    runAnalyticsSummary,
    sendOtp,
    applySessionFromAuth,
    passwordLogin,
    verifyOtp,
    tryBootstrapAdmin,
    logout,
    restoreSession,
    forceCloseRoom,
    copyText,
    goLiveStart,
    goLiveRefreshPublish,
    goLiveStop,
    loadPublishForRoom,
    banUser,
    loadBannedUsers,
    loadMutedUsers,
    loadModerationLists,
    lookupUserModeration,
    fillBanFromLookup,
    fillMuteFromLookup,
    loadUsers,
    createUser,
    resetUserPassword,
    revokeUserSessions,
    unbanUser,
    unbanUserId,
    banUserId,
    setUserStatus,
    muteUser,
    unmuteUser,
    beginEditGift,
    cancelEditGift,
    upsertGift,
    createGift,
    saveGiftEdit,
    toggleGiftActive,
    resolveReport,
    apiBase,
    envLabel,
    title,
    theme,
    themeToggleLabel,
    NAV_ICONS,
    accessToken,
    refreshToken,
    displayName,
    userId,
    nav,
    isAdmin,
    sessionRestoring,
    email,
    password,
    otpCode,
    loginBusy,
    otpSent,
    useOtpLogin,
    resendCooldown,
    roomIdInput,
    userIdInput,
    muteUserIdInput,
    unmuteUserIdInput,
    actionReason,
    actionBusy,
    giftName,
    giftPrice,
    giftBusy,
    giftEditId,
    giftEditActive,
    rooms,
    gifts,
    reports,
    audit,
    roomQuery,
    roomStatusFilter,
    roomPage,
    roomPageSize,
    roomTotalPages,
    pagedRooms,
    roomListMeta,
    setRoomPage,
    auditQuery,
    error,
    notice,
    listBusy,
    previewRoomId,
    previewHlsUrl,
    previewBusy,
    previewError,
    previewVideoEl,
    goLiveTitle,
    goLiveBusy,
    goLiveRoomId,
    goLiveRoomStatus,
    goLivePublish,
    goLiveHls,
    goLiveCopyHint,
    reconcileBusy,
    reconcileHint,
    reconcileBalanced,
    reconcileChecked,
    reconcileImbalance,
    expireBusy,
    expireHint,
    metricsBusy,
    metricsHint,
    metricsText,
    metricsLines,
    analyticsBusy,
    analyticsHint,
    analyticsRetained,
    analyticsUsers,
    analyticsByName,
    analyticsRecent,
    usersList,
    usersTotal,
    usersQuery,
    usersBusy,
    createDisplayName,
    createUsername,
    createEmail,
    createPassword,
    createBusy,
    tempPasswordNotice,
    unbanUserIdInput,
    lookupUserId,
    lookupBusy,
    lookupStatus,
    bannedUsers,
    mutedUsers,
    isAuthed,
    liveCount,
    idleCount,
    closedCount,
    reportOpen,
    pageTitle,
    pageBlurb,
    avatarLetter,
    bootstrapClosed,
    filteredRooms,
    filteredAudit,
    adminGateHint,
    sessionRoleLabel,
    prepHints,
    giftSeedCmd,
    giftSeedCopyHint,
    loginStep,
    t,
    locale,
    setLocale,
    ADMIN_NAV,
    shortId,
    roomStatusTone,
    formatTs,
  }
}

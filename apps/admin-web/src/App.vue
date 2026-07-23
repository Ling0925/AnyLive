<script setup lang="ts">
import Hls from 'hls.js'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from './i18n'
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
} from './lib/admin'
import {
  applyTheme,
  persistTheme,
  resolveInitialTheme,
  toggleTheme as flipTheme,
  type Theme,
} from './lib/theme'

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
</script>

<template>
  <!-- Session restore splash -->
  <div v-if="sessionRestoring" class="login-screen" data-testid="session-restoring">
    <div class="login-bg" aria-hidden="true">
      <div class="login-orb o1" />
      <div class="login-orb o2" />
      <div class="login-grid" />
    </div>
    <div class="login-shell" style="max-width: min(360px, 100%); grid-template-columns: minmax(0, 1fr)">
      <div class="login-restoring login-card">
        <div class="login-spinner" />
        <p class="lead">{{ t('login.restoring') }}</p>
      </div>
    </div>
  </div>

  <!-- Login -->
  <div v-else-if="!isAuthed" class="login-screen" data-testid="login-screen">
    <div class="login-bg" aria-hidden="true">
      <div class="login-orb o1" />
      <div class="login-orb o2" />
      <div class="login-orb o3" />
      <div class="login-grid" />
    </div>

    <div class="login-shell" data-testid="login-card">
      <aside class="login-hero">
        <div class="login-hero-top">
          <div class="brand">
            <div class="brand-mark">AL</div>
            <div class="brand-text">
              <div class="brand-title">{{ title }}</div>
              <div class="brand-sub">{{ t('app.console') }}</div>
            </div>
          </div>
          <span class="login-hero-tag">{{ t('login.heroTag') }}</span>
          <h2>{{ t('login.heroTitle') }}</h2>
          <p>{{ t('login.heroBody') }}</p>
        </div>
        <div class="login-features">
          <div class="login-feature">
            <span class="login-feature-dot" />
            {{ t('login.features.rooms') }}
          </div>
          <div class="login-feature">
            <span class="login-feature-dot" />
            {{ t('login.features.moderation') }}
          </div>
          <div class="login-feature">
            <span class="login-feature-dot" />
            {{ t('login.features.audit') }}
          </div>
        </div>
      </aside>

      <div class="login-card">
        <div class="login-card-head">
          <div>
            <h1>{{ t('login.headline') }}</h1>
            <p class="lead">
              {{ t('login.lead') }}
              <code class="mono">123456</code>
            </p>
          </div>
          <div class="login-toolbar">
            <div class="theme-switch" data-testid="login-theme-switch" role="group" :aria-label="t('theme.label')">
              <button
                type="button"
                :class="{ active: theme === 'light' }"
                data-testid="theme-light"
                :aria-pressed="theme === 'light'"
                @click="setTheme('light')"
              >
                {{ t('theme.light') }}
              </button>
              <button
                type="button"
                :class="{ active: theme === 'dark' }"
                data-testid="theme-dark"
                :aria-pressed="theme === 'dark'"
                @click="setTheme('dark')"
              >
                {{ t('theme.dark') }}
              </button>
            </div>
            <div class="lang-switch" data-testid="login-lang-switch">
              <button
                type="button"
                :class="{ active: locale === 'zh' }"
                data-testid="lang-zh"
                @click="setLocale('zh')"
              >
                {{ t('lang.zh') }}
              </button>
              <button
                type="button"
                :class="{ active: locale === 'en' }"
                data-testid="lang-en"
                @click="setLocale('en')"
              >
                {{ t('lang.en') }}
              </button>
            </div>
          </div>
        </div>

        <div class="login-mode-tabs" role="tablist" data-testid="login-mode-tabs">
          <button
            type="button"
            role="tab"
            :class="{ active: !useOtpLogin }"
            :aria-selected="!useOtpLogin"
            data-testid="login-tab-password"
            @click="useOtpLogin = false"
          >
            {{ t('login.modePassword') }}
          </button>
          <button
            type="button"
            role="tab"
            :class="{ active: useOtpLogin }"
            :aria-selected="useOtpLogin"
            data-testid="login-tab-otp"
            @click="useOtpLogin = true"
          >
            {{ t('login.modeOtp') }}
          </button>
        </div>

        <div class="login-steps" aria-hidden="true">
          <div class="login-step" :class="{ active: loginStep === 1, done: loginStep > 1 }" />
          <div class="login-step" :class="{ active: loginStep === 2, done: false }" />
        </div>

        <p v-if="notice" class="flash ok" data-testid="login-notice">{{ notice }}</p>
        <p v-if="error" class="flash err" data-testid="login-error">{{ error }}</p>

        <label class="field">
          <span>{{ t('login.email') }}</span>
          <input
            v-model="email"
            type="text"
            autocomplete="username"
            :placeholder="t('login.emailPlaceholder')"
            data-testid="login-email"
          />
        </label>

        <template v-if="!useOtpLogin">
          <label class="field">
            <span>{{ t('login.password') }}</span>
            <input
              v-model="password"
              type="password"
              autocomplete="current-password"
              :placeholder="t('login.passwordPlaceholder')"
              data-testid="login-password"
              @keyup.enter="passwordLogin"
            />
          </label>
          <button
            type="button"
            class="btn primary"
            data-testid="login-submit"
            :disabled="loginBusy || !email.trim() || !password"
            @click="passwordLogin"
          >
            {{ loginBusy ? t('login.submitting') : t('login.submit') }}
          </button>
        </template>

        <template v-else>
          <div class="row">
            <button
              type="button"
              class="btn"
              data-testid="login-send-otp"
              :disabled="loginBusy || !email.trim() || resendCooldown > 0"
              @click="sendOtp"
            >
              <template v-if="loginBusy && !otpSent">{{ t('login.sending') }}</template>
              <template v-else-if="resendCooldown > 0">
                {{ t('login.resendIn', { n: resendCooldown }) }}
              </template>
              <template v-else-if="otpSent">{{ t('login.resend') }}</template>
              <template v-else>{{ t('login.sendOtp') }}</template>
            </button>
          </div>
          <label class="field">
            <span>{{ t('login.otp') }}</span>
            <input
              v-model="otpCode"
              type="text"
              inputmode="numeric"
              autocomplete="one-time-code"
              :placeholder="t('login.otpPlaceholder')"
              data-testid="login-otp"
              @keyup.enter="verifyOtp"
            />
            <span class="otp-dev-tip">{{ t('login.tipDev') }} · 123456</span>
          </label>
          <button
            type="button"
            class="btn primary"
            data-testid="login-submit-otp"
            :disabled="loginBusy || !email.trim() || !otpCode.trim()"
            @click="verifyOtp"
          >
            {{ loginBusy ? t('login.submitting') : t('login.submit') }}
          </button>
        </template>

        <p class="login-secure">{{ t('login.secureNote') }} · {{ t('login.api') }} {{ apiBase }}</p>
      </div>
    </div>
  </div>

  <!-- Ops shell -->
  <div v-else class="shell" data-testid="ops-shell">
    <aside class="sidebar" data-testid="sidebar">
      <div class="brand">
        <div class="brand-mark">AL</div>
        <div class="brand-text">
          <div class="brand-title">{{ t('app.name') }}</div>
          <div class="brand-sub">{{ t('app.opsConsole') }}</div>
        </div>
      </div>

      <nav class="nav" data-testid="sidebar-nav">
        <button
          v-for="item in ADMIN_NAV"
          :key="item.key"
          type="button"
          class="nav-item"
          :class="{ active: nav === item.key }"
          :data-testid="`nav-${item.key}`"
          @click="go(item.key)"
        >
          <span class="nav-icon" aria-hidden="true">
            <svg viewBox="0 0 24 24">
              <path :d="NAV_ICONS[item.key]" />
            </svg>
          </span>
          <span>{{ t(item.labelKey) }}</span>
        </button>
      </nav>

      <div class="sidebar-foot">
        <div class="theme-switch" data-testid="sidebar-theme-switch" role="group" :aria-label="t('theme.label')">
          <button
            type="button"
            :class="{ active: theme === 'light' }"
            :aria-pressed="theme === 'light'"
            @click="setTheme('light')"
          >
            {{ t('theme.light') }}
          </button>
          <button
            type="button"
            :class="{ active: theme === 'dark' }"
            :aria-pressed="theme === 'dark'"
            @click="setTheme('dark')"
          >
            {{ t('theme.dark') }}
          </button>
        </div>
        <div class="lang-switch" data-testid="sidebar-lang-switch">
          <button
            type="button"
            :class="{ active: locale === 'zh' }"
            @click="setLocale('zh')"
          >
            {{ t('lang.zh') }}
          </button>
          <button
            type="button"
            :class="{ active: locale === 'en' }"
            @click="setLocale('en')"
          >
            {{ t('lang.en') }}
          </button>
        </div>
        <div class="api-pill">{{ apiBase }}</div>
        <button
          type="button"
          class="btn ghost"
          data-testid="refresh-lists"
          :disabled="listBusy"
          @click="refreshLists"
        >
          {{ t('topbar.refresh') }}
        </button>
        <button type="button" class="btn ghost" data-testid="logout" @click="logout">
          {{ t('topbar.logout') }}
        </button>
      </div>
    </aside>

    <div class="main">
      <header class="topbar">
        <div>
          <h1>{{ pageTitle }}</h1>
          <p class="muted">{{ pageBlurb }}</p>
        </div>
        <div class="session-tools">
          <button
            type="button"
            class="theme-icon-btn"
            data-testid="topbar-theme-toggle"
            :title="themeToggleLabel"
            :aria-label="themeToggleLabel"
            @click="onToggleTheme"
          >
            <!-- sun when dark (click → light); moon when light (click → dark) -->
            <svg v-if="theme === 'dark'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <circle cx="12" cy="12" r="4" />
              <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" />
            </svg>
            <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M21 14.5A8.5 8.5 0 1111.5 3a7 7 0 009.5 11.5z" />
            </svg>
          </button>
          <div class="session">
            <div class="avatar">{{ avatarLetter }}</div>
            <div>
              <div style="font-weight: 600">{{ displayName || 'operator' }}</div>
              <div
                class="dim mono"
                style="font-size: 0.72rem; display: flex; gap: 0.35rem; align-items: center; min-width: 0; flex-wrap: wrap"
              >
                <span style="overflow: hidden; text-overflow: ellipsis">{{ shortId(userId, 10) }}</span>
                <span
                  class="role-pill"
                  :class="{ admin: isAdmin === true }"
                  data-testid="session-role"
                >
                  {{ sessionRoleLabel }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </header>

      <div class="content" data-testid="content">
        <p v-if="isAdmin === false" class="flash err" data-testid="admin-gate">{{ adminGateHint }}</p>
        <p v-if="notice" class="flash ok" data-testid="notice">{{ notice }}</p>
        <p v-if="error && isAdmin !== false" class="flash err" data-testid="error">{{ error }}</p>

        <!-- Dashboard -->
        <template v-if="nav === 'dashboard'">
          <div class="kpis" data-testid="dashboard-kpis">
            <div class="kpi" data-testid="kpi-live">
              <div class="kpi-label">{{ t('dashboard.live') }}</div>
              <div class="kpi-value live">{{ liveCount }}</div>
            </div>
            <div class="kpi" data-testid="kpi-idle">
              <div class="kpi-label">{{ t('dashboard.idle') }}</div>
              <div class="kpi-value">{{ idleCount }}</div>
            </div>
            <div class="kpi" data-testid="kpi-reports">
              <div class="kpi-label">{{ t('dashboard.reports') }}</div>
              <div class="kpi-value">{{ reportOpen }}</div>
            </div>
            <div class="kpi" data-testid="kpi-gifts">
              <div class="kpi-label">{{ t('dashboard.gifts') }}</div>
              <div class="kpi-value">{{ gifts.length }}</div>
            </div>
          </div>

          <section
            v-if="isAdmin === true"
            class="panel"
            style="margin-bottom: 0"
            data-testid="demo-prep"
          >
            <div class="panel-head">
              <h2>{{ t('dashboard.demoPrep') }}</h2>
            </div>
            <p class="panel-desc">{{ t('dashboard.demoPrepDesc') }}</p>
            <ul class="muted" style="margin: 0.25rem 0 0.5rem; padding-left: 1.2rem; font-size: 0.88rem; overflow-wrap: anywhere">
              <li v-for="(line, i) in prepHints.lines" :key="i">{{ line }}</li>
            </ul>
            <div class="row" style="gap: 0.5rem; flex-wrap: wrap; align-items: center; min-width: 0">
              <code class="mono" data-testid="demo-prep-gift-seed">{{ prepHints.giftSeedCmd }}</code>
              <button
                type="button"
                class="btn sm"
                data-testid="demo-prep-copy-gift-seed"
                @click="copyGiftSeedCmd"
              >
                {{ t('dashboard.copySeed') }}
              </button>
              <span class="dim mono" style="font-size: 0.78rem" data-testid="demo-prep-runbook">
                {{ prepHints.runbookPath }}
              </span>
            </div>
            <p v-if="giftSeedCopyHint" class="hint" data-testid="demo-prep-copy-hint">
              {{ giftSeedCopyHint }}
            </p>
          </section>

          <section
            v-if="isAuthed"
            class="panel"
            data-testid="panel-wallet-ops"
          >
            <div class="panel-head">
              <h2>{{ t('dashboard.walletOps') }}</h2>
            </div>
            <div class="row" style="gap: 0.75rem; flex-wrap: wrap; align-items: center">
              <button
                type="button"
                class="btn primary"
                data-testid="wallet-reconcile"
                :disabled="reconcileBusy"
                @click="runWalletReconcile"
              >
                {{ reconcileBusy ? t('dashboard.walletReconciling') : t('dashboard.walletReconcile') }}
              </button>
              <button
                type="button"
                class="btn"
                data-testid="pay-expire-orders"
                :disabled="expireBusy"
                @click="runExpirePayOrders"
              >
                {{ expireBusy ? t('dashboard.expiring') : t('dashboard.expireOrders') }}
              </button>
              <button
                type="button"
                class="btn"
                data-testid="metrics-scrape"
                :disabled="metricsBusy"
                @click="runMetricsScrape"
              >
                {{ metricsBusy ? t('dashboard.scraping') : t('dashboard.scrapeMetrics') }}
              </button>
            </div>
            <p
              v-if="reconcileHint"
              class="hint"
              data-testid="wallet-reconcile-hint"
              :class="{ err: reconcileBalanced === false }"
            >
              {{ reconcileHint }}
            </p>
            <p v-if="expireHint" class="hint" data-testid="pay-expire-hint">{{ expireHint }}</p>
            <p v-if="metricsHint" class="hint" data-testid="metrics-hint">{{ metricsHint }}</p>
            <pre
              v-if="metricsText"
              class="mono"
              style="max-height: 180px; max-width: 100%; overflow: auto; font-size: 11px; margin-top: 0.5rem; white-space: pre-wrap; word-break: break-all"
            >{{ metricsText.slice(0, 4000) }}</pre>
          </section>

          <section v-if="isAuthed" class="panel">
            <div class="panel-head">
              <h2>{{ t('dashboard.analytics') }}</h2>
              <button
                type="button"
                class="btn sm"
                :disabled="analyticsBusy"
                @click="runAnalyticsSummary"
              >
                {{ analyticsBusy ? t('common.loading') : t('dashboard.analyticsRefresh') }}
              </button>
            </div>
            <p class="panel-desc">{{ t('dashboard.analyticsDesc') }}</p>
            <p v-if="analyticsHint" class="hint">{{ analyticsHint }}</p>
            <div v-if="analyticsByName.length" class="table-wrap" style="margin-top: 0.5rem">
              <table class="data">
                <thead>
                  <tr>
                    <th>{{ t('dashboard.eventName') }}</th>
                    <th>{{ t('dashboard.eventCount') }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="row in analyticsByName.slice(0, 12)" :key="row.name">
                    <td>{{ row.name }}</td>
                    <td>{{ row.count }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <ul v-if="analyticsRecent.length" class="muted" style="margin-top: 0.75rem">
              <li v-for="ev in analyticsRecent.slice(0, 5)" :key="ev.id">
                {{ ev.name }} · {{ ev.user_id.slice(0, 8) }} · {{ formatTs(ev.occurred_at) }}
              </li>
            </ul>
          </section>

          <div class="grid-2">
            <section class="panel">
              <div class="panel-head">
                <h2>{{ t('dashboard.roomsPreview') }}</h2>
                <button type="button" class="btn sm" @click="go('rooms')">{{ t('common.all') }}</button>
              </div>
              <div class="table-wrap" v-if="rooms.length">
                <table class="data">
                  <thead>
                    <tr>
                      <th>{{ t('rooms.colTitle') }}</th>
                      <th>{{ t('rooms.colStatus') }}</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="r in rooms.slice(0, 6)" :key="r.id">
                      <td>{{ r.title }}</td>
                      <td>
                        <span class="badge" :class="roomStatusTone(r.status)">{{
                          statusLabel(r.status)
                        }}</span>
                      </td>
                      <td class="actions">
                        <button type="button" class="btn sm" @click="go('rooms'); previewRoom(r)">
                          {{ t('dashboard.preview') }}
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-else class="empty">
                <div class="empty-icon">◎</div>
                {{ t('dashboard.noRooms') }}
              </div>
            </section>

            <section class="panel">
              <div class="panel-head">
                <h2>{{ t('dashboard.reportsPreview') }}</h2>
                <button type="button" class="btn sm" @click="go('reports')">{{ t('dashboard.queue') }}</button>
              </div>
              <div class="table-wrap" v-if="reports.length">
                <table class="data">
                  <thead>
                    <tr>
                      <th>{{ t('reports.colTarget') }}</th>
                      <th>{{ t('reports.colReason') }}</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="r in reports.slice(0, 6)" :key="r.id">
                      <td class="mono">{{ r.target_type }}:{{ shortId(r.target_id) }}</td>
                      <td>{{ r.reason }}</td>
                      <td>
                        <button
                          type="button"
                          class="btn sm primary"
                          :disabled="actionBusy"
                          @click="resolveReport(r.id)"
                        >
                          {{ t('dashboard.resolve') }}
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-else class="empty">
                <div class="empty-icon">◇</div>
                {{ t('dashboard.noReports') }}
              </div>
            </section>
          </div>
        </template>

        <!-- Go live -->
        <section v-else-if="nav === 'golive'" class="panel" data-testid="panel-golive">
          <div class="panel-head">
            <h2>{{ t('golive.title') }}</h2>
            <span v-if="goLiveRoomStatus" class="badge" :class="roomStatusTone(goLiveRoomStatus)">
              {{ statusLabel(goLiveRoomStatus) }}
            </span>
          </div>
          <p class="panel-desc">{{ t('golive.desc') }}</p>

          <div class="row">
            <label class="field">
              <span>{{ t('golive.liveTitle') }}</span>
              <input
                v-model="goLiveTitle"
                type="text"
                :placeholder="t('golive.liveTitlePlaceholder')"
                maxlength="80"
                data-testid="golive-title"
              />
            </label>
            <button
              type="button"
              class="btn primary"
              data-testid="golive-start"
              :disabled="goLiveBusy || !isAuthed"
              @click="goLiveStart"
            >
              {{ goLiveBusy ? t('common.processing') : t('golive.start') }}
            </button>
            <button
              type="button"
              class="btn"
              data-testid="golive-refresh-publish"
              :disabled="goLiveBusy || !goLiveRoomId || !isAuthed"
              @click="goLiveRefreshPublish"
            >
              {{ t('golive.refreshPublish') }}
            </button>
            <button
              type="button"
              class="btn danger"
              data-testid="golive-stop"
              :disabled="goLiveBusy || !goLiveRoomId || goLiveRoomStatus === 'closed' || !isAuthed"
              @click="goLiveStop"
            >
              {{ t('golive.stop') }}
            </button>
          </div>
          <p v-if="!isAuthed" class="flash err">{{ t('golive.needAuth') }}</p>
          <p v-if="goLiveCopyHint" class="flash ok" data-testid="golive-copy-hint">{{ goLiveCopyHint }}</p>

          <div
            v-if="goLiveRoomId"
            class="action-card"
            style="margin-top: 1rem"
            data-testid="golive-room-info"
          >
            <h3>{{ t('golive.roomInfo') }}</h3>
            <p class="mono" data-testid="golive-room-id">{{ t('golive.roomId') }}：{{ goLiveRoomId }}</p>
            <p class="muted">{{ t('common.status') }}：{{ statusLabel(goLiveRoomStatus) || '—' }}</p>
            <div class="actions" style="margin-top: 0.5rem">
              <button
                type="button"
                class="btn sm"
                data-testid="golive-copy-room-id"
                @click="copyText(t('golive.roomId'), goLiveRoomId)"
              >
                {{ t('golive.copyRoomId') }}
              </button>
              <button
                type="button"
                class="btn sm"
                data-testid="golive-preview"
                @click="previewRoom({ id: goLiveRoomId, status: goLiveRoomStatus || 'live' })"
              >
                {{ t('golive.hlsPreview') }}
              </button>
            </div>
          </div>

          <div
            v-if="goLivePublish"
            class="split-actions"
            style="margin-top: 1rem"
            data-testid="golive-obs"
          >
            <div class="action-card">
              <h3>{{ t('golive.obsServer') }}</h3>
              <p class="mono" style="word-break: break-all" data-testid="golive-obs-server">
                {{ goLivePublish.server }}
              </p>
              <button
                type="button"
                class="btn sm primary"
                data-testid="golive-copy-server"
                @click="copyText(t('golive.obsServer'), goLivePublish.server)"
              >
                {{ t('golive.copyServer') }}
              </button>
            </div>
            <div class="action-card">
              <h3>{{ t('golive.streamKey') }}</h3>
              <p class="mono" style="word-break: break-all" data-testid="golive-obs-stream-key">
                {{ goLivePublish.streamKey }}
              </p>
              <button
                type="button"
                class="btn sm primary"
                data-testid="golive-copy-stream-key"
                @click="copyText(t('golive.streamKey'), goLivePublish.streamKey)"
              >
                {{ t('golive.copyStreamKey') }}
              </button>
              <p class="dim" style="margin-top: 0.5rem; font-size: 0.8rem">
                {{ t('golive.streamKeyHint') }}
              </p>
            </div>
            <div class="action-card">
              <h3>{{ t('golive.pushUrl') }}</h3>
              <p class="mono" style="word-break: break-all" data-testid="golive-push-url">
                {{ goLivePublish.pushUrl }}
              </p>
              <button
                type="button"
                class="btn sm"
                data-testid="golive-copy-push-url"
                @click="copyText('push URL', goLivePublish.pushUrl)"
              >
                {{ t('golive.copyPushUrl') }}
              </button>
              <p v-if="goLivePublish.expiresAt" class="dim" style="margin-top: 0.5rem; font-size: 0.8rem">
                {{ t('golive.expires') }}：{{ formatTs(goLivePublish.expiresAt) }}
              </p>
            </div>
            <div class="action-card">
              <h3>{{ t('golive.audienceHls') }}</h3>
              <p class="mono" style="word-break: break-all" data-testid="golive-hls">
                {{ goLiveHls || '—' }}
              </p>
              <div class="actions">
                <button
                  type="button"
                  class="btn sm"
                  data-testid="golive-copy-hls"
                  :disabled="!goLiveHls"
                  @click="copyText('HLS', goLiveHls)"
                >
                  {{ t('golive.copyHls') }}
                </button>
                <a
                  v-if="goLiveHls"
                  class="btn sm"
                  :href="goLiveHls"
                  target="_blank"
                  rel="noopener"
                  data-testid="golive-open-hls"
                  >{{ t('golive.openHls') }}</a
                >
              </div>
              <p class="dim" style="margin-top: 0.5rem; font-size: 0.8rem">
                {{ t('golive.h5Hint') }}{{ goLiveRoomId || 'roomId' }}
              </p>
            </div>
          </div>

          <div class="panel" style="margin-top: 1.25rem; box-shadow: none">
            <h3 style="margin: 0 0 0.5rem; font-size: 0.95rem">{{ t('golive.obsGuide') }}</h3>
            <ol class="muted" style="margin: 0; padding-left: 1.2rem; font-size: 0.88rem">
              <li>{{ t('golive.obsStep1') }}</li>
              <li>{{ t('golive.obsStep2') }}</li>
              <li>{{ t('golive.obsStep3') }}</li>
              <li>{{ t('golive.obsStep4') }}</li>
            </ol>
          </div>
        </section>

        <!-- Rooms -->
        <section v-else-if="nav === 'rooms'" class="panel" data-testid="panel-rooms">
          <div class="panel-head">
            <h2>{{ t('rooms.title') }}</h2>
            <div class="toolbar">
              <span class="badge live">live {{ liveCount }}</span>
              <span class="badge idle">idle {{ idleCount }}</span>
              <span class="badge closed">closed {{ closedCount }}</span>
              <button
                type="button"
                class="btn sm"
                data-testid="rooms-refresh"
                :disabled="listBusy"
                @click="loadRooms"
              >
                {{ t('common.refresh') }}
              </button>
            </div>
          </div>
          <p class="panel-desc">{{ t('rooms.desc') }}</p>

          <div class="filter-bar">
            <label class="field">
              <span>{{ t('common.search') }}</span>
              <input
                v-model="roomQuery"
                type="search"
                :placeholder="t('rooms.searchPlaceholder')"
                data-testid="rooms-search"
              />
            </label>
            <label class="field" style="max-width: min(160px, 100%)">
              <span>{{ t('common.filter') }}</span>
              <select v-model="roomStatusFilter" data-testid="rooms-status-filter">
                <option value="all">{{ t('rooms.filterAll') }}</option>
                <option value="live">{{ t('status.live') }}</option>
                <option value="idle">{{ t('status.idle') }}</option>
                <option value="closed">{{ t('status.closed') }}</option>
              </select>
            </label>
          </div>

          <div class="table-wrap" v-if="filteredRooms.length" data-testid="rooms-table">
            <table class="data">
              <thead>
                <tr>
                  <th>{{ t('rooms.colTitle') }}</th>
                  <th>{{ t('rooms.colStatus') }}</th>
                  <th>{{ t('rooms.colOwner') }}</th>
                  <th>{{ t('rooms.colId') }}</th>
                  <th>{{ t('common.actions') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="r in filteredRooms" :key="r.id" :data-testid="`room-row-${r.id}`">
                  <td>{{ r.title }}</td>
                  <td>
                    <span class="badge" :class="roomStatusTone(r.status)">{{
                      statusLabel(r.status)
                    }}</span>
                  </td>
                  <td class="mono">{{ shortId(r.owner_id || '') }}</td>
                  <td class="mono" :title="r.id">{{ shortId(r.id) }}</td>
                  <td class="actions">
                    <button
                      type="button"
                      class="btn sm"
                      data-testid="room-preview"
                      :disabled="previewBusy"
                      @click="previewRoom(r)"
                    >
                      {{ t('rooms.preview') }}
                    </button>
                    <button
                      type="button"
                      class="btn sm primary"
                      data-testid="room-publish-info"
                      :disabled="goLiveBusy || !isAuthed || r.status === 'closed'"
                      @click="loadPublishForRoom(r.id)"
                    >
                      {{ t('rooms.publishInfo') }}
                    </button>
                    <button
                      type="button"
                      class="btn sm"
                      data-testid="room-fill-force-close"
                      @click="useRoomId(r.id)"
                    >
                      {{ t('rooms.fillForceClose') }}
                    </button>
                    <button
                      type="button"
                      class="btn sm danger"
                      data-testid="room-force-close"
                      :disabled="actionBusy || r.status === 'closed'"
                      @click="forceCloseRoom(r.id)"
                    >
                      {{ t('rooms.forceClose') }}
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="empty" data-testid="rooms-empty">
            <div class="empty-icon">◎</div>
            {{ t('rooms.empty') }}
          </div>

          <div v-if="previewRoomId" class="preview" data-testid="room-preview-panel">
            <div class="panel-head">
              <h2>{{ t('rooms.previewTitle') }} · {{ shortId(previewRoomId, 12) }}</h2>
              <button type="button" class="btn sm ghost" data-testid="room-preview-close" @click="closePreview">
                {{ t('common.close') }}
              </button>
            </div>
            <p v-if="previewError" class="flash err">{{ previewError }}</p>
            <p v-if="previewHlsUrl" class="preview-url mono">
              <a :href="previewHlsUrl" target="_blank" rel="noopener">{{ previewHlsUrl }}</a>
            </p>
            <video v-if="previewHlsUrl" ref="previewVideoEl" controls playsinline class="preview-video" />
          </div>
        </section>

        <!-- Reports -->
        <section v-else-if="nav === 'reports'" class="panel" data-testid="panel-reports">
          <div class="panel-head">
            <h2>{{ t('reports.title') }}</h2>
            <button
              type="button"
              class="btn sm"
              data-testid="reports-refresh"
              :disabled="listBusy"
              @click="loadReports"
            >
              {{ t('common.refresh') }}
            </button>
          </div>
          <p class="panel-desc">{{ t('reports.desc') }}</p>
          <label class="field" style="max-width: min(420px, 100%)">
            <span>{{ t('reports.note') }}</span>
            <input
              v-model="actionReason"
              type="text"
              :placeholder="t('reports.notePlaceholder')"
              data-testid="report-note"
            />
          </label>
          <div class="table-wrap" v-if="reports.length" data-testid="reports-table">
            <table class="data">
              <thead>
                <tr>
                  <th>{{ t('reports.colTarget') }}</th>
                  <th>{{ t('reports.colReason') }}</th>
                  <th>{{ t('reports.colStatus') }}</th>
                  <th>{{ t('reports.colReporter') }}</th>
                  <th>{{ t('reports.colTime') }}</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="r in reports" :key="r.id" :data-testid="`report-row-${r.id}`">
                  <td class="mono">{{ r.target_type }}:{{ shortId(r.target_id) }}</td>
                  <td>{{ r.reason }}</td>
                  <td>
                    <span class="badge" :class="r.status === 'resolved' ? 'closed' : 'live'">
                      {{
                        r.status === 'resolved' ? t('reports.statusResolved') : t('reports.statusOpen')
                      }}
                    </span>
                  </td>
                  <td class="mono">{{ shortId(r.reporter_id) }}</td>
                  <td class="mono">{{ formatTs(r.created_at) }}</td>
                  <td class="actions">
                    <button
                      type="button"
                      class="btn sm primary"
                      data-testid="report-resolve"
                      :disabled="actionBusy || r.status === 'resolved'"
                      @click="resolveReport(r.id)"
                    >
                      {{ t('reports.resolve') }}
                    </button>
                    <button
                      v-if="r.target_type === 'room'"
                      type="button"
                      class="btn sm"
                      data-testid="report-to-force-close"
                      @click="useRoomId(r.target_id)"
                    >
                      {{ t('reports.toForceClose') }}
                    </button>
                    <button
                      v-if="r.target_type === 'user'"
                      type="button"
                      class="btn sm"
                      data-testid="report-to-moderation"
                      @click="useUserId(r.target_id)"
                    >
                      {{ t('reports.toModeration') }}
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="empty" data-testid="reports-empty">
            <div class="empty-icon">◇</div>
            {{ t('reports.empty') }}
          </div>
        </section>

        <!-- Gifts -->
        <section v-else-if="nav === 'gifts'" class="panel" data-testid="panel-gifts">
          <div class="panel-head">
            <h2>{{ t('gifts.title') }}</h2>
            <button
              type="button"
              class="btn sm"
              data-testid="gifts-refresh"
              :disabled="listBusy"
              @click="loadGifts"
            >
              {{ t('common.refresh') }}
            </button>
          </div>
          <p class="panel-desc">{{ t('gifts.desc') }}</p>
          <p class="hint" data-testid="gifts-seed-hint" style="margin-top: 0">
            {{ t('gifts.seedHint') }}
            <code class="mono">./scripts/dogfood-gift-seed.sh</code>
            {{ t('gifts.seedHint2') }}
            <code class="mono">DOGFOOD_ADMIN_EMAIL</code>
            {{ t('gifts.seedHint3') }}
            <code class="mono">seed-admin-local.sh</code>
            {{ t('gifts.seedHint4') }}
            <button
              type="button"
              class="btn sm"
              style="margin-left: 0.35rem"
              data-testid="gifts-copy-seed-cmd"
              @click="copyGiftSeedCmd"
            >
              {{ t('gifts.copyCmd') }}
            </button>
            <span v-if="giftSeedCopyHint" class="dim" style="margin-left: 0.35rem">{{
              giftSeedCopyHint
            }}</span>
          </p>
          <div class="row" data-testid="gift-form">
            <label class="field">
              <span>{{ t('gifts.name') }}</span>
              <input
                v-model="giftName"
                type="text"
                placeholder="Rose"
                data-testid="gift-name"
              />
            </label>
            <label class="field" style="max-width: min(160px, 100%)">
              <span>{{ t('gifts.price') }}</span>
              <input
                v-model="giftPrice"
                type="number"
                min="1"
                step="1"
                placeholder="10"
                data-testid="gift-price"
              />
            </label>
            <label v-if="giftEditId" class="field" style="max-width: min(140px, 100%)">
              <span>{{ t('gifts.status') }}</span>
              <select v-model="giftEditActive" data-testid="gift-active">
                <option :value="true">{{ t('gifts.active') }}</option>
                <option :value="false">{{ t('gifts.inactive') }}</option>
              </select>
            </label>
            <button
              v-if="!giftEditId"
              type="button"
              class="btn primary"
              data-testid="gift-create"
              :disabled="giftBusy || !giftName.trim() || !giftPrice"
              @click="createGift"
            >
              {{ t('gifts.create') }}
            </button>
            <template v-else>
              <button
                type="button"
                class="btn primary"
                data-testid="gift-save"
                :disabled="giftBusy || !giftName.trim() || !giftPrice"
                @click="saveGiftEdit"
              >
                {{ t('gifts.save') }}
              </button>
              <button
                type="button"
                class="btn"
                data-testid="gift-cancel-edit"
                :disabled="giftBusy"
                @click="cancelEditGift"
              >
                {{ t('common.cancel') }}
              </button>
            </template>
          </div>
          <p v-if="giftEditId" class="hint" data-testid="gift-edit-hint">
            {{ t('gifts.editing', { id: shortId(giftEditId, 12) }) }}
          </p>
          <div class="table-wrap" v-if="gifts.length" data-testid="gifts-table">
            <table class="data">
              <thead>
                <tr>
                  <th>{{ t('gifts.name') }}</th>
                  <th>{{ t('common.price') }}</th>
                  <th>{{ t('gifts.status') }}</th>
                  <th>{{ t('common.id') }}</th>
                  <th>{{ t('common.actions') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="g in gifts" :key="g.id" :data-testid="`gift-row-${g.id}`">
                  <td>{{ g.name }}</td>
                  <td>{{ g.price }}</td>
                  <td>
                    <span
                      class="badge"
                      :class="g.active === false ? 'closed' : 'live'"
                      data-testid="gift-status"
                    >
                      {{ g.active === false ? t('gifts.inactive') : t('gifts.active') }}
                    </span>
                  </td>
                  <td class="mono">{{ shortId(g.id, 12) }}</td>
                  <td class="actions">
                    <button
                      type="button"
                      class="btn sm"
                      data-testid="gift-edit"
                      :disabled="giftBusy"
                      @click="beginEditGift(g)"
                    >
                      {{ t('gifts.edit') }}
                    </button>
                    <button
                      type="button"
                      class="btn sm"
                      :class="g.active === false ? 'primary' : 'danger'"
                      data-testid="gift-toggle-active"
                      :disabled="giftBusy"
                      @click="toggleGiftActive(g)"
                    >
                      {{ g.active === false ? t('gifts.enable') : t('gifts.disable') }}
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="empty" data-testid="gifts-empty">
            <div class="empty-icon">✦</div>
            {{ t('gifts.empty') }}
          </div>
        </section>

        <!-- Users -->
        <section v-else-if="nav === 'users'" class="panel" data-testid="panel-users">
          <div class="panel-head">
            <h2>{{ t('users.title') }}</h2>
            <button type="button" class="btn sm" :disabled="usersBusy" @click="loadUsers">
              {{ t('common.refresh') }}
            </button>
          </div>
          <p class="panel-desc">{{ t('navBlurb.users') }}</p>
          <p v-if="tempPasswordNotice" class="flash ok" data-testid="temp-password">
            {{ tempPasswordNotice }}
          </p>

          <div class="split-actions" style="margin-bottom: 16px">
            <div class="action-card">
              <h3>{{ t('users.create') }}</h3>
              <label class="field">
                <span>{{ t('users.displayName') }}</span>
                <input v-model="createDisplayName" type="text" data-testid="create-display-name" />
              </label>
              <label class="field">
                <span>{{ t('users.username') }}</span>
                <input v-model="createUsername" type="text" data-testid="create-username" />
              </label>
              <label class="field">
                <span>{{ t('users.email') }}</span>
                <input v-model="createEmail" type="email" data-testid="create-email" />
              </label>
              <label class="field">
                <span>{{ t('users.password') }}</span>
                <input v-model="createPassword" type="text" data-testid="create-password" />
              </label>
              <button
                type="button"
                class="btn primary"
                data-testid="create-user-submit"
                :disabled="createBusy || !createDisplayName.trim()"
                @click="createUser"
              >
                {{ t('users.create') }}
              </button>
            </div>
            <div class="action-card">
              <h3>{{ t('users.search') }}</h3>
              <label class="field">
                <span>{{ t('users.searchPlaceholder') }}</span>
                <input
                  v-model="usersQuery"
                  type="search"
                  data-testid="users-query"
                  @keyup.enter="loadUsers"
                />
              </label>
              <button type="button" class="btn" :disabled="usersBusy" @click="loadUsers">
                {{ t('users.search') }}
              </button>
              <p class="dim">{{ t('users.total', { n: usersTotal }) }}</p>
            </div>
          </div>

          <div v-if="usersList.length" class="table-wrap">
            <table data-testid="users-table">
              <thead>
                <tr>
                  <th>{{ t('users.displayName') }}</th>
                  <th>{{ t('users.username') }}</th>
                  <th>{{ t('users.email') }}</th>
                  <th>{{ t('users.status') }}</th>
                  <th>{{ t('users.actions') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="u in usersList" :key="u.id">
                  <td>
                    {{ u.display_name }}
                    <div class="dim mono" style="font-size: 0.72rem">{{ shortId(u.id) }}</div>
                  </td>
                  <td class="mono">{{ u.username || '—' }}</td>
                  <td>{{ u.email || '—' }}</td>
                  <td>
                    {{ u.status }}
                    <span v-if="u.banned" class="pill danger">{{ t('users.banned') }}</span>
                    <span v-if="u.muted" class="pill">{{ t('users.muted') }}</span>
                  </td>
                  <td class="row" style="gap: 4px; flex-wrap: wrap">
                    <button type="button" class="btn sm" :disabled="actionBusy" @click="resetUserPassword(u.id)">
                      {{ t('users.resetPassword') }}
                    </button>
                    <button type="button" class="btn sm" :disabled="actionBusy" @click="revokeUserSessions(u.id)">
                      {{ t('users.revokeSessions') }}
                    </button>
                    <button
                      v-if="!u.banned"
                      type="button"
                      class="btn sm danger"
                      :disabled="actionBusy"
                      @click="banUserId(u.id)"
                    >
                      {{ t('users.ban') }}
                    </button>
                    <button
                      v-else
                      type="button"
                      class="btn sm"
                      :disabled="actionBusy"
                      @click="unbanUserId(u.id)"
                    >
                      {{ t('users.unban') }}
                    </button>
                    <button
                      v-if="u.status === 'active'"
                      type="button"
                      class="btn sm"
                      :disabled="actionBusy"
                      @click="setUserStatus(u.id, 'disabled')"
                    >
                      {{ t('users.disable') }}
                    </button>
                    <button
                      v-else-if="u.status === 'disabled'"
                      type="button"
                      class="btn sm primary"
                      :disabled="actionBusy"
                      @click="setUserStatus(u.id, 'active')"
                    >
                      {{ t('users.enable') }}
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="empty" data-testid="users-empty">
            <div class="empty-icon">◎</div>
            {{ usersBusy ? t('common.loading') : t('users.empty') }}
          </div>
        
          <div class="panel-head" style="margin-top: 1.5rem">
            <h3>{{ t('users.moderationLists') }}</h3>
            <button
              type="button"
              class="btn sm"
              data-testid="users-refresh"
              :disabled="listBusy"
              @click="loadModerationLists"
            >
              {{ t('users.refreshLists') }}
            </button>
          </div>
          <p class="panel-desc">{{ t('users.desc') }}</p>

          <div class="action-card" style="max-width: 640px; margin-bottom: 1rem" data-testid="users-lookup">
            <h3>{{ t('users.lookup') }}</h3>
            <label class="field">
              <span>{{ t('moderation.userId') }}</span>
              <input
                v-model="lookupUserId"
                class="mono"
                type="text"
                :placeholder="t('users.lookupPlaceholder')"
                data-testid="users-lookup-id"
              />
            </label>
            <div class="row" style="gap: 0.5rem; flex-wrap: wrap">
              <button
                type="button"
                class="btn primary"
                data-testid="users-lookup-submit"
                :disabled="lookupBusy || !lookupUserId.trim()"
                @click="lookupUserModeration"
              >
                {{ t('users.lookupSubmit') }}
              </button>
              <button
                type="button"
                class="btn sm"
                data-testid="users-fill-ban"
                :disabled="!lookupUserId.trim() && !lookupStatus"
                @click="fillBanFromLookup"
              >
                {{ t('users.fillBan') }}
              </button>
              <button
                type="button"
                class="btn sm"
                data-testid="users-fill-mute"
                :disabled="!lookupUserId.trim() && !lookupStatus"
                @click="fillMuteFromLookup"
              >
                {{ t('users.fillMute') }}
              </button>
            </div>
            <div v-if="lookupStatus" class="hint" style="margin-top: 0.75rem" data-testid="users-lookup-result">
              <strong>{{ t('users.statusTitle') }}</strong>
              <div class="mono" style="margin-top: 0.25rem">{{ shortId(lookupStatus.user_id, 16) }}</div>
              <div v-if="lookupStatus.banned || lookupStatus.muted" style="margin-top: 0.35rem">
                <span v-if="lookupStatus.banned" class="badge closed" style="margin-right: 0.35rem">
                  {{ t('users.statusBanned') }}
                  <template v-if="lookupStatus.ban_reason"> · {{ lookupStatus.ban_reason }}</template>
                  <template v-if="lookupStatus.banned_at">
                    · {{ t('users.statusSince', { time: formatTs(lookupStatus.banned_at) }) }}
                  </template>
                </span>
                <span v-if="lookupStatus.muted" class="badge idle">
                  {{ t('users.statusMuted') }}
                  <template v-if="lookupStatus.mute_reason"> · {{ lookupStatus.mute_reason }}</template>
                  <template v-if="lookupStatus.muted_at">
                    · {{ t('users.statusSince', { time: formatTs(lookupStatus.muted_at) }) }}
                  </template>
                </span>
              </div>
              <div v-else class="dim" style="margin-top: 0.35rem">{{ t('users.statusClear') }}</div>
              <div class="row" style="gap: 0.5rem; margin-top: 0.5rem; flex-wrap: wrap">
                <button
                  v-if="lookupStatus.banned"
                  type="button"
                  class="btn sm primary"
                  data-testid="users-lookup-unban"
                  :disabled="actionBusy"
                  @click="unbanUser(lookupStatus.user_id)"
                >
                  {{ t('users.unban') }}
                </button>
                <button
                  v-if="lookupStatus.muted"
                  type="button"
                  class="btn sm primary"
                  data-testid="users-lookup-unmute"
                  :disabled="actionBusy"
                  @click="unmuteUser(lookupStatus.user_id)"
                >
                  {{ t('users.unmute') }}
                </button>
              </div>
            </div>
          </div>

          <div class="split-actions">
            <div class="action-card" data-testid="users-banned-card">
              <h3>{{ t('users.bannedList') }} · {{ bannedUsers.length }}</h3>
              <div class="table-wrap" v-if="bannedUsers.length" data-testid="users-banned-table">
                <table class="data">
                  <thead>
                    <tr>
                      <th>{{ t('users.colUser') }}</th>
                      <th>{{ t('users.colReason') }}</th>
                      <th>{{ t('users.colTime') }}</th>
                      <th>{{ t('common.actions') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="u in bannedUsers" :key="u.user_id" :data-testid="`banned-row-${u.user_id}`">
                      <td class="mono">{{ shortId(u.user_id, 12) }}</td>
                      <td>{{ u.reason || t('common.none') }}</td>
                      <td class="mono">{{ formatTs(u.created_at) }}</td>
                      <td class="actions">
                        <button
                          type="button"
                          class="btn sm primary"
                          data-testid="users-unban-row"
                          :disabled="actionBusy"
                          @click="unbanUser(u.user_id)"
                        >
                          {{ t('users.unban') }}
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-else class="empty" data-testid="users-banned-empty">{{ t('users.emptyBanned') }}</div>
            </div>

            <div class="action-card" data-testid="users-muted-card">
              <h3>{{ t('users.mutedList') }} · {{ mutedUsers.length }}</h3>
              <div class="table-wrap" v-if="mutedUsers.length" data-testid="users-muted-table">
                <table class="data">
                  <thead>
                    <tr>
                      <th>{{ t('users.colUser') }}</th>
                      <th>{{ t('users.colReason') }}</th>
                      <th>{{ t('users.colTime') }}</th>
                      <th>{{ t('common.actions') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="u in mutedUsers" :key="u.user_id" :data-testid="`muted-row-${u.user_id}`">
                      <td class="mono">{{ shortId(u.user_id, 12) }}</td>
                      <td>{{ u.reason || t('common.none') }}</td>
                      <td class="mono">{{ formatTs(u.created_at) }}</td>
                      <td class="actions">
                        <button
                          type="button"
                          class="btn sm primary"
                          data-testid="users-unmute-row"
                          :disabled="actionBusy"
                          @click="unmuteUser(u.user_id)"
                        >
                          {{ t('users.unmute') }}
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-else class="empty" data-testid="users-muted-empty">{{ t('users.emptyMuted') }}</div>
            </div>
          </div>

        </section>

        <!-- Moderation -->
        <section v-else-if="nav === 'moderation'" class="panel" data-testid="panel-moderation">
          <div class="panel-head">
            <h2>{{ t('moderation.title') }}</h2>
          </div>
          <p class="panel-desc">{{ t('moderation.desc') }}</p>
          <p class="danger-note">{{ t('moderation.dangerHint') }}</p>

          <label class="field" style="max-width: min(480px, 100%)">
            <span>{{ t('moderation.reason') }}</span>
            <input
              v-model="actionReason"
              type="text"
              :placeholder="t('moderation.reasonPlaceholder')"
              data-testid="moderation-reason"
            />
          </label>

          <div class="split-actions">
            <div class="action-card danger-zone" data-testid="moderation-force-close">
              <h3>{{ t('moderation.forceClose') }}</h3>
              <label class="field">
                <span>{{ t('moderation.roomId') }}</span>
                <input
                  v-model="roomIdInput"
                  class="mono"
                  type="text"
                  placeholder="uuid"
                  data-testid="moderation-room-id"
                />
              </label>
              <button
                type="button"
                class="btn danger"
                data-testid="moderation-force-close-submit"
                :disabled="actionBusy || !roomIdInput.trim()"
                @click="forceCloseRoom()"
              >
                {{ t('moderation.forceCloseSubmit') }}
              </button>
            </div>

            <div class="action-card danger-zone" data-testid="moderation-ban">
              <h3>{{ t('moderation.ban') }}</h3>
              <label class="field">
                <span>{{ t('moderation.userId') }}</span>
                <input
                  v-model="userIdInput"
                  class="mono"
                  type="text"
                  placeholder="uuid"
                  data-testid="moderation-ban-user-id"
                />
              </label>
              <button
                type="button"
                class="btn danger"
                data-testid="moderation-ban-submit"
                :disabled="actionBusy || !userIdInput.trim()"
                @click="banUser"
              >
                {{ t('moderation.banSubmit') }}
              </button>
            </div>

            <div class="action-card danger-zone" data-testid="moderation-mute">
              <h3>{{ t('moderation.mute') }}</h3>
              <label class="field">
                <span>{{ t('moderation.userId') }}</span>
                <input
                  v-model="muteUserIdInput"
                  class="mono"
                  type="text"
                  placeholder="uuid"
                  data-testid="moderation-mute-user-id"
                />
              </label>
              <button
                type="button"
                class="btn danger"
                data-testid="moderation-mute-submit"
                :disabled="actionBusy || !muteUserIdInput.trim()"
                @click="muteUser"
              >
                {{ t('moderation.muteSubmit') }}
              </button>
            </div>

            <div class="action-card" data-testid="moderation-unmute">
              <h3>{{ t('moderation.unmute') }}</h3>
              <label class="field">
                <span>{{ t('moderation.userId') }}</span>
                <input
                  v-model="unmuteUserIdInput"
                  class="mono"
                  type="text"
                  placeholder="uuid"
                  data-testid="moderation-unmute-user-id"
                />
              </label>
              <button
                type="button"
                class="btn primary"
                data-testid="moderation-unmute-submit"
                :disabled="actionBusy || !unmuteUserIdInput.trim()"
                @click="unmuteUser"
              >
                {{ t('moderation.unmuteSubmit') }}
              </button>
            </div>
          </div>
        </section>

        <!-- Audit -->
        <section v-else-if="nav === 'audit'" class="panel" data-testid="panel-audit">
          <div class="panel-head">
            <h2>{{ t('audit.title') }}</h2>
            <button
              type="button"
              class="btn sm"
              data-testid="audit-refresh"
              :disabled="listBusy"
              @click="loadAudit"
            >
              {{ t('common.refresh') }}
            </button>
          </div>
          <p class="panel-desc">{{ t('audit.desc') }}</p>
          <div class="filter-bar">
            <label class="field">
              <span>{{ t('common.search') }}</span>
              <input
                v-model="auditQuery"
                type="search"
                :placeholder="t('audit.searchPlaceholder')"
                data-testid="audit-search"
              />
            </label>
          </div>
          <div class="table-wrap" v-if="filteredAudit.length" data-testid="audit-table">
            <table class="data">
              <thead>
                <tr>
                  <th>{{ t('audit.colAction') }}</th>
                  <th>{{ t('audit.colActor') }}</th>
                  <th>{{ t('audit.colTarget') }}</th>
                  <th>{{ t('audit.colDetail') }}</th>
                  <th>{{ t('audit.colTime') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="a in filteredAudit" :key="a.id" :data-testid="`audit-row-${a.id}`">
                  <td>{{ a.action }}</td>
                  <td class="mono">{{ shortId(a.actor_id) }}</td>
                  <td class="mono">{{ shortId(a.target) }}</td>
                  <td>{{ a.detail }}</td>
                  <td class="mono">{{ formatTs(a.created_at) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="empty" data-testid="audit-empty">
            <div class="empty-icon">☰</div>
            {{ t('audit.empty') }}
          </div>
        </section>
      </div>
    </div>
  </div>
</template>

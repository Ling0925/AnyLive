<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import {
  apiUrl,
  authHeaders,
  clientEventsBody,
  createPayOrderBody,
  creatorStatsPath,
  eventsPath,
  feedHotPath,
  giftsPath,
  meExportPath,
  mePath,
  otpSendBody,
  otpSendPath,
  otpVerifyBody,
  otpVerifyPath,
  parseChatMessage,
  parseChatMessages,
  parseCreatorStats,
  parseFeedRooms,
  parseGiftCatalog,
  parseGiftOrder,
  parsePayOrder,
  parsePayProducts,
  parsePkSession,
  parseWalletBalance,
  patchMeBody,
  payOrdersPath,
  payProductsPath,
  paySandboxCompletePath,
  postMessageBody,
  roomGiftsPath,
  roomMessagesPath,
  roomPkPath,
  searchPath,
  parseSearchResult,
  sendGiftBody,
  topupBody,
  walletPath,
  walletTopupPath,
  type ChatMessage,
  type CreatorStats,
  type FeedRoom,
  type GiftItem,
  type PayProduct,
  type PkSession,
  type SearchResult,
} from './lib/chatApi'
import { attachHls, buildPlayUrl, isLiveStatus } from './lib/hlsAttach'
import { isLoggedIn, normalizeEmail, parseAuthSession, type AuthSession } from './lib/session'
import {
  buildShareUrl,
  isRoomOffline,
  isRoomTerminal,
  readRoomFromQuery,
} from './lib/share'
import {
  centrifugoWsUrl,
  connectCentrifugoChat,
  parseRealtimeToken,
  realtimeTokenBody,
  realtimeTokenPath,
} from './lib/realtime'

const TOKEN_KEY = 'anylive_h5_token'
const SESSION_KEY = 'anylive_h5_session'

const apiBase = import.meta.env.VITE_API_BASE ?? 'http://localhost:8088'

/** SPA surface without Vue Router: Home discover vs RoomWatch. */
type AppView = 'home' | 'watch'
const view = ref<AppView>('home')

const roomId = ref('')
const status = ref('')
const roomTitle = ref('')
const ownerId = ref('')
const hlsUrl = ref('')
const error = ref('')
const loading = ref(false)
const shareHint = ref('')
const videoEl = ref<HTMLVideoElement | null>(null)
let detach: (() => void) | null = null

/** Public hot rooms for Home discover grid. */
const hotRooms = ref<FeedRoom[]>([])
const hotLoading = ref(false)
const hotError = ref('')

// --- optional login ---
const loginOpen = ref(false)
const email = ref('')
const otpCode = ref('')
const authBusy = ref(false)
const authHint = ref('')
const authError = ref('')
const session = ref<AuthSession | null>(null)
const accessToken = ref('')
/** Age gate required before Verify (mirrors mobile FL-2). */
const ageConfirmed = ref(false)
/** Optional privacy acceptance — sent with PATCH /me when checked. */
const privacyAccepted = ref(false)

// --- privacy / DSAR (authed) ---
const privacyOpen = ref(false)
const privacyBusy = ref(false)
const privacyHint = ref('')
const privacyError = ref('')
const exportJson = ref('')

// --- chat / gifts (authed only for send; list is public) ---
const messages = ref<ChatMessage[]>([])
const chatBody = ref('')
const chatBusy = ref(false)
const chatHint = ref('')
const gifts = ref<GiftItem[]>([])
const balance = ref(0)
const giftBusy = ref(false)
const giftHint = ref('')
const topupAmount = ref(100)
// Pay sandbox (coin packages → create order → sandbox-complete)
const payProducts = ref<PayProduct[]>([])
const payBusy = ref(false)
const payHint = ref('')
const pk = ref<PkSession | null>(null)
/** Server FEATURE_PK (from /api/v1/meta); default off for P1 dogfood. */
const featurePk = ref(false)
const creator = ref<CreatorStats | null>(null)
const creatorHint = ref('')
const onlineCount = ref(0)
const likeCount = ref(0)
const likeBusy = ref(false)
const giftOverlay = ref('')
const wsStatus = ref('')
const searchQ = ref('')
const searchBusy = ref(false)
const searchHint = ref('')
const searchResult = ref<SearchResult | null>(null)

const canWatch = computed(() => isLiveStatus(status.value) && !!hlsUrl.value)
/** Room loaded from API (id + status) — drives meta / chat chrome. */
const hasRoom = computed(() => !!roomId.value.trim() && !!status.value)
/** Not watchable (idle host-stop or closed/ended). */
const roomOffline = computed(() => isRoomOffline(status.value))
/** Permanent end only (force-close). Host stop is idle → offline, not terminal. */
const roomTerminal = computed(() => isRoomTerminal(status.value))
const authed = computed(() => isLoggedIn(accessToken.value))
const isHome = computed(() => view.value === 'home')
const isWatch = computed(() => view.value === 'watch')
/** Display title for meta row. */
const displayTitle = computed(() => {
  const t = roomTitle.value.trim()
  if (t) return t
  const id = roomId.value.trim()
  return id ? `Room ${id.slice(0, 8)}` : 'Live room'
})

function syncRoomQuery(id: string) {
  try {
    const url = new URL(window.location.href)
    const next = id.trim()
    if (next) url.searchParams.set('room', next)
    else url.searchParams.delete('room')
    const path = `${url.pathname}${url.search}${url.hash}`
    window.history.replaceState({}, '', path)
  } catch {
    // non-browser / odd href
  }
}

/** Leave RoomWatch and show Home discover. */
function goHome() {
  stopStatusPoll()
  stopChatPoll()
  stopPresencePoll()
  stopCentrifugo()
  teardownPlayer()
  roomId.value = ''
  status.value = ''
  roomTitle.value = ''
  ownerId.value = ''
  hlsUrl.value = ''
  error.value = ''
  loading.value = false
  messages.value = []
  gifts.value = []
  giftOverlay.value = ''
  onlineCount.value = 0
  likeCount.value = 0
  chatBody.value = ''
  chatHint.value = ''
  giftHint.value = ''
  pk.value = null
  shareHint.value = ''
  view.value = 'home'
  syncRoomQuery('')
  void loadHotFeed()
}

function enterWatch(id: string) {
  const next = id.trim()
  if (!next) return
  roomId.value = next
  view.value = 'watch'
  syncRoomQuery(next)
  void loadRoom()
}

function teardownPlayer() {
  detach?.()
  detach = null
}

watch([videoEl, hlsUrl], ([el, url]) => {
  teardownPlayer()
  if (el && url) {
    const handle = attachHls(el, url)
    detach = handle.destroy
    if (handle.mode === 'unsupported') {
      error.value = 'HLS not supported in this browser'
    }
  }
})

let statusPoll: ReturnType<typeof setInterval> | null = null
let chatPoll: ReturnType<typeof setInterval> | null = null
let presencePoll: ReturnType<typeof setInterval> | null = null
let stopWs: (() => void) | null = null

function stopStatusPoll() {
  if (statusPoll) {
    clearInterval(statusPoll)
    statusPoll = null
  }
}

function stopChatPoll() {
  if (chatPoll) {
    clearInterval(chatPoll)
    chatPoll = null
  }
}

function stopPresencePoll() {
  if (presencePoll) {
    clearInterval(presencePoll)
    presencePoll = null
  }
}

function stopCentrifugo() {
  stopWs?.()
  stopWs = null
  wsStatus.value = ''
}

async function refreshStats() {
  const id = roomId.value.trim()
  if (!id) return
  try {
    const res = await fetch(`${apiBase}/api/v1/rooms/${id}/stats`)
    if (!res.ok) return
    const j = await res.json()
    onlineCount.value = Number(j.online_count ?? 0)
    likeCount.value = Number(j.like_count ?? 0)
  } catch {
    // ignore
  }
}

async function heartbeatPresence() {
  const id = roomId.value.trim()
  if (!id || !authed.value) return
  try {
    const res = await fetch(`${apiBase}/api/v1/rooms/${id}/presence`, {
      method: 'POST',
      headers: authHeaders(accessToken.value),
    })
    if (res.ok) {
      const j = await res.json()
      onlineCount.value = Number(j.online_count ?? onlineCount.value)
    }
  } catch {
    // ignore
  }
}

function startPresencePoll() {
  stopPresencePoll()
  void heartbeatPresence()
  void refreshStats()
  presencePoll = setInterval(() => {
    if (roomTerminal.value || !isLiveStatus(status.value)) {
      stopPresencePoll()
      return
    }
    void heartbeatPresence()
    void refreshStats()
  }, 20000)
}

async function likeRoom() {
  const id = roomId.value.trim()
  if (!id || !authed.value || likeBusy.value) return
  likeBusy.value = true
  try {
    const res = await fetch(`${apiBase}/api/v1/rooms/${id}/likes`, {
      method: 'POST',
      headers: { ...authHeaders(accessToken.value), 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    })
    if (res.ok) {
      const j = await res.json()
      likeCount.value = Number(j.like_count ?? likeCount.value)
    }
  } finally {
    likeBusy.value = false
  }
}

async function tryConnectCentrifugo() {
  stopCentrifugo()
  const wsBase = centrifugoWsUrl(import.meta.env.VITE_CENTRIFUGO_WS as string | undefined)
  if (!wsBase || !authed.value) return
  const id = roomId.value.trim()
  if (!id) return
  try {
    const res = await fetch(`${apiBase}${realtimeTokenPath()}`, {
      method: 'POST',
      headers: { ...authHeaders(accessToken.value), 'Content-Type': 'application/json' },
      body: JSON.stringify(realtimeTokenBody(id)),
    })
    if (!res.ok) return
    const tok = parseRealtimeToken(await res.json())
    if (!tok) return
    const channel = tok.channels[0] || `room:${id}`
    stopWs = connectCentrifugoChat({
      wsUrl: wsBase,
      token: tok.token,
      channel,
      handlers: {
        onStatus: (s) => { wsStatus.value = s },
        onMessage: (m) => {
          if (messages.value.some((x) => x.id === m.id)) return
          messages.value = [
            ...messages.value,
            {
              id: m.id,
              roomId: id,
              body: m.body,
              senderId: m.senderId,
              senderName: m.senderName,
              createdAt: new Date().toISOString(),
            },
          ]
        },
      },
    })
  } catch {
    // keep HTTP poll
  }
}

function startStatusPoll() {
  stopStatusPoll()
  // Match mobile ~8s poll so stop/force-close/webhook flips ended UI without manual refresh.
  // Idle (host stop) keeps polling so re-go-live can reattach HLS.
  statusPoll = setInterval(() => {
    if (roomTerminal.value || !roomId.value.trim()) {
      stopStatusPoll()
      stopChatPoll()
      return
    }
    void pollRoomStatus()
  }, 8000)
}

function startChatPoll() {
  stopChatPoll()
  // HTTP history poll (Centrifugo WS optional via VITE_CENTRIFUGO_WS later).
  chatPoll = setInterval(() => {
    if (roomTerminal.value || !roomId.value.trim()) {
      stopChatPoll()
      return
    }
    void refreshMessages()
  }, 3000)
}

async function pollRoomStatus() {
  const id = roomId.value.trim()
  if (!id) return
  try {
    const roomRes = await fetch(`${apiBase}/api/v1/rooms/${id}`)
    if (!roomRes.ok) return
    const room = await roomRes.json()
    const next = typeof room.status === 'string' ? room.status : ''
    if (next && next !== status.value) {
      const wasLive = isLiveStatus(status.value)
      status.value = next
      if (typeof room.title === 'string' && room.title) {
        roomTitle.value = room.title
      }
      if (!isLiveStatus(next)) {
        hlsUrl.value = ''
        teardownPlayer()
        stopPresencePoll()
        if (isRoomTerminal(next)) {
          stopStatusPoll()
          stopChatPoll()
        }
      } else if (!wasLive && isLiveStatus(next)) {
        // Idle → live: re-fetch play URL.
        try {
          const playRes = await fetch(`${apiBase}/api/v1/rooms/${id}/media/play`)
          if (playRes.ok) {
            const play = await playRes.json()
            hlsUrl.value = play.hls ?? ''
          }
        } catch {
          // play may lag until OBS
        }
        startPresencePoll()
        void refreshGifts()
      }
    } else if (typeof room.title === 'string' && room.title && room.title !== roomTitle.value) {
      roomTitle.value = room.title
    }
    void refreshPk()
  } catch {
    // ignore transient poll errors
  }
}

onBeforeUnmount(() => {
  stopStatusPoll()
  stopChatPoll()
  stopPresencePoll()
  stopCentrifugo()
  teardownPlayer()
})

onMounted(() => {
  restoreSession()
  void loadFeatureFlags()
  void loadHotFeed()
  const fromQuery = readRoomFromQuery(window.location.search)
  if (fromQuery) {
    enterWatch(fromQuery)
  } else {
    view.value = 'home'
  }
})

async function loadHotFeed() {
  hotLoading.value = true
  hotError.value = ''
  try {
    const res = await fetch(apiUrl(apiBase, feedHotPath(12)))
    if (!res.ok) {
      hotError.value = `feed ${res.status}`
      hotRooms.value = []
      return
    }
    hotRooms.value = parseFeedRooms(await res.json())
  } catch (e) {
    hotError.value = e instanceof Error ? e.message : String(e)
    hotRooms.value = []
  } finally {
    hotLoading.value = false
  }
}

function openHotRoom(id: string) {
  enterWatch(id)
}

/** Soft-hide PK banner when FEATURE_PK is off (default for P1 dogfood). */
async function loadFeatureFlags() {
  try {
    const res = await fetch(apiUrl(apiBase, '/api/v1/meta'))
    if (!res.ok) return
    const j = (await res.json()) as { features?: { pk?: boolean } }
    featurePk.value = j.features?.pk === true
  } catch {
    featurePk.value = false
  }
}

function restoreSession() {
  try {
    const tok = localStorage.getItem(TOKEN_KEY) ?? ''
    const raw = localStorage.getItem(SESSION_KEY)
    if (tok && raw) {
      const parsed = JSON.parse(raw) as AuthSession
      if (parsed?.accessToken) {
        accessToken.value = tok
        session.value = parsed
        void refreshCreator()
      }
    }
  } catch {
    // ignore corrupt storage
  }
}

function persistSession(s: AuthSession) {
  accessToken.value = s.accessToken
  session.value = s
  try {
    localStorage.setItem(TOKEN_KEY, s.accessToken)
    localStorage.setItem(SESSION_KEY, JSON.stringify(s))
  } catch {
    // private mode
  }
}

function logout() {
  accessToken.value = ''
  session.value = null
  balance.value = 0
  creator.value = null
  creatorHint.value = ''
  privacyOpen.value = false
  privacyHint.value = ''
  privacyError.value = ''
  exportJson.value = ''
  ageConfirmed.value = false
  privacyAccepted.value = false
  try {
    localStorage.removeItem(TOKEN_KEY)
    localStorage.removeItem(SESSION_KEY)
  } catch {
    // ignore
  }
  authHint.value = 'Logged out'
}

/** Best-effort PATCH /me after OTP (age / privacy flags). Never blocks login. */
async function patchAgePrivacy(token: string) {
  if (!ageConfirmed.value && !privacyAccepted.value) return
  try {
    await fetch(apiUrl(apiBase, mePath()), {
      method: 'PATCH',
      headers: authHeaders(token),
      body: JSON.stringify(
        patchMeBody({
          ageConfirmed: ageConfirmed.value ? true : undefined,
          privacyAccepted: privacyAccepted.value ? true : undefined,
        }),
      ),
    })
  } catch {
    // non-fatal — session already persisted
  }
}

async function exportMyData() {
  privacyHint.value = ''
  privacyError.value = ''
  exportJson.value = ''
  if (!authed.value) {
    loginOpen.value = true
    return
  }
  privacyBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, meExportPath()), {
      headers: authHeaders(accessToken.value),
    })
    if (!res.ok) {
      privacyError.value = `export ${res.status}`
      return
    }
    const json = await res.json()
    const pretty = JSON.stringify(json, null, 2)
    exportJson.value = pretty
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(pretty)
        privacyHint.value = `Export copied (${pretty.length} chars)`
        return
      }
    } catch {
      // fall through
    }
    privacyHint.value = `Export ready (${pretty.length} chars) — select below to copy`
  } catch (e) {
    privacyError.value = e instanceof Error ? e.message : String(e)
  } finally {
    privacyBusy.value = false
  }
}

async function deleteAccount() {
  privacyHint.value = ''
  privacyError.value = ''
  if (!authed.value) {
    loginOpen.value = true
    return
  }
  const ok = window.confirm(
    'Delete account? This soft-deletes your account and signs you out.',
  )
  if (!ok) return
  privacyBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, mePath()), {
      method: 'DELETE',
      headers: authHeaders(accessToken.value),
    })
    if (!res.ok && res.status !== 204) {
      privacyError.value = `delete ${res.status}`
      return
    }
    logout()
    // logout clears authHint; surface delete outcome for the next login panel open.
    authHint.value = 'Account deleted — signed out'
  } catch (e) {
    privacyError.value = e instanceof Error ? e.message : String(e)
  } finally {
    privacyBusy.value = false
  }
}

async function runSearch() {
  const q = searchQ.value.trim()
  searchHint.value = ''
  searchResult.value = null
  if (!q) {
    searchHint.value = 'Enter a query'
    return
  }
  searchBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, searchPath(q, { type: 'all', limit: 10 })))
    if (!res.ok) {
      searchHint.value = `search ${res.status}`
      return
    }
    searchResult.value = parseSearchResult(await res.json())
    const n =
      (searchResult.value?.rooms.length ?? 0) + (searchResult.value?.users.length ?? 0)
    searchHint.value = n === 0 ? 'No matches' : `${n} hit(s)`
  } catch (e) {
    searchHint.value = e instanceof Error ? e.message : String(e)
  } finally {
    searchBusy.value = false
  }
}

function pickSearchRoom(id: string) {
  enterWatch(id)
}

function loadRoomFromInput() {
  const id = roomId.value.trim()
  if (!id) {
    error.value = 'Enter a room id'
    return
  }
  enterWatch(id)
}

async function loadRoom() {
  error.value = ''
  shareHint.value = ''
  hlsUrl.value = ''
  status.value = ''
  roomTitle.value = ''
  ownerId.value = ''
  messages.value = []
  gifts.value = []
  loading.value = true
  try {
    const id = roomId.value.trim()
    if (!id) {
      error.value = 'Enter a room id'
      return
    }
    const roomRes = await fetch(`${apiBase}/api/v1/rooms/${id}`)
    if (!roomRes.ok) {
      error.value = `room ${roomRes.status}`
      return
    }
    const room = await roomRes.json()
    status.value = typeof room.status === 'string' ? room.status : ''
    roomTitle.value = typeof room.title === 'string' ? room.title : ''
    ownerId.value = typeof room.owner_id === 'string' ? room.owner_id : ''
    // Always poll status so idle→live and closed are visible without full reload.
    startStatusPoll()

    // RoomWatch chrome (chat history + gift catalog) whenever room is known —
    // not only when HLS is ready, so the page never looks like "player only".
    if (!isRoomTerminal(status.value)) {
      startChatPoll()
      void refreshMessages()
    }
    void refreshGifts()
    void refreshPayProducts()
    void refreshPk()
    void refreshStats()
    if (authed.value) {
      void refreshBalance()
      void trackEvent('room.view', { room_id: id, status: status.value })
    }

    if (isRoomOffline(room.status) || !isLiveStatus(room.status)) {
      // Offline / non-live: keep meta+chat+gifts; no HLS until live.
      return
    }

    const playRes = await fetch(`${apiBase}/api/v1/rooms/${id}/media/play`)
    if (!playRes.ok) {
      error.value = `play ${playRes.status}`
      return
    }
    const play = await playRes.json()
    hlsUrl.value = play.hls ?? buildPlayUrl('http://localhost:8080/live', id)
    if (authed.value) {
      void tryConnectCentrifugo()
      startPresencePoll()
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

async function shareRoom() {
  shareHint.value = ''
  const id = roomId.value.trim()
  if (!id) {
    shareHint.value = 'Enter a room id first'
    return
  }
  const url = buildShareUrl(window.location.href, id)
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(url)
      shareHint.value = 'Link copied'
      return
    }
  } catch {
    // fall through to prompt
  }
  // Fallback when clipboard is unavailable
  window.prompt('Copy share link', url)
  shareHint.value = 'Share link ready'
}

async function sendOtp() {
  authError.value = ''
  authHint.value = ''
  const em = normalizeEmail(email.value)
  if (!em || !em.includes('@')) {
    authError.value = 'Enter a valid email'
    return
  }
  authBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, otpSendPath()), {
      method: 'POST',
      headers: authHeaders(null),
      body: JSON.stringify(otpSendBody(em)),
    })
    if (res.status !== 204 && !res.ok) {
      authError.value = `OTP send ${res.status}`
      return
    }
    authHint.value = 'Code sent (dev OTP: 123456)'
  } catch (e) {
    authError.value = String(e)
  } finally {
    authBusy.value = false
  }
}

async function verifyOtp() {
  authError.value = ''
  authHint.value = ''
  if (!ageConfirmed.value) {
    authError.value = 'Confirm you are 18 or older to continue'
    return
  }
  const em = normalizeEmail(email.value)
  const code = otpCode.value.trim()
  if (!em || !code) {
    authError.value = 'Email and code required'
    return
  }
  authBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, otpVerifyPath()), {
      method: 'POST',
      headers: authHeaders(null),
      body: JSON.stringify(otpVerifyBody(em, code)),
    })
    if (!res.ok) {
      authError.value = `OTP verify ${res.status}`
      return
    }
    const json = await res.json()
    const s = parseAuthSession(json)
    if (!s) {
      authError.value = 'Invalid session response'
      return
    }
    persistSession(s)
    // Best-effort age/privacy flags — same contract as mobile login.
    await patchAgePrivacy(s.accessToken)
    loginOpen.value = false
    authHint.value = `Hi, ${s.displayName || s.email || 'user'}`
    otpCode.value = ''
    void refreshBalance()
    void refreshCreator()
    void trackEvent('auth.login', { method: 'otp' })
  } catch (e) {
    authError.value = String(e)
  } finally {
    authBusy.value = false
  }
}

async function refreshMessages() {
  const id = roomId.value.trim()
  if (!id) return
  try {
    const res = await fetch(apiUrl(apiBase, roomMessagesPath(id, 30)))
    if (!res.ok) return
    messages.value = parseChatMessages(await res.json())
  } catch {
    // non-fatal
  }
}

async function sendChat() {
  chatHint.value = ''
  if (roomTerminal.value) {
    chatHint.value = 'Room ended'
    return
  }
  if (!canWatch.value && roomOffline.value) {
    chatHint.value = 'Room is offline (host stop) — wait for re-live or refresh'
    return
  }
  if (!canWatch.value) {
    chatHint.value = 'Room is not live'
    return
  }
  const id = roomId.value.trim()
  const body = chatBody.value.trim()
  if (!id || !body) {
    chatHint.value = 'Enter a message'
    return
  }
  if (!authed.value) {
    chatHint.value = 'Login to chat'
    loginOpen.value = true
    return
  }
  chatBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, roomMessagesPath(id)), {
      method: 'POST',
      headers: authHeaders(accessToken.value),
      body: JSON.stringify(postMessageBody(body)),
    })
    if (!res.ok) {
      chatHint.value = `send ${res.status}`
      return
    }
    const msg = parseChatMessage(await res.json())
    if (msg) {
      messages.value = [...messages.value, msg]
    }
    chatBody.value = ''
    chatHint.value = 'Sent'
    void trackEvent('chat.send', { room_id: id })
  } catch (e) {
    chatHint.value = String(e)
  } finally {
    chatBusy.value = false
  }
}

async function refreshGifts() {
  try {
    const res = await fetch(apiUrl(apiBase, giftsPath()))
    if (!res.ok) return
    gifts.value = parseGiftCatalog(await res.json())
  } catch {
    // non-fatal
  }
}

async function refreshBalance() {
  if (!authed.value) return
  try {
    const res = await fetch(apiUrl(apiBase, walletPath()), {
      headers: authHeaders(accessToken.value),
    })
    if (!res.ok) return
    balance.value = parseWalletBalance(await res.json())
  } catch {
    // non-fatal
  }
}

async function refreshPayProducts() {
  payHint.value = ''
  try {
    const res = await fetch(apiUrl(apiBase, payProductsPath()))
    if (!res.ok) {
      payHint.value = `pay products ${res.status}`
      return
    }
    payProducts.value = parsePayProducts(await res.json())
  } catch (e) {
    payHint.value = String(e)
  }
}

async function buyCoins(product: PayProduct) {
  payHint.value = ''
  if (!authed.value) {
    loginOpen.value = true
    return
  }
  payBusy.value = true
  try {
    const createRes = await fetch(apiUrl(apiBase, payOrdersPath()), {
      method: 'POST',
      headers: authHeaders(accessToken.value),
      body: JSON.stringify(
        createPayOrderBody({
          productId: product.id,
          channel: 'mock',
          clientRequestId: crypto.randomUUID?.() ?? `pay-${Date.now()}`,
        }),
      ),
    })
    if (!createRes.ok) {
      payHint.value = `create order ${createRes.status}`
      return
    }
    const order = parsePayOrder(await createRes.json())
    if (!order?.id) {
      payHint.value = 'invalid pay order response'
      return
    }
    // Server-side mock complete — no client secret required.
    const doneRes = await fetch(apiUrl(apiBase, paySandboxCompletePath(order.id)), {
      method: 'POST',
      headers: authHeaders(accessToken.value),
    })
    if (!doneRes.ok) {
      payHint.value = `sandbox complete ${doneRes.status}`
      return
    }
    const credited = parsePayOrder(await doneRes.json())
    void trackEvent('pay.order_create', {
      product_id: product.id,
      channel: order.channel ?? 'mock',
      order_id: order.id,
    })
    void trackEvent('pay.order_credit', {
      order_id: credited?.id ?? order.id,
      channel: credited?.channel ?? order.channel ?? 'mock',
    })
    await refreshBalance()
    payHint.value = credited
      ? `Paid ${credited.coins} coins (${credited.status}) · balance ${balance.value}`
      : `Paid · balance ${balance.value}`
  } catch (e) {
    payHint.value = String(e)
  } finally {
    payBusy.value = false
  }
}

async function refreshCreator() {
  creatorHint.value = ''
  if (!authed.value) {
    creator.value = null
    return
  }
  try {
    const res = await fetch(apiUrl(apiBase, creatorStatsPath()), {
      headers: authHeaders(accessToken.value),
    })
    if (!res.ok) {
      creatorHint.value = `creator ${res.status}`
      creator.value = null
      return
    }
    creator.value = parseCreatorStats(await res.json())
  } catch (e) {
    creatorHint.value = String(e)
    creator.value = null
  }
}

async function doTopup() {
  giftHint.value = ''
  if (!authed.value) {
    loginOpen.value = true
    return
  }
  const amount = Number(topupAmount.value) || 0
  if (amount <= 0) {
    giftHint.value = 'Topup amount must be > 0'
    return
  }
  giftBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, walletTopupPath()), {
      method: 'POST',
      headers: authHeaders(accessToken.value),
      body: JSON.stringify(topupBody(amount)),
    })
    if (!res.ok) {
      giftHint.value = `topup ${res.status}`
      return
    }
    balance.value = parseWalletBalance(await res.json())
    giftHint.value = `Balance ${balance.value}`
  } catch (e) {
    giftHint.value = String(e)
  } finally {
    giftBusy.value = false
    // gift overlay cleared via timeout below if set
  }
}

async function sendGift(gift: GiftItem) {
  giftHint.value = ''
  if (roomOffline.value || !canWatch.value) {
    giftHint.value = 'Room is not live'
    return
  }
  const id = roomId.value.trim()
  if (!id) {
    giftHint.value = 'Load a room first'
    return
  }
  if (!authed.value) {
    loginOpen.value = true
    giftHint.value = 'Login to send gifts'
    return
  }
  if (!ownerId.value) {
    giftHint.value = 'Room owner unknown'
    return
  }
  giftBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, roomGiftsPath(id)), {
      method: 'POST',
      headers: authHeaders(accessToken.value),
      body: JSON.stringify(
        sendGiftBody({
          giftId: gift.id,
          receiverId: ownerId.value,
          clientRequestId: crypto.randomUUID?.() ?? `h5-${Date.now()}`,
        }),
      ),
    })
    if (!res.ok) {
      giftHint.value = `gift ${res.status}`
      return
    }
    const order = parseGiftOrder(await res.json())
    giftHint.value = order
      ? `Sent ${gift.name} (−${order.totalCoins})`
      : `Sent ${gift.name}`
    void refreshBalance()
    void refreshPk()
    giftOverlay.value = 'Gift'; setTimeout(() => { if (giftOverlay.value === 'Gift') giftOverlay.value = '' }, 1800); void trackEvent('gift.tap', { room_id: id, gift_id: gift.id })
  } catch (e) {
    giftHint.value = String(e)
  } finally {
    giftBusy.value = false
  }
}

async function refreshPk() {
  const id = roomId.value.trim()
  if (!id || !featurePk.value) {
    pk.value = null
    return
  }
  try {
    const res = await fetch(apiUrl(apiBase, roomPkPath(id)), {
      headers: authHeaders(accessToken.value),
    })
    if (!res.ok) {
      pk.value = null
      return
    }
    pk.value = parsePkSession(await res.json())
  } catch {
    pk.value = null
  }
}

/** Best-effort analytics; never blocks UX. */
async function trackEvent(name: string, props?: Record<string, unknown>) {
  if (!authed.value) return
  try {
    await fetch(apiUrl(apiBase, eventsPath()), {
      method: 'POST',
      headers: authHeaders(accessToken.value),
      body: JSON.stringify(clientEventsBody([{ name, props }])),
    })
  } catch {
    // ignore
  }
}
</script>

<template>
  <main class="page" :class="{ 'page-home': isHome, 'page-watch': isWatch }">
    <!-- Sticky topbar: brand · nav · LIVE · auth -->
    <header class="top topbar">
      <div class="brand">
        <button
          type="button"
          class="brand-btn"
          data-testid="nav-home"
          :aria-current="isHome ? 'page' : undefined"
          @click="goHome"
        >
          <span class="logo-mark" aria-hidden="true" />
          <span class="brand-name">AnyLive</span>
        </button>
        <nav class="top-nav" aria-label="Primary">
          <button
            type="button"
            class="nav-link"
            :class="{ active: isHome }"
            data-testid="nav-home-tab"
            @click="goHome"
          >
            Home
          </button>
          <span v-if="isWatch" class="nav-sep" aria-hidden="true">/</span>
          <span v-if="isWatch" class="nav-watch-label muted">Watch</span>
        </nav>
        <span v-if="isWatch && canWatch" class="live-chip" role="status">
          <span class="live-dot" aria-hidden="true" />
          LIVE
        </span>
      </div>
      <div class="auth-chip">
        <template v-if="authed">
          <span class="chip">{{ session?.displayName || session?.email || 'signed in' }}</span>
          <button type="button" class="ghost" @click="privacyOpen = !privacyOpen">
            {{ privacyOpen ? 'Hide privacy' : 'Privacy' }}
          </button>
          <button type="button" class="ghost" @click="logout">Logout</button>
        </template>
        <button v-else type="button" class="ghost" @click="loginOpen = !loginOpen">
          {{ loginOpen ? 'Hide login' : 'Login' }}
        </button>
      </div>
    </header>

    <!-- Overlays: login / privacy (not main watch chrome) -->
    <section v-if="loginOpen && !authed" class="panel login" data-testid="login-panel">
      <h2>Login</h2>
      <p class="muted">Email OTP — optional. Watch works without login.</p>
      <div class="row">
        <input v-model="email" type="email" placeholder="you@example.com" autocomplete="email" />
        <button type="button" class="btn primary" :disabled="authBusy" @click="sendOtp">Send OTP</button>
      </div>
      <div class="row">
        <input v-model="otpCode" placeholder="OTP code" inputmode="numeric" autocomplete="one-time-code" />
        <button
          type="button"
          class="btn primary"
          data-testid="verify-otp"
          :disabled="authBusy || !ageConfirmed"
          @click="verifyOtp"
        >
          Verify
        </button>
      </div>
      <label class="check">
        <input v-model="ageConfirmed" type="checkbox" data-testid="age-confirmed" />
        I confirm I am 18 or older
      </label>
      <label class="check">
        <input v-model="privacyAccepted" type="checkbox" data-testid="privacy-accepted" />
        I accept the privacy policy
      </label>
      <p class="muted legal-links">
        <a href="https://anylive.example/privacy" target="_blank" rel="noopener">Privacy</a>
        ·
        <a href="https://anylive.example/terms" target="_blank" rel="noopener">Terms</a>
      </p>
      <p v-if="authHint" class="hint">{{ authHint }}</p>
      <p v-if="authError" class="err">{{ authError }}</p>
    </section>

    <section v-if="authed && privacyOpen" class="panel privacy" data-testid="privacy-panel">
      <div class="panel-head">
        <h2>Privacy</h2>
        <button type="button" class="ghost" @click="privacyOpen = false">Close</button>
      </div>
      <p class="muted">Export your data or delete this account (GDPR self-service).</p>
      <div class="row">
        <button
          type="button"
          class="btn primary"
          data-testid="export-data"
          :disabled="privacyBusy"
          @click="exportMyData"
        >
          Export my data
        </button>
        <button
          type="button"
          class="danger"
          data-testid="delete-account"
          :disabled="privacyBusy"
          @click="deleteAccount"
        >
          Delete account
        </button>
      </div>
      <p v-if="privacyHint" class="hint">{{ privacyHint }}</p>
      <p v-if="privacyError" class="err">{{ privacyError }}</p>
      <textarea
        v-if="exportJson"
        class="export-box"
        data-testid="export-json"
        readonly
        :value="exportJson"
        rows="8"
      />
    </section>

    <!-- ===== HOME: discover (not a room) ===== -->
    <section v-if="isHome" class="home-view" data-testid="home-view">
      <div class="home-hero">
        <h1 class="home-title">Live now</h1>
        <p class="muted home-sub">Pick a stream to watch. Login is optional for chat and gifts.</p>
      </div>

      <div class="home-search row">
        <input
          v-model="searchQ"
          placeholder="Search rooms / users"
          data-testid="home-search"
          @keyup.enter="runSearch"
        />
        <button type="button" class="btn primary" :disabled="searchBusy" @click="runSearch">Search</button>
        <button type="button" class="ghost" :disabled="hotLoading" @click="loadHotFeed">Refresh</button>
      </div>
      <p v-if="searchHint" class="hint">{{ searchHint }}</p>
      <ul v-if="searchResult" class="msg-list home-search-hits" data-testid="search-panel">
        <li v-for="r in searchResult.rooms" :key="'room-' + r.id">
          <button type="button" class="link" @click="pickSearchRoom(r.id)">
            {{ r.title || r.id }} ({{ r.status }})
          </button>
        </li>
        <li v-for="u in searchResult.users" :key="'user-' + u.id" class="muted">
          user · {{ u.displayName || u.id }}
        </li>
      </ul>

      <p v-if="hotError" class="err">{{ hotError }}</p>
      <div v-if="hotLoading && !hotRooms.length" class="home-skeleton" aria-busy="true">
        <div class="home-skel-card skeleton-block" />
        <div class="home-skel-card skeleton-block" />
        <div class="home-skel-card skeleton-block" />
      </div>
      <ul v-else-if="hotRooms.length" class="home-grid" data-testid="hot-feed">
        <li v-for="r in hotRooms" :key="r.id">
          <button type="button" class="home-card" @click="openHotRoom(r.id)">
            <div class="home-card-thumb" aria-hidden="true">
              <span
                class="live-chip live-chip-solid"
                :class="{ dim: r.status !== 'live' }"
              >{{ r.status === 'live' ? 'LIVE' : (r.status || '—') }}</span>
            </div>
            <div class="home-card-body">
              <span class="home-card-title">{{ r.title || `Room ${r.id.slice(0, 8)}` }}</span>
              <span class="muted mono home-card-id">{{ r.id.slice(0, 8) }}…</span>
            </div>
          </button>
        </li>
      </ul>
      <p v-else-if="!hotLoading" class="muted home-empty">No live rooms right now. Try search or paste a UUID below.</p>

      <details class="util-details home-tools">
        <summary class="util-summary muted">Paste room UUID · Tools</summary>
        <div class="util-strip row">
          <input v-model="roomId" placeholder="Room UUID" data-testid="room-id-input" />
          <button
            type="button"
            class="btn primary"
            data-testid="load-room"
            :disabled="loading"
            @click="loadRoomFromInput"
          >
            Open
          </button>
        </div>
        <p class="muted api-line mono">API · {{ apiBase }}</p>
      </details>
    </section>

    <div v-if="shareHint" class="share-toast" role="status">{{ shareHint }}</div>
    <p v-if="authHint && authed" class="hint">{{ authHint }}</p>
    <p v-if="error" class="err">{{ error }}</p>

    <!-- ===== WATCH: single room RoomWatch ===== -->
    <template v-if="isWatch">
      <div class="watch-toolbar">
        <button type="button" class="ghost back-home" data-testid="back-home" @click="goHome">
          ← Home
        </button>
        <details class="util-details watch-tools">
          <summary class="util-summary muted">Room tools</summary>
          <div class="util-strip row">
            <input v-model="roomId" placeholder="Room UUID" />
            <button type="button" class="btn primary" :disabled="loading" @click="loadRoomFromInput">
              Load
            </button>
            <button type="button" class="ghost" :disabled="!roomId.trim()" @click="shareRoom">Share</button>
          </div>
        </details>
      </div>

      <!-- Feature-gated PK — de-emphasized, unmounted when flag false -->
      <section v-if="featurePk && pk" class="panel pk pk-deemph" data-testid="pk-banner">
        <h2>PK {{ pk.status }}</h2>
        <p class="pk-score">
          {{ pk.scoreA }} – {{ pk.scoreB }}
          <span v-if="pk.winnerRoomId" class="muted"> · win {{ pk.winnerRoomId }}</span>
        </p>
      </section>

      <!-- RoomWatch layout: player → meta → channel → chat → gifts -->
      <div class="watch-layout" data-testid="watch-view">
        <div class="primary-col">
          <!-- Player stage (16:9) or end/offline overlays -->
          <div class="player-stage">
            <section v-if="roomTerminal" class="ended" role="status" data-testid="room-ended">
              <p class="ended-title">Stream ended</p>
              <p class="ended-sub">This room was force-closed</p>
              <p v-if="status" class="muted">status: {{ status }}</p>
              <button type="button" class="btn primary" @click="goHome">Back to Home</button>
            </section>

            <section
              v-else-if="roomOffline && status === 'idle'"
              class="ended offline"
              role="status"
              data-testid="room-offline"
            >
              <p class="ended-title">Host offline</p>
              <p class="ended-sub">Host stopped — room idle (may go live again)</p>
              <p class="muted">status: idle</p>
              <button type="button" class="ghost" @click="goHome">Back to Home</button>
            </section>

            <section v-else-if="canWatch" class="stage">
              <div class="player">
                <video ref="videoEl" controls playsinline class="player-video" />
                <span v-if="giftOverlay" class="gift-overlay">🎁 {{ giftOverlay }}</span>
              </div>
            </section>

            <div v-else-if="loading" class="player-skeleton" aria-busy="true">
              Loading room…
            </div>

            <div v-else class="player player-placeholder">
              <p class="muted">
                {{
                  hasRoom
                    ? `status: ${status}${hlsUrl ? '' : ' · waiting for stream'}`
                    : 'Loading room…'
                }}
              </p>
            </div>
          </div>

          <!-- Meta row: title · LIVE · online · like (always when room loaded) -->
          <div v-if="hasRoom" class="meta-row" data-testid="meta-row">
            <div class="meta-title">
              <span v-if="canWatch" class="live-chip live-chip-solid" role="status">
                <span class="live-dot" aria-hidden="true" />
                LIVE
              </span>
              <span v-else-if="status" class="status-chip muted">{{ status }}</span>
              <h2 class="room-title">{{ displayTitle }}</h2>
            </div>
            <div id="room-stats" class="room-stats meta-stats">
              <span class="stat-pill">{{ onlineCount }} watching</span>
              <button
                type="button"
                class="like-btn"
                :disabled="!authed || likeBusy || roomOffline || !canWatch"
                @click="likeRoom"
              >
                ♥ {{ likeCount }}
              </button>
              <span v-if="wsStatus" class="muted ws-pill">ws: {{ wsStatus }}</span>
            </div>
          </div>

          <!-- Channel row: host chip · more (creator) -->
          <div v-if="hasRoom" class="channel-row" data-testid="channel-row">
            <span class="channel-chip">
              <span class="channel-avatar" aria-hidden="true" />
              <span class="channel-meta">
                <span class="channel-name">{{ ownerId ? ownerId.slice(0, 8) : 'Host' }}</span>
                <span class="muted channel-id mono">{{ roomId.slice(0, 8) }}…</span>
              </span>
            </span>
            <details v-if="authed && creator" class="channel-more">
              <summary class="ghost channel-more-btn">⋯</summary>
              <section class="panel creator" data-testid="creator-panel">
                <div class="panel-head">
                  <h2>Creator</h2>
                  <button type="button" class="ghost" @click="refreshCreator">Refresh</button>
                </div>
                <p class="muted">
                  followers {{ creator.followerCount }} · following {{ creator.followingCount }} · live
                  {{ creator.liveRooms }}/{{ creator.totalRooms }} · gift coins
                  {{ creator.giftCoinsReceived }}
                </p>
                <p v-if="creatorHint" class="hint">{{ creatorHint }}</p>
              </section>
            </details>
            <button
              v-else
              type="button"
              class="ghost"
              :disabled="!roomId.trim()"
              @click="shareRoom"
            >
              Share
            </button>
          </div>

          <details v-if="canWatch && hlsUrl" class="hls-details">
            <summary class="mono dim">HLS URL</summary>
            <p class="mono dim">{{ hlsUrl }}</p>
          </details>
        </div>

        <div class="side-col">
          <!-- Chat panel — show whenever room is known (not only while HLS is up) -->
          <section
            v-if="hasRoom && !roomTerminal"
            class="panel chat chat-panel"
            data-testid="chat-panel"
          >
            <div class="panel-head">
              <h2>Live chat</h2>
              <button type="button" class="ghost" @click="refreshMessages">Refresh</button>
            </div>
            <ul class="msg-list">
              <li v-for="m in messages" :key="m.id">
                <strong>{{ m.senderName || m.senderId.slice(0, 6) }}</strong>
                <span>{{ m.body }}</span>
              </li>
              <li v-if="!messages.length" class="muted empty-msg">No messages yet</li>
            </ul>
            <div v-if="authed && canWatch" class="row composer">
              <input v-model="chatBody" placeholder="Say something…" @keyup.enter="sendChat" />
              <button type="button" class="btn primary" :disabled="chatBusy" @click="sendChat">Send</button>
            </div>
            <p v-else-if="authed && roomOffline" class="muted">Room offline — chat send disabled</p>
            <p v-else-if="authed && !canWatch" class="muted">Waiting for live stream — chat send disabled</p>
            <p v-else class="muted">
              <button type="button" class="link" @click="loginOpen = true">Login</button>
              to send chat
            </p>
            <p v-if="chatHint" class="hint">{{ chatHint }}</p>
          </section>

          <!-- Gift dock — catalog visible for any loaded room; send still requires live + auth -->
          <section
            v-if="hasRoom && !roomTerminal"
            class="panel gifts gift-dock"
            data-testid="gift-dock"
          >
            <div class="panel-head">
              <h2>Gifts</h2>
              <span v-if="authed" class="chip balance-chip">{{ balance }} coins</span>
            </div>
            <div v-if="authed" class="row topup">
              <input v-model.number="topupAmount" type="number" min="1" placeholder="Topup amount" />
              <button type="button" :disabled="giftBusy" @click="doTopup">Top up (legacy mock)</button>
              <button type="button" class="ghost" :disabled="giftBusy" @click="refreshBalance">Refresh</button>
            </div>
            <div v-if="authed && payProducts.length" class="pay-packs">
              <p class="muted">Coin packs (pay mock)</p>
              <div class="gift-bar">
                <button
                  v-for="p in payProducts"
                  :key="p.id"
                  type="button"
                  class="gift-btn"
                  :disabled="payBusy"
                  @click="buyCoins(p)"
                >
                  {{ p.title }}
                  <span class="price">{{ p.amount }} {{ p.currency }}</span>
                </button>
              </div>
              <p v-if="payHint" class="hint">{{ payHint }}</p>
            </div>
            <div class="gift-bar">
              <button
                v-for="g in gifts"
                :key="g.id"
                type="button"
                class="gift-btn"
                :disabled="giftBusy || !authed || !canWatch"
                @click="sendGift(g)"
              >
                {{ g.name }}
                <span class="price">{{ g.price }}</span>
              </button>
              <p v-if="!gifts.length" class="muted">No gifts in catalog</p>
            </div>
            <p v-if="!authed" class="muted">
              <button type="button" class="link" @click="loginOpen = true">Login</button>
              to send gifts &amp; top up
            </p>
            <p v-else-if="!canWatch" class="muted">Gifts send when the room is live</p>
            <p v-if="giftHint" class="hint">{{ giftHint }}</p>
          </section>
        </div>
      </div>
    </template>
  </main>
</template>

<style scoped>
.page {
  max-width: var(--page-max);
  margin: 0 auto;
  padding: 0 1rem 3.5rem;
  color: var(--text);
  background: transparent;
}

/* --- Topbar (compact sticky) --- */
.top,
.topbar {
  position: sticky;
  top: 0;
  z-index: 20;
  display: flex;
  justify-content: space-between;
  gap: var(--gap);
  align-items: center;
  min-height: var(--topbar-h);
  margin: 0 -1rem 0.5rem;
  padding: 0.5rem 1rem;
  background: rgba(15, 15, 15, 0.92);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border-bottom: 1px solid var(--border);
}

.brand h1,
.brand {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  margin: 0;
  font-size: 1rem;
  font-weight: 650;
  letter-spacing: 0.01em;
  min-width: 0;
  flex-wrap: wrap;
}

.brand-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--text);
  cursor: pointer;
  font: inherit;
  font-weight: 650;
}

.brand-btn:hover .brand-name {
  color: #e0b3ff;
}

.brand-name {
  font-size: 1rem;
  font-weight: 650;
}

.top-nav {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  margin-left: 0.35rem;
}

.nav-link {
  padding: 0.25rem 0.55rem;
  border: 0;
  border-radius: var(--radius-pill);
  background: transparent;
  color: var(--text-muted);
  font-size: var(--fs-sm);
  font-weight: 600;
  cursor: pointer;
}

.nav-link.active,
.nav-link:hover {
  color: var(--text);
  background: rgba(255, 255, 255, 0.06);
}

.nav-sep {
  color: var(--text-dim);
  font-size: var(--fs-sm);
}

.nav-watch-label {
  font-size: var(--fs-sm);
  font-weight: 500;
}

.logo-mark {
  display: inline-block;
  width: 22px;
  height: 22px;
  border-radius: 7px;
  background: linear-gradient(135deg, var(--accent), var(--accent-2));
  box-shadow: 0 0 12px var(--accent-glow);
  flex-shrink: 0;
}

/* --- Home discover --- */
.home-view {
  margin-top: 0.25rem;
}

.home-hero {
  margin: 0.5rem 0 1rem;
}

.home-title {
  margin: 0 0 0.25rem;
  font-size: var(--fs-xl);
  font-weight: 700;
  letter-spacing: 0.01em;
}

.home-sub {
  margin: 0;
  font-size: var(--fs-sm);
}

.home-search {
  margin: 0 0 0.75rem;
}

.home-search-hits {
  margin-bottom: 1rem;
  max-height: 160px;
}

.home-grid {
  list-style: none;
  margin: 0 0 1.25rem;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 0.85rem;
}

.home-card {
  width: 100%;
  display: flex;
  flex-direction: column;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--bg-elevated);
  color: var(--text);
  cursor: pointer;
  text-align: left;
  transition: border-color 0.15s ease, box-shadow 0.15s ease, transform 0.12s ease;
}

.home-card:hover {
  border-color: var(--border-accent);
  box-shadow: var(--shadow-sm);
  transform: translateY(-1px);
}

.home-card-thumb {
  position: relative;
  aspect-ratio: 16 / 9;
  background:
    linear-gradient(135deg, rgba(200, 80, 255, 0.18), transparent 55%),
    linear-gradient(180deg, #1a1a1a, #0a0a0a);
  display: flex;
  align-items: flex-start;
  justify-content: flex-start;
  padding: 0.55rem;
}

.home-card-body {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.65rem 0.75rem 0.75rem;
}

.home-card-title {
  font-weight: 650;
  font-size: var(--fs-sm);
  line-height: 1.3;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.home-card-id {
  font-size: var(--fs-xs);
}

.home-skeleton {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 0.85rem;
  margin-bottom: 1.25rem;
}

.home-skel-card {
  aspect-ratio: 16 / 10;
  border-radius: var(--radius-md);
}

.home-empty {
  padding: 1.5rem 0.5rem;
  text-align: center;
}

.home-tools {
  margin-top: 0.5rem;
}

.watch-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem 0.75rem;
  margin: 0 0 0.5rem;
}

.back-home {
  flex-shrink: 0;
}

.watch-tools {
  flex: 1;
  min-width: min(100%, 240px);
  margin: 0;
}

/* LIVE: red bg + white text (never magenta) */
.live-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  margin-left: 0.15rem;
  padding: 0.12rem 0.55rem;
  border-radius: var(--radius-pill);
  background: rgba(255, 0, 51, 0.14);
  border: 1px solid rgba(255, 0, 51, 0.45);
  color: var(--live);
  font-size: var(--fs-xs);
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.live-chip-solid {
  margin-left: 0;
  background: var(--live);
  border-color: var(--live);
  color: #fff;
}

.live-chip-solid .live-dot {
  background: #fff;
  box-shadow: none;
  animation: none;
}

.live-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--live);
  box-shadow: 0 0 0 0 rgba(255, 0, 51, 0.55);
  animation: live-pulse 1.4s ease-out infinite;
}

@keyframes live-pulse {
  0% {
    box-shadow: 0 0 0 0 rgba(255, 0, 51, 0.55);
  }
  70% {
    box-shadow: 0 0 0 6px rgba(255, 0, 51, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(255, 0, 51, 0);
  }
}

.api-line {
  margin: 0.35rem 0 0;
  font-size: var(--fs-xs);
}

.auth-chip {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  flex-shrink: 0;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.auth-chip .chip {
  border-color: var(--border-accent);
  background: var(--accent-soft);
}

/* --- Util strip (collapsible) --- */
.util-details {
  margin: 0 0 0.75rem;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
  padding: 0 0.75rem 0.25rem;
}

.util-summary {
  cursor: pointer;
  user-select: none;
  list-style: none;
  padding: 0.55rem 0;
  font-size: var(--fs-sm);
  font-weight: 500;
}

.util-summary::-webkit-details-marker {
  display: none;
}

.util-strip {
  margin: 0.25rem 0 0.5rem;
}

.hot-feed {
  margin: 0.5rem 0 0.75rem;
  padding: 0.75rem;
}

.hot-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 0.5rem;
}

.hot-card {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.35rem;
  padding: 0.65rem 0.75rem;
  text-align: left;
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  background: rgba(255, 255, 255, 0.03);
  color: var(--text);
  cursor: pointer;
}

.hot-card:hover {
  border-color: var(--border-accent);
  background: var(--accent-soft);
}

.hot-card-title {
  font-weight: 600;
  font-size: var(--fs-sm);
  line-height: 1.3;
}

.hot-card-meta {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: var(--fs-xs);
}

.status-chip {
  font-size: var(--fs-xs);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  padding: 0.1rem 0.45rem;
  border-radius: var(--radius-pill);
  border: 1px solid var(--border);
}

/* --- Rows / form controls --- */
.row {
  display: flex;
  gap: 0.5rem;
  margin: 0.75rem 0;
  flex-wrap: wrap;
  align-items: center;
}

input {
  flex: 1;
  min-width: 8rem;
  padding: 0.55rem 0.75rem;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text);
}

input:focus-visible {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}

button {
  padding: 0.55rem 1rem;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  color: var(--text);
  cursor: pointer;
  font-weight: 500;
  transition: border-color 0.15s ease, box-shadow 0.15s ease, transform 0.12s ease;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

button.btn.primary,
button.primary {
  background: var(--accent);
  border: 0;
  font-weight: 600;
  color: #fff;
  box-shadow: 0 4px 14px rgba(200, 80, 255, 0.25);
}

button.btn.primary:hover:not(:disabled),
button.primary:hover:not(:disabled) {
  box-shadow: var(--shadow-glow);
  filter: brightness(1.06);
}

button.ghost {
  background: transparent;
  border: 1px solid var(--border-strong);
}

button.ghost:hover:not(:disabled) {
  border-color: var(--border-accent);
  background: var(--accent-soft);
}

button.link {
  background: none;
  border: none;
  color: var(--accent);
  padding: 0;
  text-decoration: underline;
  font-weight: 500;
}

button.danger {
  background: var(--danger-bg);
  border-color: rgba(255, 77, 79, 0.4);
  color: var(--danger);
}

/* --- Utility text --- */
.muted {
  color: var(--text-muted);
}

.err {
  color: var(--danger);
}

.hint {
  color: var(--success);
  font-size: var(--fs-sm);
}

.mono {
  font-family: var(--mono);
  font-size: var(--fs-xs);
  word-break: break-all;
}

.dim {
  opacity: 0.55;
  color: var(--text-dim);
}

.chip {
  font-size: var(--fs-sm);
  padding: 0.2rem 0.6rem;
  border-radius: var(--radius-pill);
  background: var(--accent-soft);
  border: 1px solid var(--border-accent);
  color: var(--text);
}

.balance-chip {
  color: var(--coin);
  border-color: rgba(251, 191, 36, 0.35);
  background: rgba(251, 191, 36, 0.1);
}

/* --- Panels --- */
.panel {
  margin: 0.75rem 0;
  padding: 1rem;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
  box-shadow: var(--shadow-sm);
}

.panel.login,
.panel.privacy {
  max-width: 420px;
}

.panel h2 {
  margin: 0;
  font-size: var(--fs-md);
  font-weight: 650;
}

.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
  gap: 0.5rem;
}

/* --- RoomWatch layout (mobile-first stack) --- */
.watch-layout {
  display: flex;
  flex-direction: column;
  gap: var(--gap);
  margin-top: 0.25rem;
}

.primary-col,
.side-col {
  min-width: 0;
}

/* Player stage */
.player-stage {
  position: relative;
  width: 100%;
}

.stage {
  position: relative;
}

.player {
  position: relative;
  aspect-ratio: 16 / 9;
  background: var(--bg-stage);
  border-radius: var(--radius-md);
  overflow: hidden;
  box-shadow: var(--shadow-md);
  max-width: var(--stage-max);
}

.player-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px dashed var(--border-strong);
}

.player-video {
  width: 100%;
  height: 100%;
  object-fit: contain;
  max-width: none;
  display: block;
  background: var(--bg-stage);
}

/* Meta row under player */
.meta-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.65rem 1rem;
  padding: 0.35rem 0 0.15rem;
  max-width: var(--stage-max);
}

.meta-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-width: 0;
}

.room-title {
  margin: 0;
  font-size: var(--fs-lg);
  font-weight: 650;
  color: var(--text);
  line-height: 1.3;
}

.room-stats,
.meta-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  font-size: var(--fs-sm);
}

.stat-pill,
.ws-pill {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}

.like-btn {
  padding: 0.25rem 0.65rem;
  border-radius: var(--radius-pill);
  border: 1px solid var(--border);
  background: rgba(255, 255, 255, 0.06);
  font-size: var(--fs-sm);
}

.like-btn:not(:disabled):hover {
  border-color: var(--accent-hot);
  color: var(--accent-hot);
}

/* Channel row */
.channel-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.35rem 0 0.25rem;
  max-width: var(--stage-max);
}

.channel-chip {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  min-width: 0;
}

.channel-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--accent-soft), var(--bg-elevated));
  border: 1px solid var(--border-accent);
  flex-shrink: 0;
}

.channel-meta {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.channel-name {
  font-size: var(--fs-sm);
  font-weight: 600;
  color: var(--text);
}

.channel-id {
  font-size: var(--fs-xs);
}

.channel-more {
  position: relative;
}

.channel-more-btn {
  list-style: none;
  cursor: pointer;
  user-select: none;
  min-width: 2.25rem;
  text-align: center;
  font-size: 1.15rem;
  line-height: 1;
  padding: 0.35rem 0.55rem;
}

.channel-more-btn::-webkit-details-marker {
  display: none;
}

.channel-more[open] > .creator {
  position: absolute;
  right: 0;
  top: calc(100% + 6px);
  z-index: 12;
  min-width: min(280px, 80vw);
  margin: 0;
  box-shadow: var(--shadow-md);
}

.gift-overlay {
  position: absolute;
  top: 12px;
  right: 12px;
  z-index: 3;
  padding: 0.4rem 0.85rem;
  border-radius: var(--radius-pill);
  background: rgba(200, 80, 255, 0.22);
  border: 1px solid rgba(200, 80, 255, 0.5);
  color: #f0d4ff;
  font-weight: 650;
  font-size: var(--fs-sm);
  box-shadow: 0 0 20px rgba(200, 80, 255, 0.4);
  animation: gift-pop 1.6s ease-out forwards;
  pointer-events: none;
}

@keyframes gift-pop {
  0% {
    opacity: 0;
    transform: scale(0.85) translateY(6px);
  }
  18% {
    opacity: 1;
    transform: scale(1.06) translateY(0);
  }
  70% {
    opacity: 1;
    transform: scale(1);
  }
  100% {
    opacity: 0;
    transform: scale(0.98) translateY(-4px);
  }
}

.hls-details {
  margin-top: 0.25rem;
  max-width: var(--stage-max);
}

.hls-details summary {
  cursor: pointer;
  user-select: none;
  list-style: none;
  font-size: var(--fs-xs);
}

.hls-details summary::-webkit-details-marker {
  display: none;
}

/* --- Chat --- */
.msg-list {
  list-style: none;
  margin: 0 0 0.75rem;
  padding: 0;
  max-height: 220px;
  min-height: 120px;
  overflow-y: auto;
}

.msg-list li {
  padding: 6px 10px;
  margin: 4px 0;
  border: none;
  border-radius: 10px 10px 10px 4px;
  background: rgba(255, 255, 255, 0.04);
  font-size: var(--fs-sm);
}

.msg-list li.empty-msg {
  background: transparent;
  text-align: center;
  padding: 1.25rem 0.5rem;
  color: var(--text-dim);
}

.msg-list strong {
  margin-right: 0.45rem;
  color: var(--chat-name);
  font-weight: 600;
}

.composer {
  margin-bottom: 0;
}

/* --- Gift dock --- */
.gift-dock {
  /* mobile sticky dock so gifts stay reachable */
  position: sticky;
  bottom: 0;
  z-index: 10;
  margin-bottom: 0;
  border-color: var(--border-strong);
  background: rgba(33, 33, 33, 0.96);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
}

.gift-bar {
  display: flex;
  flex-wrap: nowrap;
  gap: 0.5rem;
  overflow-x: auto;
  padding-bottom: 0.15rem;
  -webkit-overflow-scrolling: touch;
  scrollbar-width: thin;
}

.gift-btn {
  display: inline-flex;
  flex-direction: column;
  align-items: center;
  min-width: 4.5rem;
  flex-shrink: 0;
  padding: 0.55rem 0.75rem;
  border-radius: var(--radius-md);
  background: linear-gradient(180deg, rgba(200, 80, 255, 0.12), transparent);
  border: 1px solid var(--border-accent);
  color: var(--text);
}

.gift-btn:hover:not(:disabled) {
  box-shadow: var(--shadow-glow);
  transform: translateY(-1px);
}

.gift-btn .price {
  font-size: var(--fs-xs);
  color: var(--coin);
  margin-top: 0.15rem;
  font-weight: 600;
}

.pay-packs {
  margin-bottom: 0.75rem;
}

.pay-packs .gift-btn {
  min-height: 3.4rem;
  justify-content: center;
}

/* --- Ended / offline (centered in stage area) --- */
.ended {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  aspect-ratio: 16 / 9;
  min-height: min(48vw, 220px);
  margin: 0;
  padding: 2rem 1.25rem;
  text-align: center;
  background: var(--bg-stage);
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-sm);
  max-width: var(--stage-max);
}

.ended.offline {
  border-color: rgba(251, 191, 36, 0.25);
  background: linear-gradient(180deg, rgba(251, 191, 36, 0.06), var(--bg-stage));
}

.ended-title {
  font-size: var(--fs-xl);
  font-weight: 650;
  margin: 0 0 0.35rem;
  color: var(--text);
}

.ended-sub {
  color: var(--text-muted);
  margin: 0 0 0.75rem;
  font-size: var(--fs-sm);
}

/* --- PK de-emphasized / creator / checks / export --- */
.pk {
  border-color: rgba(251, 191, 36, 0.22);
  background: linear-gradient(180deg, rgba(251, 191, 36, 0.05), var(--bg-elevated));
}

.pk-deemph {
  opacity: 0.85;
  font-size: var(--fs-sm);
}

.pk-score {
  font-size: var(--fs-lg);
  font-weight: 650;
  margin: 0.25rem 0 0;
  color: var(--coin);
}

.check {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0.5rem 0;
  font-size: var(--fs-sm);
  cursor: pointer;
}

.check input {
  flex: none;
  width: auto;
  min-width: 0;
  accent-color: var(--accent);
}

.check input:focus-visible {
  outline: none;
  box-shadow: 0 0 0 3px var(--accent-soft);
  border-radius: 3px;
}

.legal-links a {
  color: var(--accent);
}

.export-box {
  width: 100%;
  margin-top: 0.75rem;
  padding: 0.55rem 0.75rem;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text-muted);
  font-family: var(--mono);
  font-size: var(--fs-xs);
  resize: vertical;
}

/* --- Desktop ≥900px: player + side chat --- */
@media (min-width: 900px) {
  .watch-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--chat-w);
    gap: var(--gap-lg);
    align-items: start;
  }

  .primary-col {
    grid-column: 1;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .side-col {
    grid-column: 2;
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    position: sticky;
    top: calc(var(--topbar-h) + 12px);
    max-height: calc(100svh - var(--topbar-h) - 24px);
  }

  .side-col .chat-panel {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    margin: 0;
  }

  .side-col .msg-list {
    flex: 1;
    max-height: none;
    min-height: 280px;
  }

  .side-col .gift-dock {
    margin: 0;
    flex-shrink: 0;
    position: static;
  }

  .ended {
    min-height: min(42vw, 360px);
    max-width: none;
  }

  .player {
    max-width: none;
  }

  .meta-row,
  .channel-row {
    max-width: none;
  }
}
</style>

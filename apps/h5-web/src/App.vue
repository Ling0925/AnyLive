<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import {
  apiUrl,
  authHeaders,
  clientEventsBody,
  createPayOrderBody,
  creatorStatsPath,
  eventsPath,
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
const roomId = ref('')
const status = ref('')
const ownerId = ref('')
const hlsUrl = ref('')
const error = ref('')
const loading = ref(false)
const shareHint = ref('')
const videoEl = ref<HTMLVideoElement | null>(null)
let detach: (() => void) | null = null

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
/** Not watchable (idle host-stop or closed/ended). */
const roomOffline = computed(() => isRoomOffline(status.value))
/** Permanent end only (force-close). Host stop is idle → offline, not terminal. */
const roomTerminal = computed(() => isRoomTerminal(status.value))
const authed = computed(() => isLoggedIn(accessToken.value))

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
      }
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
  const fromQuery = readRoomFromQuery(window.location.search)
  if (fromQuery) {
    roomId.value = fromQuery
    void loadRoom()
  }
})

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
  roomId.value = id
  void loadRoom()
}

async function loadRoom() {
  error.value = ''
  shareHint.value = ''
  hlsUrl.value = ''
  status.value = ''
  ownerId.value = ''
  messages.value = []
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
    status.value = room.status
    ownerId.value = typeof room.owner_id === 'string' ? room.owner_id : ''
    // Always poll status so idle→live and closed are visible without full reload.
    startStatusPoll()
    if (isRoomOffline(room.status)) {
      // Terminal / temporary offline UI — no HLS until live again.
      if (!isRoomTerminal(room.status)) {
        startChatPoll()
        void refreshMessages()
      }
      if (authed.value) {
        void trackEvent('room.view', { room_id: id, status: status.value })
      }
      return
    }
    if (!isLiveStatus(room.status)) {
      error.value = `Room status: ${room.status}`
      return
    }
    const playRes = await fetch(`${apiBase}/api/v1/rooms/${id}/media/play`)
    if (!playRes.ok) {
      error.value = `play ${playRes.status}`
      return
    }
    const play = await playRes.json()
    hlsUrl.value = play.hls ?? buildPlayUrl('http://localhost:8080/live', id)
    startChatPoll()
    if (authed.value) {
      void tryConnectCentrifugo()
      startPresencePoll()
    }

    // Public chat history + gift catalog (no auth required)
    void refreshMessages()
    void refreshGifts()
    void refreshPayProducts()
    void refreshPk()
    if (authed.value) {
      void refreshBalance()
      void trackEvent('room.view', { room_id: id, status: status.value })
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
  <main class="page">
    <header class="top topbar">
      <div class="brand">
        <h1>
          <span class="logo-mark" aria-hidden="true" />
          AnyLive
          <span v-if="canWatch" class="live-chip" role="status">
            <span class="live-dot" aria-hidden="true" />
            LIVE
          </span>
        </h1>
        <p class="muted api-line">Watch · {{ apiBase }}</p>
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

    <div class="util-strip row">
      <input v-model="roomId" placeholder="Room UUID" />
      <button type="button" class="btn primary" :disabled="loading" @click="loadRoom">Load</button>
      <button type="button" class="ghost" :disabled="!roomId.trim()" @click="shareRoom">Share</button>
    </div>
    <section class="panel search" data-testid="search-panel">
      <div class="row">
        <input
          v-model="searchQ"
          placeholder="Search rooms / users"
          @keyup.enter="runSearch"
        />
        <button type="button" class="btn primary" :disabled="searchBusy" @click="runSearch">Search</button>
      </div>
      <p v-if="searchHint" class="hint">{{ searchHint }}</p>
      <ul v-if="searchResult" class="msg-list">
        <li v-for="r in searchResult.rooms" :key="'room-' + r.id">
          <button type="button" class="link" @click="pickSearchRoom(r.id)">
            {{ r.title || r.id }} ({{ r.status }})
          </button>
        </li>
        <li v-for="u in searchResult.users" :key="'user-' + u.id" class="muted">
          user · {{ u.displayName || u.id }}
        </li>
      </ul>
    </section>
    <p v-if="shareHint" class="hint">{{ shareHint }}</p>
    <p v-if="authHint && authed" class="hint">{{ authHint }}</p>
    <p v-if="error" class="err">{{ error }}</p>

    <section v-if="featurePk && pk" class="panel pk" data-testid="pk-banner">
      <h2>PK {{ pk.status }}</h2>
      <p class="pk-score">
        {{ pk.scoreA }} – {{ pk.scoreB }}
        <span v-if="pk.winnerRoomId" class="muted"> · win {{ pk.winnerRoomId }}</span>
      </p>
    </section>

    <section v-if="authed && creator" class="panel creator" data-testid="creator-panel">
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

    <div class="watch-grid">
      <div class="stage-col">
        <section v-if="roomTerminal" class="ended" role="status" data-testid="room-ended">
          <p class="ended-title">直播已结束</p>
          <p class="ended-sub">Room ended (force-closed)</p>
          <p v-if="status" class="muted">status: {{ status }}</p>
        </section>

        <section
          v-else-if="roomOffline && status === 'idle'"
          class="ended offline"
          role="status"
          data-testid="room-offline"
        >
          <p class="ended-title">暂时离线</p>
          <p class="ended-sub">Host stopped — room idle (may go live again)</p>
          <p class="muted">status: idle</p>
        </section>

        <p v-else-if="status" class="muted status-line">status: {{ status }}</p>

        <section v-if="canWatch" class="stage">
          <div class="player">
            <video ref="videoEl" controls playsinline class="player-video" />
            <div id="room-stats" class="room-stats">
              <span class="stat-pill">{{ onlineCount }} online</span>
              <button
                type="button"
                class="like-btn"
                :disabled="!authed || likeBusy || roomOffline"
                @click="likeRoom"
              >
                ♥ {{ likeCount }}
              </button>
              <span v-if="wsStatus" class="muted ws-pill">ws: {{ wsStatus }}</span>
              <span v-if="giftOverlay" class="gift-overlay">🎁 {{ giftOverlay }}</span>
            </div>
          </div>
          <details class="hls-details">
            <summary class="mono dim">HLS URL</summary>
            <p class="mono dim">{{ hlsUrl }}</p>
          </details>
        </section>
      </div>

      <div class="side-col">
        <!-- Chat: list is public while not permanently closed; send requires live + login -->
        <section v-if="roomId.trim() && status && !roomTerminal" class="panel chat">
          <div class="panel-head">
            <h2>Chat</h2>
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
          <p v-else class="muted">
            <button type="button" class="link" @click="loginOpen = true">Login</button>
            to send chat
          </p>
          <p v-if="chatHint" class="hint">{{ chatHint }}</p>
        </section>

        <!-- Gifts + wallet: only when live + can watch -->
        <section v-if="roomId.trim() && status && canWatch" class="panel gifts">
          <div class="panel-head">
            <h2>Gifts</h2>
            <span v-if="authed" class="chip balance-chip">balance: {{ balance }}</span>
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
              :disabled="giftBusy || !authed"
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
          <p v-if="giftHint" class="hint">{{ giftHint }}</p>
        </section>
      </div>
    </div>
  </main>
</template>

<style scoped>
.page {
  max-width: var(--page-max);
  margin: 0 auto;
  padding: 0 1rem 3rem;
  color: var(--text);
  background: transparent;
}

/* --- Topbar --- */
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
  margin: 0 -1rem 0.75rem;
  padding: 0.65rem 1rem;
  background: rgba(10, 10, 16, 0.72);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border-bottom: 1px solid var(--border);
}

.brand h1 {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  margin: 0;
  font-size: 1rem;
  font-weight: 650;
  letter-spacing: 0.01em;
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

.live-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  margin-left: 0.25rem;
  padding: 0.12rem 0.55rem;
  border-radius: var(--radius-pill);
  background: rgba(255, 51, 85, 0.16);
  border: 1px solid rgba(255, 51, 85, 0.45);
  color: var(--live);
  font-size: var(--fs-xs);
  font-weight: 700;
  letter-spacing: 0.04em;
}

.live-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--live);
  box-shadow: 0 0 0 0 rgba(255, 51, 85, 0.55);
  animation: live-pulse 1.4s ease-out infinite;
}

@keyframes live-pulse {
  0% {
    box-shadow: 0 0 0 0 rgba(255, 51, 85, 0.55);
  }
  70% {
    box-shadow: 0 0 0 6px rgba(255, 51, 85, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(255, 51, 85, 0);
  }
}

.api-line {
  margin: 0.15rem 0 0;
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

/* --- Rows / form controls --- */
.row {
  display: flex;
  gap: 0.5rem;
  margin: 0.75rem 0;
  flex-wrap: wrap;
  align-items: center;
}

.util-strip {
  margin: 0.5rem 0 0.75rem;
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
  background: linear-gradient(135deg, var(--accent), var(--accent-2));
  border: 0;
  font-weight: 600;
  color: #fff;
  box-shadow: 0 4px 14px rgba(200, 80, 255, 0.25);
}

button.btn.primary:hover:not(:disabled),
button.primary:hover:not(:disabled) {
  box-shadow: var(--shadow-glow);
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
  border-color: rgba(248, 113, 113, 0.4);
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
}

/* --- Glass panels --- */
.panel {
  margin: 0.85rem 0;
  padding: 1rem;
  border: var(--glass-border);
  border-radius: var(--radius-md);
  background: var(--bg-panel);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
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

/* --- Watch grid --- */
.watch-grid {
  display: flex;
  flex-direction: column;
  gap: var(--gap);
  margin-top: 0.5rem;
}

.stage-col,
.side-col {
  min-width: 0;
}

/* --- Stage / player --- */
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

.player-video {
  width: 100%;
  height: 100%;
  object-fit: contain;
  max-width: none;
  display: block;
  background: var(--bg-stage);
}

.room-stats {
  position: absolute;
  left: 10px;
  bottom: 10px;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  background: var(--bg-panel);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--border);
  font-size: var(--fs-sm);
  z-index: 2;
}

.stat-pill,
.ws-pill {
  font-size: var(--fs-xs);
}

.like-btn {
  padding: 0.2rem 0.55rem;
  border-radius: var(--radius-pill);
  border: 1px solid var(--border);
  background: rgba(255, 255, 255, 0.06);
  font-size: var(--fs-sm);
}

.like-btn:not(:disabled):hover {
  border-color: var(--accent-hot);
  color: var(--accent-hot);
}

.gift-overlay {
  position: absolute;
  top: -2.6rem;
  right: 0;
  padding: 0.35rem 0.7rem;
  border-radius: var(--radius-pill);
  background: rgba(255, 61, 154, 0.18);
  border: 1px solid rgba(255, 61, 154, 0.45);
  color: var(--accent-hot);
  font-weight: 650;
  font-size: var(--fs-sm);
  box-shadow: 0 0 18px rgba(255, 61, 154, 0.35);
  animation: gift-fade 1.8s ease-out forwards;
  pointer-events: none;
}

@keyframes gift-fade {
  0% {
    opacity: 0;
    transform: translateY(4px);
  }
  15% {
    opacity: 1;
    transform: translateY(0);
  }
  70% {
    opacity: 1;
  }
  100% {
    opacity: 0;
  }
}

.hls-details {
  margin-top: 0.45rem;
  max-width: var(--stage-max);
}

.hls-details summary {
  cursor: pointer;
  user-select: none;
  list-style: none;
}

.hls-details summary::-webkit-details-marker {
  display: none;
}

.status-line {
  margin: 0.5rem 0;
  font-size: var(--fs-sm);
}

/* --- Chat bubbles --- */
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

/* --- Gifts --- */
.gift-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.gift-btn {
  display: inline-flex;
  flex-direction: column;
  align-items: center;
  min-width: 4.5rem;
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

/* --- Ended / offline --- */
.ended {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  min-height: min(48vw, 280px);
  margin: 0.5rem 0;
  padding: 2rem 1.25rem;
  text-align: center;
  background: var(--bg-panel-solid);
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-sm);
  max-width: var(--stage-max);
}

.ended.offline {
  border-color: rgba(251, 191, 36, 0.25);
  background: linear-gradient(180deg, rgba(251, 191, 36, 0.06), var(--bg-panel-solid));
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

/* --- PK / creator / checks / export --- */
.pk {
  border-color: rgba(251, 191, 36, 0.28);
  background: linear-gradient(180deg, rgba(251, 191, 36, 0.08), var(--bg-panel));
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

/* --- Desktop ≥900px --- */
@media (min-width: 900px) {
  .watch-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--chat-w);
    grid-template-rows: auto 1fr;
    gap: var(--gap-lg);
    align-items: start;
  }

  .stage-col {
    grid-column: 1;
  }

  .side-col {
    grid-column: 2;
    grid-row: 1 / span 2;
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    position: sticky;
    top: calc(var(--topbar-h) + 12px);
    max-height: calc(100svh - var(--topbar-h) - 24px);
  }

  .side-col .chat {
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

  .side-col .gifts {
    margin: 0;
    flex-shrink: 0;
  }

  .ended {
    min-height: min(42vw, 360px);
  }

  .player {
    max-width: none;
  }
}
</style>

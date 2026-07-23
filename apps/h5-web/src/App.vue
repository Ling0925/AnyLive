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
    <header class="top">
      <div>
        <h1>AnyLive Watch</h1>
        <p class="muted">Public H5 player (hls.js / native). API: {{ apiBase }}</p>
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
        <button type="button" :disabled="authBusy" @click="sendOtp">Send OTP</button>
      </div>
      <div class="row">
        <input v-model="otpCode" placeholder="OTP code" inputmode="numeric" autocomplete="one-time-code" />
        <button
          type="button"
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

    <div class="row">
      <input v-model="roomId" placeholder="Room UUID" />
      <button :disabled="loading" @click="loadRoom">Load</button>
      <button type="button" :disabled="!roomId.trim()" @click="shareRoom">Share</button>
    </div>
    <section class="panel search" data-testid="search-panel">
      <div class="row">
        <input
          v-model="searchQ"
          placeholder="Search rooms / users"
          @keyup.enter="runSearch"
        />
        <button type="button" :disabled="searchBusy" @click="runSearch">Search</button>
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

    <p v-else-if="status" class="muted">status: {{ status }}</p>

    <section v-if="canWatch" class="player">
      <p class="mono">{{ hlsUrl }}</p>
            <div id="room-stats" class="row" style="gap:12px;align-items:center;margin:8px 0">
        <span>{{ onlineCount }} online</span>
        <button type="button" :disabled="!authed || likeBusy || roomOffline" @click="likeRoom">♥ {{ likeCount }}</button>
        <span v-if="wsStatus" class="muted">ws: {{ wsStatus }}</span>
        <span v-if="giftOverlay" class="gift-overlay">🎁 {{ giftOverlay }}</span>
      </div>
      <video ref="videoEl" controls playsinline style="width: 100%; max-width: 720px; background: #000" />
    </section>

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
        <li v-if="!messages.length" class="muted">No messages yet</li>
      </ul>
      <div v-if="authed && canWatch" class="row">
        <input v-model="chatBody" placeholder="Say something…" @keyup.enter="sendChat" />
        <button type="button" :disabled="chatBusy" @click="sendChat">Send</button>
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
        <span v-if="authed" class="chip">balance: {{ balance }}</span>
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
  </main>
</template>

<style scoped>
.page {
  font-family: system-ui, sans-serif;
  max-width: 800px;
  margin: 2rem auto;
  padding: 0 1rem 3rem;
  color: #e8e8e8;
  background: transparent;
}
.top {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: flex-start;
}
.top h1 {
  margin: 0 0 0.25rem;
}
.auth-chip {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  flex-shrink: 0;
}
.row {
  display: flex;
  gap: 0.5rem;
  margin: 1rem 0;
  flex-wrap: wrap;
}
input {
  flex: 1;
  min-width: 8rem;
  padding: 0.5rem;
  border: 1px solid #333;
  border-radius: 6px;
  background: #1a1a1a;
  color: #eee;
}
button {
  padding: 0.5rem 1rem;
  border: 1px solid #444;
  border-radius: 6px;
  background: #2a2a2a;
  color: #eee;
  cursor: pointer;
}
button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
button.ghost {
  background: transparent;
}
button.link {
  background: none;
  border: none;
  color: #6af;
  padding: 0;
  text-decoration: underline;
}
.muted {
  color: #888;
}
.err {
  color: #f66;
}
.hint {
  color: #5c8;
  font-size: 0.9rem;
}
.mono {
  font-family: ui-monospace, monospace;
  font-size: 0.85rem;
  word-break: break-all;
  color: #aaa;
}
.panel {
  margin: 1.25rem 0;
  padding: 1rem;
  border: 1px solid #2a2a2a;
  border-radius: 8px;
  background: #141414;
}
.panel h2 {
  margin: 0;
  font-size: 1.05rem;
}
.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
}
.chip {
  font-size: 0.85rem;
  padding: 0.2rem 0.55rem;
  border-radius: 999px;
  background: #222;
  border: 1px solid #333;
}
.msg-list {
  list-style: none;
  margin: 0 0 0.75rem;
  padding: 0;
  max-height: 180px;
  overflow-y: auto;
}
.msg-list li {
  padding: 0.25rem 0;
  border-bottom: 1px solid #222;
  font-size: 0.9rem;
}
.msg-list strong {
  margin-right: 0.5rem;
  color: #9cf;
}
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
  background: #1e1e28;
  border-color: #3a3a55;
}
.gift-btn .price {
  font-size: 0.75rem;
  color: #fc6;
  margin-top: 0.15rem;
}
.ended {
  margin: 2rem 0;
  padding: 2rem 1rem;
  text-align: center;
  background: #1a1a1a;
  border-radius: 8px;
  border: 1px solid #333;
}
.ended-title {
  font-size: 1.5rem;
  font-weight: 600;
  margin: 0 0 0.25rem;
}
.ended-sub {
  color: #888;
  margin: 0 0 0.75rem;
}
.pk {
  border-color: #5a4a20;
  background: #1f1a10;
}
.pk-score {
  font-size: 1.25rem;
  font-weight: 600;
  margin: 0.25rem 0 0;
  color: #fc6;
}
.check {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0.5rem 0;
  font-size: 0.95rem;
  cursor: pointer;
}
.check input {
  flex: none;
  width: auto;
  min-width: 0;
}
.legal-links a {
  color: #6af;
}
button.danger {
  background: #3a1a1a;
  border-color: #833;
  color: #f99;
}
.export-box {
  width: 100%;
  margin-top: 0.75rem;
  padding: 0.5rem;
  border: 1px solid #333;
  border-radius: 6px;
  background: #0d0d0d;
  color: #ccc;
  font-family: ui-monospace, monospace;
  font-size: 0.75rem;
  box-sizing: border-box;
  resize: vertical;
}
</style>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import {
  apiUrl,
  authHeaders,
  giftsPath,
  otpSendBody,
  otpSendPath,
  otpVerifyBody,
  otpVerifyPath,
  parseChatMessage,
  parseChatMessages,
  parseGiftCatalog,
  parseGiftOrder,
  parseWalletBalance,
  postMessageBody,
  roomGiftsPath,
  roomMessagesPath,
  sendGiftBody,
  topupBody,
  walletPath,
  walletTopupPath,
  type ChatMessage,
  type GiftItem,
} from './lib/chatApi'
import { attachHls, buildPlayUrl, isLiveStatus } from './lib/hlsAttach'
import { isLoggedIn, normalizeEmail, parseAuthSession, type AuthSession } from './lib/session'
import { buildShareUrl, isRoomEnded, readRoomFromQuery } from './lib/share'

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

const canWatch = computed(() => isLiveStatus(status.value) && !!hlsUrl.value)
const roomEnded = computed(() => isRoomEnded(status.value))
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

onBeforeUnmount(() => teardownPlayer())

onMounted(() => {
  restoreSession()
  const fromQuery = readRoomFromQuery(window.location.search)
  if (fromQuery) {
    roomId.value = fromQuery
    void loadRoom()
  }
})

function restoreSession() {
  try {
    const tok = localStorage.getItem(TOKEN_KEY) ?? ''
    const raw = localStorage.getItem(SESSION_KEY)
    if (tok && raw) {
      const parsed = JSON.parse(raw) as AuthSession
      if (parsed?.accessToken) {
        accessToken.value = tok
        session.value = parsed
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
  try {
    localStorage.removeItem(TOKEN_KEY)
    localStorage.removeItem(SESSION_KEY)
  } catch {
    // ignore
  }
  authHint.value = 'Logged out'
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
    if (isRoomEnded(room.status)) {
      // Dedicated ended UI — do not set a raw error string.
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

    // Public chat history + gift catalog (no auth required)
    void refreshMessages()
    void refreshGifts()
    if (authed.value) {
      void refreshBalance()
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
    loginOpen.value = false
    authHint.value = `Hi, ${s.displayName || s.email || 'user'}`
    otpCode.value = ''
    void refreshBalance()
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
  }
}

async function sendGift(gift: GiftItem) {
  giftHint.value = ''
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
  } catch (e) {
    giftHint.value = String(e)
  } finally {
    giftBusy.value = false
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
          <button type="button" class="ghost" @click="logout">Logout</button>
        </template>
        <button v-else type="button" class="ghost" @click="loginOpen = !loginOpen">
          {{ loginOpen ? 'Hide login' : 'Login' }}
        </button>
      </div>
    </header>

    <section v-if="loginOpen && !authed" class="panel login">
      <h2>Login</h2>
      <p class="muted">Email OTP — optional. Watch works without login.</p>
      <div class="row">
        <input v-model="email" type="email" placeholder="you@example.com" autocomplete="email" />
        <button type="button" :disabled="authBusy" @click="sendOtp">Send OTP</button>
      </div>
      <div class="row">
        <input v-model="otpCode" placeholder="OTP code" inputmode="numeric" autocomplete="one-time-code" />
        <button type="button" :disabled="authBusy" @click="verifyOtp">Verify</button>
      </div>
      <p v-if="authHint" class="hint">{{ authHint }}</p>
      <p v-if="authError" class="err">{{ authError }}</p>
    </section>

    <div class="row">
      <input v-model="roomId" placeholder="Room UUID" />
      <button :disabled="loading" @click="loadRoom">Load</button>
      <button type="button" :disabled="!roomId.trim()" @click="shareRoom">Share</button>
    </div>
    <p v-if="shareHint" class="hint">{{ shareHint }}</p>
    <p v-if="authHint && authed" class="hint">{{ authHint }}</p>
    <p v-if="error" class="err">{{ error }}</p>

    <section v-if="roomEnded" class="ended" role="status">
      <p class="ended-title">直播已结束</p>
      <p class="ended-sub">Room ended</p>
      <p v-if="status" class="muted">status: {{ status }}</p>
    </section>

    <p v-else-if="status" class="muted">status: {{ status }}</p>

    <section v-if="canWatch" class="player">
      <p class="mono">{{ hlsUrl }}</p>
      <video ref="videoEl" controls playsinline style="width: 100%; max-width: 720px; background: #000" />
    </section>

    <!-- Chat: list is public; send requires login -->
    <section v-if="roomId.trim() && status" class="panel chat">
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
      <div v-if="authed" class="row">
        <input v-model="chatBody" placeholder="Say something…" @keyup.enter="sendChat" />
        <button type="button" :disabled="chatBusy" @click="sendChat">Send</button>
      </div>
      <p v-else class="muted">
        <button type="button" class="link" @click="loginOpen = true">Login</button>
        to send chat
      </p>
      <p v-if="chatHint" class="hint">{{ chatHint }}</p>
    </section>

    <!-- Gifts + wallet: visible when room loaded; send/topup require login -->
    <section v-if="roomId.trim() && status" class="panel gifts">
      <div class="panel-head">
        <h2>Gifts</h2>
        <span v-if="authed" class="chip">balance: {{ balance }}</span>
      </div>
      <div v-if="authed" class="row topup">
        <input v-model.number="topupAmount" type="number" min="1" placeholder="Topup amount" />
        <button type="button" :disabled="giftBusy" @click="doTopup">Top up</button>
        <button type="button" class="ghost" :disabled="giftBusy" @click="refreshBalance">Refresh</button>
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
</style>

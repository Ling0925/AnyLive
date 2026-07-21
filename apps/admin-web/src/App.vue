<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  adminTitle,
  apiUrl,
  banUserPath,
  canAccessModule,
  forceCloseRoomPath,
  giftsListPath,
  otpSendPath,
  otpVerifyPath,
  roomsPath,
} from './lib/admin'

const apiBase = import.meta.env.VITE_API_BASE ?? 'http://localhost:8088'
const title = adminTitle('local')

/** Access token kept in memory only (ref). */
const accessToken = ref<string | null>(null)
const displayName = ref('')
const role = ref('admin')

const email = ref('')
const otpCode = ref('')
const loginBusy = ref(false)

const roomIdInput = ref('')
const userIdInput = ref('')
const actionReason = ref('')
const actionBusy = ref(false)

const rooms = ref<Array<{ id: string; title: string; status: string }>>([])
const gifts = ref<Array<{ id: string; name: string; price: number; active?: boolean }>>([])
const error = ref('')
const notice = ref('')

const isAuthed = computed(() => Boolean(accessToken.value))

function authHeaders(json = true): HeadersInit {
  const h: Record<string, string> = {}
  if (json) h['Content-Type'] = 'application/json'
  if (accessToken.value) h.Authorization = `Bearer ${accessToken.value}`
  return h
}

async function loadRooms() {
  const res = await fetch(apiUrl(apiBase, roomsPath()))
  if (!res.ok) {
    throw new Error(`rooms ${res.status}`)
  }
  const data = await res.json()
  rooms.value = data.items ?? []
}

async function loadGifts() {
  const path = giftsListPath(isAuthed.value)
  const res = await fetch(apiUrl(apiBase, path), {
    headers: authHeaders(false),
  })
  if (!res.ok) {
    throw new Error(`gifts ${res.status}`)
  }
  const data = await res.json()
  gifts.value = data.items ?? []
}

async function refreshLists() {
  error.value = ''
  try {
    await Promise.all([loadRooms(), loadGifts()])
  } catch (e) {
    error.value = String(e)
  }
}

async function sendOtp() {
  notice.value = ''
  error.value = ''
  loginBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, otpSendPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({ email: email.value.trim() }),
    })
    if (res.status !== 204) {
      throw new Error(`otp send ${res.status}`)
    }
    notice.value = 'OTP sent (dev code often 123456).'
  } catch (e) {
    error.value = String(e)
  } finally {
    loginBusy.value = false
  }
}

async function verifyOtp() {
  notice.value = ''
  error.value = ''
  loginBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, otpVerifyPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        email: email.value.trim(),
        code: otpCode.value.trim(),
      }),
    })
    if (!res.ok) {
      throw new Error(`otp verify ${res.status}`)
    }
    const data = await res.json()
    accessToken.value = data.access_token ?? null
    displayName.value = data.user?.display_name ?? data.user?.email ?? email.value
    notice.value = 'Logged in.'
    await refreshLists()
  } catch (e) {
    error.value = String(e)
  } finally {
    loginBusy.value = false
  }
}

function logout() {
  accessToken.value = null
  displayName.value = ''
  notice.value = 'Logged out (token cleared from memory).'
  void refreshLists()
}

async function forceCloseRoom() {
  if (!accessToken.value) {
    error.value = 'Login required'
    return
  }
  notice.value = ''
  error.value = ''
  actionBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, forceCloseRoomPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        room_id: roomIdInput.value.trim(),
        reason: actionReason.value.trim() || undefined,
      }),
    })
    if (!res.ok) {
      throw new Error(`force-close ${res.status}`)
    }
    notice.value = `Force-closed room ${roomIdInput.value.trim()}`
    roomIdInput.value = ''
    await loadRooms()
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

async function banUser() {
  if (!accessToken.value) {
    error.value = 'Login required'
    return
  }
  notice.value = ''
  error.value = ''
  actionBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, banUserPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        user_id: userIdInput.value.trim(),
        reason: actionReason.value.trim() || undefined,
      }),
    })
    if (res.status !== 204) {
      throw new Error(`ban ${res.status}`)
    }
    notice.value = `Banned user ${userIdInput.value.trim()}`
    userIdInput.value = ''
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

onMounted(() => {
  void refreshLists()
})
</script>

<template>
  <main class="page">
    <header class="header">
      <div>
        <h1>{{ title }}</h1>
        <p class="muted">API {{ apiBase }}</p>
      </div>
      <div class="session">
        <template v-if="isAuthed">
          <span>{{ displayName || 'admin' }}</span>
          <button type="button" class="btn ghost" @click="logout">Log out</button>
        </template>
        <span v-else class="muted">Not signed in</span>
      </div>
    </header>

    <p>
      Role: {{ role }} · rooms module: {{ canAccessModule(role, 'rooms') }} · gifts:
      {{ giftsListPath(isAuthed) }}
    </p>

    <p v-if="notice" class="ok">{{ notice }}</p>
    <p v-if="error" class="err">{{ error }}</p>

    <section class="card" v-if="!isAuthed">
      <h2>Login (OTP)</h2>
      <div class="row">
        <label>
          Email
          <input v-model="email" type="email" autocomplete="username" placeholder="you@example.com" />
        </label>
        <button type="button" class="btn" :disabled="loginBusy || !email.trim()" @click="sendOtp">
          Send OTP
        </button>
      </div>
      <div class="row">
        <label>
          Code
          <input v-model="otpCode" type="text" autocomplete="one-time-code" placeholder="123456" />
        </label>
        <button
          type="button"
          class="btn primary"
          :disabled="loginBusy || !email.trim() || !otpCode.trim()"
          @click="verifyOtp"
        >
          Verify
        </button>
      </div>
    </section>

    <section class="card" v-else>
      <h2>Moderation actions</h2>
      <div class="row">
        <label>
          Reason (optional)
          <input v-model="actionReason" type="text" placeholder="policy" />
        </label>
      </div>
      <div class="row">
        <label>
          Room id
          <input v-model="roomIdInput" type="text" class="mono-input" placeholder="uuid" />
        </label>
        <button
          type="button"
          class="btn danger"
          :disabled="actionBusy || !roomIdInput.trim()"
          @click="forceCloseRoom"
        >
          Force-close room
        </button>
      </div>
      <div class="row">
        <label>
          User id
          <input v-model="userIdInput" type="text" class="mono-input" placeholder="uuid" />
        </label>
        <button
          type="button"
          class="btn danger"
          :disabled="actionBusy || !userIdInput.trim()"
          @click="banUser"
        >
          Ban user
        </button>
      </div>
    </section>

    <section class="card">
      <div class="section-head">
        <h2>Gifts</h2>
        <button type="button" class="btn ghost" @click="refreshLists">Refresh</button>
      </div>
      <p class="muted">
        Source: {{ isAuthed ? 'GET /api/v1/admin/gifts' : 'GET /api/v1/gifts' }}
      </p>
      <table v-if="gifts.length">
        <thead>
          <tr>
            <th>Name</th>
            <th>Price</th>
            <th>Id</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="g in gifts" :key="g.id">
            <td>{{ g.name }}</td>
            <td>{{ g.price }}</td>
            <td class="mono">{{ g.id }}</td>
          </tr>
        </tbody>
      </table>
      <p v-else>No gifts.</p>
    </section>

    <section class="card">
      <h2>Rooms</h2>
      <table v-if="rooms.length">
        <thead>
          <tr>
            <th>Title</th>
            <th>Status</th>
            <th>Id</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="r in rooms" :key="r.id">
            <td>{{ r.title }}</td>
            <td>{{ r.status }}</td>
            <td class="mono">{{ r.id }}</td>
          </tr>
        </tbody>
      </table>
      <p v-else>No rooms (start API and create a room).</p>
    </section>
  </main>
</template>

<style scoped>
.page {
  font-family: system-ui, sans-serif;
  max-width: 960px;
  margin: 2rem auto;
  padding: 0 1rem;
}
.header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
  margin-bottom: 1rem;
}
.session {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}
.card {
  border: 1px solid #e5e5e5;
  border-radius: 8px;
  padding: 1rem 1.25rem;
  margin: 1rem 0;
  background: #fafafa;
}
.section-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: flex-end;
  margin: 0.75rem 0;
}
label {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.85rem;
  color: #444;
  flex: 1;
  min-width: 200px;
}
input {
  font: inherit;
  padding: 0.45rem 0.6rem;
  border: 1px solid #ccc;
  border-radius: 6px;
  background: #fff;
}
.mono-input {
  font-family: ui-monospace, monospace;
  font-size: 0.85rem;
}
.btn {
  font: inherit;
  padding: 0.45rem 0.85rem;
  border-radius: 6px;
  border: 1px solid #ccc;
  background: #fff;
  cursor: pointer;
}
.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.btn.primary {
  background: #1a73e8;
  border-color: #1a73e8;
  color: #fff;
}
.btn.danger {
  background: #b00020;
  border-color: #b00020;
  color: #fff;
}
.btn.ghost {
  background: transparent;
}
table {
  width: 100%;
  border-collapse: collapse;
}
th,
td {
  border-bottom: 1px solid #ddd;
  text-align: left;
  padding: 0.5rem;
}
.mono {
  font-family: ui-monospace, monospace;
  font-size: 0.85rem;
}
.muted {
  color: #666;
}
.err {
  color: #b00020;
}
.ok {
  color: #0a7a2f;
}
h2 {
  margin: 0 0 0.5rem;
  font-size: 1.1rem;
}
</style>

<script setup lang="ts">
import Hls from 'hls.js'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import {
  ADMIN_NAV,
  adminGiftsPath,
  adminTitle,
  apiUrl,
  auditPath,
  banUserPath,
  buildHls,
  countByStatus,
  forceCloseRoomPath,
  giftsListPath,
  grantAdminPath,
  mePath,
  muteUserPath,
  openReportCount,
  otpSendPath,
  otpVerifyPath,
  reportResolvePath,
  reportsListPath,
  roomPlayPath,
  roomStatusTone,
  roomsPath,
  shortId,
  unmuteUserPath,
  type AdminNavKey,
} from './lib/admin'

const apiBase = import.meta.env.VITE_API_BASE ?? 'http://localhost:8088'
const envLabel = import.meta.env.MODE === 'production' ? 'prod' : 'local'
const title = adminTitle(envLabel)

const accessToken = ref<string | null>(null)
const displayName = ref('')
const userId = ref('')
const nav = ref<AdminNavKey>('dashboard')

const email = ref('')
const otpCode = ref('')
const loginBusy = ref(false)

const roomIdInput = ref('')
const userIdInput = ref('')
const muteUserIdInput = ref('')
const unmuteUserIdInput = ref('')
const actionReason = ref('')
const actionBusy = ref(false)

const giftName = ref('')
const giftPrice = ref('')
const giftBusy = ref(false)

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

const error = ref('')
const notice = ref('')
const listBusy = ref(false)

const previewRoomId = ref<string | null>(null)
const previewHlsUrl = ref('')
const previewBusy = ref(false)
const previewError = ref('')
const previewVideoEl = ref<HTMLVideoElement | null>(null)
let previewDetach: (() => void) | null = null

const isAuthed = computed(() => Boolean(accessToken.value))
const liveCount = computed(() => countByStatus(rooms.value, 'live'))
const idleCount = computed(() => countByStatus(rooms.value, 'idle'))
const closedCount = computed(() => countByStatus(rooms.value, 'closed'))
const reportOpen = computed(() => openReportCount(reports.value))
const pageTitle = computed(() => ADMIN_NAV.find((n) => n.key === nav.value)?.label ?? '运营后台')
const avatarLetter = computed(() => (displayName.value || 'A').slice(0, 1).toUpperCase())

function authHeaders(json = true): HeadersInit {
  const h: Record<string, string> = {}
  if (json) h['Content-Type'] = 'application/json'
  if (accessToken.value) h.Authorization = `Bearer ${accessToken.value}`
  return h
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
  previewError.value = '当前浏览器不支持 HLS，请直接打开 m3u8 链接。'
}

watch([previewVideoEl, previewHlsUrl], ([el, url]) => {
  if (el && url) attachPreviewHls(el, url)
  else teardownPreviewPlayer()
})

onBeforeUnmount(() => teardownPreviewPlayer())

function go(key: AdminNavKey) {
  nav.value = key
  notice.value = ''
  error.value = ''
}

async function previewRoom(room: { id: string; status: string }) {
  previewError.value = ''
  if (room.status !== 'live') {
    previewError.value = '房间未在直播'
    previewRoomId.value = room.id
    previewHlsUrl.value = ''
    return
  }
  previewBusy.value = true
  previewRoomId.value = room.id
  previewHlsUrl.value = ''
  try {
    const res = await fetch(apiUrl(apiBase, roomPlayPath(room.id)))
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
  const res = await fetch(apiUrl(apiBase, roomsPath()))
  if (!res.ok) throw new Error(`rooms ${res.status}`)
  const data = await res.json()
  rooms.value = data.items ?? []
}

async function loadGifts() {
  const path = giftsListPath(isAuthed.value)
  const res = await fetch(apiUrl(apiBase, path), { headers: authHeaders(false) })
  if (!res.ok) throw new Error(`gifts ${res.status}`)
  const data = await res.json()
  gifts.value = data.items ?? []
}

async function loadReports() {
  if (!accessToken.value) {
    reports.value = []
    return
  }
  const res = await fetch(apiUrl(apiBase, reportsListPath()), { headers: authHeaders(false) })
  if (!res.ok) throw new Error(`reports ${res.status}`)
  const data = await res.json()
  reports.value = data.items ?? []
}

async function loadAudit() {
  if (!accessToken.value) {
    audit.value = []
    return
  }
  const res = await fetch(apiUrl(apiBase, auditPath()), { headers: authHeaders(false) })
  if (!res.ok) throw new Error(`audit ${res.status}`)
  const data = await res.json()
  audit.value = data.items ?? []
}

async function refreshLists() {
  listBusy.value = true
  error.value = ''
  try {
    const tasks: Promise<void>[] = [loadRooms(), loadGifts()]
    if (isAuthed.value) {
      tasks.push(loadReports(), loadAudit())
    } else {
      reports.value = []
      audit.value = []
    }
    await Promise.all(tasks)
  } catch (e) {
    error.value = String(e)
  } finally {
    listBusy.value = false
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
    if (res.status !== 204) throw new Error(`otp send ${res.status}`)
    notice.value = '验证码已发送（开发环境常用 123456）'
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
    if (!res.ok) throw new Error(`otp verify ${res.status}`)
    const data = await res.json()
    accessToken.value = data.access_token ?? null
    displayName.value = data.user?.display_name ?? data.user?.email ?? email.value
    userId.value = data.user?.id ?? ''
    notice.value = '登录成功'
    nav.value = 'dashboard'
    await refreshLists()
    // Bootstrap first admin if needed (idempotent for already-admin in multi-admin setups may 403 after first).
    if (userId.value) {
      void tryBootstrapAdmin(userId.value)
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loginBusy.value = false
  }
}

async function tryBootstrapAdmin(id: string) {
  try {
    await fetch(apiUrl(apiBase, grantAdminPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({ user_id: id }),
    })
  } catch {
    // ignore — may already be admin or bootstrap closed
  }
}

function logout() {
  accessToken.value = null
  displayName.value = ''
  userId.value = ''
  notice.value = '已退出登录'
  closePreview()
  void refreshLists()
}

async function forceCloseRoom(id?: string) {
  const roomId = (id ?? roomIdInput.value).trim()
  if (!accessToken.value) {
    error.value = '请先登录'
    return
  }
  if (!roomId) {
    error.value = '请填写房间 ID'
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
        room_id: roomId,
        reason: actionReason.value.trim() || undefined,
      }),
    })
    if (!res.ok) throw new Error(`force-close ${res.status}`)
    notice.value = `已强关房间 ${shortId(roomId)}`
    roomIdInput.value = ''
    await Promise.all([loadRooms(), loadAudit()])
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

async function banUser() {
  if (!accessToken.value) {
    error.value = '请先登录'
    return
  }
  const id = userIdInput.value.trim()
  if (!id) {
    error.value = '请填写用户 ID'
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
        user_id: id,
        reason: actionReason.value.trim() || undefined,
      }),
    })
    if (res.status !== 204) throw new Error(`ban ${res.status}`)
    notice.value = `已封禁用户 ${shortId(id)}`
    userIdInput.value = ''
    await loadAudit()
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

async function muteUser() {
  if (!accessToken.value) {
    error.value = '请先登录'
    return
  }
  const id = muteUserIdInput.value.trim()
  if (!id) {
    error.value = '请填写用户 ID'
    return
  }
  notice.value = ''
  error.value = ''
  actionBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, muteUserPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        user_id: id,
        reason: actionReason.value.trim() || undefined,
      }),
    })
    if (res.status !== 204) throw new Error(`mute ${res.status}`)
    notice.value = `已禁言用户 ${shortId(id)}`
    muteUserIdInput.value = ''
    await loadAudit()
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

async function unmuteUser() {
  if (!accessToken.value) {
    error.value = '请先登录'
    return
  }
  const id = unmuteUserIdInput.value.trim()
  if (!id) {
    error.value = '请填写用户 ID'
    return
  }
  notice.value = ''
  error.value = ''
  actionBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, unmuteUserPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        user_id: id,
        reason: actionReason.value.trim() || undefined,
      }),
    })
    if (res.status !== 204) throw new Error(`unmute ${res.status}`)
    notice.value = `已解除禁言 ${shortId(id)}`
    unmuteUserIdInput.value = ''
    await loadAudit()
  } catch (e) {
    error.value = String(e)
  } finally {
    actionBusy.value = false
  }
}

async function createGift() {
  if (!accessToken.value) {
    error.value = '请先登录'
    return
  }
  const name = giftName.value.trim()
  const price = Number(giftPrice.value)
  if (!name || !Number.isFinite(price) || price <= 0) {
    error.value = '请填写礼物名称和正整数价格'
    return
  }
  notice.value = ''
  error.value = ''
  giftBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, adminGiftsPath()), {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({ name, price, active: true }),
    })
    if (res.status !== 201 && !res.ok) throw new Error(`create gift ${res.status}`)
    notice.value = `已创建礼物「${name}」`
    giftName.value = ''
    giftPrice.value = ''
    await loadGifts()
  } catch (e) {
    error.value = String(e)
  } finally {
    giftBusy.value = false
  }
}

async function resolveReport(reportId: string) {
  if (!accessToken.value) {
    error.value = '请先登录'
    return
  }
  notice.value = ''
  error.value = ''
  actionBusy.value = true
  try {
    const res = await fetch(apiUrl(apiBase, reportResolvePath(reportId)), {
      method: 'PATCH',
      headers: authHeaders(),
      body: JSON.stringify({
        status: 'resolved',
        note: actionReason.value.trim() || undefined,
      }),
    })
    if (!res.ok) throw new Error(`resolve report ${res.status}`)
    notice.value = `已处理举报 ${shortId(reportId)}`
    await Promise.all([loadReports(), loadAudit()])
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
  <!-- Login -->
  <div v-if="!isAuthed" class="login-screen">
    <div class="login-card">
      <div class="brand" style="margin-bottom: 1rem; padding: 0">
        <div class="brand-mark">AL</div>
        <div class="brand-text">
          <div class="brand-title">{{ title }}</div>
          <div class="brand-sub">运营控制台</div>
        </div>
      </div>
      <h1>登录</h1>
      <p class="lead">使用邮箱 OTP 登录。开发环境验证码一般为 <code class="mono">123456</code>。</p>

      <p v-if="notice" class="flash ok">{{ notice }}</p>
      <p v-if="error" class="flash err">{{ error }}</p>

      <label class="field">
        <span>邮箱</span>
        <input v-model="email" type="email" autocomplete="username" placeholder="ops@anylive.local" />
      </label>
      <div class="row">
        <button type="button" class="btn" :disabled="loginBusy || !email.trim()" @click="sendOtp">
          发送验证码
        </button>
      </div>
      <label class="field">
        <span>验证码</span>
        <input v-model="otpCode" type="text" autocomplete="one-time-code" placeholder="123456" />
      </label>
      <button
        type="button"
        class="btn primary"
        :disabled="loginBusy || !email.trim() || !otpCode.trim()"
        @click="verifyOtp"
      >
        进入控制台
      </button>
      <p class="dim" style="margin-top: 1rem; font-size: 0.78rem">API {{ apiBase }}</p>
    </div>
  </div>

  <!-- Ops shell -->
  <div v-else class="shell">
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-mark">AL</div>
        <div class="brand-text">
          <div class="brand-title">AnyLive</div>
          <div class="brand-sub">Ops Console</div>
        </div>
      </div>

      <nav class="nav">
        <button
          v-for="item in ADMIN_NAV"
          :key="item.key"
          type="button"
          class="nav-item"
          :class="{ active: nav === item.key }"
          @click="go(item.key)"
        >
          <span>{{ item.label }}</span>
        </button>
      </nav>

      <div class="sidebar-foot">
        <div class="api-pill">{{ apiBase }}</div>
        <button type="button" class="btn ghost" :disabled="listBusy" @click="refreshLists">
          刷新数据
        </button>
        <button type="button" class="btn ghost" @click="logout">退出登录</button>
      </div>
    </aside>

    <div class="main">
      <header class="topbar">
        <div>
          <h1>{{ pageTitle }}</h1>
          <p class="muted">{{ ADMIN_NAV.find((n) => n.key === nav)?.blurb }}</p>
        </div>
        <div class="session">
          <div class="avatar">{{ avatarLetter }}</div>
          <div>
            <div style="font-weight: 600">{{ displayName || 'operator' }}</div>
            <div class="dim mono" style="font-size: 0.72rem">{{ shortId(userId, 10) }}</div>
          </div>
        </div>
      </header>

      <div class="content">
        <p v-if="notice" class="flash ok">{{ notice }}</p>
        <p v-if="error" class="flash err">{{ error }}</p>

        <!-- Dashboard -->
        <template v-if="nav === 'dashboard'">
          <div class="kpis">
            <div class="kpi">
              <div class="kpi-label">直播中</div>
              <div class="kpi-value live">{{ liveCount }}</div>
            </div>
            <div class="kpi">
              <div class="kpi-label">空闲房间</div>
              <div class="kpi-value">{{ idleCount }}</div>
            </div>
            <div class="kpi">
              <div class="kpi-label">待处理举报</div>
              <div class="kpi-value">{{ reportOpen }}</div>
            </div>
            <div class="kpi">
              <div class="kpi-label">礼物种类</div>
              <div class="kpi-value">{{ gifts.length }}</div>
            </div>
          </div>

          <div class="grid-2">
            <section class="panel">
              <div class="panel-head">
                <h2>直播间速览</h2>
                <button type="button" class="btn sm" @click="go('rooms')">全部</button>
              </div>
              <div class="table-wrap" v-if="rooms.length">
                <table class="data">
                  <thead>
                    <tr>
                      <th>标题</th>
                      <th>状态</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="r in rooms.slice(0, 6)" :key="r.id">
                      <td>{{ r.title }}</td>
                      <td><span class="badge" :class="roomStatusTone(r.status)">{{ r.status }}</span></td>
                      <td class="actions">
                        <button type="button" class="btn sm" @click="go('rooms'); previewRoom(r)">
                          预览
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <p v-else class="empty">暂无房间</p>
            </section>

            <section class="panel">
              <div class="panel-head">
                <h2>最新举报</h2>
                <button type="button" class="btn sm" @click="go('reports')">队列</button>
              </div>
              <div class="table-wrap" v-if="reports.length">
                <table class="data">
                  <thead>
                    <tr>
                      <th>目标</th>
                      <th>原因</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="r in reports.slice(0, 6)" :key="r.id">
                      <td class="mono">{{ r.target_type }}:{{ shortId(r.target_id) }}</td>
                      <td>{{ r.reason }}</td>
                      <td>
                        <button type="button" class="btn sm primary" :disabled="actionBusy" @click="resolveReport(r.id)">
                          处理
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <p v-else class="empty">暂无举报</p>
            </section>
          </div>
        </template>

        <!-- Rooms -->
        <section v-else-if="nav === 'rooms'" class="panel">
          <div class="panel-head">
            <h2>直播间管理</h2>
            <div class="toolbar">
              <span class="badge live">live {{ liveCount }}</span>
              <span class="badge idle">idle {{ idleCount }}</span>
              <span class="badge closed">closed {{ closedCount }}</span>
              <button type="button" class="btn sm" :disabled="listBusy" @click="loadRooms">刷新</button>
            </div>
          </div>
          <p class="panel-desc">查看房间状态、HLS 预览，或一键强关违规直播。</p>

          <div class="table-wrap" v-if="rooms.length">
            <table class="data">
              <thead>
                <tr>
                  <th>标题</th>
                  <th>状态</th>
                  <th>Owner</th>
                  <th>Room ID</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="r in rooms" :key="r.id">
                  <td>{{ r.title }}</td>
                  <td><span class="badge" :class="roomStatusTone(r.status)">{{ r.status }}</span></td>
                  <td class="mono">{{ shortId(r.owner_id || '') }}</td>
                  <td class="mono" :title="r.id">{{ shortId(r.id) }}</td>
                  <td class="actions">
                    <button type="button" class="btn sm" :disabled="previewBusy" @click="previewRoom(r)">
                      预览
                    </button>
                    <button type="button" class="btn sm" @click="useRoomId(r.id)">填入强关</button>
                    <button
                      type="button"
                      class="btn sm danger"
                      :disabled="actionBusy || r.status === 'closed'"
                      @click="forceCloseRoom(r.id)"
                    >
                      强关
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <p v-else class="empty">暂无房间。先通过 App / API 创建并开播。</p>

          <div v-if="previewRoomId" class="preview">
            <div class="panel-head">
              <h2>HLS 预览 · {{ shortId(previewRoomId, 12) }}</h2>
              <button type="button" class="btn sm ghost" @click="closePreview">关闭</button>
            </div>
            <p v-if="previewError" class="flash err">{{ previewError }}</p>
            <p v-if="previewHlsUrl" class="preview-url mono">
              <a :href="previewHlsUrl" target="_blank" rel="noopener">{{ previewHlsUrl }}</a>
            </p>
            <video v-if="previewHlsUrl" ref="previewVideoEl" controls playsinline class="preview-video" />
          </div>
        </section>

        <!-- Reports -->
        <section v-else-if="nav === 'reports'" class="panel">
          <div class="panel-head">
            <h2>举报队列</h2>
            <button type="button" class="btn sm" :disabled="listBusy" @click="loadReports">刷新</button>
          </div>
          <p class="panel-desc">处理用户提交的房间 / 用户举报。可选填写处理备注（使用下方通用原因框）。</p>
          <label class="field" style="max-width: 420px">
            <span>处理备注（可选）</span>
            <input v-model="actionReason" type="text" placeholder="已核实违规 / 误报等" />
          </label>
          <div class="table-wrap" v-if="reports.length">
            <table class="data">
              <thead>
                <tr>
                  <th>目标</th>
                  <th>原因</th>
                  <th>状态</th>
                  <th>举报人</th>
                  <th>时间</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="r in reports" :key="r.id">
                  <td class="mono">{{ r.target_type }}:{{ shortId(r.target_id) }}</td>
                  <td>{{ r.reason }}</td>
                  <td>{{ r.status || 'open' }}</td>
                  <td class="mono">{{ shortId(r.reporter_id) }}</td>
                  <td class="mono">{{ r.created_at }}</td>
                  <td class="actions">
                    <button
                      type="button"
                      class="btn sm primary"
                      :disabled="actionBusy || r.status === 'resolved'"
                      @click="resolveReport(r.id)"
                    >
                      标记已处理
                    </button>
                    <button
                      v-if="r.target_type === 'room'"
                      type="button"
                      class="btn sm"
                      @click="useRoomId(r.target_id)"
                    >
                      去强关
                    </button>
                    <button
                      v-if="r.target_type === 'user'"
                      type="button"
                      class="btn sm"
                      @click="useUserId(r.target_id)"
                    >
                      去处置
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <p v-else class="empty">队列为空</p>
        </section>

        <!-- Gifts -->
        <section v-else-if="nav === 'gifts'" class="panel">
          <div class="panel-head">
            <h2>礼物配置</h2>
            <button type="button" class="btn sm" :disabled="listBusy" @click="loadGifts">刷新</button>
          </div>
          <p class="panel-desc">维护礼物目录（名称、价格）。公开接口仅返回 active 礼物。</p>
          <div class="row">
            <label class="field">
              <span>名称</span>
              <input v-model="giftName" type="text" placeholder="Rose" />
            </label>
            <label class="field" style="max-width: 160px">
              <span>价格（币）</span>
              <input v-model="giftPrice" type="number" min="1" step="1" placeholder="10" />
            </label>
            <button
              type="button"
              class="btn primary"
              :disabled="giftBusy || !giftName.trim() || !giftPrice"
              @click="createGift"
            >
              新增礼物
            </button>
          </div>
          <div class="table-wrap" v-if="gifts.length">
            <table class="data">
              <thead>
                <tr>
                  <th>名称</th>
                  <th>价格</th>
                  <th>状态</th>
                  <th>ID</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="g in gifts" :key="g.id">
                  <td>{{ g.name }}</td>
                  <td>{{ g.price }}</td>
                  <td>{{ g.active === false ? 'inactive' : 'active' }}</td>
                  <td class="mono">{{ shortId(g.id, 12) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <p v-else class="empty">暂无礼物</p>
        </section>

        <!-- Moderation -->
        <section v-else-if="nav === 'moderation'" class="panel">
          <div class="panel-head">
            <h2>处置中心</h2>
          </div>
          <p class="panel-desc">封禁用户、禁言/解禁、房间强关。可从房间/举报表一键带入 ID。</p>

          <label class="field" style="max-width: 480px">
            <span>通用原因 / 备注</span>
            <input v-model="actionReason" type="text" placeholder="违反社区规范 / 垃圾广告…" />
          </label>

          <div class="split-actions">
            <div class="action-card">
              <h3>强关房间</h3>
              <label class="field">
                <span>Room ID</span>
                <input v-model="roomIdInput" class="mono" type="text" placeholder="uuid" />
              </label>
              <button
                type="button"
                class="btn danger"
                :disabled="actionBusy || !roomIdInput.trim()"
                @click="forceCloseRoom()"
              >
                强制关闭
              </button>
            </div>

            <div class="action-card">
              <h3>封禁用户</h3>
              <label class="field">
                <span>User ID</span>
                <input v-model="userIdInput" class="mono" type="text" placeholder="uuid" />
              </label>
              <button type="button" class="btn danger" :disabled="actionBusy || !userIdInput.trim()" @click="banUser">
                封禁
              </button>
            </div>

            <div class="action-card">
              <h3>禁言</h3>
              <label class="field">
                <span>User ID</span>
                <input v-model="muteUserIdInput" class="mono" type="text" placeholder="uuid" />
              </label>
              <button type="button" class="btn danger" :disabled="actionBusy || !muteUserIdInput.trim()" @click="muteUser">
                禁言
              </button>
            </div>

            <div class="action-card">
              <h3>解除禁言</h3>
              <label class="field">
                <span>User ID</span>
                <input v-model="unmuteUserIdInput" class="mono" type="text" placeholder="uuid" />
              </label>
              <button
                type="button"
                class="btn primary"
                :disabled="actionBusy || !unmuteUserIdInput.trim()"
                @click="unmuteUser"
              >
                解禁
              </button>
            </div>
          </div>
        </section>

        <!-- Audit -->
        <section v-else-if="nav === 'audit'" class="panel">
          <div class="panel-head">
            <h2>审计日志</h2>
            <button type="button" class="btn sm" :disabled="listBusy" @click="loadAudit">刷新</button>
          </div>
          <p class="panel-desc">运营写操作记录（封禁、禁言、强关等）。</p>
          <div class="table-wrap" v-if="audit.length">
            <table class="data">
              <thead>
                <tr>
                  <th>动作</th>
                  <th>操作人</th>
                  <th>目标</th>
                  <th>详情</th>
                  <th>时间</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="a in audit" :key="a.id">
                  <td>{{ a.action }}</td>
                  <td class="mono">{{ shortId(a.actor_id) }}</td>
                  <td class="mono">{{ shortId(a.target) }}</td>
                  <td>{{ a.detail }}</td>
                  <td class="mono">{{ a.created_at }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <p v-else class="empty">暂无审计记录</p>
        </section>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { attachHls, buildPlayUrl, isLiveStatus } from './lib/hlsAttach'
import { buildShareUrl, isRoomEnded, readRoomFromQuery } from './lib/share'

const apiBase = import.meta.env.VITE_API_BASE ?? 'http://localhost:8088'
const roomId = ref('')
const status = ref('')
const hlsUrl = ref('')
const error = ref('')
const loading = ref(false)
const shareHint = ref('')
const videoEl = ref<HTMLVideoElement | null>(null)
let detach: (() => void) | null = null

const canWatch = computed(() => isLiveStatus(status.value) && !!hlsUrl.value)
const roomEnded = computed(() => isRoomEnded(status.value))

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
  const fromQuery = readRoomFromQuery(window.location.search)
  if (fromQuery) {
    roomId.value = fromQuery
    void loadRoom()
  }
})

async function loadRoom() {
  error.value = ''
  shareHint.value = ''
  hlsUrl.value = ''
  status.value = ''
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
</script>

<template>
  <main class="page">
    <h1>AnyLive Watch</h1>
    <p class="muted">Public H5 player (hls.js / native). API: {{ apiBase }}</p>
    <div class="row">
      <input v-model="roomId" placeholder="Room UUID" />
      <button :disabled="loading" @click="loadRoom">Load</button>
      <button type="button" :disabled="!roomId.trim()" @click="shareRoom">Share</button>
    </div>
    <p v-if="shareHint" class="hint">{{ shareHint }}</p>
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
  </main>
</template>

<style scoped>
.page {
  font-family: system-ui, sans-serif;
  max-width: 800px;
  margin: 2rem auto;
  padding: 0 1rem;
}
.row {
  display: flex;
  gap: 0.5rem;
  margin: 1rem 0;
}
input {
  flex: 1;
  padding: 0.5rem;
}
button {
  padding: 0.5rem 1rem;
}
.muted {
  color: #666;
}
.err {
  color: #b00020;
}
.hint {
  color: #0a7;
  font-size: 0.9rem;
}
.mono {
  font-family: ui-monospace, monospace;
  font-size: 0.85rem;
  word-break: break-all;
}
.ended {
  margin: 2rem 0;
  padding: 2rem 1rem;
  text-align: center;
  background: #f5f5f5;
  border-radius: 8px;
  border: 1px solid #e0e0e0;
}
.ended-title {
  font-size: 1.5rem;
  font-weight: 600;
  margin: 0 0 0.25rem;
}
.ended-sub {
  color: #666;
  margin: 0 0 0.75rem;
}
</style>

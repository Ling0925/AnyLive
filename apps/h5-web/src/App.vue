<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { attachHls, buildPlayUrl, isLiveStatus } from './lib/hlsAttach'

const apiBase = import.meta.env.VITE_API_BASE ?? 'http://localhost:8088'
const roomId = ref('')
const status = ref('idle')
const hlsUrl = ref('')
const error = ref('')
const loading = ref(false)
const videoEl = ref<HTMLVideoElement | null>(null)
let detach: (() => void) | null = null

const canWatch = computed(() => isLiveStatus(status.value) && !!hlsUrl.value)

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

async function loadRoom() {
  error.value = ''
  hlsUrl.value = ''
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
    if (!isLiveStatus(room.status)) {
      error.value = 'Room is not live'
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
</script>

<template>
  <main class="page">
    <h1>AnyLive Watch</h1>
    <p class="muted">Public H5 player (hls.js / native). API: {{ apiBase }}</p>
    <div class="row">
      <input v-model="roomId" placeholder="Room UUID" />
      <button :disabled="loading" @click="loadRoom">Load</button>
    </div>
    <p v-if="error" class="err">{{ error }}</p>
    <p v-if="status">status: {{ status }}</p>
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
.mono {
  font-family: ui-monospace, monospace;
  font-size: 0.85rem;
  word-break: break-all;
}
</style>

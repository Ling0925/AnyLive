<script setup lang="ts">
import { computed, ref } from 'vue'
import { buildPlayUrl, isLiveStatus } from './lib/player'

const apiBase = import.meta.env.VITE_API_BASE ?? 'http://localhost:8088'
const roomId = ref('')
const status = ref('idle')
const hlsUrl = ref('')
const error = ref('')
const loading = ref(false)

const canWatch = computed(() => isLiveStatus(status.value) && !!hlsUrl.value)

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
    <p class="muted">Public H5 player shell (P1). API: {{ apiBase }}</p>
    <div class="row">
      <input v-model="roomId" placeholder="Room UUID" />
      <button :disabled="loading" @click="loadRoom">Load</button>
    </div>
    <p v-if="error" class="err">{{ error }}</p>
    <p v-if="status">status: {{ status }}</p>
    <section v-if="canWatch" class="player">
      <p>HLS: {{ hlsUrl }}</p>
      <video controls playsinline :src="hlsUrl" style="width: 100%; max-width: 720px; background: #000" />
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
</style>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { adminTitle, canAccessModule } from './lib/admin'

const apiBase = import.meta.env.VITE_API_BASE ?? 'http://localhost:8088'
const role = ref('admin')
const rooms = ref<Array<{ id: string; title: string; status: string }>>([])
const error = ref('')
const title = adminTitle('local')

onMounted(async () => {
  try {
    const res = await fetch(`${apiBase}/api/v1/rooms`)
    if (!res.ok) {
      error.value = `rooms ${res.status}`
      return
    }
    const data = await res.json()
    rooms.value = data.items ?? []
  } catch (e) {
    error.value = String(e)
  }
})
</script>

<template>
  <main class="page">
    <h1>{{ title }}</h1>
    <p>Role: {{ role }} · rooms module: {{ canAccessModule(role, 'rooms') }}</p>
    <p class="muted">API {{ apiBase }}</p>
    <p v-if="error" class="err">{{ error }}</p>
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
  </main>
</template>

<style scoped>
.page {
  font-family: system-ui, sans-serif;
  max-width: 960px;
  margin: 2rem auto;
  padding: 0 1rem;
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
</style>

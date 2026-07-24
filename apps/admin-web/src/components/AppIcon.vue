<script setup lang="ts">
import { computed, type Component } from 'vue'
import {
  APP_ICON_SIZE,
  APP_ICON_STROKE_WIDTH,
  NAV_ICON_COMPONENTS,
  type AppLucideIcon,
} from './icons'
import type { AdminNavKey } from '../lib/admin'

const props = withDefaults(
  defineProps<{
    /** Admin nav key (dashboard, rooms, ...) resolved via NAV_ICON_COMPONENTS. */
    name?: AdminNavKey | string
    /** Explicit Lucide (or compatible) component; wins over `name` when both set. */
    component?: AppLucideIcon | Component
    size?: number | string
    strokeWidth?: number | string
  }>(),
  {
    size: APP_ICON_SIZE,
    strokeWidth: APP_ICON_STROKE_WIDTH,
  },
)

const resolved = computed<AppLucideIcon | Component | null>(() => {
  if (props.component) return props.component
  if (props.name && props.name in NAV_ICON_COMPONENTS) {
    return NAV_ICON_COMPONENTS[props.name as AdminNavKey]
  }
  return null
})

const iconProps = computed(() => ({
  size: props.size,
  strokeWidth: props.strokeWidth,
  'aria-hidden': true as const,
  focusable: 'false' as const,
}))
</script>

<template>
  <component
    :is="resolved"
    v-if="resolved"
    class="app-icon"
    v-bind="iconProps"
  />
</template>

<style scoped>
.app-icon {
  display: inline-block;
  flex-shrink: 0;
  vertical-align: middle;
  color: currentColor;
}
</style>

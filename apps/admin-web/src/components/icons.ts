import type { LucideIcon, LucideProps } from 'lucide-vue-next'
import {
  LayoutDashboard,
  Radio,
  DoorOpen,
  Flag,
  Gift,
  Users,
  Shield,
  ShieldAlert,
  ScrollText,
  Sun,
  Moon,
  LogOut,
  RefreshCw,
  Search,
  ChevronLeft,
  ChevronRight,
  Plus,
  X,
  Play,
  Square,
  Copy,
  Eye,
  Ban,
  VolumeX,
  Volume2,
  KeyRound,
  UserPlus,
  Filter,
  MoreHorizontal,
  Loader2,
  Inbox,
  FolderOpen,
  AlertTriangle,
  Check,
  ExternalLink,
  Settings2,
} from 'lucide-vue-next'

import type { AdminNavKey } from '../lib/admin'

/** Lucide functional icon component used by the admin shell. */
export type AppLucideIcon = LucideIcon

export {
  LayoutDashboard,
  Radio,
  DoorOpen,
  Flag,
  Gift,
  Users,
  Shield,
  ShieldAlert,
  ScrollText,
  Sun,
  Moon,
  LogOut,
  RefreshCw,
  Search,
  ChevronLeft,
  ChevronRight,
  Plus,
  X,
  Play,
  Square,
  Copy,
  Eye,
  Ban,
  VolumeX,
  Volume2,
  KeyRound,
  UserPlus,
  Filter,
  MoreHorizontal,
  Loader2,
  Inbox,
  FolderOpen,
  AlertTriangle,
  Check,
  ExternalLink,
  Settings2,
}

/** Default stroke width for admin UI icons (slightly thinner than Lucide 2). */
export const APP_ICON_STROKE_WIDTH = 1.75

/** Default pixel size for inline admin UI icons. */
export const APP_ICON_SIZE = 16

/**
 * Resolve a Lucide icon component with shared admin defaults.
 * Call sites can still override size / strokeWidth / class via props.
 */
export function withAppIconDefaults(
  icon: AppLucideIcon,
  overrides: Partial<LucideProps> = {},
): { icon: AppLucideIcon; props: LucideProps } {
  return {
    icon,
    props: {
      size: APP_ICON_SIZE,
      strokeWidth: APP_ICON_STROKE_WIDTH,
      ...overrides,
    },
  }
}

/**
 * Sidebar nav key -> Lucide component.
 * Keys match AdminNavKey / ADMIN_NAV in lib/admin.ts.
 */
export const NAV_ICON_COMPONENTS: Record<AdminNavKey, AppLucideIcon> = {
  dashboard: LayoutDashboard,
  golive: Radio,
  rooms: DoorOpen,
  reports: Flag,
  gifts: Gift,
  users: Users,
  moderation: ShieldAlert,
  audit: ScrollText,
}

export function navIcon(key: AdminNavKey): AppLucideIcon {
  return NAV_ICON_COMPONENTS[key]
}

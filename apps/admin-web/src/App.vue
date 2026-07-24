<script setup lang="ts">
import { useAdminApp } from './composables/useAdminApp'
import AppIcon from './components/AppIcon.vue'
import {
  Inbox,
  FolderOpen,
  Sun,
  Moon,
  Radio,
  Shield,
  ScrollText,
  RefreshCw,
  LogOut,
} from './components/icons'

const {
  ADMIN_NAV,
  actionBusy,
  actionReason,
  adminGateHint,
  analyticsBusy,
  analyticsByName,
  analyticsHint,
  analyticsRecent,
  apiBase,
  auditQuery,
  avatarLetter,
  banUser,
  banUserId,
  bannedUsers,
  beginEditGift,
  cancelEditGift,
  closePreview,
  closedCount,
  copyGiftSeedCmd,
  copyText,
  createBusy,
  createDisplayName,
  createEmail,
  createGift,
  createPassword,
  createUser,
  createUsername,
  displayName,
  email,
  error,
  expireBusy,
  expireHint,
  fillBanFromLookup,
  fillMuteFromLookup,
  filteredAudit,
  forceCloseRoom,
  formatTs,
  giftBusy,
  giftEditActive,
  giftEditId,
  giftName,
  giftPrice,
  giftSeedCopyHint,
  gifts,
  go,
  goLiveBusy,
  goLiveCopyHint,
  goLiveHls,
  goLivePublish,
  goLiveRefreshPublish,
  goLiveRoomId,
  goLiveRoomStatus,
  goLiveStart,
  goLiveStop,
  goLiveTitle,
  idleCount,
  isAdmin,
  isAuthed,
  listBusy,
  liveCount,
  loadAudit,
  loadGifts,
  loadModerationLists,
  loadPublishForRoom,
  loadReports,
  loadRooms,
  loadUsers,
  locale,
  loginBusy,
  loginStep,
  logout,
  lookupBusy,
  lookupStatus,
  lookupUserId,
  lookupUserModeration,
  metricsBusy,
  metricsHint,
  metricsText,
  muteUser,
  muteUserIdInput,
  mutedUsers,
  nav,
  notice,
  onToggleTheme,
  otpCode,
  otpSent,
  pageBlurb,
  pageTitle,
  pagedRooms,
  password,
  passwordLogin,
  prepHints,
  previewBusy,
  previewError,
  previewHlsUrl,
  previewRoom,
  previewRoomId,
  previewVideoEl,
  reconcileBalanced,
  reconcileBusy,
  reconcileHint,
  refreshLists,
  reportOpen,
  reports,
  resendCooldown,
  resetUserPassword,
  resolveReport,
  revokeUserSessions,
  roomIdInput,
  roomListMeta,
  roomPageSize,
  roomQuery,
  roomStatusFilter,
  roomStatusTone,
  rooms,
  runAnalyticsSummary,
  runExpirePayOrders,
  runMetricsScrape,
  runWalletReconcile,
  saveGiftEdit,
  sendOtp,
  sessionRestoring,
  sessionRoleLabel,
  setLocale,
  setRoomPage,
  setTheme,
  setUserStatus,
  shortId,
  statusLabel,
  t,
  tempPasswordNotice,
  theme,
  themeToggleLabel,
  title,
  toggleGiftActive,
  unbanUser,
  unbanUserId,
  unmuteUser,
  unmuteUserIdInput,
  useOtpLogin,
  useRoomId,
  useUserId,
  userId,
  userIdInput,
  usersBusy,
  usersList,
  usersListMeta,
  usersQuery,
  usersStatusFilter,
  usersPageSize,
  usersTotal,
  searchUsers,
  setUsersPage,
  verifyOtp,
} = useAdminApp()

// Template ref target (assigned by Vue; kept referenced for noUnusedLocals).
void previewVideoEl
</script>

<template>
  <!-- Session restore splash -->
  <div v-if="sessionRestoring" class="login-screen" data-testid="session-restoring">
    <div class="login-bg" aria-hidden="true">
      <div class="login-orb o1" />
      <div class="login-orb o2" />
      <div class="login-grid" />
    </div>
    <div class="login-shell is-restoring">
      <div class="login-restoring login-card">
        <div class="login-spinner" />
        <p class="lead">{{ t('login.restoring') }}</p>
      </div>
    </div>
  </div>

  <!-- Login -->
  <div v-else-if="!isAuthed" class="login-screen" data-testid="login-screen">
    <div class="login-bg" aria-hidden="true">
      <div class="login-orb o1" />
      <div class="login-orb o2" />
      <div class="login-orb o3" />
      <div class="login-grid" />
    </div>

    <div class="login-shell" data-testid="login-card">
      <aside class="login-hero">
        <div class="login-hero-top">
          <div class="brand">
            <div class="brand-mark">AL</div>
            <div class="brand-text">
              <div class="brand-title">{{ title }}</div>
              <div class="brand-sub">{{ t('app.console') }}</div>
            </div>
          </div>
          <span class="login-hero-tag">{{ t('login.heroTag') }}</span>
          <h2>{{ t('login.heroTitle') }}</h2>
          <p>{{ t('login.heroBody') }}</p>
        </div>
        <div class="login-features">
          <div class="login-feature">
            <span class="login-feature-icon" aria-hidden="true">
              <AppIcon :component="Radio" :size="16" />
            </span>
            {{ t('login.features.rooms') }}
          </div>
          <div class="login-feature">
            <span class="login-feature-icon" aria-hidden="true">
              <AppIcon :component="Shield" :size="16" />
            </span>
            {{ t('login.features.moderation') }}
          </div>
          <div class="login-feature">
            <span class="login-feature-icon" aria-hidden="true">
              <AppIcon :component="ScrollText" :size="16" />
            </span>
            {{ t('login.features.audit') }}
          </div>
        </div>
      </aside>

      <div class="login-card">
        <div class="login-card-head">
          <div>
            <h1>{{ t('login.headline') }}</h1>
            <p class="lead">
              {{ t('login.lead') }}
              <code class="mono">123456</code>
            </p>
          </div>
          <div class="login-toolbar">
            <div class="theme-switch" data-testid="login-theme-switch" role="group" :aria-label="t('theme.label')">
              <button
                type="button"
                :class="{ active: theme === 'light' }"
                data-testid="theme-light"
                :aria-pressed="theme === 'light'"
                @click="setTheme('light')"
              >
                {{ t('theme.light') }}
              </button>
              <button
                type="button"
                :class="{ active: theme === 'dark' }"
                data-testid="theme-dark"
                :aria-pressed="theme === 'dark'"
                @click="setTheme('dark')"
              >
                {{ t('theme.dark') }}
              </button>
            </div>
            <div class="lang-switch" data-testid="login-lang-switch">
              <button
                type="button"
                :class="{ active: locale === 'zh' }"
                data-testid="lang-zh"
                @click="setLocale('zh')"
              >
                {{ t('lang.zh') }}
              </button>
              <button
                type="button"
                :class="{ active: locale === 'en' }"
                data-testid="lang-en"
                @click="setLocale('en')"
              >
                {{ t('lang.en') }}
              </button>
            </div>
          </div>
        </div>

        <div class="login-mode-tabs" role="tablist" data-testid="login-mode-tabs">
          <button
            type="button"
            role="tab"
            :class="{ active: !useOtpLogin }"
            :aria-selected="!useOtpLogin"
            data-testid="login-tab-password"
            @click="useOtpLogin = false"
          >
            {{ t('login.modePassword') }}
          </button>
          <button
            type="button"
            role="tab"
            :class="{ active: useOtpLogin }"
            :aria-selected="useOtpLogin"
            data-testid="login-tab-otp"
            @click="useOtpLogin = true"
          >
            {{ t('login.modeOtp') }}
          </button>
        </div>

        <div class="login-steps" aria-hidden="true">
          <div class="login-step" :class="{ active: loginStep === 1, done: loginStep > 1 }" />
          <div class="login-step" :class="{ active: loginStep === 2, done: false }" />
        </div>

        <p v-if="notice" class="flash ok" data-testid="login-notice">{{ notice }}</p>
        <p v-if="error" class="flash err" data-testid="login-error">{{ error }}</p>

        <label class="field">
          <span>{{ t('login.email') }}</span>
          <input
            v-model="email"
            type="text"
            autocomplete="username"
            :placeholder="t('login.emailPlaceholder')"
            data-testid="login-email"
          />
        </label>

        <template v-if="!useOtpLogin">
          <label class="field">
            <span>{{ t('login.password') }}</span>
            <input
              v-model="password"
              type="password"
              autocomplete="current-password"
              :placeholder="t('login.passwordPlaceholder')"
              data-testid="login-password"
              @keyup.enter="passwordLogin"
            />
          </label>
          <button
            type="button"
            class="btn primary"
            data-testid="login-submit"
            :disabled="loginBusy || !email.trim() || !password"
            @click="passwordLogin"
          >
            {{ loginBusy ? t('login.submitting') : t('login.submit') }}
          </button>
        </template>

        <template v-else>
          <div class="row">
            <button
              type="button"
              class="btn"
              data-testid="login-send-otp"
              :disabled="loginBusy || !email.trim() || resendCooldown > 0"
              @click="sendOtp"
            >
              <template v-if="loginBusy && !otpSent">{{ t('login.sending') }}</template>
              <template v-else-if="resendCooldown > 0">
                {{ t('login.resendIn', { n: resendCooldown }) }}
              </template>
              <template v-else-if="otpSent">{{ t('login.resend') }}</template>
              <template v-else>{{ t('login.sendOtp') }}</template>
            </button>
          </div>
          <label class="field">
            <span>{{ t('login.otp') }}</span>
            <input
              v-model="otpCode"
              type="text"
              inputmode="numeric"
              autocomplete="one-time-code"
              :placeholder="t('login.otpPlaceholder')"
              data-testid="login-otp"
              @keyup.enter="verifyOtp"
            />
            <span class="otp-dev-tip">{{ t('login.tipDev') }} · 123456</span>
          </label>
          <button
            type="button"
            class="btn primary"
            data-testid="login-submit-otp"
            :disabled="loginBusy || !email.trim() || !otpCode.trim()"
            @click="verifyOtp"
          >
            {{ loginBusy ? t('login.submitting') : t('login.submit') }}
          </button>
        </template>

        <p class="login-secure">{{ t('login.secureNote') }} · {{ t('login.api') }} {{ apiBase }}</p>
      </div>
    </div>
  </div>

  <!-- Ops shell -->
  <div v-else class="shell" data-testid="ops-shell">
    <aside class="sidebar" data-testid="sidebar">
      <div class="brand">
        <div class="brand-mark">AL</div>
        <div class="brand-text">
          <div class="brand-title">{{ t('app.name') }}</div>
          <div class="brand-sub">{{ t('app.opsConsole') }}</div>
        </div>
      </div>

      <nav class="nav" data-testid="sidebar-nav">
        <button
          v-for="item in ADMIN_NAV"
          :key="item.key"
          type="button"
          class="nav-item"
          :class="{ active: nav === item.key }"
          :data-testid="`nav-${item.key}`"
          @click="go(item.key)"
        >
          <span class="nav-icon" aria-hidden="true">
            <AppIcon :name="item.key" :size="15" />
          </span>
          <span>{{ t(item.labelKey) }}</span>
        </button>
      </nav>

      <div class="sidebar-foot">
        <div class="theme-switch" data-testid="sidebar-theme-switch" role="group" :aria-label="t('theme.label')">
          <button
            type="button"
            :class="{ active: theme === 'light' }"
            :aria-pressed="theme === 'light'"
            @click="setTheme('light')"
          >
            {{ t('theme.light') }}
          </button>
          <button
            type="button"
            :class="{ active: theme === 'dark' }"
            :aria-pressed="theme === 'dark'"
            @click="setTheme('dark')"
          >
            {{ t('theme.dark') }}
          </button>
        </div>
        <div class="lang-switch" data-testid="sidebar-lang-switch">
          <button
            type="button"
            :class="{ active: locale === 'zh' }"
            @click="setLocale('zh')"
          >
            {{ t('lang.zh') }}
          </button>
          <button
            type="button"
            :class="{ active: locale === 'en' }"
            @click="setLocale('en')"
          >
            {{ t('lang.en') }}
          </button>
        </div>
        <div class="api-pill">{{ apiBase }}</div>
        <button
          type="button"
          class="btn ghost"
          data-testid="refresh-lists"
          :disabled="listBusy"
          @click="refreshLists"
        >
          <AppIcon :component="RefreshCw" :size="14" />
          {{ t('topbar.refresh') }}
        </button>
        <button type="button" class="btn ghost" data-testid="logout" @click="logout">
          <AppIcon :component="LogOut" :size="14" />
          {{ t('topbar.logout') }}
        </button>
      </div>
    </aside>

    <div class="main">
      <header class="topbar">
        <div class="topbar-title">
          <h1>{{ pageTitle }}</h1>
          <p class="muted">{{ pageBlurb }}</p>
        </div>
        <div class="session-tools">
          <button
            type="button"
            class="theme-icon-btn"
            data-testid="topbar-theme-toggle"
            :title="themeToggleLabel"
            :aria-label="themeToggleLabel"
            @click="onToggleTheme"
          >
            <!-- sun when dark (click → light); moon when light (click → dark) -->
            <AppIcon v-if="theme === 'dark'" :component="Sun" :size="16" />
            <AppIcon v-else :component="Moon" :size="16" />
          </button>
          <div class="session">
            <div class="avatar">{{ avatarLetter }}</div>
            <div class="session-meta">
              <div class="session-name">{{ displayName || 'operator' }}</div>
              <div class="session-sub">
                <span>{{ shortId(userId, 10) }}</span>
                <span
                  class="role-pill"
                  :class="{ admin: isAdmin === true }"
                  data-testid="session-role"
                >
                  {{ sessionRoleLabel }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </header>

      <div class="content" data-testid="content">
        <p v-if="isAdmin === false" class="flash err" data-testid="admin-gate">{{ adminGateHint }}</p>
        <p v-if="notice" class="flash ok" data-testid="notice">{{ notice }}</p>
        <p v-if="error && isAdmin !== false" class="flash err" data-testid="error">{{ error }}</p>

        <!-- Dashboard -->
        <template v-if="nav === 'dashboard'">
          <div class="kpis" data-testid="dashboard-kpis">
            <div class="kpi" data-testid="kpi-live">
              <div class="kpi-label">{{ t('dashboard.live') }}</div>
              <div class="kpi-value live">{{ liveCount }}</div>
            </div>
            <div class="kpi" data-testid="kpi-idle">
              <div class="kpi-label">{{ t('dashboard.idle') }}</div>
              <div class="kpi-value">{{ idleCount }}</div>
            </div>
            <div class="kpi" data-testid="kpi-reports">
              <div class="kpi-label">{{ t('dashboard.reports') }}</div>
              <div class="kpi-value">{{ reportOpen }}</div>
            </div>
            <div class="kpi" data-testid="kpi-gifts">
              <div class="kpi-label">{{ t('dashboard.gifts') }}</div>
              <div class="kpi-value">{{ gifts.length }}</div>
            </div>
          </div>

          <section
            v-if="isAdmin === true"
            class="panel"
            data-testid="demo-prep"
          >
            <div class="panel-head">
              <h2>{{ t('dashboard.demoPrep') }}</h2>
            </div>
            <p class="panel-desc">{{ t('dashboard.demoPrepDesc') }}</p>
            <ul class="muted list-bullets">
              <li v-for="(line, i) in prepHints.lines" :key="i">{{ line }}</li>
            </ul>
            <div class="row">
              <code class="mono" data-testid="demo-prep-gift-seed">{{ prepHints.giftSeedCmd }}</code>
              <button
                type="button"
                class="btn sm"
                data-testid="demo-prep-copy-gift-seed"
                @click="copyGiftSeedCmd"
              >
                {{ t('dashboard.copySeed') }}
              </button>
              <span class="dim mono" data-testid="demo-prep-runbook">
                {{ prepHints.runbookPath }}
              </span>
            </div>
            <p v-if="giftSeedCopyHint" class="hint" data-testid="demo-prep-copy-hint">
              {{ giftSeedCopyHint }}
            </p>
          </section>

          <section
            v-if="isAuthed"
            class="panel"
            data-testid="panel-wallet-ops"
          >
            <div class="panel-head">
              <h2>{{ t('dashboard.walletOps') }}</h2>
            </div>
            <div class="row">
              <button
                type="button"
                class="btn primary"
                data-testid="wallet-reconcile"
                :disabled="reconcileBusy"
                @click="runWalletReconcile"
              >
                {{ reconcileBusy ? t('dashboard.walletReconciling') : t('dashboard.walletReconcile') }}
              </button>
              <button
                type="button"
                class="btn"
                data-testid="pay-expire-orders"
                :disabled="expireBusy"
                @click="runExpirePayOrders"
              >
                {{ expireBusy ? t('dashboard.expiring') : t('dashboard.expireOrders') }}
              </button>
              <button
                type="button"
                class="btn"
                data-testid="metrics-scrape"
                :disabled="metricsBusy"
                @click="runMetricsScrape"
              >
                {{ metricsBusy ? t('dashboard.scraping') : t('dashboard.scrapeMetrics') }}
              </button>
            </div>
            <p
              v-if="reconcileHint"
              class="hint"
              data-testid="wallet-reconcile-hint"
              :class="{ err: reconcileBalanced === false }"
            >
              {{ reconcileHint }}
            </p>
            <p v-if="expireHint" class="hint" data-testid="pay-expire-hint">{{ expireHint }}</p>
            <p v-if="metricsHint" class="hint" data-testid="metrics-hint">{{ metricsHint }}</p>
            <pre
              v-if="metricsText"
              class="mono"
            >{{ metricsText.slice(0, 4000) }}</pre>
          </section>

          <section v-if="isAuthed" class="panel">
            <div class="panel-head">
              <h2>{{ t('dashboard.analytics') }}</h2>
              <button
                type="button"
                class="btn sm"
                :disabled="analyticsBusy"
                @click="runAnalyticsSummary"
              >
                {{ analyticsBusy ? t('common.loading') : t('dashboard.analyticsRefresh') }}
              </button>
            </div>
            <p class="panel-desc">{{ t('dashboard.analyticsDesc') }}</p>
            <p v-if="analyticsHint" class="hint">{{ analyticsHint }}</p>
            <div v-if="analyticsByName.length" class="table-wrap">
              <table class="data">
                <thead>
                  <tr>
                    <th>{{ t('dashboard.eventName') }}</th>
                    <th>{{ t('dashboard.eventCount') }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="row in analyticsByName.slice(0, 12)" :key="row.name">
                    <td>{{ row.name }}</td>
                    <td>{{ row.count }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <ul v-if="analyticsRecent.length" class="muted">
              <li v-for="ev in analyticsRecent.slice(0, 5)" :key="ev.id">
                {{ ev.name }} · {{ ev.user_id.slice(0, 8) }} · {{ formatTs(ev.occurred_at) }}
              </li>
            </ul>
          </section>

          <div class="grid-2">
            <section class="panel">
              <div class="panel-head">
                <h2>{{ t('dashboard.roomsPreview') }}</h2>
                <button type="button" class="btn sm" @click="go('rooms')">{{ t('common.all') }}</button>
              </div>
              <div class="table-wrap is-compact" v-if="rooms.length">
                <table class="data data-compact">
                  <thead>
                    <tr>
                      <th>{{ t('rooms.colTitle') }}</th>
                      <th>{{ t('rooms.colStatus') }}</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="r in rooms.slice(0, 6)" :key="r.id">
                      <td>{{ r.title }}</td>
                      <td>
                        <span class="badge" :class="roomStatusTone(r.status)">{{
                          statusLabel(r.status)
                        }}</span>
                      </td>
                      <td class="actions">
                        <button type="button" class="btn sm" @click="go('rooms'); previewRoom(r)">
                          {{ t('dashboard.preview') }}
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-else class="empty">
                <div class="empty-icon">
                  <AppIcon :component="Inbox" :size="28" />
                </div>
                {{ t('dashboard.noRooms') }}
              </div>
            </section>

            <section class="panel">
              <div class="panel-head">
                <h2>{{ t('dashboard.reportsPreview') }}</h2>
                <button type="button" class="btn sm" @click="go('reports')">{{ t('dashboard.queue') }}</button>
              </div>
              <div class="table-wrap is-compact" v-if="reports.length">
                <table class="data data-compact">
                  <thead>
                    <tr>
                      <th>{{ t('reports.colTarget') }}</th>
                      <th>{{ t('reports.colReason') }}</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="r in reports.slice(0, 6)" :key="r.id">
                      <td class="mono">{{ r.target_type }}:{{ shortId(r.target_id) }}</td>
                      <td>{{ r.reason }}</td>
                      <td>
                        <button
                          type="button"
                          class="btn sm primary"
                          :disabled="actionBusy"
                          @click="resolveReport(r.id)"
                        >
                          {{ t('dashboard.resolve') }}
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-else class="empty">
                <div class="empty-icon">
                  <AppIcon :component="FolderOpen" :size="28" />
                </div>
                {{ t('dashboard.noReports') }}
              </div>
            </section>
          </div>
        </template>

        <!-- Go live -->
        <section v-else-if="nav === 'golive'" class="panel" data-testid="panel-golive">
          <div class="panel-head">
            <h2>{{ t('golive.title') }}</h2>
            <span v-if="goLiveRoomStatus" class="badge" :class="roomStatusTone(goLiveRoomStatus)">
              {{ statusLabel(goLiveRoomStatus) }}
            </span>
          </div>
          <p class="panel-desc">{{ t('golive.desc') }}</p>

          <div class="row">
            <label class="field">
              <span>{{ t('golive.liveTitle') }}</span>
              <input
                v-model="goLiveTitle"
                type="text"
                :placeholder="t('golive.liveTitlePlaceholder')"
                maxlength="80"
                data-testid="golive-title"
              />
            </label>
            <button
              type="button"
              class="btn primary"
              data-testid="golive-start"
              :disabled="goLiveBusy || !isAuthed"
              @click="goLiveStart"
            >
              {{ goLiveBusy ? t('common.processing') : t('golive.start') }}
            </button>
            <button
              type="button"
              class="btn"
              data-testid="golive-refresh-publish"
              :disabled="goLiveBusy || !goLiveRoomId || !isAuthed"
              @click="goLiveRefreshPublish"
            >
              {{ t('golive.refreshPublish') }}
            </button>
            <button
              type="button"
              class="btn danger"
              data-testid="golive-stop"
              :disabled="goLiveBusy || !goLiveRoomId || goLiveRoomStatus === 'closed' || !isAuthed"
              @click="goLiveStop"
            >
              {{ t('golive.stop') }}
            </button>
          </div>
          <p v-if="!isAuthed" class="flash err">{{ t('golive.needAuth') }}</p>
          <p v-if="goLiveCopyHint" class="flash ok" data-testid="golive-copy-hint">{{ goLiveCopyHint }}</p>

          <div
            v-if="goLiveRoomId"
            class="action-card"
            data-testid="golive-room-info"
          >
            <h3>{{ t('golive.roomInfo') }}</h3>
            <p class="mono" data-testid="golive-room-id">{{ t('golive.roomId') }}：{{ goLiveRoomId }}</p>
            <p class="muted">{{ t('common.status') }}：{{ statusLabel(goLiveRoomStatus) || '—' }}</p>
            <div class="actions">
              <button
                type="button"
                class="btn sm"
                data-testid="golive-copy-room-id"
                @click="copyText(t('golive.roomId'), goLiveRoomId)"
              >
                {{ t('golive.copyRoomId') }}
              </button>
              <button
                type="button"
                class="btn sm"
                data-testid="golive-preview"
                @click="previewRoom({ id: goLiveRoomId, status: goLiveRoomStatus || 'live' })"
              >
                {{ t('golive.hlsPreview') }}
              </button>
            </div>
          </div>

          <div
            v-if="goLivePublish"
            class="split-actions"
            data-testid="golive-obs"
          >
            <div class="action-card">
              <h3>{{ t('golive.obsServer') }}</h3>
              <p class="mono" data-testid="golive-obs-server">
                {{ goLivePublish.server }}
              </p>
              <button
                type="button"
                class="btn sm primary"
                data-testid="golive-copy-server"
                @click="copyText(t('golive.obsServer'), goLivePublish.server)"
              >
                {{ t('golive.copyServer') }}
              </button>
            </div>
            <div class="action-card">
              <h3>{{ t('golive.streamKey') }}</h3>
              <p class="mono" data-testid="golive-obs-stream-key">
                {{ goLivePublish.streamKey }}
              </p>
              <button
                type="button"
                class="btn sm primary"
                data-testid="golive-copy-stream-key"
                @click="copyText(t('golive.streamKey'), goLivePublish.streamKey)"
              >
                {{ t('golive.copyStreamKey') }}
              </button>
              <p class="card-note">
                {{ t('golive.streamKeyHint') }}
              </p>
            </div>
            <div class="action-card">
              <h3>{{ t('golive.pushUrl') }}</h3>
              <p class="mono" data-testid="golive-push-url">
                {{ goLivePublish.pushUrl }}
              </p>
              <button
                type="button"
                class="btn sm"
                data-testid="golive-copy-push-url"
                @click="copyText('push URL', goLivePublish.pushUrl)"
              >
                {{ t('golive.copyPushUrl') }}
              </button>
              <p v-if="goLivePublish.expiresAt" class="card-note">
                {{ t('golive.expires') }}：{{ formatTs(goLivePublish.expiresAt) }}
              </p>
            </div>
            <div class="action-card">
              <h3>{{ t('golive.audienceHls') }}</h3>
              <p class="mono" data-testid="golive-hls">
                {{ goLiveHls || '—' }}
              </p>
              <div class="actions">
                <button
                  type="button"
                  class="btn sm"
                  data-testid="golive-copy-hls"
                  :disabled="!goLiveHls"
                  @click="copyText('HLS', goLiveHls)"
                >
                  {{ t('golive.copyHls') }}
                </button>
                <a
                  v-if="goLiveHls"
                  class="btn sm"
                  :href="goLiveHls"
                  target="_blank"
                  rel="noopener"
                  data-testid="golive-open-hls"
                  >{{ t('golive.openHls') }}</a
                >
              </div>
              <p class="card-note">
                {{ t('golive.h5Hint') }}{{ goLiveRoomId || 'roomId' }}
              </p>
            </div>
          </div>

          <div class="inset-guide" data-testid="golive-obs-guide">
            <h3>{{ t('golive.obsGuide') }}</h3>
            <ol class="guide-list">
              <li>{{ t('golive.obsStep1') }}</li>
              <li>{{ t('golive.obsStep2') }}</li>
              <li>{{ t('golive.obsStep3') }}</li>
              <li>{{ t('golive.obsStep4') }}</li>
            </ol>
          </div>
        </section>

        <!-- Rooms -->
        <section v-else-if="nav === 'rooms'" class="panel" data-testid="panel-rooms">
          <div class="panel-head">
            <h2>{{ t('rooms.title') }}</h2>
            <div class="toolbar">
              <span class="badge live">live {{ liveCount }}</span>
              <span class="badge idle">idle {{ idleCount }}</span>
              <span class="badge closed">closed {{ closedCount }}</span>
              <button
                type="button"
                class="btn sm"
                data-testid="rooms-refresh"
                :disabled="listBusy"
                @click="loadRooms"
              >
                {{ t('common.refresh') }}
              </button>
            </div>
          </div>
          <p class="panel-desc">{{ t('rooms.desc') }}</p>

          <div class="filter-bar">
            <label class="field">
              <span>{{ t('common.search') }}</span>
              <input
                v-model="roomQuery"
                type="search"
                :placeholder="t('rooms.searchPlaceholder')"
                data-testid="rooms-search"
              />
            </label>
            <label class="field field-sm">
              <span>{{ t('common.filter') }}</span>
              <select v-model="roomStatusFilter" data-testid="rooms-status-filter">
                <option value="all">{{ t('rooms.filterAll') }}</option>
                <option value="live">{{ t('status.live') }}</option>
                <option value="idle">{{ t('status.idle') }}</option>
                <option value="closed">{{ t('status.closed') }}</option>
              </select>
            </label>
            <label class="field field-sm">
              <span>{{ t('common.pageSize') }}</span>
              <select v-model.number="roomPageSize" data-testid="rooms-page-size">
                <option :value="10">10</option>
                <option :value="20">20</option>
                <option :value="50">50</option>
              </select>
            </label>
          </div>

          <div class="list-meta" v-if="roomListMeta.total" data-testid="rooms-meta">
            <span>{{ t('rooms.resultCount', { n: roomListMeta.total }) }}</span>
            <span class="dim">{{
              t('common.showing', {
                from: roomListMeta.from,
                to: roomListMeta.to,
                total: roomListMeta.total,
              })
            }}</span>
          </div>

          <div class="table-wrap is-dense" v-if="pagedRooms.length" data-testid="rooms-table">
            <table class="data data-rooms">
              <thead>
                <tr>
                  <th class="col-title">{{ t('rooms.colTitle') }}</th>
                  <th class="col-status">{{ t('rooms.colStatus') }}</th>
                  <th class="col-owner">{{ t('rooms.colOwner') }}</th>
                  <th class="col-id">{{ t('rooms.colId') }}</th>
                  <th class="col-actions">{{ t('common.actions') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="r in pagedRooms"
                  :key="r.id"
                  :data-testid="`room-row-${r.id}`"
                  :class="{ 'is-previewing': previewRoomId === r.id }"
                >
                  <td class="col-title" :title="r.title">{{ r.title }}</td>
                  <td class="col-status">
                    <span class="badge" :class="roomStatusTone(r.status)">{{
                      statusLabel(r.status)
                    }}</span>
                  </td>
                  <td class="col-owner mono" :title="r.owner_id || ''">{{
                    shortId(r.owner_id || '')
                  }}</td>
                  <td class="col-id mono" :title="r.id">{{ shortId(r.id) }}</td>
                  <td class="actions">
                    <div class="row-actions">
                      <button
                        type="button"
                        class="btn sm"
                        :class="{ primary: previewRoomId === r.id }"
                        data-testid="room-preview"
                        :disabled="previewBusy"
                        @click="previewRoom(r)"
                      >
                        {{ t('rooms.preview') }}
                      </button>
                      <button
                        type="button"
                        class="btn sm primary"
                        data-testid="room-publish-info"
                        :disabled="goLiveBusy || !isAuthed || r.status === 'closed'"
                        @click="loadPublishForRoom(r.id)"
                      >
                        {{ t('rooms.publishInfo') }}
                      </button>
                      <button
                        type="button"
                        class="btn sm"
                        data-testid="room-fill-force-close"
                        @click="useRoomId(r.id)"
                      >
                        {{ t('rooms.fillForceClose') }}
                      </button>
                      <button
                        type="button"
                        class="btn sm danger"
                        data-testid="room-force-close"
                        :disabled="actionBusy || r.status === 'closed'"
                        @click="forceCloseRoom(r.id)"
                      >
                        {{ t('rooms.forceClose') }}
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="empty" data-testid="rooms-empty">
            <div class="empty-icon">
              <AppIcon :component="Inbox" :size="28" />
            </div>
            {{ t('rooms.empty') }}
          </div>

          <div
            v-if="roomListMeta.total > 0"
            class="pager"
            data-testid="rooms-pager"
          >
            <button
              type="button"
              class="btn sm"
              data-testid="rooms-page-prev"
              :disabled="roomListMeta.page <= 1"
              @click="setRoomPage(roomListMeta.page - 1)"
            >
              {{ t('common.prev') }}
            </button>
            <span class="pager-label">{{
              t('common.page', { page: roomListMeta.page, pages: roomListMeta.pages })
            }}</span>
            <button
              type="button"
              class="btn sm"
              data-testid="rooms-page-next"
              :disabled="roomListMeta.page >= roomListMeta.pages"
              @click="setRoomPage(roomListMeta.page + 1)"
            >
              {{ t('common.next') }}
            </button>
          </div>
        </section>

        <!-- Reports -->
        <section v-else-if="nav === 'reports'" class="panel" data-testid="panel-reports">
          <div class="panel-head">
            <h2>{{ t('reports.title') }}</h2>
            <button
              type="button"
              class="btn sm"
              data-testid="reports-refresh"
              :disabled="listBusy"
              @click="loadReports"
            >
              {{ t('common.refresh') }}
            </button>
          </div>
          <p class="panel-desc">{{ t('reports.desc') }}</p>
          <label class="field field-md">
            <span>{{ t('reports.note') }}</span>
            <input
              v-model="actionReason"
              type="text"
              :placeholder="t('reports.notePlaceholder')"
              data-testid="report-note"
            />
          </label>
          <div class="table-wrap is-dense" v-if="reports.length" data-testid="reports-table">
            <table class="data">
              <thead>
                <tr>
                  <th>{{ t('reports.colTarget') }}</th>
                  <th>{{ t('reports.colReason') }}</th>
                  <th>{{ t('reports.colStatus') }}</th>
                  <th>{{ t('reports.colReporter') }}</th>
                  <th>{{ t('reports.colTime') }}</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="r in reports" :key="r.id" :data-testid="`report-row-${r.id}`">
                  <td class="mono">{{ r.target_type }}:{{ shortId(r.target_id) }}</td>
                  <td>{{ r.reason }}</td>
                  <td>
                    <span class="badge" :class="r.status === 'resolved' ? 'closed' : 'live'">
                      {{
                        r.status === 'resolved' ? t('reports.statusResolved') : t('reports.statusOpen')
                      }}
                    </span>
                  </td>
                  <td class="mono">{{ shortId(r.reporter_id) }}</td>
                  <td class="mono">{{ formatTs(r.created_at) }}</td>
                  <td class="actions">
                    <div class="row-actions">
                      <button
                        type="button"
                        class="btn sm primary"
                        data-testid="report-resolve"
                        :disabled="actionBusy || r.status === 'resolved'"
                        @click="resolveReport(r.id)"
                      >
                        {{ t('reports.resolve') }}
                      </button>
                      <button
                        v-if="r.target_type === 'room'"
                        type="button"
                        class="btn sm"
                        data-testid="report-to-force-close"
                        @click="useRoomId(r.target_id)"
                      >
                        {{ t('reports.toForceClose') }}
                      </button>
                      <button
                        v-if="r.target_type === 'user'"
                        type="button"
                        class="btn sm"
                        data-testid="report-to-moderation"
                        @click="useUserId(r.target_id)"
                      >
                        {{ t('reports.toModeration') }}
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="empty" data-testid="reports-empty">
            <div class="empty-icon">
              <AppIcon :component="FolderOpen" :size="28" />
            </div>
            {{ t('reports.empty') }}
          </div>
        </section>

        <!-- Gifts -->
        <section v-else-if="nav === 'gifts'" class="panel" data-testid="panel-gifts">
          <div class="panel-head">
            <h2>{{ t('gifts.title') }}</h2>
            <button
              type="button"
              class="btn sm"
              data-testid="gifts-refresh"
              :disabled="listBusy"
              @click="loadGifts"
            >
              {{ t('common.refresh') }}
            </button>
          </div>
          <p class="panel-desc">{{ t('gifts.desc') }}</p>
          <p class="hint seed-hint" data-testid="gifts-seed-hint">
            {{ t('gifts.seedHint') }}
            <code class="mono">./scripts/dogfood-gift-seed.sh</code>
            {{ t('gifts.seedHint2') }}
            <code class="mono">DOGFOOD_ADMIN_EMAIL</code>
            {{ t('gifts.seedHint3') }}
            <code class="mono">seed-admin-local.sh</code>
            {{ t('gifts.seedHint4') }}
            <button
              type="button"
              class="btn sm inline-btn"
              data-testid="gifts-copy-seed-cmd"
              @click="copyGiftSeedCmd"
            >
              {{ t('gifts.copyCmd') }}
            </button>
            <span v-if="giftSeedCopyHint" class="dim inline-hint">{{
              giftSeedCopyHint
            }}</span>
          </p>
          <div class="row" data-testid="gift-form">
            <label class="field">
              <span>{{ t('gifts.name') }}</span>
              <input
                v-model="giftName"
                type="text"
                placeholder="Rose"
                data-testid="gift-name"
              />
            </label>
            <label class="field field-sm">
              <span>{{ t('gifts.price') }}</span>
              <input
                v-model="giftPrice"
                type="number"
                min="1"
                step="1"
                placeholder="10"
                data-testid="gift-price"
              />
            </label>
            <label v-if="giftEditId" class="field field-sm">
              <span>{{ t('gifts.status') }}</span>
              <select v-model="giftEditActive" data-testid="gift-active">
                <option :value="true">{{ t('gifts.active') }}</option>
                <option :value="false">{{ t('gifts.inactive') }}</option>
              </select>
            </label>
            <button
              v-if="!giftEditId"
              type="button"
              class="btn primary"
              data-testid="gift-create"
              :disabled="giftBusy || !giftName.trim() || !giftPrice"
              @click="createGift"
            >
              {{ t('gifts.create') }}
            </button>
            <template v-else>
              <button
                type="button"
                class="btn primary"
                data-testid="gift-save"
                :disabled="giftBusy || !giftName.trim() || !giftPrice"
                @click="saveGiftEdit"
              >
                {{ t('gifts.save') }}
              </button>
              <button
                type="button"
                class="btn"
                data-testid="gift-cancel-edit"
                :disabled="giftBusy"
                @click="cancelEditGift"
              >
                {{ t('common.cancel') }}
              </button>
            </template>
          </div>
          <p v-if="giftEditId" class="hint" data-testid="gift-edit-hint">
            {{ t('gifts.editing', { id: shortId(giftEditId, 12) }) }}
          </p>
          <div class="table-wrap is-dense" v-if="gifts.length" data-testid="gifts-table">
            <table class="data">
              <thead>
                <tr>
                  <th>{{ t('gifts.name') }}</th>
                  <th>{{ t('common.price') }}</th>
                  <th>{{ t('gifts.status') }}</th>
                  <th>{{ t('common.id') }}</th>
                  <th>{{ t('common.actions') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="g in gifts" :key="g.id" :data-testid="`gift-row-${g.id}`">
                  <td>{{ g.name }}</td>
                  <td>{{ g.price }}</td>
                  <td>
                    <span
                      class="badge"
                      :class="g.active === false ? 'closed' : 'live'"
                      data-testid="gift-status"
                    >
                      {{ g.active === false ? t('gifts.inactive') : t('gifts.active') }}
                    </span>
                  </td>
                  <td class="mono">{{ shortId(g.id, 12) }}</td>
                  <td class="actions">
                    <div class="row-actions">
                      <button
                        type="button"
                        class="btn sm"
                        data-testid="gift-edit"
                        :disabled="giftBusy"
                        @click="beginEditGift(g)"
                      >
                        {{ t('gifts.edit') }}
                      </button>
                      <button
                        type="button"
                        class="btn sm"
                        :class="g.active === false ? 'primary' : 'danger'"
                        data-testid="gift-toggle-active"
                        :disabled="giftBusy"
                        @click="toggleGiftActive(g)"
                      >
                        {{ g.active === false ? t('gifts.enable') : t('gifts.disable') }}
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="empty" data-testid="gifts-empty">
            <div class="empty-icon">
              <AppIcon :component="Inbox" :size="28" />
            </div>
            {{ t('gifts.empty') }}
          </div>
        </section>

        <!-- Users -->
        <section v-else-if="nav === 'users'" class="panel" data-testid="panel-users">
          <div class="panel-head">
            <h2>{{ t('users.title') }}</h2>
            <div class="toolbar">
              <span class="stat-chip">{{ t('users.total', { n: usersTotal }) }}</span>
              <span class="stat-chip">{{ t('users.bannedList') }} · {{ bannedUsers.length }}</span>
              <span class="stat-chip">{{ t('users.mutedList') }} · {{ mutedUsers.length }}</span>
              <button
                type="button"
                class="btn sm"
                data-testid="users-list-refresh"
                :disabled="usersBusy"
                @click="loadUsers"
              >
                {{ t('common.refresh') }}
              </button>
            </div>
          </div>
          <p class="panel-desc">{{ t('navBlurb.users') }}</p>
          <p v-if="tempPasswordNotice" class="flash ok" data-testid="temp-password">
            {{ tempPasswordNotice }}
          </p>

          <!-- Search + status filter (primary work surface) -->
          <div class="filter-bar" data-testid="users-filter-bar">
            <label class="field field-lg">
              <span>{{ t('common.search') }}</span>
              <input
                v-model="usersQuery"
                type="search"
                :placeholder="t('users.searchPlaceholder')"
                data-testid="users-query"
                @keyup.enter="searchUsers"
              />
            </label>
            <label class="field field-sm">
              <span>{{ t('users.status') }}</span>
              <select v-model="usersStatusFilter" data-testid="users-status-filter">
                <option value="all">{{ t('common.all') }}</option>
                <option value="active">{{ t('users.statusActive') }}</option>
                <option value="disabled">{{ t('users.statusDisabled') }}</option>
                <option value="deleted">{{ t('users.statusDeleted') }}</option>
              </select>
            </label>
            <label class="field field-sm">
              <span>{{ t('common.pageSize') }}</span>
              <select v-model.number="usersPageSize" data-testid="users-page-size">
                <option :value="10">10</option>
                <option :value="20">20</option>
                <option :value="50">50</option>
              </select>
            </label>
            <button
              type="button"
              class="btn primary"
              data-testid="users-search-submit"
              :disabled="usersBusy"
              @click="searchUsers"
            >
              {{ t('users.search') }}
            </button>
          </div>

          <div class="list-meta" v-if="usersListMeta.total" data-testid="users-meta">
            <span>{{ t('users.total', { n: usersListMeta.total }) }}</span>
            <span class="dim">{{
              t('common.showing', {
                from: usersListMeta.from,
                to: usersListMeta.to,
                total: usersListMeta.total,
              })
            }}</span>
          </div>

          <div v-if="usersList.length" class="table-wrap is-dense" data-testid="users-table-wrap">
            <table class="data data-users" data-testid="users-table">
              <thead>
                <tr>
                  <th class="col-user">{{ t('users.displayName') }}</th>
                  <th class="col-email">{{ t('users.email') }}</th>
                  <th class="col-status">{{ t('users.status') }}</th>
                  <th class="col-actions">{{ t('users.actions') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="u in usersList" :key="u.id" :data-testid="`user-row-${u.id}`">
                  <td class="col-user">
                    <div class="user-cell">
                      <span class="user-name" :title="u.display_name">{{ u.display_name }}</span>
                      <span v-if="u.admin_role" class="role-pill admin">{{ u.admin_role }}</span>
                    </div>
                    <div class="cell-sub mono" :title="u.id">{{ shortId(u.id) }}</div>
                    <div v-if="u.username" class="cell-sub mono" :title="u.username">
                      @{{ u.username }}
                    </div>
                  </td>
                  <td class="col-email" :title="u.email || ''">{{ u.email || '—' }}</td>
                  <td class="col-status col-status-mix">
                    <div class="badge-row">
                      <span
                        class="badge"
                        :class="
                          u.status === 'active'
                            ? 'ok'
                            : u.status === 'disabled'
                              ? 'idle'
                              : 'closed'
                        "
                      >
                        {{
                          u.status === 'active'
                            ? t('users.statusActive')
                            : u.status === 'disabled'
                              ? t('users.statusDisabled')
                              : u.status === 'deleted'
                                ? t('users.statusDeleted')
                                : u.status
                        }}
                      </span>
                      <span v-if="u.banned" class="badge closed">{{ t('users.banned') }}</span>
                      <span v-if="u.muted" class="badge idle">{{ t('users.muted') }}</span>
                      <span v-if="u.must_change_password" class="badge unknown">{{
                        t('users.mustChangePassword')
                      }}</span>
                    </div>
                  </td>
                  <td class="actions col-actions">
                    <div class="row-actions">
                      <button
                        type="button"
                        class="btn sm"
                        :disabled="actionBusy"
                        :title="t('users.resetPassword')"
                        @click="resetUserPassword(u.id)"
                      >
                        {{ t('users.resetPasswordShort') }}
                      </button>
                      <button
                        type="button"
                        class="btn sm"
                        :disabled="actionBusy"
                        :title="t('users.revokeSessions')"
                        @click="revokeUserSessions(u.id)"
                      >
                        {{ t('users.revokeSessionsShort') }}
                      </button>
                      <button
                        v-if="!u.banned"
                        type="button"
                        class="btn sm danger"
                        :disabled="actionBusy"
                        @click="banUserId(u.id)"
                      >
                        {{ t('users.ban') }}
                      </button>
                      <button
                        v-else
                        type="button"
                        class="btn sm"
                        :disabled="actionBusy"
                        @click="unbanUserId(u.id)"
                      >
                        {{ t('users.unban') }}
                      </button>
                      <button
                        v-if="u.status === 'active'"
                        type="button"
                        class="btn sm"
                        :disabled="actionBusy"
                        @click="setUserStatus(u.id, 'disabled')"
                      >
                        {{ t('users.disable') }}
                      </button>
                      <button
                        v-else-if="u.status === 'disabled'"
                        type="button"
                        class="btn sm primary"
                        :disabled="actionBusy"
                        @click="setUserStatus(u.id, 'active')"
                      >
                        {{ t('users.enable') }}
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="empty" data-testid="users-empty">
            <div class="empty-icon">
              <AppIcon :component="Inbox" :size="28" />
            </div>
            {{ usersBusy ? t('common.loading') : t('users.empty') }}
          </div>

          <div
            v-if="usersListMeta.total > 0"
            class="pager"
            data-testid="users-pager"
          >
            <button
              type="button"
              class="btn sm"
              data-testid="users-page-prev"
              :disabled="usersListMeta.page <= 1 || usersBusy"
              @click="setUsersPage(usersListMeta.page - 1)"
            >
              {{ t('common.prev') }}
            </button>
            <span class="pager-label">{{
              t('common.page', { page: usersListMeta.page, pages: usersListMeta.pages })
            }}</span>
            <button
              type="button"
              class="btn sm"
              data-testid="users-page-next"
              :disabled="usersListMeta.page >= usersListMeta.pages || usersBusy"
              @click="setUsersPage(usersListMeta.page + 1)"
            >
              {{ t('common.next') }}
            </button>
          </div>

          <!-- Compact provision form -->
          <details class="section-block users-create-fold" data-testid="users-create-fold">
            <summary class="users-create-summary">{{ t('users.create') }}</summary>
            <div class="action-card action-card-md mt-sm">
              <div class="row">
                <label class="field">
                  <span>{{ t('users.displayName') }}</span>
                  <input v-model="createDisplayName" type="text" data-testid="create-display-name" />
                </label>
                <label class="field">
                  <span>{{ t('users.username') }}</span>
                  <input v-model="createUsername" type="text" data-testid="create-username" />
                </label>
              </div>
              <div class="row">
                <label class="field">
                  <span>{{ t('users.email') }}</span>
                  <input v-model="createEmail" type="email" data-testid="create-email" />
                </label>
                <label class="field">
                  <span>{{ t('users.password') }}</span>
                  <input v-model="createPassword" type="text" data-testid="create-password" />
                </label>
              </div>
              <button
                type="button"
                class="btn primary"
                data-testid="create-user-submit"
                :disabled="createBusy || !createDisplayName.trim()"
                @click="createUser"
              >
                {{ t('users.create') }}
              </button>
            </div>
          </details>

          <div class="panel-head section-divider">
            <h3>{{ t('users.moderationLists') }}</h3>
            <button
              type="button"
              class="btn sm"
              data-testid="users-refresh"
              :disabled="listBusy"
              @click="loadModerationLists"
            >
              {{ t('users.refreshLists') }}
            </button>
          </div>
          <p class="panel-desc">{{ t('users.desc') }}</p>

          <div class="action-card action-card-md section-block" data-testid="users-lookup">
            <h3>{{ t('users.lookup') }}</h3>
            <label class="field">
              <span>{{ t('moderation.userId') }}</span>
              <input
                v-model="lookupUserId"
                class="mono"
                type="text"
                :placeholder="t('users.lookupPlaceholder')"
                data-testid="users-lookup-id"
              />
            </label>
            <div class="row row-tight">
              <button
                type="button"
                class="btn primary"
                data-testid="users-lookup-submit"
                :disabled="lookupBusy || !lookupUserId.trim()"
                @click="lookupUserModeration"
              >
                {{ t('users.lookupSubmit') }}
              </button>
              <button
                type="button"
                class="btn sm"
                data-testid="users-fill-ban"
                :disabled="!lookupUserId.trim() && !lookupStatus"
                @click="fillBanFromLookup"
              >
                {{ t('users.fillBan') }}
              </button>
              <button
                type="button"
                class="btn sm"
                data-testid="users-fill-mute"
                :disabled="!lookupUserId.trim() && !lookupStatus"
                @click="fillMuteFromLookup"
              >
                {{ t('users.fillMute') }}
              </button>
            </div>
            <div v-if="lookupStatus" class="hint" data-testid="users-lookup-result">
              <strong>{{ t('users.statusTitle') }}</strong>
              <div class="mono mt-xs">{{ shortId(lookupStatus.user_id, 16) }}</div>
              <div v-if="lookupStatus.banned || lookupStatus.muted" class="mt-xs badge-row">
                <span v-if="lookupStatus.banned" class="badge closed">
                  {{ t('users.statusBanned') }}
                  <template v-if="lookupStatus.ban_reason"> · {{ lookupStatus.ban_reason }}</template>
                  <template v-if="lookupStatus.banned_at">
                    · {{ t('users.statusSince', { time: formatTs(lookupStatus.banned_at) }) }}
                  </template>
                </span>
                <span v-if="lookupStatus.muted" class="badge idle">
                  {{ t('users.statusMuted') }}
                  <template v-if="lookupStatus.mute_reason"> · {{ lookupStatus.mute_reason }}</template>
                  <template v-if="lookupStatus.muted_at">
                    · {{ t('users.statusSince', { time: formatTs(lookupStatus.muted_at) }) }}
                  </template>
                </span>
              </div>
              <div v-else class="dim mt-xs">{{ t('users.statusClear') }}</div>
              <div class="row row-tight mt-sm">
                <button
                  v-if="lookupStatus.banned"
                  type="button"
                  class="btn sm primary"
                  data-testid="users-lookup-unban"
                  :disabled="actionBusy"
                  @click="unbanUser(lookupStatus.user_id)"
                >
                  {{ t('users.unban') }}
                </button>
                <button
                  v-if="lookupStatus.muted"
                  type="button"
                  class="btn sm primary"
                  data-testid="users-lookup-unmute"
                  :disabled="actionBusy"
                  @click="unmuteUser(lookupStatus.user_id)"
                >
                  {{ t('users.unmute') }}
                </button>
              </div>
            </div>
          </div>

          <div class="split-actions">
            <div class="action-card" data-testid="users-banned-card">
              <h3>{{ t('users.bannedList') }} · {{ bannedUsers.length }}</h3>
              <div class="table-wrap is-compact" v-if="bannedUsers.length" data-testid="users-banned-table">
                <table class="data data-compact">
                  <thead>
                    <tr>
                      <th>{{ t('users.colUser') }}</th>
                      <th>{{ t('users.colReason') }}</th>
                      <th>{{ t('users.colTime') }}</th>
                      <th>{{ t('common.actions') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="u in bannedUsers" :key="u.user_id" :data-testid="`banned-row-${u.user_id}`">
                      <td class="mono" :title="u.user_id">{{ shortId(u.user_id, 12) }}</td>
                      <td :title="u.reason || ''">{{ u.reason || t('common.none') }}</td>
                      <td class="mono">{{ formatTs(u.created_at) }}</td>
                      <td class="actions">
                        <button
                          type="button"
                          class="btn sm primary"
                          data-testid="users-unban-row"
                          :disabled="actionBusy"
                          @click="unbanUser(u.user_id)"
                        >
                          {{ t('users.unban') }}
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-else class="empty" data-testid="users-banned-empty">{{ t('users.emptyBanned') }}</div>
            </div>

            <div class="action-card" data-testid="users-muted-card">
              <h3>{{ t('users.mutedList') }} · {{ mutedUsers.length }}</h3>
              <div class="table-wrap is-compact" v-if="mutedUsers.length" data-testid="users-muted-table">
                <table class="data data-compact">
                  <thead>
                    <tr>
                      <th>{{ t('users.colUser') }}</th>
                      <th>{{ t('users.colReason') }}</th>
                      <th>{{ t('users.colTime') }}</th>
                      <th>{{ t('common.actions') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="u in mutedUsers" :key="u.user_id" :data-testid="`muted-row-${u.user_id}`">
                      <td class="mono" :title="u.user_id">{{ shortId(u.user_id, 12) }}</td>
                      <td :title="u.reason || ''">{{ u.reason || t('common.none') }}</td>
                      <td class="mono">{{ formatTs(u.created_at) }}</td>
                      <td class="actions">
                        <button
                          type="button"
                          class="btn sm primary"
                          data-testid="users-unmute-row"
                          :disabled="actionBusy"
                          @click="unmuteUser(u.user_id)"
                        >
                          {{ t('users.unmute') }}
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <div v-else class="empty" data-testid="users-muted-empty">{{ t('users.emptyMuted') }}</div>
            </div>
          </div>
        </section>

        <!-- Moderation -->
        <section v-else-if="nav === 'moderation'" class="panel" data-testid="panel-moderation">
          <div class="panel-head">
            <h2>{{ t('moderation.title') }}</h2>
          </div>
          <p class="panel-desc">{{ t('moderation.desc') }}</p>
          <p class="danger-note">{{ t('moderation.dangerHint') }}</p>

          <label class="field field-lg">
            <span>{{ t('moderation.reason') }}</span>
            <input
              v-model="actionReason"
              type="text"
              :placeholder="t('moderation.reasonPlaceholder')"
              data-testid="moderation-reason"
            />
          </label>

          <div class="split-actions">
            <div class="action-card danger-zone" data-testid="moderation-force-close">
              <h3>{{ t('moderation.forceClose') }}</h3>
              <label class="field">
                <span>{{ t('moderation.roomId') }}</span>
                <input
                  v-model="roomIdInput"
                  class="mono"
                  type="text"
                  placeholder="uuid"
                  data-testid="moderation-room-id"
                />
              </label>
              <button
                type="button"
                class="btn danger"
                data-testid="moderation-force-close-submit"
                :disabled="actionBusy || !roomIdInput.trim()"
                @click="forceCloseRoom()"
              >
                {{ t('moderation.forceCloseSubmit') }}
              </button>
            </div>

            <div class="action-card danger-zone" data-testid="moderation-ban">
              <h3>{{ t('moderation.ban') }}</h3>
              <label class="field">
                <span>{{ t('moderation.userId') }}</span>
                <input
                  v-model="userIdInput"
                  class="mono"
                  type="text"
                  placeholder="uuid"
                  data-testid="moderation-ban-user-id"
                />
              </label>
              <button
                type="button"
                class="btn danger"
                data-testid="moderation-ban-submit"
                :disabled="actionBusy || !userIdInput.trim()"
                @click="banUser"
              >
                {{ t('moderation.banSubmit') }}
              </button>
            </div>

            <div class="action-card danger-zone" data-testid="moderation-mute">
              <h3>{{ t('moderation.mute') }}</h3>
              <label class="field">
                <span>{{ t('moderation.userId') }}</span>
                <input
                  v-model="muteUserIdInput"
                  class="mono"
                  type="text"
                  placeholder="uuid"
                  data-testid="moderation-mute-user-id"
                />
              </label>
              <button
                type="button"
                class="btn danger"
                data-testid="moderation-mute-submit"
                :disabled="actionBusy || !muteUserIdInput.trim()"
                @click="muteUser"
              >
                {{ t('moderation.muteSubmit') }}
              </button>
            </div>

            <div class="action-card" data-testid="moderation-unmute">
              <h3>{{ t('moderation.unmute') }}</h3>
              <label class="field">
                <span>{{ t('moderation.userId') }}</span>
                <input
                  v-model="unmuteUserIdInput"
                  class="mono"
                  type="text"
                  placeholder="uuid"
                  data-testid="moderation-unmute-user-id"
                />
              </label>
              <button
                type="button"
                class="btn primary"
                data-testid="moderation-unmute-submit"
                :disabled="actionBusy || !unmuteUserIdInput.trim()"
                @click="() => unmuteUser()"
              >
                {{ t('moderation.unmuteSubmit') }}
              </button>
            </div>
          </div>
        </section>

        <!-- Audit -->
        <section v-else-if="nav === 'audit'" class="panel" data-testid="panel-audit">
          <div class="panel-head">
            <h2>{{ t('audit.title') }}</h2>
            <button
              type="button"
              class="btn sm"
              data-testid="audit-refresh"
              :disabled="listBusy"
              @click="loadAudit"
            >
              {{ t('common.refresh') }}
            </button>
          </div>
          <p class="panel-desc">{{ t('audit.desc') }}</p>
          <div class="filter-bar">
            <label class="field">
              <span>{{ t('common.search') }}</span>
              <input
                v-model="auditQuery"
                type="search"
                :placeholder="t('audit.searchPlaceholder')"
                data-testid="audit-search"
              />
            </label>
          </div>
          <div class="table-wrap is-dense" v-if="filteredAudit.length" data-testid="audit-table">
            <table class="data">
              <thead>
                <tr>
                  <th>{{ t('audit.colAction') }}</th>
                  <th>{{ t('audit.colActor') }}</th>
                  <th>{{ t('audit.colTarget') }}</th>
                  <th>{{ t('audit.colDetail') }}</th>
                  <th>{{ t('audit.colTime') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="a in filteredAudit" :key="a.id" :data-testid="`audit-row-${a.id}`">
                  <td>{{ a.action }}</td>
                  <td class="mono">{{ shortId(a.actor_id) }}</td>
                  <td class="mono">{{ shortId(a.target) }}</td>
                  <td>{{ a.detail }}</td>
                  <td class="mono">{{ formatTs(a.created_at) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="empty" data-testid="audit-empty">
            <div class="empty-icon">
              <AppIcon :component="FolderOpen" :size="28" />
            </div>
            {{ t('audit.empty') }}
          </div>
        </section>
      </div>
    </div>
  </div>

  <!-- Room HLS preview modal (above list, doesn't bury player under rows) -->
  <Teleport to="body">
    <div
      v-if="previewRoomId"
      class="modal-overlay"
      data-testid="room-preview-panel"
      role="presentation"
      @click.self="closePreview"
    >
      <div
        class="modal-dialog preview-modal"
        role="dialog"
        aria-modal="true"
        :aria-label="t('rooms.previewTitle')"
        data-testid="room-preview-dialog"
        @keydown.escape.prevent="closePreview"
      >
        <div class="modal-head">
          <div class="modal-title">
            <h2>{{ t('rooms.previewTitle') }}</h2>
            <span class="mono dim">{{ shortId(previewRoomId, 12) }}</span>
          </div>
          <button
            type="button"
            class="btn sm ghost"
            data-testid="room-preview-close"
            @click="closePreview"
          >
            {{ t('common.close') }}
          </button>
        </div>
        <div class="modal-body">
          <p v-if="previewBusy" class="hint">{{ t('common.loading') }}</p>
          <p v-if="previewError" class="flash err">{{ previewError }}</p>
          <p v-if="previewHlsUrl" class="preview-url mono">
            <a :href="previewHlsUrl" target="_blank" rel="noopener">{{ previewHlsUrl }}</a>
          </p>
          <video
            v-if="previewHlsUrl"
            ref="previewVideoEl"
            controls
            playsinline
            autoplay
            class="preview-video"
          />
        </div>
      </div>
    </div>
  </Teleport>

</template>

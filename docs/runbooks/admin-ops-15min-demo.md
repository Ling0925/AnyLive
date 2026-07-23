# Admin 运营 15 分钟演示走查（V-AD-1）

对应验收：**V-AD-1** — 运营预置礼物目录、admin 账号、演示脚本走查（[p1-parallel-tracks](../product/p1-parallel-tracks.md)）。  
产品演示脚本原文：[mvp-scope.md §4](../product/mvp-scope.md)（本 runbook 把 §4 七步落到具体命令、URL、`data-testid`）。

> **本文件是走查清单。** Solo 项目**无签字关闸** — [solo-owner-mode.md](../product/solo-owner-mode.md)。  
> 看板口径见 [p1-parallel-tracks.md](../product/p1-parallel-tracks.md)。

---

## 0. 预检（~2 min）

| # | 检查 | 命令 / 证据 |
|---|---|---|
| 1 | 测试栈健康 | `./scripts/deploy-test.sh`（或 API `GET http://localhost:8088/health` 200 + Admin 可开） |
| 2 | Admin 账号 | `./scripts/seed-admin-local.sh ops@anylive.local` **或** `DOGFOOD_ADMIN_EMAIL` 已在 `admin_users`；首次空库可走 UI bootstrap |
| 3 | 礼物目录 | `./scripts/dogfood-gift-seed.sh` → Rose/1 · Heart/10 · Rocket/100（固定 UUID，可重复跑） |
| 4 |（可选）控制面 10 分钟路径 | `./scripts/dogfood-10min-path.sh` → 末行含 `DOGFOOD_10MIN_PATH_PASS` |

共用 env：`API_BASE`（默认 `http://localhost:8088`）、`OTP_CODE`（默认 `123456`）、`DOGFOOD_ADMIN_EMAIL`。详见 [go-live-local.md §1–2](./go-live-local.md)。

**Admin URL**

| 方式 | URL |
|---|---|
| Compose 管理台 | **http://localhost:8090/** |
| 源码热更新 | `cd apps/admin-web && pnpm dev`（Vite 默认端口，需 `VITE_API_BASE=http://localhost:8088`） |

**OTP（仅开发 / 测试栈）**：发送验证码后填 **`123456`**（`ALLOW_DEV_OTP=1`）。勿用于生产。

---

## 1. 演示脚本（mvp-scope §4 → 点击路径）

计时目标 **约 15 分钟**。下列 `data-testid` 与 `apps/admin-web` 当前壳一致。

### 步骤 1 — Admin 配置 3 个礼物（~2 min）

| 动作 | 锚点 |
|---|---|
| 打开登录页 | `data-testid="login-screen"` / `login-card` |
| 填邮箱 | `login-email`（建议 `ops@anylive.local` 或已 seed 的 `DOGFOOD_ADMIN_EMAIL`） |
| 发送验证码 | `login-send-otp` |
| 填 OTP `123456` | `login-otp` → `login-submit` |
| 确认会话为 admin | 顶栏角色 `admin`；若见 `admin-gate` → 回到预检 §0.2 seed 后重登 |
| 侧栏「礼物配置」 | `nav-gifts` → 面板 `panel-gifts` |
| 一键目录（推荐） | 宿主机跑 `./scripts/dogfood-gift-seed.sh`（面板内有 seed 提示）；或 UI 用 `gift-name` / `gift-price` / `gift-create` 手建 3 条 |
| 校验列表 | `gifts-table` 至少 3 行（Rose / Heart / Rocket 或等价） |

### 步骤 2 — 主播 A 开播（OBS）（~3 min）

| 动作 | 锚点 |
|---|---|
| 侧栏「开播」 | `nav-golive` → `panel-golive` |
| 标题 + 一键开播 | `golive-title` → `golive-start` |
| 复制 OBS 字段 | `golive-obs`：`golive-obs-server` / `golive-obs-stream-key`（复制钮 `golive-copy-server` / `golive-copy-stream-key`） |
| OBS | 服务=自定义；服务器=Server；串流密钥=**完整** Stream Key（含 `?exp=&sig=`） |
| 可选 HLS 预览 | `golive-preview` 或 `golive-open-hls` |

（API-only 主播路径见 [go-live-local.md §4](./go-live-local.md)。）

### 步骤 3 — 观众 B/C 进房聊天（~2 min）

| 动作 | 证据 |
|---|---|
| H5 / Flutter / 控制面 | H5：`?room={room_id}`；或 `./scripts/dogfood-10min-path.sh` 观众段 |
| 管理台核对房间 | `nav-rooms` → `panel-rooms` / `rooms-table`，状态 `live` |

### 步骤 4 — B 充值测试币并送礼（~2 min）

| 动作 | 证据 |
|---|---|
| Mock 充值 + 送礼 | 客户端 mock topup / `dogfood-10min-path`（`ALLOW_MOCK_TOPUP=1`） |
| 礼物来自目录 | 步骤 1 seed 的 Rose/Heart/Rocket（或 UI 自建） |

### 步骤 5 — A 端/全员看到礼物效果（~1 min）

| 动作 | 证据 |
|---|---|
| 主播端 / H5 / App | 礼物动画或系统通知（产品路径） |
| 管理台 | 总览 KPI `kpi-gifts` 种类数与目录一致即可（不替代客户端观感） |

### 步骤 6 — C 举报 → Admin 处理 → 可选强关（~3 min）

| 动作 | 锚点 |
|---|---|
| 观众侧举报 | App/H5 举报入口（产品路径） |
| 管理台队列 | `nav-reports` → `panel-reports` / `reports-table` |
| 标记已处理 | `report-resolve`（可选备注 `report-note`） |
| 可选强关 | `report-to-force-close` 或 `nav-moderation` → `panel-moderation` / `moderation-force-close-submit`；房间列表 `room-force-close` |

### 步骤 7 — 展示钱包流水与审计日志（~2 min）

| 动作 | 锚点 |
|---|---|
| 钱包对账 | `nav-dashboard` → `panel-wallet-ops` → `wallet-reconcile` → 读 `wallet-reconcile-hint`（期望 balanced） |
| 审计日志 | `nav-audit` → `panel-audit` / `audit-table`（应有 force-close / resolve / gift upsert 等写操作） |

---

## 2. 失败速查

| 现象 | 处理 |
|---|---|
| 登录后 `data-testid="admin-gate"` | bootstrap 已关闭且当前邮箱非 admin → `./scripts/seed-admin-local.sh <email>` 或换 `DOGFOOD_ADMIN_EMAIL` |
| 礼物列表空 | `./scripts/dogfood-gift-seed.sh`；确认用 admin token 的 admin gifts 路径 |
| OBS 拒推 / on_publish 失败 | 串流密钥必须是完整 `stream_key`，勿用裸 UUID — 见 go-live-local §4 |
| 对账不平衡 | 停演示，查 ledger / 资金单测；**不得**伪造 balanced 签字 |

---

## 3. 相关文档

- 本地全栈与 Admin 初始化：[go-live-local.md](./go-live-local.md) §1–2  
- Dogfood 队列与预检：[dogfood-cohort.md](./dogfood-cohort.md)  
- Admin 源码快速开始：[`apps/admin-web/README.md`](../../apps/admin-web/README.md)  
- MVP 演示原文：[mvp-scope.md §4](../product/mvp-scope.md)  
- 并行轨状态板：[p1-parallel-tracks.md](../product/p1-parallel-tracks.md)（**勿在此自动关闭 V-AD-1**）  
- Wave2 Admin 演示包（prep 证据，**≠ 关闭 V-AD-1**）：[`reports/wave2-ad-demo-pack-20260723.md`](../../reports/wave2-ad-demo-pack-20260723.md)

---

## 4. 自测记录（可选 · solo 无签字关闸）

> 个人项目**不要求** footer 签字。见 [solo-owner-mode.md](../product/solo-owner-mode.md)。需要时自行勾选备忘即可。

- [ ] 预检 §0 通过（栈 + admin + 礼物 seed）
- [ ] §1 主路径点通（OBS/H5/App 按你环境）
- [ ] 钱包对账看过一眼

| 字段 | 填写（可选） |
|---|---|
| 自测人 | |
| 日期 | |
| 环境 | |
| 备注 | |

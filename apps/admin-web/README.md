# AnyLive Admin（运营控制台）

Vue 3 + TypeScript + Vite。本地 dogfood / 测试栈运营台：OTP 登录、开播 OBS 凭证、房间/举报/礼物/处置/审计、钱包对账。

UI 为 Claude 风格暖色运营台；**中英双语**（默认中文，侧栏/登录页可切换，`localStorage` 键 `anylive_admin_locale_v1`）。登录页为双栏炫光 OTP 流。

## 快速开始

### Compose 测试栈（推荐）

```bash
# 仓库根目录
./scripts/deploy-test.sh
# Admin: http://localhost:8090/  ·  API: http://localhost:8088
```

源码热更新：

```bash
cd apps/admin-web
# 默认已指向本地 API；可覆盖：
# VITE_API_BASE=http://localhost:8088 pnpm dev
pnpm install
pnpm dev
```

| 变量 | 默认 | 说明 |
|---|---|---|
| `VITE_API_BASE` | `http://localhost:8088` | 浏览器请求的 API 基址（构建时注入） |

Docker 镜像 `anylive-admin` 需重建后才含源码改动：

```bash
docker compose -f deploy/docker-compose.yml --profile app build admin
```

### 登录

1. 打开 Admin（`:8090` 或 Vite dev）
2. 邮箱 OTP — **开发/测试栈固定码 `123456`**（`ALLOW_DEV_OTP=1`，勿用于生产）
3. 侧栏角色应为 `admin`；若出现「非管理员」横幅（`admin-gate`），见下方授权

### 管理员授权

| 场景 | 做法 |
|---|---|
| 首次（`admin_users` 为空） | 登录后 UI 自动 `POST /api/v1/admin/grant` bootstrap |
| 已有管理员（常见 dogfood 反复跑） | `./scripts/seed-admin-local.sh ops@example.com` 后用该邮箱 + `123456` 登录 |
| 脚本默认管理员 | 设置 `DOGFOOD_ADMIN_EMAIL` 指向已在 `admin_users` 的邮箱 |
| 手工 | `docker exec` 向 `admin_users` 插入 `user_id`（见 gate 文案） |

### 礼物一键 seed

```bash
# 仓库根目录 — upsert Rose/1 · Heart/10 · Rocket/100（固定 UUID，可重复跑）
./scripts/dogfood-gift-seed.sh
# 已有管理员时：
DOGFOOD_ADMIN_EMAIL=ops@example.com ./scripts/dogfood-gift-seed.sh
```

UI「礼物配置」面板有同命令提示；不假装已 seed 成功状态。

## 15 分钟运营演示（V-AD-1）

逐步预检 + 面板点击 + 签字清单：

- **[docs/runbooks/admin-ops-15min-demo.md](../../docs/runbooks/admin-ops-15min-demo.md)**
- 本地栈与网页开播：[docs/runbooks/go-live-local.md](../../docs/runbooks/go-live-local.md) §2

**仅人工签字关闭 V-AD-1** — 勿在未走查时把 `p1-parallel-tracks` 标 done。

## 开发命令

```bash
pnpm test      # vitest（admin helpers）
pnpm build
pnpm preview
```

## 目录

```
src/
  App.vue              # 登录壳 + 运营面板（i18n 模板）
  style.css            # Claude 暖色设计 token
  i18n/
    messages.ts        # zh / en 文案
    index.ts           # t() / useI18n / locale 持久化
  lib/admin.ts         # 路径 / RBAC / 展示 helper
  lib/admin.spec.ts
```

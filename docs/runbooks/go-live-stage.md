# Stage / 生产上线 Runbook

> 本地 dogfood 见 [go-live-local.md](./go-live-local.md)。  
> 合规闸门总表见 [海外合规与上架闸门](../compliance/海外合规与上架闸门.md)。  
> **Stage 可填 env 模板：** [deploy/.env.stage.example](../../deploy/.env.stage.example)（复制后填密钥；勿提交真密钥）。

本文面向 **stage（预发）→ 小流量 prod**。生产环境禁止一切 mock / fixed-OTP 开关。

---

## 1. 环境分层

| 环境 | `APP_ENV` | Postgres | 用途 |
|---|---|---|---|
| local / dogfood | `local` | 可选 | 开发、OBS 联调 |
| stage | `staging` 或 `production`（建议 `staging` 若代码区分） | 必须 | 联调、商店内测、支付沙箱 |
| prod | `production` / `prod` | 必须 | 公开流量 |

当前 API 生产守卫在 `APP_ENV=production|prod` 时 **fail-closed**。Stage 若未用 production 别名，须人工对照下文清单，勿打开 mock。

**快速起步（stage 草稿 env）：**

```bash
cp deploy/.env.stage.example deploy/.env.stage
# 编辑 deploy/.env.stage：DATABASE_URL、JWT_*、OTP_HTTP_*、SRS_*、CORS
# 要点：APP_ENV=staging · USE_POSTGRES=1 · ALLOW_DEV_OTP=0 · ALLOW_MOCK_TOPUP=0
#       OTP_NOTIFIER=http · FEATURE_PK=0 FEATURE_COHOST=0 · 无 PAY mock
```

**本地 stage 拓扑排练（无云账号也可）：**

```bash
# 叠层 compose + 铸造密钥；无 ESP 时加 STAGE_LOCAL_ALLOW_DEV_OTP=1
STAGE_LOCAL_ALLOW_DEV_OTP=1 ./scripts/stage-up.sh
# 备份 / 隔离恢复
./scripts/backup-pg.sh
./scripts/restore-pg-drill.sh reports/pg-backup-*.dump
```

工件：`deploy/docker-compose.stage.yml` · `deploy/.env.stage.local.example` · `docs/product/p2-status.md`。

本地对照默认见根目录 [.env.example](../../.env.example)；测试栈 mock 默认见 [deploy/.env.test](../../deploy/.env.test)（**勿**当 stage 用）。

---

## 2. 生产禁止项（启动即失败 / 禁止上线）

| 变量 / 行为 | 生产 |
|---|---|
| `ALLOW_DEV_OTP=1` | **禁止** |
| `ALLOW_MOCK_TOPUP=1` | **禁止** |
| `ALLOW_INSECURE_JWT=1` | **禁止** |
| `PAY_ENABLE_MOCK` / `PAY_CHANNELS=mock` | **禁止** |
| `OTP_NOTIFIER=log\|noop` | **禁止** |
| 默认 JWT / Centrifugo / PAY_MOCK 密钥 | **禁止** |
| `USE_POSTGRES=0` 内存存储 | **禁止** |
| 空 `SRS_WEBHOOK_SECRET` | **禁止** |
| 未配置 `CORS_ALLOWED_ORIGINS` | **禁止**（production） |

---

## 3. 必配环境变量清单

### 3.1 核心

```bash
APP_ENV=production
API_BIND=0.0.0.0:8088
USE_POSTGRES=1
DATABASE_URL=postgres://USER:PASS@HOST:5432/anylive

JWT_ACCESS_SECRET=<≥32 随机，与 refresh 不同>
JWT_REFRESH_SECRET=<≥32 随机>
CENTRIFUGO_TOKEN_SECRET=<非默认，≥16>
CENTRIFUGO_URL=https://centrifugo.internal
CENTRIFUGO_API_KEY=<secret>

CORS_ALLOWED_ORIGINS=https://app.example.com,https://admin.example.com,https://h5.example.com
```

### 3.2 OTP 真投递

```bash
# 禁止 ALLOW_DEV_OTP
OTP_NOTIFIER=http          # 或 smtp（同 HttpOtpNotifier）
OTP_HTTP_URL=https://mailer.example/v1/send
OTP_HTTP_BEARER=<optional but recommended>
OTP_HTTP_FROM=noreply@example.com
OTP_HTTP_SUBJECT=Your AnyLive login code
```

生产要求：`OTP_HTTP_URL` 为 **https://**（紧急可用 localhost break-glass，勿用于真产）。

### 3.3 媒体

```bash
SRS_RTMP_URL=rtmp://origin.example.com/live
SRS_HLS_BASE=https://cdn.example.com/live
SRS_PUBLISH_SECRET=<长随机>
SRS_WEBHOOK_SECRET=<长随机>
# 公网 API 基址供回调文档 / 运维
# SRS on_publish → https://api.example.com/api/v1/webhooks/srs/on_publish?secret=...
```

### 3.4 支付（真实通道未接前）

```bash
# 不要设置 PAY_CHANNELS=mock / PAY_ENABLE_MOCK
# 用户充值入口：关闭 H5/App mock 购买；仅保留账本与礼物消耗
# 后续：PAY_CHANNELS=jeepay|epay|tokenpay + 渠道密钥（见 payment-channels.md）
```

### 3.5 合规文案 URL

```bash
LEGAL_PRIVACY_URL=https://example.com/privacy
LEGAL_TERMS_URL=https://example.com/terms
LEGAL_PRIVACY_VERSION=1.0
LEGAL_TERMS_VERSION=1.0
```

---

## 4. 部署步骤（容器 / VM 通用）

1. **密钥进密钥托管**（勿写入 git）。从密码管理器注入 CI/CD 或 runtime env。  
2. **数据库**：创建空库 → 启动 API（自动 `sqlx migrate` 001–007）。  
3. **依赖**：Postgres 健康、Redis、Centrifugo、SRS/CDN origin 可达。  
4. **启动 API**，确认日志无 production guard 错误。  
5. **探活**：
   - `GET /health` → `status=ok`
   - `GET /ready` → `ready=true`（含 DB 时）
   - `GET /api/v1/meta` → 版本号
6. **CORS**：用浏览器从正式前端 origin 调一次 OTP send，确认无 CORS 失败。  
7. **OTP 实发**：对测试邮箱发码，确认邮件到达且日志 **无明文 code**。  
8. **媒体**：签发 publish → OBS 推流 → HLS 可播 → unpublish 后房间态正确。  
9. **管理端**：bootstrap 首个 admin（审计日志可查），试封禁 / 强关。  
10. **合规**：登录可见隐私/条款；`GET /me/export` 返回 JSON；`DELETE /me` 后不可再用 refresh。

### 冒烟（控制面，**勿**对生产开 mock topup）

Stage 可临时用沙箱支付/mock 时：

```bash
API_BASE=https://api.stage.example.com OTP_CODE=<real> ./scripts/dogfood-api-smoke.sh
```

注意：脚本默认 OTP `123456` 与 mock topup/pay — **仅适用于 dogfood**。生产/stage 冒烟应使用：

- 真实 OTP  
- 跳过 topup/pay mock：`DOGFOOD_STRICT=1 OTP_CODE=<real> API_BASE=… ./scripts/dogfood-api-smoke.sh`
- 可选归档：`DOGFOOD_REPORT_DIR=reports DOGFOOD_STRICT=1 OTP_CODE=<real> API_BASE=… ./scripts/dogfood-api-smoke.sh`

本地全绿路径：

```bash
./scripts/deploy-test.sh   # 含 api + media smoke；失败默认非致命
# 严格：DOGFOOD_SMOKE_REQUIRED=1 ./scripts/deploy-test.sh
```

---

## 5. 发布前检查清单（打印勾选）

### 工程

- [ ] `APP_ENV=production` 且进程启动无 guard 错误  
- [ ] Postgres 迁移已应用（含 `pay_*` 表）  
- [ ] JWT / Centrifugo / SRS 密钥均为非默认  
- [ ] `CORS_ALLOWED_ORIGINS` 仅正式域名  
- [ ] 日志采样无 token / stream key / OTP code  
- [ ] 备份与恢复演练至少 1 次（stage 可）  

### 产品闭环

- [ ] 注册/登录（真 OTP）  
- [ ] 开播 → 观看（HLS）→ 聊天 → 礼物（有币时）  
- [ ] 管理端强关 / 封禁 / 举报  
- [ ] 账号导出 / 删除  

### 支付

- [ ] 生产 **未** 启用 mock topup / mock pay  
- [ ] 真实 PayProvider 就绪 **或** 产品确认「暂不开放充值」并隐藏客户端入口  
- [ ] 账本对账任务 / 人工对账 SOP  

### 合规 / 商店

- [ ] 隐私政策 / 用户协议正式 URL  
- [ ] 年龄门槛 UI  
- [ ] 举报入口  
- [ ] App Store / Play 元数据与演示账号（P2+）  

---

## 6. 回滚

1. 流量切回上一版本镜像 / 二进制。  
2. **禁止** down-migration 除非有备份验证；账本表只追加。  
3. 若错误开启 mock：立即去掉 `ALLOW_*` / `PAY_*mock*` 并重启，审计期间 `wallet_ledger` topup 行。  

---

## 7. 相关路径

| 资源 | 路径 |
|---|---|
| 本地开播 | `docs/runbooks/go-live-local.md` |
| Stage env 模板 | `deploy/.env.stage.example` |
| 本地 env 示例 | `.env.example` |
| 支付设计 | `docs/architecture/payment-channels.md` |
| 合规闸门 | `docs/compliance/海外合规与上架闸门.md` |
| 测试 compose | `deploy/docker-compose.yml` + `deploy/.env.test` |
| API 冒烟 | `scripts/dogfood-api-smoke.sh` |
| V-BE-1 风险接受（未签） | `docs/runbooks/otp-dev-only-risk-accept.md` |
| V-BE-2 风险接受（未签） | `docs/runbooks/ws-1k-soak-risk-accept.md` |
| P1 状态 | `docs/product/p1-status.md` |
| 并行轨 Wave2 | `docs/product/p1-parallel-tracks.md` §5 V-BE-* |


### 发布开关速查（E12.5）

`FEATURE_PUBLIC_REGISTER|REAL_PAY|PK|COHOST|CLIENT_EVENTS` — 见 `crate features` 与 `.env.example`。事故时优先拨开关再回滚镜像。

**P1-safe / stage·prod 推荐默认（plan 06）：**

| 变量 | 推荐 | 说明 |
|---|---|---|
| `FEATURE_PUBLIC_REGISTER` | `1`（或配合 `INVITE_ONLY`） | 注册闸 |
| `FEATURE_REAL_PAY` | 按通道 readiness | 非 mock 建单；生产另禁 mock |
| `FEATURE_PK` | **`0`** | P3 experimental；**非 P1 退出**；unset 亦默认 off |
| `FEATURE_COHOST` | **`0`** | 同上 |
| `FEATURE_CLIENT_EVENTS` | `1` | P4 脚手架；可关 |

启用 `FEATURE_PK=1` / `FEATURE_COHOST=1` 仅用于 **P1 签字后** 的 P3 dogfood。详见 [p3-p4-experimental.md](../product/p3-p4-experimental.md)。

# P1 实现状态

最后更新：2026-07-23

范围见 [mvp-scope.md](./mvp-scope.md)。状态依据 `git log` 与当前代码树（API 路由、crate、客户端）。后端双存储细节以代码为准，本表汇总树内已落地能力。

---

## 已实现（含测试 / 功能提交）

| 领域 | 状态 | 说明 |
|---|---|---|
| Monorepo + docker-compose（PG/Redis/NATS/Centrifugo/SRS/MinIO） | 完成 | `deploy/docker-compose.yml` |
| OpenAPI 契约 + CI | 完成 | `contracts/`、GHA |
| Rust Axum health / meta / ready | 完成 | `/health`、`/ready`、`/api/v1/meta` |
| 邮箱 OTP + JWT access/refresh + `/me` + logout | 完成 | `feat(auth)` |
| `PATCH /me` 资料（昵称 + 年龄/隐私声明） | 完成 | 双存储 `AnyProfileExtras` + 迁移 `003` |
| 房间生命周期 + SRS MediaProvider 推流/播放 | 完成 | `feat(rooms)` |
| 钱包复式账本 + 礼物目录 + 幂等送礼 | 完成 | `GET /api/v1/wallet/ledger` |
| 礼物收款人校验 + 幂等键复用参数一致性 | 完成 | 相关 fix 提交 |
| 聊天历史 + Centrifugo 连接 token | 完成 | 实时 + PG 双存储 `AnyChat` |
| Centrifugo HTTP 推送（聊天/礼物，env 门控） | 完成 | `publisher_from_env` |
| 管理端 封禁 / 强关 / 审计 | 完成 | moderation crate |
| 管理端 禁言 / 解禁（拦聊天+礼物） | 完成 | `POST /api/v1/admin/mute\|unmute` |
| 管理端 礼物目录 + 举报队列处理 | 完成 | admin gifts/reports |
| 社交 关注 / 取关 + 关注列表 | 完成 | `feat(social)` |
| 热门 + 关注中的直播 Feed | 完成 | `feat(feed)` |
| 用户举报 API | 完成 | `POST /api/v1/reports` |
| Postgres 迁移 001–007 | 完成 | 含 OTP、充值幂等、pay_products/orders |
| Postgres 双存储（users/rooms/wallet/social/moderation/reports/chat/profile/**deleted**/**refresh**/**otp**） | 完成 | `USE_POSTGRES=1` |
| SRS on_publish / on_unpublish 回调 | 完成 | HMAC 签名推流 key；秘钥仅 header |
| 生产密钥守卫（OTP / JWT / SRS / 特性开关） | 完成 | `ALLOW_DEV_OTP`、`ALLOW_MOCK_TOPUP`、`OTP_NOTIFIER`、`SRS_WEBHOOK_SECRET` 等 |
| OTP 哈希落库 + 通知端口 + IP 限流 | 完成 | peppered SHA-256；`OtpNotifier`；发送/校验限流 |
| 管理权限 fail-closed + 可审计 bootstrap | 完成 | try_* 路径；原子 bootstrap |
| 合规桩：隐私/条款、导出、软删 | 完成 | API + Flutter |
| 聊天限流（5 条 / 10s） | 完成 | `ChatRateLimiter` |
| 仅直播中可送礼 + 公开有效礼物目录 | 完成 | `ROOM_NOT_LIVE` / active 过滤 |
| Flutter 登录 + 隐私条款 + 年龄门 | 完成 | 登录前 18+ 勾选 |
| Flutter 房间 / 礼物 / 资料 / Feed / 关注 / 举报 / 结束态 | 完成 | Discover + 房间控制面；**idle=暂时离线 / closed=永久结束**；owner 可 Start/Stop live |
| Flutter 开播 OBS 对话框 + 复制 HLS | 完成 | go-live 对话框；房间页复制流地址 |
| Flutter media_kit 站内 HLS 预览 | 完成 | `StreamPreview` 真嵌 media_kit（`ANYLIVE_EMBEDDED_PLAYER`）；进房后 idle→live 轮询补拉 HLS；`flutter test` 默认关 native |
| Flutter 结束态文案对齐 H5 | 完成 | idle → **Host offline**；closed/ended → **Stream ended**（`hls_player_logic`） |
| Flutter 会话持久化（SharedPreferences） | 完成 | `SessionStore` + 启动恢复 + Logout 清会话 |
| Flutter 资料导出 / 删除账号 UI | 完成 | Profile 页 export/delete；复制 payload |
| Flutter/H5 聊天历史轮询 | 完成 | 3s HTTP 历史轮询；H5 可选 Centrifugo JSON WS（`VITE_CENTRIFUGO_WS`） |
| H5 HLS 观看 + 分享深链 + 结束态 | 完成 | hls.js + share；**terminal vs offline**；idle 保持 status poll 可回 live |
| H5 Home 发现 + RoomWatch 分视图 | 完成 | 冷开 Home 卡片网格；`?room=` / 点选进 Watch；← Home 清状态；无 Vue Router |
| H5 可选登录 + 聊天 + 礼物 + 模拟充值 | 完成 | localStorage 会话；8s 状态轮询；terminal 关聊天；非 live 禁发/禁礼 |
| H5 Pay mock 币包 ref 修复 | 完成 | `payProducts`/`payBusy`/`payHint` 已声明 |
| OpenAPI pay/* + /metrics | 完成 | channels/products/orders/sandbox/webhooks + Prometheus 文本 |
| CI Flutter job | 完成 | `.github/workflows/ci.yml` mobile 矩阵 |
| 管理端深色运维台（侧栏模块） | 完成 | 登录 + 总览/房间/举报/礼物/处置/审计；非管理员 gate 文案 + `scripts/seed-admin-local.sh` |
| 生产 CORS 白名单 | 完成 | `APP_ENV=production` 时要求 `CORS_ALLOWED_ORIGINS` |
| 1k WS 压测（live Centrifugo） | 完成 | `ws-centrifugo-load.py`：1000/1000 连接、loss 0%、held_p50=180s（`reports/ws-1k-baseline-20260722T121825Z.md`）；15 min soak 仍运维 |
| Dogfood 控制面 cohort seed | 完成 | `dogfood-cohort-seed.sh`：20 hosts + 500 users（OTP 节流）；`reports/dogfood-cohort-20260722T115217Z.md`；真人 OBS/表单仍人工 |
| Docker 测试部署（API + Admin） | 完成 | `./scripts/deploy-test.sh` → API `:8088`，Admin `:8090` |
| 媒体 dogfood 冒烟自动化 | 完成 | `dogfood-media-smoke.sh` + `media_smoke_lib.py` |
| 本地开播栈（compose + dogfood 开关 + 手册） | 完成 | `./scripts/deploy-test.sh` + `docs/runbooks/go-live-local.md` |
| 签名推流 key 与 HLS 播放路径对齐 | 完成 | active stream 映射；stop/强关/unpublish 清除 |
| PayProvider 控制面（币包/订单/Mock webhook 入账） | 完成 | `anylive-pay`；`POST /pay/orders`；`pay:{order_id}` 幂等入账；生产禁 mock |
| 账号导出实质 payload | 完成 | 资料/房间/钱包流水/关注；截断 + 省略聊天与 stream key |
| OTP HTTP 投递（smtp/http） | 完成 | `HttpOtpNotifier`；生产禁 log/noop；需 URL |
| H5 Pay mock 收银台 | 完成 | 币包列表 + 建单 + sandbox-complete |
| Dogfood API smoke 含 pay/export/reconcile | 完成 | `dogfood-api-smoke.sh` 建单+入账+导出+admin 对账 |
| 账本对账任务 v0 | 完成 | `MemoryWallet`/`PostgresWallet`/`AnyWallet::reconcile`；`GET /api/v1/admin/wallet/reconcile`；OpenAPI + dogfood |
| Stage/生产上线 Runbook | 完成 | `docs/runbooks/go-live-stage.md` |
| Stripe + IAP 沙箱适配器（P2 提前落地） | 完成 | `PayChannel::{Stripe,Iap}`；HMAC sandbox + Stripe-Signature 路径；webhook 入账；dev 注册三渠道 |
| 支付超时关单 | 完成 | `PayStore::expire_stale_orders` + `POST /api/v1/admin/pay/expire-orders` |
| 软开邀请闸门 | 完成 | `INVITE_ONLY` + allowlist/codes；OTP verify `invite_code` |
| 聊天敏感词过滤 v1 | 完成 | `CHAT_BLOCKLIST` → `WordFilter`；`FORBIDDEN_POLICY` |
| LiveKit join token 签发 | 完成（**P3 实验**，非 P1 退出） | `POST /rooms/{id}/livekit/join`；env 门控；HS256 JWT；见 [p3-p4-experimental](./p3-p4-experimental.md) |
| 连麦邀请/接受/挂断控制面 | 完成（**P3 实验**，`FEATURE_COHOST` 默认 **OFF**） | interactive invite/respond/leave + list；Centrifugo fan-out；**非 P1 退出条件** |
| PK 状态机 + 礼物计分 | 完成（**P3 实验**，`FEATURE_PK` 默认 **OFF**） | pk/start|end|get；送礼 `pk.score`；内存 store；**非 P1 退出条件** |
| 客户端埋点批入 | 完成（**P4 脚手架**，非签字项） | `POST /api/v1/events` 内存环缓冲 |
| 热门 Feed 轻排序 v1 | 完成 | `GET /feed/hot` 按粉丝数 + 近 1h 新鲜度排序 |
| 创作者中心 stats | 完成 | `GET /me/creator` 粉丝/房间/礼物收入 |
| GA 功能开关 | 完成 | `FEATURE_*` 环境开关；**P1-safe：`FEATURE_PK`/`FEATURE_COHOST` 默认 OFF**；运行时 `GET /api/v1/meta.features` 已暴露；PK/连麦启用属 P3 后置 |
| Jeepay/EPay/TokenPay 沙箱适配器 | 完成 | `PayProvider` HMAC sandbox + 注册进 `PAY_CHANNELS` |
| Flutter 创作者中心 | 完成 | Profile `GET /me/creator` 卡片（粉丝/房间/礼物） |
| Flutter 埋点 SDK 最小集 | 完成 | `EventsRepository` + `room.view`/`gift.tap`/`chat.send`/`auth.login`/`pk.*`/`cohost.invite`/`pay.*` |
| Flutter 连麦/PK 控制面 UI | 完成（**P3 实验 UI**；meta 软隐藏） | 房间菜单：邀请/接受/拒绝连麦、PK 启停、LiveKit join 凭证、比分横幅；flag 关时隐藏/容忍 403 |
| Flutter 钱包 + 沙箱买币 | 完成 | `PayRepository` + `WalletPage`（Home → Wallet）；createOrder + sandbox-complete |
| H5 创作者/PK/埋点 helpers | 完成（PK 为 **P3 实验**） | `chatApi` path/parse/body + 单测 |
| H5 进房埋点 + PK 横幅 | 完成（横幅随 `features.pk` 软隐藏） | `room.view`/`gift.tap`/`chat.send`/`auth.login`/`pay.*` + PK score panel + Creator 面板 |
| Admin 资金运维动作 | 完成 | 总览：钱包对账 + 支付超时关单 + `/metrics` 抓取预览 |
| Admin 埋点缓冲汇总 | 完成 | `GET /api/v1/admin/analytics/summary` + 总览面板 |
| P2/P5 运维手册补齐 | 完成 | backup-restore / store-internal / slo-alerts / livekit-stage / dogfood-cohort |
| 压测与演练报告模板 | 完成 | `reports/*-TEMPLATE.md`（1k WS / 设备矩阵 / 备份 / 桌面推演） |
| 客户端事件字典 | 完成 | `docs/product/event-dictionary.md` |
| 举报 SLA 手册 | 完成 | `docs/runbooks/report-sla.md` |
| 礼物 100 TPS 压测脚手架 | 完成 | `scripts/loadtest/gift-tps-baseline.sh`（dry-run） |
| 房间在线人数 + 点赞 | 完成 | `presence`/`likes`/`stats`；Flutter AppBar；H5 stats 条；dogfood |
| 用户/房间搜索 | 完成 | `GET /api/v1/search`；Flutter Discover 搜索框 |
| 刷新会话列表 / 登出全端 | 完成 | `GET|DELETE /me/sessions`；Profile 按钮 |
| H5 可选 Centrifugo WS | 完成 | `connectCentrifugoChat` + `VITE_CENTRIFUGO_WS`；HTTP poll 回退 |
| Flutter 礼物简动画 | 完成 | 送礼成功 overlay（非 Rive 资产包） |
| 头像 MinIO/合成上传 + confirm | 完成 | `POST /me/avatar/presign|confirm`；`MINIO_ENABLED`；migration `008` |
| NATS gift.sent 可选发布 | 完成 | `NATS_URL` → TCP PUB；schema `anylive.gift.sent.v1`；送礼路径 fire-and-forget |
| 录制开关控制面 | 完成 | `GET|PUT /rooms/{id}/recording`；stats 含 `recording_enabled` |
| Flutter 可选 Centrifugo WS | 完成 | `CENTRIFUGO_WS` dart-define；HTTP poll 回退 |
| 单会话吊销 | 完成 | `DELETE /me/sessions/{jti}` + Flutter Profile 列表 Revoke |
| 推送 token 注册脚手架 | 完成 | `GET|POST|DELETE /me/push-tokens` + `POST .../test`；`PUSH_DELIVERY=noop|log|http`；真实 FCM/APNs 密钥待账号 |
| OAuth exchange 脚手架 | 完成 | `POST /auth/oauth/exchange`；本地 `stub:<email>`；生产禁 `OAUTH_STUB`；JWKS 待账号 |
| Admin 多角色 RBAC | 完成 | `admin|moderator|ops` + migration `010`；grant body `role` |
| H5 搜索房间/用户 | 完成 | `GET /search` + 搜索面板点选进房 |
| OpenAPI TS 生成脚手架 | 完成 | `scripts/gen-openapi-ts.sh` → admin/h5 `src/generated/openapi.d.ts`；CI 校验 |
| 结构化日志 / 观测脚手架 | 完成 | `init_tracing` + `RUST_LOG_FORMAT=json`；OTLP 导出运维侧 `docs/runbooks/otel.md` |
| Webhook/事件契约工件 | 完成 | `contracts/events/gift.sent.v1.json` + `contracts/webhooks/*`；校验脚本 |
| Flutter flavor + 路由常量 | 完成 | `APP_FLAVOR` dart-define + `AppRoutes`；非完整 Riverpod/go_router |
| 年龄声明 + 地区码 | 完成 | `age_confirmed` + `region`（ISO）；migration `009`；Flutter Profile |
| Cloudflare Stream MediaProvider 脚手架 | 完成 | `CloudflareStreamProvider` + `MEDIA_PROVIDER`；无 REST 副作用 |
| 商店打包元数据脚手架 | 完成 | `apps/mobile/store/` listing + flavor 构建说明 |

---

## 部分完成 / 桩（能用，但未达 dogfood 完结）

| 项 | 缺口 |
|---|---|
| Centrifugo 推送 / 客户端 WS | 服务端 publish + token 已接线；H5/Flutter 均可选 JSON WS（`VITE_CENTRIFUGO_WS` / `CENTRIFUGO_WS`）；HTTP poll 回退；全量 fan-out 仍需真实密钥/URL |
| 管理 UI | 深色运维壳可用；非完整 Vben 套件 |
| OTP 投递 | `OTP_NOTIFIER=http|smtp` + `OTP_HTTP_URL` 已接；生产禁 log/noop；本地可用 log/dev OTP；**缺真实 ESP 账号** |
| 充值 | Mock topup + Pay mock + **Stripe/IAP 沙箱**；真实 Stripe 密钥 / Store 收据校验待账号 |
| 观测 | `/metrics` + JSON 日志脚手架已接；**完整 OTLP/看板仍运维**（`docs/runbooks/otel.md`） |
| SRS 本地回调配置 | `deploy/srs/srs.conf` 已指向 API `:8088`（host.docker.internal） |

---

## 完整 P1 dogfood 出口仍待（偏运营/外部账号，非控制面缺口）

1. **真人 OBS 连续推流一周**（栈 + 冒烟 + 手册已就绪：`./scripts/deploy-test.sh`、`docs/runbooks/go-live-local.md`；控制面已绿；多端字节面仍靠人工）
2. **真邮件 OTP 提供商账号**（`OTP_NOTIFIER=http|smtp` + URL 已实现；待接真实 ESP/SMTP 桥；**未签字**风险接受草案：`docs/runbooks/otp-dev-only-risk-accept.md`）
3. **15 min 1k soak + 设备矩阵**（本地 1k×3min held_p50=180s / loss 0%：`reports/ws-1k-baseline-20260722T121825Z.md`；soak 状态/未关闸：`reports/ws-1k-soak-status-20260722.md`；**未签字**风险接受草案：`docs/runbooks/ws-1k-soak-risk-accept.md`；Mid Android 装机启动证据：`reports/device-matrix-20260722-android-23116PN5BC.md` — 非全路径冒烟；iOS/H5 未跑；模板 `reports/device-matrix-TEMPLATE.md`）
4. **完整 Vben 管理模块**（若超出当前运维台需求；可选）

说明：`dogfood-api-smoke.sh` 在 admin 已存在时会尝试 `DOGFOOD_ADMIN_EMAIL` 或 docker 种子 `admin_users`（本地 compose），不再因 bootstrap 403 硬失败。

(1) 手册：`docs/runbooks/go-live-local.md`、`scripts/dogfood-media.md`

---

## MVP 验收清单（摘自 mvp-scope）

### 功能

- [x] 注册/登录 OTP（开发）+ 浏览 Feed + 房间聊天/礼物（API/Flutter/H5）
- [x] H5 观看 + 分享 + 结束态
- [x] H5 可选登录 + 发聊天 + 送礼 + 模拟充值
- [x] 关注主播 + 举报房间（Flutter）
- [x] 管理端 封禁/禁言/强关/礼物/举报/预览
- [x] 同幂等键送礼不双扣（单测/API 覆盖；`dogfood-10min-path` 同 `client_request_id` 双发）
- [x] 禁言策略控制面断言（`dogfood-api-smoke` mute→chat/gift 403 → unmute 恢复；非真人运营演练）
- [x] 封禁策略控制面断言（`dogfood-api-smoke` 独立用户 ban→authed chat 403 + OTP re-login 403；P1 无 unban 路由，目标账号不复用）
- [x] 仅直播中可送礼
- [x] 主播 OBS 推流控制面 + 媒体冒烟 + 手册（**连续一周 dogfood 仍人工**）
- [x] 礼物目录预设 + 10 分钟控制面路径脚本（`dogfood-gift-seed` / `dogfood-10min-path`；不含真人 OBS）
- [x] 停播/强关结束态控制面（host stop + admin force-close；`dogfood-api-smoke` / `dogfood-10min-path`）

### 质量

- [ ] 无开放 P0（人工缺陷会）
- [x] 钱包/礼物自动化测试通过
- [x] 账本对账任务跑通（admin reconcile + 单测/集成/dogfood）
- [x] 1k WS 脚手架 + 报告路径（`scripts/loadtest/`）
- [x] 本地 1k Centrifugo 数字填入报告（1000 连接 / 0% loss / held_p50=180s；`reports/ws-1k-baseline-20260722T121825Z.md`）
- [x] 控制面 cohort seed 20h/500u（`reports/dogfood-cohort-20260722T115217Z.md`）
- [ ] 15 min 1k soak on stage（运维；本地 3min baseline + 状态：`reports/ws-1k-soak-status-20260722.md`）
- [ ] 中档 Android + 近几代 iPhone 冒烟（仅 Android 装机启动：`reports/device-matrix-20260722-android-23116PN5BC.md`；全路径/iOS/H5 仍开）

### 合规钩子

- [x] 登录可见隐私/条款
- [x] 登录 + 资料年龄声明
- [x] 举报 API + Flutter 房间举报
- [x] 账号删除/导出 API 实质 payload + 移动端 + 文档

---


## Stage-1 控制面 vs 出口签字（2026-07-23）

| 出口项 | 控制面 | 签字缺口 |
|---|---|---|
| 10min 路径 | `dogfood-10min-path` PASS | 真人看播/OBS |
| 幂等送礼 | 10min 双发 PASS | 保持绿即可 |
| 封禁/禁言 | smoke mute+ban PASS | 可选运营演练 |
| 停播/强关 | stop + force-close PASS | 抽检结束态 UX |
| 结束态 UX（idle vs closed） | Flutter/H5 区分「暂时离线」与「直播已结束」；host start/stop 按 owner | 真机/OBS 抽检 |
| FEATURE_PK/COHOST | meta 默认 false；P3 soft-skip | 保持 OFF |
| 合规四钩子 | API + Flutter + export smoke | 人眼走查 |
| 设备矩阵 | 仅装机启动历史 | 设备现离线；iOS/H5/全路径 |
| 15min soak / 无 P0 / 真 OTP / git 拆批 | 报告 + **未签字** risk-accept 草案在 | TL 签字 / 缺陷会 / ESP / 人工拆批 commit |

拆批草案：`docs/product/p1-git-split-plan.md`（A→B→C→F；禁止 monorepo 单 PR）。  
Risk-accept（未签字）：`docs/runbooks/otp-dev-only-risk-accept.md` · `docs/runbooks/ws-1k-soak-risk-accept.md`。

## 如何运行

```bash
# 完整本地开播栈（API :8088、Admin :8090、SRS，并跑 dogfood 冒烟）
./scripts/deploy-test.sh
# 手册：docs/runbooks/go-live-local.md
# 跳过冒烟：SKIP_DOGFOOD_SMOKE=1 ./scripts/deploy-test.sh

cp .env.example .env
# 内存模式 API dogfood 可不依赖 docker
docker compose -f deploy/docker-compose.yml up -d   # 可选依赖
docker compose -f deploy/docker-compose.yml up -d srs   # 仅 OBS/HLS 时

cd backend && cargo test --workspace
cargo run -p anylive-api   # :8088  开发 OTP = 123456

# 控制面 happy path（需 API 已在跑）
./scripts/dogfood-api-smoke.sh
# 媒体路径：scripts/dogfood-media.md
./scripts/dogfood-media-smoke.sh

# 可选 Postgres 双存储
USE_POSTGRES=1 DATABASE_URL=postgres://anylive:anylive@127.0.0.1:5432/anylive \
  cargo run -p anylive-api

cd apps/mobile && flutter test
cd apps/admin-web && pnpm test
cd apps/h5-web && pnpm test
python3 -m unittest discover -s scripts -p 'test_*.py'
```

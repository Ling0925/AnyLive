# P1 实现状态

最后更新：2026-07-22

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
| Flutter 房间 / 礼物 / 资料 / Feed / 关注 / 举报 / 结束态 | 完成 | Discover + 房间控制面 |
| Flutter 开播 OBS 对话框 + 复制 HLS | 完成 | go-live 对话框；房间页复制流地址 |
| Flutter 站内 HLS 预览脚手架 | 完成 | `StreamPreview`（media_kit 真播仍待接入） |
| H5 HLS 观看 + 分享深链 + 结束态 | 完成 | hls.js + share |
| H5 可选登录 + 聊天 + 礼物 + 模拟充值 | 完成 | localStorage 会话；8s 状态轮询；非 live 禁发 |
| 管理端深色运维台（侧栏模块） | 完成 | 登录 + 总览/房间/举报/礼物/处置/审计 |
| 生产 CORS 白名单 | 完成 | `APP_ENV=production` 时要求 `CORS_ALLOWED_ORIGINS` |
| 1k WS 压测脚手架（dry-run） | 完成 | `scripts/loadtest/`（完整 Centrifugo 填数仍靠人工） |
| Docker 测试部署（API + Admin） | 完成 | `./scripts/deploy-test.sh` → API `:8088`，Admin `:8090` |
| 媒体 dogfood 冒烟自动化 | 完成 | `dogfood-media-smoke.sh` + `media_smoke_lib.py` |
| 本地开播栈（compose + dogfood 开关 + 手册） | 完成 | `./scripts/deploy-test.sh` + `docs/runbooks/go-live-local.md` |
| 签名推流 key 与 HLS 播放路径对齐 | 完成 | active stream 映射；stop/强关/unpublish 清除 |
| PayProvider 控制面（币包/订单/Mock webhook 入账） | 完成 | `anylive-pay`；`POST /pay/orders`；`pay:{order_id}` 幂等入账；生产禁 mock |
| 账号导出实质 payload | 完成 | 资料/房间/钱包流水/关注；截断 + 省略聊天与 stream key |
| OTP HTTP 投递（smtp/http） | 完成 | `HttpOtpNotifier`；生产禁 log/noop；需 URL |
| H5 Pay mock 收银台 | 完成 | 币包列表 + 建单 + sandbox-complete |

---

## 部分完成 / 桩（能用，但未达 dogfood 完结）

| 项 | 缺口 |
|---|---|
| 账号导出 / 删除 | 软删双存储已有；`GET /me/export` 含资料/房间/钱包流水/关注 |
| Centrifugo 推送 | 已接线；需真实 URL/密钥才能线上 fan-out |
| 管理 UI | 深色运维壳可用；非完整 Vben 套件 |
| Flutter 播放器 | StreamPreview 已交付；media_kit / video_player 真嵌仍开放 |
| OTP 投递 | `OTP_NOTIFIER=http|smtp` + `OTP_HTTP_URL` 已接；生产禁 log/noop；本地可用 log/dev OTP |
| 充值 | Mock topup + Pay mock（H5 币包 + sandbox-complete）；真实 PSP 未接 |
| SRS 本地回调配置 | `deploy/srs/srs.conf` 已指向 API `:8088`（host.docker.internal） |

---

## 完整 P1 dogfood 出口仍待

1. **真人 OBS 连续推流一周**（栈 + 冒烟 + 手册已就绪：`./scripts/deploy-test.sh`、`docs/runbooks/go-live-local.md`；控制面已绿；多端字节面仍靠人工）
2. **真邮件 OTP 提供商账号**（`OTP_NOTIFIER=http|smtp` + URL 已实现；待接真实 ESP/SMTP 桥）
3. **Flutter media_kit / video_player 内嵌**（外开 URL 路径已可用）
4. **完整 Centrifugo 1k WS 填数报告 + 设备矩阵**
5. **完整 Vben 管理模块**（若超出当前运维台需求）

(1) 手册：`docs/runbooks/go-live-local.md`、`scripts/dogfood-media.md`  
(4) 脚手架：`./scripts/loadtest/ws-1k-baseline.sh`

---

## MVP 验收清单（摘自 mvp-scope）

### 功能

- [x] 注册/登录 OTP（开发）+ 浏览 Feed + 房间聊天/礼物（API/Flutter/H5）
- [x] H5 观看 + 分享 + 结束态
- [x] H5 可选登录 + 发聊天 + 送礼 + 模拟充值
- [x] 关注主播 + 举报房间（Flutter）
- [x] 管理端 封禁/禁言/强关/礼物/举报/预览
- [x] 同幂等键送礼不双扣（单测/API 覆盖）
- [x] 仅直播中可送礼
- [ ] 主播 OBS 推流 dogfood 一周 + 多端播放

### 质量

- [ ] 无开放 P0
- [x] 钱包/礼物自动化测试通过
- [x] 1k WS 脚手架 + 报告路径（`scripts/loadtest/`）
- [ ] 完整 1k Centrifugo 数字填入报告
- [ ] 中档 Android + 近几代 iPhone 冒烟

### 合规钩子

- [x] 登录可见隐私/条款
- [x] 登录 + 资料年龄声明
- [x] 举报 API + Flutter 房间举报
- [x] 账号删除/导出 API 桩 + 移动端 + 文档

---

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

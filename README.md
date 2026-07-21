# AnyLive

秀场娱乐直播产品 —— 海外优先，多端客户端 + Rust 业务后端 + Vue 运营后台 / H5。

## 当前进度（P0 + P1 核心控制面）

已落地并全部带单元/集成测试、按功能独立 git 提交。细节见 [P1 status](./docs/product/p1-status.md)。

| 能力 | 状态 |
|---|---|
| monorepo + docker-compose (PG/Redis/NATS/Centrifugo/SRS/MinIO) | ✅ |
| Rust Axum API health / meta / ready | ✅ |
| OpenAPI contracts + CI | ✅ |
| Flutter / Vue admin / H5 shells | ✅ |
| Email OTP + JWT access/refresh + `/me` (+ PATCH profile) | ✅ |
| Rooms + SRS MediaProvider publish/play | ✅ |
| Wallet ledger API + topup + idempotent gifts | ✅ |
| Chat history + Centrifugo token + env-gated publish | ✅ |
| Admin ban / mute / unmute / force-close / audit / gifts / reports | ✅ |
| Follow / unfollow + hot / following feeds | ✅ |
| Postgres dual store (users/rooms/wallet/social/moderation/reports) | ✅ |
| SRS on_publish / on_unpublish webhooks | ✅ |
| Compliance stubs (legal + export + soft-delete) | ✅ |
| Chat rate limit + live-only gifts | ✅ |
| H5 HLS watch + share + room-ended | ✅ |
| Admin ops shell (OTP, gifts, moderation, reports, HLS preview) | ✅ |
| Flutter feed/follow/report/profile + room control-plane | ✅ |

默认仍是 **内存后端**（`cargo test` 无需 PG）。设 `USE_POSTGRES=1` + `DATABASE_URL` 时 users/rooms/wallet/social/moderation/reports 切到 SQLx；chat 历史与 soft-delete/age-privacy 仍为进程内内存。

## 文档

| 文档 | 说明 |
|---|---|
| [技术评定与架构方案](./docs/技术评定与架构方案.md) | 架构 ADR |
| [产品与开发规划索引](./docs/product/README.md) | 全周期规划分册 |
| [MVP 范围](./docs/product/mvp-scope.md) | P1 验收 |
| [P1 实现状态](./docs/product/p1-status.md) | 已实现 / 剩余清单 |

## 本地开发

```bash
cp .env.example .env
docker compose -f deploy/docker-compose.yml up -d   # 可选依赖

cd backend && cargo test --workspace
cargo run -p anylive-api   # :8088  开发 OTP = 123456

# 客户端
cd apps/mobile && flutter test
cd apps/admin-web && pnpm test
cd apps/h5-web && pnpm test
```

### 关键 API（摘要）

- `POST /api/v1/auth/otp/send|verify` · `POST /api/v1/auth/token/refresh` · `GET|PATCH|DELETE /api/v1/me` · `GET /api/v1/me/export`
- `GET /api/v1/legal/privacy|terms`
- `POST/GET /api/v1/rooms` · `POST .../start|stop` · `.../media/publish|play`
- `GET /api/v1/wallet` · `GET /api/v1/wallet/ledger` · `POST /api/v1/wallet/topups` · `GET /api/v1/gifts` · `POST .../gifts`
- `POST /api/v1/realtime/token` · `POST/GET /api/v1/rooms/{id}/messages`
- `POST /api/v1/admin/grant|ban|mute|unmute|rooms/force-close` · `GET /api/v1/admin/audit`
- `GET|POST /api/v1/admin/gifts` · `GET /api/v1/admin/reports` · `PATCH .../reports/{id}`
- `POST/DELETE /api/v1/users/{id}/follow` · `GET /api/v1/me/following` · `GET /api/v1/feed/hot|following`
- `POST /api/v1/reports` · `POST /api/v1/webhooks/srs/on_publish|on_unpublish`

## 仓库结构

```
AnyLive/
├── apps/{mobile,admin-web,h5-web}
├── backend/crates/{api,auth,common,domain,db,media,wallet,realtime,moderation,social}
├── contracts/  deploy/  docs/  scripts/
```

## 技术栈

Rust Axum · Flutter · Vue3 · Centrifugo · SRS · Postgres/Redis/NATS（compose 已备）

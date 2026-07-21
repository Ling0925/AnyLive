# AnyLive

秀场娱乐直播产品 —— 海外优先，多端客户端 + Rust 业务后端 + Vue 运营后台 / H5。

## 当前进度（P0 + P1 核心控制面）

已落地并全部带单元/集成测试、按功能独立 git 提交：

| 能力 | 状态 |
|---|---|
| monorepo + docker-compose (PG/Redis/NATS/Centrifugo/SRS/MinIO) | ✅ |
| Rust Axum API health/meta | ✅ |
| OpenAPI contracts + CI | ✅ |
| Flutter / Vue admin / H5 shells | ✅ |
| Email OTP + JWT access/refresh + `/me` | ✅ |
| Rooms + SRS MediaProvider publish/play | ✅ |
| Wallet ledger + idempotent gifts | ✅ |
| Chat history + Centrifugo token | ✅ |
| Admin ban / force-close / audit | ✅ |
| Follow / unfollow | ✅ |

存储当前为 **内存实现**（可单测、可本地 dogfood）；Postgres/SQLx 与真实 Centrifugo 发布为后续提交。

## 文档

| 文档 | 说明 |
|---|---|
| [技术评定与架构方案](./docs/技术评定与架构方案.md) | 架构 ADR |
| [产品与开发规划索引](./docs/product/README.md) | 全周期规划分册 |
| [MVP 范围](./docs/product/mvp-scope.md) | P1 验收 |

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

- `POST /api/v1/auth/otp/send|verify` · `POST /api/v1/auth/token/refresh` · `GET /api/v1/me`
- `POST/GET /api/v1/rooms` · `POST .../start|stop` · `.../media/publish|play`
- `GET /api/v1/wallet` · `POST /api/v1/wallet/topups` · `GET /api/v1/gifts` · `POST .../gifts`
- `POST /api/v1/realtime/token` · `POST/GET /api/v1/rooms/{id}/messages`
- `POST /api/v1/admin/grant|ban|rooms/force-close` · `GET /api/v1/admin/audit`
- `POST/DELETE /api/v1/users/{id}/follow` · `GET /api/v1/me/following`

## 仓库结构

```
AnyLive/
├── apps/{mobile,admin-web,h5-web}
├── backend/crates/{api,auth,common,domain,db,media,wallet,realtime,moderation,social}
├── contracts/  deploy/  docs/  scripts/
```

## 技术栈

Rust Axum · Flutter · Vue3 · Centrifugo · SRS · Postgres/Redis/NATS（compose 已备）

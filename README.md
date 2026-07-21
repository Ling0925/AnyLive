# AnyLive

秀场娱乐直播产品 —— 海外优先，多端客户端 + Rust 业务后端 + Vue 运营后台 / H5。

## 文档导航

| 文档 | 说明 |
|---|---|
| [技术评定与架构方案](./docs/技术评定与架构方案.md) | 技术栈裁定与系统架构 ADR |
| [架构执行摘要](./docs/architecture/overview.md) | 一页架构总览 |
| [**产品与开发规划索引**](./docs/product/README.md) | **完整开发规划入口（分册）** |
| [MVP 范围](./docs/product/mvp-scope.md) | P1 内测功能与验收 |
| [海外合规与上架闸门](./docs/compliance/海外合规与上架闸门.md) | 商店 / 隐私 / 支付检查清单 |

## 仓库结构

```
AnyLive/
├── apps/mobile          # Flutter
├── apps/admin-web       # Vue admin shell
├── apps/h5-web          # Vue public watch shell
├── backend/             # Rust workspace (Axum)
├── contracts/           # OpenAPI + error codes
├── deploy/              # docker-compose (PG/Redis/NATS/Centrifugo/SRS/MinIO)
├── docs/
└── scripts/
```

## 本地开发（P0）

```bash
# 依赖
cp .env.example .env
docker compose -f deploy/docker-compose.yml up -d

# API
cd backend && cargo run -p anylive-api
# health: http://localhost:8088/health

# 测试
cd backend && cargo test --workspace
python3 scripts/validate-contracts.py
cd apps/admin-web && pnpm test
cd apps/h5-web && pnpm test
cd apps/mobile && flutter test
```

## 技术裁定（摘要）

| 域 | 选型 |
|---|---|
| 业务 API | Rust · Axum · SQLx · Postgres · Redis · NATS |
| 实时 IM | Centrifugo |
| 互动媒体 | LiveKit（P3） |
| 广播媒体 | SRS + CDN |
| 移动端 | Flutter |
| 公网 H5 / Admin | Vue3 |

当前进度：**P0 基建落地中**（monorepo、API health、契约 v0、三端壳）。

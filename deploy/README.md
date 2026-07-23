# 部署（本地 / 测试）

## 仅依赖服务

```bash
docker compose -f deploy/docker-compose.yml up -d
```

包含：Postgres、Redis、NATS、MinIO、Centrifugo、SRS。

## 完整测试栈（API + 管理后台）

```bash
./scripts/deploy-test.sh
```

或：

```bash
docker compose -f deploy/docker-compose.yml --env-file deploy/.env.test --profile app up -d --build
```

| 服务 | 地址 |
|---|---|
| API | http://localhost:8088/health |
| 管理后台 | http://localhost:8090/ |
| SRS HLS | http://localhost:8080/live |
| Centrifugo | http://localhost:8001 |

- 开发 OTP：**123456**（`APP_ENV=local` + `ALLOW_DEV_OTP`）
- API 使用 Postgres（`USE_POSTGRES=1`），启动时自动迁移
- 管理端构建时 `VITE_API_BASE=http://localhost:8088`（浏览器访问宿主机端口）
- Dogfood 开关见 `deploy/.env.test`：`ALLOW_MOCK_TOPUP`、`ALLOW_DEV_OTP`、`OTP_NOTIFIER`、`SRS_*`、`PAY_CHANNELS`/`PAY_MOCK_SECRET`、`FEATURE_PK=0`/`FEATURE_COHOST=0`

开播步骤见：`docs/runbooks/go-live-local.md`。  
Stage/生产清单见：`docs/runbooks/go-live-stage.md`。  
**Stage env 模板（可填、无 mock）：** `deploy/.env.stage.example` — 复制后填密钥；`FEATURE_PK`/`FEATURE_COHOST` 保持 `0`。

## Stage 拓扑排练（P2 · M2.1）

叠层 compose + 本地 env（密钥可自动铸造）：

```bash
# 无 ESP 时本地 OTP 排练（诚实标签：≠ 真邮件）
STAGE_LOCAL_ALLOW_DEV_OTP=1 ./scripts/stage-up.sh

# 有 OTP_HTTP_URL 时默认 ALLOW_DEV_OTP=0
./scripts/stage-up.sh
```

| 工件 | 路径 |
|---|---|
| compose 叠层 | `deploy/docker-compose.stage.yml` |
| 本地 env 例 | `deploy/.env.stage.local.example` → `deploy/.env.stage.local`（gitignore） |
| 远端 env 例 | `deploy/.env.stage.example` |
| metrics scrape | `deploy/prometheus/scrape-anylive.example.yml` |
| 备份 / 恢复演练 | `scripts/backup-pg.sh` · `scripts/restore-pg-drill.sh` |
| 状态表 | `docs/product/p2-status.md` |

`deploy-test.sh` 仍是 **dogfood 测试栈**（mock pay / dev OTP）。Stage-up 默认关 mock。

### 停止

```bash
docker compose -f deploy/docker-compose.yml --profile app down
# 保留数据卷：
docker compose -f deploy/docker-compose.yml --profile app down
# 清空数据库：
docker compose -f deploy/docker-compose.yml --profile app down -v
```

### 改代码后重建

```bash
docker compose -f deploy/docker-compose.yml --env-file deploy/.env.test --profile app up -d --build api admin
```

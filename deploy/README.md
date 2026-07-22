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
- Dogfood 开关见 `deploy/.env.test`：`ALLOW_MOCK_TOPUP`、`ALLOW_DEV_OTP`、`OTP_NOTIFIER`、`SRS_PUBLISH_SECRET`、`SRS_WEBHOOK_SECRET`

开播步骤见：`docs/runbooks/go-live-local.md`。

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

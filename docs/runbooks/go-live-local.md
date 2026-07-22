# 本地开播（Dogfood 测试栈）

端到端：主播推流 → SRS → HLS 观看。基于 Docker 测试栈。

## 1. 启动全栈

```bash
./scripts/deploy-test.sh
```

会拉起：Postgres、Redis、NATS、MinIO、Centrifugo、SRS、API（`:8088`）、管理后台（`:8090`）。

API 服务已注入 dogfood 开关（与 `deploy/.env.test` 一致）：

| 环境变量 | 测试默认 | 作用 |
|---|---|---|
| `ALLOW_DEV_OTP` | `1` | 固定验证码 `123456` |
| `ALLOW_MOCK_TOPUP` | `1` | 模拟充值（送礼用） |
| `OTP_NOTIFIER` | `noop` | 不发真实邮件/短信 |
| `SRS_PUBLISH_SECRET` | 测试值 | 推流 stream key 的 HMAC 密钥 |
| `SRS_WEBHOOK_SECRET` | 测试值 | SRS 回调鉴权（请求头） |

启动成功后会打印 OBS 说明，并自动跑：

- `./scripts/dogfood-api-smoke.sh`
- `./scripts/dogfood-media-smoke.sh`

跳过冒烟：`SKIP_DOGFOOD_SMOKE=1 ./scripts/deploy-test.sh`。

停止：

```bash
docker compose -f deploy/docker-compose.yml --profile app down
```

## 2. 管理后台初始化与网页开播

1. 打开 **http://localhost:8090/**
2. 邮箱 OTP 登录 — 验证码 **`123456`**（开发固定码）
3. 首次启动（尚无管理员）时，UI 会对当前用户调用 `POST /api/v1/admin/grant` 做 **bootstrap 授权**
4. 侧栏进入 **「开播」**：
   - 填写直播标题 → **一键开播**
   - 页面直接显示 **OBS 服务器**、**串流密钥**、完整推流 URL、观众 HLS
   - 一键复制，无需手调 API
5. 「直播间」列表中可点 **推流信息**，对已有房间重新签发并展示 OBS 凭证
6. 其它运营能力：强关、封禁/禁言、举报、礼物配置、审计

管理端打包时的 API 地址：`http://localhost:8088`（`VITE_API_BASE`）。

## 3. 各端 API 地址

| 客户端 | API 基址 |
|---|---|
| API 健康检查 | `http://localhost:8088/health` |
| Flutter 手机 | `API_BASE_URL` 默认 `http://localhost:8088`（见 `apps/mobile/lib/config/app_config.dart`） |
| H5 观看 | `VITE_API_BASE` 默认 `http://localhost:8088` |
| 管理后台 | `VITE_API_BASE` 默认 `http://localhost:8088` |

设备注意：

- iOS 模拟器 / 桌面浏览器：可用 `localhost:8088`
- Android 模拟器：用 `http://10.0.2.2:8088`（或本机局域网 IP）
- 真机：用电脑局域网 IP，例如 `http://192.168.x.x:8088`

## 4. 主播：建房 + OBS 推流

控制面（OTP `123456` 登录后拿 host token）：

1. `POST /api/v1/rooms` → 创建房间  
2. `POST /api/v1/rooms/{id}/start` → `status: live`  
3. `POST /api/v1/rooms/{id}/media/publish` → 复制凭证：

| 字段 | 用途 |
|---|---|
| `push_url` | 完整 RTMP 地址（含流名 + 查询串） |
| `stream_key` | **签名密钥** `{room_id}?exp={unix}&sig={hex}` — **不是**裸房间 UUID |
| `expires_at` | 密钥过期时间；过期后重新调 publish |

也可跑 `./scripts/dogfood-api-smoke.sh` 或 Flutter 开播流程，从输出/对话框复制。

### OBS 自定义 RTMP

| 字段 | 填写 |
|---|---|
| 服务 | 自定义… |
| 服务器 | `rtmp://localhost:1935/live` |
| 串流密钥 | media/publish 返回的 **完整** `stream_key`（含 `?exp=&sig=`） |

用裸房间 UUID 当串流密钥会被 API 的 `on_publish` 校验 **拒绝**。SRS 会把 stream 拆成裸 UUID + query 参数回调 API：

- `on_publish` / `on_unpublish` → `http://host.docker.internal:8088/api/v1/webhooks/srs/...?secret=...`
- 回调校验 HMAC，以及（若配置）`SRS_WEBHOOK_SECRET`

详见 `deploy/srs/srs.conf` 与 `scripts/dogfood-media.md`。

## 5. 观看 HLS

主播开播后，公开播放接口：

```http
GET /api/v1/rooms/{id}/media/play
→ { "hls": "http://localhost:8080/live/{room_id}.m3u8", "flv": "..." }
```

播放路径始终是 **裸房间 UUID**（与 RTMP stream name 一致；签名只在推流 query 里）。观众只需 HLS 地址，不需要 RTMP 密钥。

| 观众端 | 方式 |
|---|---|
| H5 | 启动 `apps/h5-web`，打开 `?room={room_id}`（如 `http://localhost:5173/?room=<uuid>`） |
| 直接 HLS | 把 `hls` 粘贴到 Safari / VLC / ffplay |
| Flutter | 房间页复制流地址到外部播放器（内嵌 media_kit 仍待接入） |

## 6. 快速检查清单

- [ ] `./scripts/deploy-test.sh` — API 健康、Admin 可访问  
- [ ] 管理后台 OTP `123456` + bootstrap 管理员  
- [ ] 主播开播 + media/publish → OBS 服务器 + **签名**串流密钥  
- [ ] 观众 H5 `?room=` 或 play 接口 HLS  
- [ ] 可选：停 OBS → webhook unpublish（或主播 `POST .../stop`）  

## 相关文档

- `scripts/dogfood-media.md` — 媒体面细节  
- `scripts/dogfood-api-smoke.sh` / `scripts/dogfood-media-smoke.sh`  
- `deploy/.env.test` — compose 环境默认值  

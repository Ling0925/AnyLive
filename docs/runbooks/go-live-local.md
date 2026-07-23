# 本地开播（Dogfood 测试栈）

> Stage / 生产上线清单见 [go-live-stage.md](./go-live-stage.md)。

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
| `PAY_CHANNELS` / `PAY_MOCK_SECRET` | mock / 测试值 | Mock 充值通道 + sandbox-complete |
| `FEATURE_PK` / `FEATURE_COHOST` | `0` | **P1-safe 默认关**；连麦/PK 属 P3 实验，见 [p3-p4-experimental](../product/p3-p4-experimental.md) |
| `CHAT_BLOCKLIST` | `spamword` | 聊天词过滤；`dogfood-api-smoke` soft-assert 403 |

启动成功后会打印 OBS 说明、一键重跑命令，并自动跑：

- `./scripts/dogfood-api-smoke.sh`（日志 tee → `reports/dogfood-api-smoke-<UTC>.log`）
- `./scripts/dogfood-media-smoke.sh`（同上 media）

跳过冒烟：`SKIP_DOGFOOD_SMOKE=1 ./scripts/deploy-test.sh`。  
冒烟失败默认 **不** 拆栈；若需严格：`DOGFOOD_SMOKE_REQUIRED=1 ./scripts/deploy-test.sh`。

**Stage 再冒烟（真 OTP + 跳过 mock）：**

```bash
DOGFOOD_STRICT=1 OTP_CODE=<real> API_BASE=https://api.stage.example \
  ./scripts/dogfood-api-smoke.sh
# 可选：DOGFOOD_REPORT_DIR=reports
```

Stage 模板：`deploy/.env.stage.example`。  
V-BE 风险接受（**未签**）：[otp-dev-only-risk-accept.md](./otp-dev-only-risk-accept.md) · [ws-1k-soak-risk-accept.md](./ws-1k-soak-risk-accept.md) — CI/脚本 **不得** 自动标 V-BE done。

## 1.1 运营预设与 10 分钟路径（控制面脚本）

全栈起来后，可用下面两条脚本补齐「礼物目录预设」与「主播+观众 10 分钟路径」（仅控制面，不含真实 OBS 推流）：

```bash
# 管理员登录 → upsert Rose/1 · Heart/10 · Rocket/100（固定 UUID 可重复跑）→ 打印 catalog
./scripts/dogfood-gift-seed.sh
# 已有管理员时：DOGFOOD_ADMIN_EMAIL=ops@example.com ./scripts/dogfood-gift-seed.sh

# 主播 OTP → 建房/开播/publish → 打印 OBS 字段 + HLS
# 观众 OTP → feed → 进房 → 聊天 → mock topup → 送礼（同 client_request_id 打两次，断言不双扣）
# → 可选 admin force-close
./scripts/dogfood-10min-path.sh

# 全量控制面冒烟（含 token refresh→/me；mute→chat/gift 403→unmute；ban→authed 403 + re-login 403；
# admin reports list+resolve；CHAT_BLOCKLIST soft-assert；P3 invite/PK 在 FEATURE_*=0 时 soft-skip）
./scripts/dogfood-api-smoke.sh
```

通过时 stdout 末行含 `DOGFOOD_API_SMOKE_PASS` / `DOGFOOD_10MIN_PATH_PASS`。
本地证据样例：`reports/dogfood-api-smoke-*.log`、`reports/dogfood-10min-path-*.log`。
CI 非阻塞 job：`dogfood-api-smoke`（memory API + smoke，`continue-on-error: true`）。

环境变量与 `dogfood-api-smoke.sh` 一致：`API_BASE`（默认 `http://localhost:8088`）、`OTP_CODE`（默认 `123456`）、`DOGFOOD_STRICT=1`（跳过 mock topup/pay）、`DOGFOOD_ADMIN_EMAIL`、`DOGFOOD_PG_CONTAINER`、`DOGFOOD_REPORT_DIR`（tee 日志）。force-close 可跳过：`SKIP_FORCE_CLOSE=1`（**OBS 周推荐**）。P3 误开时默认 FAIL：`ALLOW_P3_FEATURES=1` 才软放行。

人工 OBS / H5 / Flutter 路径仍按下文清单与 [dogfood-cohort.md](./dogfood-cohort.md) 操作。  
Wave2 **三端自测打包**（重部署 + APK + admin/h5 preview + dogfood 顺序 + 人工签字项）：[wave2-self-test.md](./wave2-self-test.md)。
`GET /api/v1/meta` 应返回 `features.pk=false` / `features.cohost=false`（测试栈 `deploy/.env.test`）；Flutter 房间页据此 soft-hide 连麦/PK 菜单。

停止：

```bash
docker compose -f deploy/docker-compose.yml --profile app down
```

## 2. 管理后台初始化与网页开播

1. 打开 **http://localhost:8090/**（源码热更新：`cd apps/admin-web && pnpm dev`，默认 Vite 端口）
2. 邮箱 OTP 登录 — 验证码 **`123456`**（开发固定码）
3. **管理员授权**
   - **首次**（`admin_users` 为空）：UI 对当前用户调用 `POST /api/v1/admin/grant` 做 bootstrap，成功后侧栏显示 `admin`
   - **已有管理员时**（常见于 dogfood 反复跑）：bootstrap 会 403，控制台顶部会提示「非管理员」并给出补救命令。任选其一：
     ```bash
     # 推荐：把任意邮箱登记为本地管理员（OTP 登录后即可强关/禁言）
     ./scripts/seed-admin-local.sh ops@example.com
     # 之后用 ops@example.com + 123456 登录后台
     ```
     或设置 `DOGFOOD_ADMIN_EMAIL` 指向已在 `admin_users` 中的邮箱；或已有管理员账号登录后 `POST /api/v1/admin/grant` 授权他人。
4. 侧栏进入 **「开播」**：
   - 填写直播标题 → **一键开播**
   - 页面直接显示 **OBS 服务器**、**串流密钥**、完整推流 URL、观众 HLS
   - 一键复制，无需手调 API
5. 「直播间」列表中可点 **推流信息**，对已有房间重新签发并展示 OBS 凭证
6. 其它运营能力：强关、封禁/禁言、举报、礼物配置、审计

**15 分钟运营走查（V-AD-1）：** 预检 + mvp-scope §4 逐步点击清单见 [admin-ops-15min-demo.md](./admin-ops-15min-demo.md)（人工签字才关闭 V-AD-1）。

管理端打包时的 API 地址：`http://localhost:8088`（`VITE_API_BASE`）。Docker 镜像 `anylive-admin` 需重建后才含源码改动：`docker compose -f deploy/docker-compose.yml --profile app build admin`。

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
| 服务器 | **从 `push_url` 推导**（本地常见 `rtmp://localhost:1935/live`；`dogfood-*-path/smoke` 会打印 paste-ready Server） |
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
| Flutter | 房间页 `StreamPreview` 可嵌 media_kit HLS（`ANYLIVE_EMBEDDED_PLAYER=true` dart-define；测试默认关）；也可复制流地址到外部播放器 |

## 6. 快速检查清单

- [ ] `./scripts/deploy-test.sh` — API 健康、Admin 可访问  
- [ ] 管理后台 OTP `123456` + bootstrap 管理员  
- [ ] 主播开播 + media/publish → OBS **Server（from push_url）** + **签名**串流密钥  
- [ ] 观众 H5 `?room=` 或 play 接口 HLS  
- [ ] 可选：停 OBS → webhook unpublish（或主播 `POST .../stop`）  
- [ ] **未**把控制面 PASS 写成 V-BE-1/2 已关闭  

## 相关文档

- `scripts/dogfood-media.md` — 媒体面细节  
- `scripts/dogfood-api-smoke.sh` / `scripts/dogfood-media-smoke.sh`  
- `scripts/dogfood-gift-seed.sh` / `scripts/dogfood-10min-path.sh` — 礼物预设 + 10 分钟控制面路径（§1.1）  
- `deploy/.env.test` — compose 环境默认值  
- `deploy/.env.stage.example` — stage 可填模板（mock/OTP 固定码 OFF）  
- [otp-dev-only-risk-accept.md](./otp-dev-only-risk-accept.md) · [ws-1k-soak-risk-accept.md](./ws-1k-soak-risk-accept.md) — **unsigned** V-BE drafts  

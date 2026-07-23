# 媒体 Dogfood：OBS → SRS → H5 / Flutter

控制面 URL 由 API 下发；媒体字节走 SRS（不经过 Rust 进程）。

> **控制面 PASS ≠ 出口签字。** `dogfood-*-smoke` / `dogfood-10min-path` 通过只证明 API 字段与房间态；不关闭 V-BE-1/2，也不等于 plan 06 #1/#2 真人路径已签。

## 1. 启动 SRS

```bash
docker compose -f deploy/docker-compose.yml up -d srs
# RTMP :1935  ·  HTTP 播放 :8080  ·  API :1985
```

API 内存模式 **不必** 起整套 compose —— 真推流/真播放时只需 SRS。

## 2. 主播：建房 + 开播，复制推流信息

```bash
cargo run -p anylive-api   # :8088，OTP 123456
# 或: ./scripts/dogfood-api-smoke.sh   # 打印 push_url / stream_key / hls / OBS Server
# 或: SKIP_FORCE_CLOSE=1 ./scripts/dogfood-10min-path.sh  # OBS 周：留下 live 房间
# 或: ./scripts/dogfood-media-smoke.sh # 健康检查 + publish/play 一致性 + 可选 SRS
```

主播流程：

1. OTP 登录为主播  
2. `POST /api/v1/rooms` → `POST /api/v1/rooms/{id}/start`  
3. `POST /api/v1/rooms/{id}/media/publish` → 复制：  
   - `push_url` — 完整 RTMP 地址（含流名 + 查询串）  
   - `stream_key` — **签名令牌** `{room_id}?exp={unix}&sig={hex}`（**不是**裸房间 UUID）  
   - **OBS Server** — 从 `push_url` 去掉末段 stream key 推导（`media_smoke_lib.obs_server_from_push_url`；**不要**写死 `localhost`，stage 会不同）

裸房间 UUID 会被 `on_publish` 拒绝。RTMP stream name 是裸 UUID（稳定 HLS），HMAC 在 query 里。

## 3. OBS 自定义 RTMP

| 字段 | 值 |
|---|---|
| 服务 | 自定义… |
| 服务器 | **从 `push_url` 推导**（本地常见 `rtmp://localhost:1935/live`；脚本会打印 paste-ready Server） |
| 串流密钥 | media/publish 返回的 **完整** `stream_key`（`{room}?exp=&sig=`） |

在 OBS 中开始推流。串流密钥整段粘贴（含 `?exp=&sig=`）。

脚本输出示例块：

```
---- OBS (custom RTMP) paste-ready ----
Server:      rtmp://localhost:1935/live
Stream Key:  <room>?exp=...&sig=...
HLS:         http://localhost:8080/live/<room>.m3u8
---------------------------------------
```

## 4. 播放 HLS（H5 或 Flutter）

观众 / 公开：

```http
GET /api/v1/rooms/{id}/media/play
→ { "hls": "http://localhost:8080/live/{room_id}.m3u8", "flv": "..." }
```

HLS/FLV 始终使用 **裸房间 id** 作为 stream name（与 SRS 默认 HLS 模板一致）。  
签名只用于推流鉴权，不进播放路径。

- **H5**：用房间 id 打开观看页；`hls.js`（或原生 HLS）拉流。  
- **Flutter**：房间页目前以控制面为主 — 复制 HLS 到外部播放器，或 Safari/VLC，直至接入 media_kit。

## 5. 断流自动停播

OBS 停止时，SRS 应回调 API（回调地址指向 API 主机）：

- `POST /api/v1/webhooks/srs/on_unpublish` → 房间离开 live，并清除 active stream 映射  
- `POST /api/v1/webhooks/srs/on_publish` → 可选准入/审计（必须签名 key）  

本地 `deploy/srs/srs.conf` 启用了 `http_hooks` →  
`http://host.docker.internal:8088/api/v1/webhooks/srs/on_publish|on_unpublish?secret=...`。  
Linux Docker 若无该主机别名，请改 conf 或为 `srs` 服务加  
`extra_hosts: ["host.docker.internal:host-gateway"]`。  
若回调打不到 API，请用 `POST .../stop` 或管理端强关房间。

## 6. 人工 OBS 周清单（每次 dogfood 结束可勾选）

- [ ] OBS → Custom → paste **Server**（from push_url）+ **full Stream Key**（`?exp=&sig=`）  
- [ ] Start Streaming → SRS 有流  
- [ ] H5 `?room=` 和/或 Flutter 房间页 HLS 可播  
- [ ] Stop OBS / unpublish → 房间非 live（webhook 或 host stop）  
- [ ] **未**把控制面 PASS 写成 V-BE-1/2 或 plan 06 #1/#2 已签  

OBS 周推荐：`SKIP_FORCE_CLOSE=1 ./scripts/dogfood-10min-path.sh`，让脚本留下 live 房间。

## 冒烟顺序

1. `cargo run -p anylive-api` 或 `./scripts/deploy-test.sh`  
2. `./scripts/dogfood-api-smoke.sh`（控制面：登录、礼物、聊天、Feed；打印 OBS 块）  
3. `./scripts/dogfood-media-smoke.sh`（媒体向：`/health`、可选 SRS `:1985`、OTP → 房间 → publish/play 一致性）  
4. Compose 起 `srs` + OBS 推流 + H5/Flutter 播放（媒体面字节）  

### 媒体冒烟组件

| 组件 | 路径 |
|---|---|
| 纯函数（解析 publish/play、签名 stream_key 形态、SRS 探测 URL） | `scripts/media_smoke_lib.py` |
| 单元测试 | `python3 -m unittest scripts/test_media_smoke_lib.py` |
| 在线冒烟脚本 | `./scripts/dogfood-media-smoke.sh` |

环境变量：

| 变量 | 默认 | 含义 |
|---|---|---|
| `API_BASE` | `http://localhost:8088` | 控制面 API |
| `OTP_CODE` | `123456` | 开发 OTP（与 dogfood-api-smoke 相同） |
| `SRS_API_BASE` | `http://127.0.0.1:1985` | 可选 SRS HTTP API |
| `SKIP_SRS` | `0` | 设为 `1` 跳过 SRS 探测 |
| `SKIP_FORCE_CLOSE` | `0` | `1` 时 10min 路径留下 live 房间（OBS 周） |
| `DOGFOOD_REPORT_DIR` | unset | 若设置则 tee 到该目录 `dogfood-*-<UTC>.log` |
| `ALLOW_P3_FEATURES` | `0` | 默认 dogfood 在 meta.pk/cohost 为 true 时 FAIL |

`dogfood-media-smoke` **不会** 推 RTMP，也 **不会** 等待 HLS 分片 — 它检查 publish/play 响应是否一致（签名 `stream_key` 形态 `room?exp=&sig=`、HLS 路径含房间 id、从 `push_url` 推导 OBS Server）。  
字节面验证仍靠 OBS → SRS → 播放器。  

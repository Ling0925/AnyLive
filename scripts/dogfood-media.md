# 媒体 Dogfood：OBS → SRS → H5 / Flutter

控制面 URL 由 API 下发；媒体字节走 SRS（不经过 Rust 进程）。

## 1. 启动 SRS

```bash
docker compose -f deploy/docker-compose.yml up -d srs
# RTMP :1935  ·  HTTP 播放 :8080  ·  API :1985
```

API 内存模式 **不必** 起整套 compose —— 真推流/真播放时只需 SRS。

## 2. 主播：建房 + 开播，复制推流信息

```bash
cargo run -p anylive-api   # :8088，OTP 123456
# 或: ./scripts/dogfood-api-smoke.sh   # 打印 push_url / stream_key / hls
# 或: ./scripts/dogfood-media-smoke.sh # 健康检查 + publish/play 一致性 + 可选 SRS
```

主播流程：

1. OTP 登录为主播  
2. `POST /api/v1/rooms` → `POST /api/v1/rooms/{id}/start`  
3. `POST /api/v1/rooms/{id}/media/publish` → 复制：  
   - `push_url` — 完整 RTMP 地址（含流名）  
   - `stream_key` — **签名令牌** `{room_id}_{exp}_{sig}`（**不是**裸房间 UUID）  

裸房间 UUID 会被 `on_publish` 拒绝。API 会记住已签发的 key，使 play 地址与 SRS 写出的 HLS/FLV 流名一致。

## 3. OBS 自定义 RTMP

| 字段 | 值 |
|---|---|
| 服务 | 自定义… |
| 服务器 | `rtmp://localhost:1935/live`（或从 `push_url` 去掉流名后的 host/app） |
| 串流密钥 | media/publish 返回的 **完整** `stream_key`（`{room}_{exp}_{sig}`） |

在 OBS 中开始推流。流名必须等于完整的签名 `stream_key`。

## 4. 播放 HLS（H5 或 Flutter）

观众 / 公开：

```http
GET /api/v1/rooms/{id}/media/play
→ { "hls": "http://localhost:8080/live/{stream_key}.m3u8", "flv": "..." }
```

在 publish 凭证有效期间，HLS/FLV 使用与 OBS 相同的 **签名流名**（SRS 写出 `{stream_key}.m3u8`）。  
房间 stop / unpublish / 强关后会清除映射，play 回退到裸房间 id。

- **H5**：用房间 id 打开观看页；`hls.js`（或原生 HLS）拉流。  
- **Flutter**：房间页目前以控制面为主 — 复制 HLS 到外部播放器，或 Safari/VLC，直至接入 media_kit。

## 5. 断流自动停播

OBS 停止时，SRS 应回调 API（回调地址指向 API 主机）：

- `POST /api/v1/webhooks/srs/on_unpublish` → 房间离开 live，并清除 active stream 映射  
- `POST /api/v1/webhooks/srs/on_publish` → 可选准入/审计（必须签名 key）  

本地 `deploy/srs/srs.conf` 启用了 `http_hooks` →  
`http://host.docker.internal:8088/api/v1/webhooks/srs/on_publish|on_unpublish`。  
Linux Docker 若无该主机别名，请改 conf 或为 `srs` 服务加  
`extra_hosts: ["host.docker.internal:host-gateway"]`。  
若回调打不到 API，请用 `POST .../stop` 或管理端强关房间。

## 冒烟顺序

1. `cargo run -p anylive-api`  
2. `./scripts/dogfood-api-smoke.sh`（控制面：登录、礼物、聊天、Feed）  
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

`dogfood-media-smoke` **不会** 推 RTMP，也 **不会** 等待 HLS 分片 — 它检查 publish/play 响应是否一致（签名 `stream_key` 形态、HLS 路径含 stream key/房间 id、从 `push_url` 推导 OBS Server）。  
字节面验证仍靠 OBS → SRS → 播放器。  

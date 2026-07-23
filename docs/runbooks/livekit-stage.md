# LiveKit stage 部署与 RTC 联调（P3）

## 控制面（已实现）

- `POST /api/v1/rooms/{id}/livekit/join` — 签发 HS256 兼容 join JWT  
- 环境变量：`LIVEKIT_URL`、`LIVEKIT_API_KEY`、`LIVEKIT_API_SECRET`（见 `.env.example`）  
- 未配置时 join 路由返回媒体未启用类错误；不影响 HLS 广播面  
- Flutter：房间菜单 **LiveKit join** 展示 url/room/token（凭证对话框，非真 WebRTC 订阅）

## Stage 集群（人工）

1. 部署 LiveKit Server（或 LiveKit Cloud project）。
2. 配置 API key/secret；WS URL 对客户端可达（wss）。
3. API 进程注入：
   ```bash
   export LIVEKIT_URL=wss://livekit.stage.example
   export LIVEKIT_API_KEY=...
   export LIVEKIT_API_SECRET=...
   ```
4. 验证：
   ```bash
   # 登录后
   curl -sS -X POST -H "Authorization: Bearer $TOKEN" \
     -H 'content-type: application/json' \
     -d '{"role":"viewer"}' \
     "$API_BASE/api/v1/rooms/$ROOM_ID/livekit/join"
   ```
5. 用官方客户端 / Flutter `livekit_client` 订阅（**真 RTC UI 仍为后续依赖包接入**）。

## 合流 / 转推

- 台上 WebRTC → 观众 HLS：使用 LiveKit egress / 独立转推 worker（不在 Axum 内做媒体）。
- 失败率门禁 <2%：需 stage 压测周 + egress 监控，本仓库仅文档化门禁。

## 不做清单（防止假实现）

- 不在 Rust 内嵌 WebRTC 栈  
- 不伪造「已连上 LiveKit 集群」状态  
- Flutter 真 pub/sub 需 `livekit_client` + 权限与设备矩阵


## 台上转推台下策略（WBS E10.5 · 控制面约定）

目标：连麦/PK 的 WebRTC 台上画面合成后，观众面仍走 **HLS**（SRS 或 Cloudflare）。

| 阶段 | 负责方 | 本仓库状态 |
|---|---|---|
| Join token | API `POST .../livekit/join` | ✅ |
| 真 RTC 订阅 | 客户端 `livekit_client` + 权限 | 待包接入 |
| Egress / 转推 worker | LiveKit egress → RTMP 回推 SRS/CDN | **集群运维** |
| 失败率门禁 <2% | 监控 + 压测周 | 运维数字 |

推荐路径（stage）：

1. Host + guest 进 LiveKit room（join token）。  
2. Egress 启动 composite → `rtmp://$SRS_RTMP_URL/$roomId?exp&sig`（复用控制面签发的 publish key）。  
3. 观众 `play_urls` 不变（HLS）。  
4. Egress 失败：API 不伪造 live 态；运营强关或主播停播。

本仓库 **不** 内嵌 egress 进程；仅约定接口与失败语义。

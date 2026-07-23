# 02 · WBS 与排期

关联：[01-阶段与里程碑](./01-阶段与里程碑.md) · [03-契约与接口冻结](./03-契约与接口冻结.md)

团队默认：**10 人并行**（TL、后端2–3、媒体1–2、Flutter2、Web1–2、QA1、SRE1、产品1）。

---

## 1. 史诗级 WBS（全周期）

### E0 工程底座
- E0.1 monorepo 目录与工具链（Rust workspace / Flutter melos / pnpm） ✅
- E0.2 docker-compose：Postgres、Redis、NATS、Centrifugo、SRS、MinIO ✅
- E0.3 CI：lint、test、build、镜像 ✅
- E0.4 配置与密钥规范、多环境 ✅（`.env.example` + production guards）
- E0.5 观测：OTel tracing、Prometheus metrics、结构化日志、health/ready ✅（`init_tracing` + `RUST_LOG_FORMAT=json` + `/metrics`；OTLP 导出见 `docs/runbooks/otel.md` 运维侧）

### E1 契约
- E1.1 错误码 `contracts/errors/codes.yaml` ✅
- E1.2 OpenAPI 分模块（见契约文档） ✅（`contracts/openapi/openapi.yaml`）
- E1.3 NATS 事件 schema ✅（`gift.sent` v1 + 可选 TCP publisher）
- E1.4 媒体/支付 Webhook 契约 ✅（`contracts/events/*.json` + `contracts/webhooks/*`；`validate-contracts.py` 校验）
- E1.5 Orval / Flutter 代码生成流水线 ✅（`scripts/gen-openapi-ts.sh` openapi-typescript；Orval 可后续替换）

### E2 账号与用户
- E2.1 注册登录（Email OTP / 可选 OAuth） ✅ — **Email OTP + `POST /auth/oauth/exchange` 脚手架**（stub:`email` 本地；真实 JWKS 待账号）
- E2.2 JWT access + refresh（可吊销） ✅
- E2.3 资料、头像（对象存储） ✅（presign/confirm + MinIO/合成）
- E2.4 设备会话列表、登出全端 ✅ — **`GET|DELETE /me/sessions` + `DELETE /me/sessions/{jti}` + Flutter 列表/Revoke**
- E2.5 年龄声明 / 地区 ✅（`age_confirmed` + `region` on `/me`；migration `009`；Flutter Profile）

### E3 房间与媒体控制面
- E3.1 房间 CRUD、状态机（idle/live/closed） ✅
- E3.2 MediaProvider 接口 + SRS/Cloudflare 实现 ✅ — **SRS 默认；`CloudflareStreamProvider` 控制面脚手架（`MEDIA_PROVIDER=cloudflare`）；真 CF 账号/Live Input 仍外部**
- E3.3 推流 key 签发与轮换；播放 URL ✅
- E3.4 开播/停播 webhook 校验 ✅（`/webhooks/srs/on_publish|on_unpublish` + secret；契约 JSON）
- E3.5 录制开关（可 P2） ✅（控制面 flag，无 egress）

### E4 实时
- E4.1 Centrifugo 部署与 JWT 连接 ✅ — **compose + JWT token API**；生产密钥/1k 实填数仍运维
- E4.2 频道模型：`room:{id}`、`user:{id}` ✅
- E4.3 聊天发送 API + 扇出 ✅（HTTP + Centrifugo publish）
- E4.4 在线人数、点赞聚合 ✅ — **`POST .../presence` + `POST .../likes` + `GET .../stats`；Flutter/H5 展示**
- E4.5 限流、敏感词钩子、禁言 ✅

### E5 礼物与钱包
- E5.1 礼物目录 ✅
- E5.2 钱包账户 + **只追加 ledger** ✅
- E5.3 送礼：幂等键 + 事务 + NATS `gift.sent` ✅（Centrifugo + 可选 NATS）
- E5.4 礼物动画事件（IM） ✅ — **Centrifugo gift envelope + Flutter 简动画 overlay**（非 Rive 全量）
- E5.5 充值订单 + PayProvider（Stripe test） ✅（mock/stripe/iap 沙箱；真实密钥待账号）
- E5.6 对账任务与管理端流水 ✅（`GET /admin/wallet/reconcile` + admin UI）

### E6 社交与首页
- E6.1 关注/取关 ✅
- E6.2 首页：热门 / 关注中直播 ✅
- E6.3 搜索用户/房间（简易） ✅ — **`GET /api/v1/search` + Flutter Discover 搜索框**

### E7 审核与运营后台
- E7.1 Admin 登录 + RBAC ✅ — **admin grant + bearer + `role`（admin|moderator|ops）**；migration `010`；细粒度 mute 可后续
- E7.2 用户封禁/禁言 ✅
- E7.3 房间强关 ✅
- E7.4 举报队列 ✅
- E7.5 礼物 CRUD、经济参数 ✅（admin gift upsert）
- E7.6 审计日志 ✅（`GET /admin/audit` + admin 审计面板）
- E7.7 直播预览（mpegts.js / HLS） ✅（admin HLS preview）

### E8 客户端 Flutter
- E8.1 工程：Riverpod、go_router、flavor ✅（`APP_FLAVOR` dart-define + `AppRoutes` 路径常量脚手架；完整 Riverpod/go_router 迁移可后续）
- E8.2 登录与资料 ✅
- E8.3 列表与房间页 ✅
- E8.4 播放器（media_kit / HLS） ✅（stream_preview + media_kit）
- E8.5 主播开播 ✅ — **RTMP/OBS 主路径（控制面 key/play）**；App 内推流非本阶段
- E8.6 聊天与弹幕层 ✅（HTTP + 可选 Centrifugo WS）
- E8.7 礼物面板与动画 ✅ — **面板 + 简动画 overlay**（非 Rive 资产包）
- E8.8 钱包与充值 UI ✅（Flutter wallet + mock/sandbox）
- E8.9 推送（P2） ✅ — **token 注册 + `PUSH_DELIVERY=noop|log|http` 投递端口 + `/me/push-tokens/test`**；真实 FCM/APNs 密钥待账号

### E9 H5
- E9.1 观看页 + 分享落地 ✅
- E9.2 登录态轻量（可选） ✅
- E9.3 hls.js 播放 ✅

### E10 媒体深度（P2–P3）
- E10.1 生产 CDN/Stream 配置 — **本地 SRS ✅；CF Stream URL 脚手架 ✅；生产 CDN 账号/证书运维**
- E10.2 LiveKit 集群 — **控制面 token ✅；`livekit-stage.md` ✅；真集群运维**
- E10.3 连麦信令与权限 ✅ — **API + Flutter Co-host 邀请 UI**
- E10.4 PK 玩法状态机与结算 ✅ — **API + Flutter 比分横幅/Start·End PK**
- E10.5 台上转推台下策略 — **join token ✅；egress→RTMP 回推约定见 `docs/runbooks/livekit-stage.md`；集群仍运维**

### E11 增长（P4）
- E11.1 埋点 SDK 与事件字典 ✅ — **`POST /api/v1/events` + 事件字典**
- E11.2 看板（DAU/开播/付费）— **`/metrics` + admin analytics summary ✅**；完整仓外看板待外部
- E11.3 轻推荐（规则 + 简单特征） ✅ — **`/feed/hot` 粉丝+新鲜度排序 v1**
- E11.4 创作者中心 ✅ — **`GET /me/creator` + Flutter/H5**

### E12 质量与发布
- E12.1 资金路径自动化测试 ✅（wallet/gift/pay 单测 + dogfood）
- E12.2 设备矩阵 — **模板 `reports/device-matrix-TEMPLATE.md` ✅；真机冒烟人工**
- E12.3 压测门禁 — **脚手架 + live `ws-centrifugo-load.py` ✅**；本地 1000 连接 100% / loss 0% / held_p50=180s（`reports/ws-1k-baseline-20260722T121825Z.md`）；**15 min soak + stage 集群仍运维**
- E12.4 商店打包与元数据 — **`apps/mobile/store/` 文案/身份脚手架 ✅；提包账号人工**
- E12.5 发布开关与回滚 ✅ — **`FEATURE_*` kill-switch + `go-live-stage.md` §6**

---

## 2. 前 4 周细排（P0 + P1 启动）

### Week 1 — 骨架与契约

| 轨道 | 任务 | 产出 |
|---|---|---|
| SRE | monorepo、compose、CI 模板 | PR 可合并流水线 |
| 后端 | Cargo workspace crates 空壳；utoipa 骨架 | `anylive-api` health |
| 后端 | OpenAPI：auth/user/room 草案 | `contracts/openapi/*.yaml` |
| 媒体 | SRS + Centrifugo 进 compose | 本地 RTMP 推拉通 |
| Flutter | 应用壳、路由、主题、环境切换 | 跑起来的空白 App |
| Web | Vben 初始化 + H5 Vite 壳 | 两个 dev server |
| TL/产品 | 错误码、房间状态机、用户故事地图 | 文档冻结会 |

**W1 出口：** 全员能 `docker compose up` + 调通 health；契约评审会召开。

### Week 2 — 登录与房间 + 媒体控制面

| 轨道 | 任务 |
|---|---|
| 后端 | Email/OTP 登录、JWT、用户资料 |
| 后端 | Room CRUD、状态、MediaProvider trait + SRS 实现 |
| 媒体 | 推流鉴权回调；播放 URL 约定 |
| Flutter | 登录页、房间列表、进房占位 |
| Admin | 登录 + 空的房间列表页（接 OpenAPI） |
| QA | 用例大纲：认证、房间 |
| SRE | stage 命名空间；镜像仓库 |

**W2 出口：** API 可登录建房；OBS 推到 SRS；文档中的 curl 示例可用。

### Week 3 — 可看 + 契约消费

| 轨道 | 任务 |
|---|---|
| Flutter | media_kit/HLS 播放；房间页布局 |
| H5 | 观看页播放 |
| 后端 | 开播/停播 API 与 webhook 落库 |
| 媒体 | 转码/多码率策略草案（可先单码率） |
| Web Admin | 房间列表真实数据 |
| 全员 | OpenAPI 生成客户端接入 CI |

**W3 出口：** 手机/OBS 推流，Flutter + H5 能看。

### Week 4 — 实时进房

| 轨道 | 任务 |
|---|---|
| 媒体/后端 | Centrifugo JWT；发布封装 `realtime` crate |
| 后端 | 聊天 API、入房鉴权、基础限流 |
| Flutter | IM 连接、消息列表、发送 |
| H5 | 只读弹幕（可选） |
| QA | 双端聊天延迟抽样 |
| 产品 | 礼物经济数值草案 |

**W4 出口：** 双端同房聊天 <300ms（同区域）；P1 进入礼物设计冻结。

---

## 3. M2–M3 月排期（完成 P1）

### Month 2（约 W5–W8）

| 周 | 焦点 |
|---|---|
| W5 | 礼物目录 API；钱包 schema 与 ledger；Admin 礼物 CRUD |
| W6 | 送礼事务 + NATS + IM 动画事件；Flutter 礼物面板 |
| W7 | 关注/首页；敏感词与禁言；举报提交 |
| W8 | Admin 封禁/强关/举报队列；审计日志；对账任务 v0 |

**M2 出口：** 内测可打赏；运营可处置。

### Month 3（约 W9–W12）

| 周 | 焦点 |
|---|---|
| W9 | Stripe test 充值；钱包 UI；幂等与并发单测加强 |
| W10 | 性能：聊天 1k CCU 压测；礼物扇出压测；修复 |
| W11 | Dogfood 预热；崩溃与 ANR；隐私钩子（导出/删除桩） |
| W12 | **P1 退出周**：缺陷清零、验收演示、软上线计划评审 |

---

## 4. M4–M5（P2 软上线）

| 周 | 焦点 |
|---|---|
| W13–W14 | 生产拓扑、CDN/Stream 生产、备份、密钥 |
| W15–W16 | 商店内测包、元数据、年龄分级材料 |
| W17–W18 | 支付沙箱加固、防刷、限流、邀请码 |
| W19–W20 | 封闭公测运营、SOP、P2 退出评审 |

---

## 5. M6–M7（P3 连麦/PK）

| 周 | 焦点 |
|---|---|
| W21–W22 | LiveKit 部署；token 签发；Flutter 发布/订阅 |
| W23–W24 | 连麦邀请/接受/挂断；权限与麦位 |
| W25–W26 | PK 状态机、计时、礼物计入 PK 分、结果展示 |
| W27–W28 | 转推台下、弱网策略、失败率达标、P3 退出 |

---

## 6. M7–M9（P4 + P5 GA）

| 周 | 焦点 |
|---|---|
| W24–W28 | 埋点与看板（与 P3 并行） |
| W29–W30 | 轻推荐、创作者中心、运营配置 |
| W31–W32 | 容量压测、混沌/故障演练、on-call |
| W33–W36 | 商店公开提交、支付生产切流、GA 开关、发布 |

> 日期可按商店审核排队整体平移 ±2–4 周；缓冲已含在 M8–M9。

---

## 7. 轨道负荷示意（P1 期间）

| 轨道 | W1–4 | W5–8 | W9–12 |
|---|---|---|---|
| 后端 | 契约/认证/房间 | 礼物/钱包/社交 | 支付/硬化 |
| 媒体 | SRS/Centrifugo | 稳定与观测 | 压测配合 |
| Flutter | 壳/登录/播 | 礼物/社交 | 打磨 dogfood |
| Web | Admin/H5 壳 | Admin 运营 | H5 分享打磨 |
| QA | 大纲 | 资金用例 | 回归+压测 |
| SRE | CI/compose | stage | 观测与备份 |

---

## 8. 任务粒度约定

- 史诗（E*）在项目管理工具建 Epic
- 故事 ≤ 3–5 人日；超则拆
- 契约变更走 RFC PR；破坏性变更必须 bump 版本并通知三端
- 每周三契约例会 30min；每周五 Demo 30min

---

→ 下一篇：[03-契约与接口冻结](./03-契约与接口冻结.md)

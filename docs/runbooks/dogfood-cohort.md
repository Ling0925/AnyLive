# Dogfood 队列记录（P1 · 20 主播 / 500 用户）

运营填表用。控制面与自动化冒烟见 `scripts/dogfood-api-smoke.sh`、`docs/runbooks/go-live-local.md`。

## 运营预设脚本（控制面）

| 脚本 | 用途 |
|---|---|
| `./scripts/dogfood-gift-seed.sh` | Admin upsert 礼物目录（Rose/Heart/Rocket），打印 public catalog |
| `./scripts/dogfood-10min-path.sh` | 主播开播 + 观众聊天/送礼（含幂等双发）+ 可选 force-close |
| `./scripts/dogfood-api-smoke.sh` | 更全的 API 冒烟（含 pay sandbox / export / PK 等） |
| `./scripts/dogfood-cohort-seed.sh` | 合成 20 主播 / 500 用户（不替代真人 OBS 周） |

共用 env：`API_BASE`、`OTP_CODE`、`DOGFOOD_ADMIN_EMAIL`。详见 [go-live-local.md §1.1](./go-live-local.md)。

## 启动前预检（控制面 · 不替代真人 OBS）

在填主播表 / 邀约真人之前，建议本地或 test 栈先绿：

| # | 检查 | 命令 / 证据 |
|---|---|---|
| 1 | 栈健康 | `./scripts/deploy-test.sh` 或 API `/health` + Admin 可开 |
| 2 | 礼物目录 | `./scripts/dogfood-gift-seed.sh` |
| 3 | 10 分钟控制面路径 | `./scripts/dogfood-10min-path.sh` → PASS |
| 4 | 全量 API smoke（含 mute/ban/pay/export） | `./scripts/dogfood-api-smoke.sh` → PASS |
| 5 | P3 开关默认关 | `GET /api/v1/meta` → `features.pk` / `features.cohost` **false** |
| 6 | Admin 账号 | `scripts/seed-admin-local.sh` 或 `DOGFOOD_ADMIN_EMAIL` 已是 admin |
| 7 | OTP 风险 | 若仍用 dev 固定码：确认范围并参见 [otp-dev-only-risk-accept.md](./otp-dev-only-risk-accept.md)（**未签字不算关闭 V-BE-1 / 出口 #9**） |
| 8 | WS 容量 | 本地 1k baseline 或 [ws-1k-soak-risk-accept.md](./ws-1k-soak-risk-accept.md)（**未签字不算关闭 V-BE-2**） |
| 9 | 合成 cohort（可选） | `./scripts/dogfood-cohort-seed.sh` — **不**替代 20 真人主播周 |
| 10 | Admin 15min 演示（V-AD-1） | [admin-ops-15min-demo.md](./admin-ops-15min-demo.md) 走查 + **人工签字**（勿在 p1-parallel-tracks 自动标 done） |

**明确仍人工：** 连续 OBS ≥7 天、多端真机播放、缺陷会无 P0、设备矩阵签字、V-AD-1 演示签字。

## V-FL-2 — 10 分钟真人路径（操作者清单）

> 对应 [p1-parallel-tracks.md](../product/p1-parallel-tracks.md) **V-FL-2**。  
> 控制面脚本绿 **≠** V-FL-2 完成；必须有**真人操作 + 录屏/录像 URL** 归档。

### 前置（控制面）

| # | 检查 | 通过标准 |
|---|---|---|
| 1 | 栈可用 | API `/health`；本地可用 `./scripts/deploy-test.sh` |
| 2 | 礼物目录（建议） | `./scripts/dogfood-gift-seed.sh` |
| 3 | **10 分钟控制面路径** | `./scripts/dogfood-10min-path.sh` → stdout 含 **`DOGFOOD_10MIN_PATH_PASS`**（或证据日志 `reports/dogfood-10min-path-*.log`） |
| 4 | 可选全量 smoke | `./scripts/dogfood-api-smoke.sh` → PASS（不替代本清单） |

控制面 PASS **先于** 真人路径；若 10min-path 失败，先修 API/环境，再录屏。

### 真人路径（Flutter 与/或 H5）

操作者在客户端按顺序走通并**录像**（屏幕录制即可）：

1. **Login** — OTP 登录（本地 dev 码见 go-live-local）  
2. **Feed** — 热门/关注列表可见，点进直播间  
3. **HLS** — 房间内播放或明确降级文案（有可播 URL 时须能播）  
4. **Chat** — 发送至少 1 条消息并可见  
5. **Gift** — 测试币路径送礼成功（非 live 应被拒）  
6. **End-state** — 关播/强关后房间结束态可理解（文案或状态）

安装与 `API_BASE_URL`：`apps/mobile/README.md`、`apps/mobile/store/README.md`。  
H5：`apps/h5-web/README.md`（`VITE_API_BASE`、`pnpm build` / `preview`）。

### 录屏与报告路径

| 项 | 要求 |
|---|---|
| 录屏 / 录像 URL | **必填**（网盘、Issue 附件、内网对象存储等可点击链接） |
| 建议报告文件 | `reports/dogfood-vfl2-YYYYMMDD.md`（可自建）或本页下方「V-FL-2 记录」表 |
| 控制面证据 | 附 `reports/dogfood-10min-path-*.log` 路径或同次运行时间戳 |
| 设备说明 | 机型 / OS / 构建（debug APK / H5 preview URL） |

**禁止：** 无 recording URL 时把 V-FL-2 标为 done；禁止用仅控制面 `DOGFOOD_10MIN_PATH_PASS` 顶替真人路径。

### V-FL-2 记录（复制填）

| 字段 | 值 |
|---|---|
| 日期 | |
| 操作者 | |
| 控制面 10min-path | PASS / FAIL · 日志：`reports/dogfood-10min-path-________.log` |
| 客户端 | Flutter Android / iOS / H5 Safari / H5 Chrome |
| 路径勾选 | Login□ Feed□ HLS□ Chat□ Gift□ End-state□ |
| **Recording URL** | （必填，否则不得勾 V-FL-2） |
| 备注 / 缺陷 | |

勾选里程碑或看板 **V-FL-2 = done** 时，本表（或等价纪要）须含 **非空 Recording URL**。

### 与 V-FL-1 的关系

| ID | 内容 | 证据 |
|---|---|---|
| **V-FL-1** | 设备矩阵多行（Mid Android + iPhone + H5） | `reports/device-matrix-*.md`；prefill：`./scripts/device-matrix-prefill.sh`；**Pass 不自动勾** |
| **V-FL-2** | 单次完整 10 分钟真人路径 + 录屏 | 上表 + **Recording URL** |

## 目标

| 指标 | 目标 |
|---|---|
| 活跃主播 | ≥ 20（完成至少 1 次有效开播） |
| 注册用户 | ≥ 500（含测试号需标注） |
| 账本 | reconcile balanced |
| 推流 | 至少 1 名主播连续 OBS ≥ 7 天（可轮值） |

## 主播表

| # | 昵称/ID | 首次开播日 | OBS/推流 | 多端播放确认 | 备注 |
|---|---|---|---|---|---|
| 1 |  |  |  |  |  |
| … |  |  |  |  |  |
| 20 |  |  |  |  |  |

## 用户增长检查点

| 日期 | 累计用户 | 日活约 | 礼物订单 | 事故 |
|---|---|---|---|---|
|  |  |  |  |  |

## 签字

- 产品：  
- 工程：  
- 日期：  

勾选 `01-阶段与里程碑` P1「20 主播 / 500 用户」时，本表或等价运营表需归档。

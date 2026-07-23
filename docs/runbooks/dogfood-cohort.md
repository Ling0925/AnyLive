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
| 7 | OTP 风险 | 若仍用 dev 固定码：确认范围并参见 [otp-dev-only-risk-accept.md](./otp-dev-only-risk-accept.md)（**未签字不算关闭出口**） |
| 8 | 合成 cohort（可选） | `./scripts/dogfood-cohort-seed.sh` — **不**替代 20 真人主播周 |

**明确仍人工：** 连续 OBS ≥7 天、多端真机播放、缺陷会无 P0、设备矩阵签字。

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

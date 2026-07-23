# P2 实现状态（Soft Launch 入口）

最后更新：2026-07-23

对照：[01-阶段与里程碑](./01-阶段与里程碑.md) · [06 §8.4](./06-P1进度评审与后续规划.md) · [solo-owner-mode](./solo-owner-mode.md) · [go-live-stage](../runbooks/go-live-stage.md)

**运作：** solo owner — 完整实现优先；无签字/矩阵 Pass 关闸。诚实标签：本地 stage-up ≠ 云 stage ≠ 商店已过审。

**范围纪律：** `FEATURE_PK` / `FEATURE_COHOST` 默认 **OFF**（P3）；不把连麦/PK 当 soft-launch 成功标准。

---

## 子里程碑看板

| 里程碑 | 主题 | 控制面 / 工程 | Solo 剩余 |
|---|---|---|---|
| **M2.1** | 生产拓扑 | stage compose 叠层、env 模板、备份/恢复脚本、metrics scrape 样例 | 云主机/密钥托管账号；备份演练你本地可跑 |
| **M2.2** | 商店内测 | `apps/mobile/store/` + [store-internal](../runbooks/store-internal.md) | Apple/Google 账号与提包 |
| **M2.3** | 支付就绪 | Stripe/IAP **沙箱适配器**已在仓库；对账 API 绿 | 真 Stripe/IAP 密钥与收据校验账号 |
| **M2.4** | 软开量 | `INVITE_ONLY` + allowlist/codes 已实现 | 运营发码；防刷阈值按流量调 |

---

## M2.1 已落地（本批）

| 项 | 路径 | 说明 |
|---|---|---|
| Stage compose 叠层 | `deploy/docker-compose.stage.yml` | 叠在 base compose 上；`APP_ENV=staging`；默认关 mock pay/topup |
| 远端 stage env 模板 | `deploy/.env.stage.example` | 填密钥用；无 mock 默认 |
| 本地 stage 排练 env | `deploy/.env.stage.local.example` → `.env.stage.local`（gitignore） | `stage-up.sh` 可铸造密钥 |
| 一键 stage-up | `scripts/stage-up.sh` | build/up + health；可选 `STAGE_LOCAL_ALLOW_DEV_OTP=1` 本地 OTP 排练 |
| PG 备份 | `scripts/backup-pg.sh` | custom-format dump → `reports/` |
| 隔离恢复演练 | `scripts/restore-pg-drill.sh` | 新建 `anylive_drill_*`，**不**覆盖 live DB；写 `reports/backup-restore-*.md` |
| Prometheus 抓取样例 | `deploy/prometheus/scrape-anylive.example.yml` | `/metrics` 15s；非完整 OTLP |
| 观测手册 | [otel.md](../runbooks/otel.md) · [slo-alerts.md](../runbooks/slo-alerts.md) | 已有 |

### 本地怎么跑 M2.1

```bash
# 拓扑排练（推荐先本地 OTP，避免无 ESP 起不来）
STAGE_LOCAL_ALLOW_DEV_OTP=1 ./scripts/stage-up.sh

# 备份 + 隔离恢复
./scripts/backup-pg.sh
./scripts/restore-pg-drill.sh reports/pg-backup-*.dump

# 有真 OTP 时
# DOGFOOD_STRICT=1 OTP_CODE=<real> API_BASE=http://127.0.0.1:8088 ./scripts/dogfood-api-smoke.sh
```

Dogfood 测试栈（dev OTP / mock pay）仍用：`./scripts/deploy-test.sh` + `deploy/.env.test`。

---

## 后续批次（实现顺序建议）

1. **M2.1 收口** — 你本地跑通 stage-up + backup-restore 一次；需要时再上云 compose/K8s。  
2. **M2.3 加固** — Stripe test 密钥进 env；sandbox 建单→webhook→对账一条龙自测（无密钥则保持适配器绿）。  
3. **M2.4 运营** — `INVITE_ONLY=1` + codes 列表写进 stage env；Flutter/H5 登录可选 invite 字段 UX 若缺再补。  
4. **M2.2 商店** — 账号就绪后按 store-internal 提包；工程侧不挡。  
5. **观测** — 把 scrape 样例接进你自己的 Prometheus/Alloy；告警用 slo-alerts。

**明确后置：** P3 真 WebRTC 连麦/PK 上线；P4 事件仓产品化；公开商店（P5）。

---

## 与 P1 的边界

| P1（已 dogfood 收口） | P2（本阶段） |
|---|---|
| 本地 mock/dev OTP 可测主路径 | stage 默认关 mock；OTP 走 http/smtp 或诚实本地排练标签 |
| `deploy-test` + dogfood smoke | `stage-up` + strict smoke / 备份演练 |
| 功能面冻结 + PK/COHOST OFF | 拓扑/支付沙箱/邀请/商店内测；仍 OFF PK/COHOST |

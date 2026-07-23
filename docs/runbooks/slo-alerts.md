# SLO 与告警脚手架（P5）

## 1. 控制面 SLI（来自 `/metrics` + 日志）

| SLI | 目标（初值） | 信号源 |
|---|---|---|
| API 可用性 | 99.9% / 30d | 合成探测 `/health` + 网关 5xx |
| 关键 P95 | ≤ 300ms（读） | 直方图 / APM |
| 钱包对账 | imbalance=0 | 定时 `GET /admin/wallet/reconcile` |
| 支付超时关单 | 每日成功 | 定时 `POST /admin/pay/expire-orders` |
| 媒体回调 | on_publish 失败率 <1% | SRS webhook 日志 |

## 2. 合成探测（建议 cron）

```bash
# 每分钟
curl -fsS "$API_BASE/health" >/dev/null
curl -fsS "$API_BASE/api/v1/meta" >/dev/null
curl -fsS "$API_BASE/metrics" | head -1 >/dev/null
```

## 3. 告警分级

| 级别 | 条件 | 响应 |
|---|---|---|
| P0 | 全站 5xx >5% 5min / 支付入账中断 | 立即 on-call |
| P1 | 对账 imbalance>0 / 强关失败 | 15 min |
| P2 | P95 超阈 30 min | 工作时间 |

## 4. 事故桌面推演议程（90 min）

1. 场景：Postgres 主库不可用 / 支付 webhook 风暴 / 直播全挂  
2. 角色：TL、后端、SRE、产品  
3. 输出：决策树、沟通模板、回滚开关（`FEATURE_*`）  
4. 归档：`reports/incident-tabletop-YYYYMMDD.md`

## 5. 与代码衔接

- Prometheus 文本：`GET /metrics`（Admin 总览可抓取预览）
- Feature kill-switch：`FEATURE_PUBLIC_REGISTER|REAL_PAY|PK|COHOST|CLIENT_EVENTS`
- Stage 发布：`docs/runbooks/go-live-stage.md`

_状态：脚手架文档；完整看板与寻呼集成仍属外部运维。_

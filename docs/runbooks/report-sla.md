# 举报 SLA 流程（P2 运营）

状态：控制面已就绪（`POST /api/v1/reports`、Admin 举报队列 resolve、审计日志）。本页约定运营 SLA，供 dogfood / 软开使用。

## 目标

| 优先级 | 定义 | 首次响应 | 结案 |
|---|---|---|---|
| P0 | 未成年人 / 暴力威胁 / 自杀自残 / 违法明确证据 | 15 min | 2 h |
| P1 | 色情骚扰、仇恨言论、明显诈骗 | 1 h | 24 h |
| P2 | 垃圾广告、轻微不当、房间质量 | 4 h | 72 h |

时区默认 UTC；软开阶段可人工轮值。

## 流程

1. **用户举报** — App/H5 `POST /api/v1/reports`（`target_type=room|user`，`reason` 文本）。
2. **进队** — Admin「举报队列」列表；状态 `open`。
3. **分拣** — 运营按 reason 关键词升为 P0–P2（v0 人工；后续可规则引擎）。
4. **处置** — 视情况：
   - 用户：`POST /api/v1/admin/ban` / `mute`
   - 房间：`POST /api/v1/admin/rooms/force-close`
5. **结案** — Admin resolve report（`PATCH .../reports/{id}`）；写审计。
6. **复盘** — 周会抽 P0/P1；严重事件记入事故桌面推演。

## 值班

- 软开：工作日覆盖 + on-call 手机；P0 必须有人应答。
- GA：双人 on-call；升级路径见 stage runbook。

## 与产品钩子

- 房间页举报入口（Flutter）
- Admin 队列 + 处置中心
- 审计 `GET /api/v1/admin/audit`

真实工单系统（Jira/Linear）对接为运营后置，不阻塞控制面。

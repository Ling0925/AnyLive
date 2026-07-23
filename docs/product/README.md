# AnyLive 产品与开发规划索引

> 市场：**海外优先（Overseas-first）**  
> 团队：**8–12 人多轨并行**  
> 周期：基建 → MVP 内测 → 封闭公测 → 连麦/PK → 增长 → GA  
> 技术基线：见 [技术评定与架构方案](../技术评定与架构方案.md)

## 文档地图

| 文档 | 内容 | 读者 |
|---|---|---|
| [00-规划总览](./00-规划总览.md) | 目标、成功指标、总时间线、团队编制、并行轨道 | 决策 / 全员 |
| [01-阶段与里程碑](./01-阶段与里程碑.md) | P0–P6 进出标准、交付物、合规闸门 | PM / TL |
| [02-WBS与排期](./02-WBS与排期.md) | 史诗拆解、前 4 周细排、M2–M9 月排期 | 研发 |
| [03-契约与接口冻结](./03-契约与接口冻结.md) | OpenAPI / 事件 / Webhook 冻结顺序 | 后端 + 客户端 |
| [04-非功能与容量](./04-非功能与容量.md) | SLO、压测门禁、容量阶梯 | 后端 / SRE |
| [05-风险与第一刀](./05-风险与第一刀.md) | 风险、关键路径、批准后立即执行项 | TL / PM |
| [06-P1进度评审与后续规划](./06-P1进度评审与后续规划.md) | P1 进度裁决、范围跑偏、出口闸门与 2–4 周规划 | 决策 / TL / PM |
| [p3-p4-experimental](./p3-p4-experimental.md) | 连麦/PK/埋点/OAuth/推送 **非 P1 退出**；默认关开关 | 决策 / 全员 |
| [p1-git-split-plan](./p1-git-split-plan.md) | Stage 0 脏工作区拆分草案（A–F，**不自动提交**） | TL / 全员 |
| [p1-parallel-tracks](./p1-parallel-tracks.md) | 后端 / Admin / Flutter **三轨并行**任务板与接口契约 | 研发 / TL |
| [solo-owner-mode](./solo-owner-mode.md) | **个人项目**运作：完整实现、无签字关闸 | Owner / Agent |
| [p2-status](./p2-status.md) | P2 Soft Launch 子里程碑（M2.1–M2.4）与工程落地 | Owner / SRE |
| [flutter-youtube-ux](./flutter-youtube-ux.md) | Flutter **YouTube 风格**直播 UX / IA / 组件与分阶段 | 产品 / Flutter / 设计 |
| [h5-youtube-ux](./h5-youtube-ux.md) | H5 **RoomWatch** 观播 UX / 共享 Token / 单页 IA（对齐 Flutter） | 产品 / H5 / 设计 |
| [event-dictionary](./event-dictionary.md) | 客户端埋点事件名与 props 约定（P4） | 客户端 / 数据 |
| [海外合规与上架闸门](../compliance/海外合规与上架闸门.md) | App Store/Play、GDPR、支付、审核策略钩子 | 产品 / 法务对接 |
| [架构总览](../architecture/overview.md) | 平面划分、硬约束、阶段映射 | 全员 |
| [支付通道接入设计](../architecture/payment-channels.md) | PayProvider、Jeepay/易支付/TokenPay、订单与 webhook | 后端 / 客户端 |
| [本地开播 Runbook](../runbooks/go-live-local.md) | Dogfood 测试栈开播 | 全员 |
| [Stage/生产 Runbook](../runbooks/go-live-stage.md) | 生产禁止项、环境变量、发布清单 | SRE / TL |
| [举报 SLA](../runbooks/report-sla.md) | 举报分级与运营处置时效 | 运营 / 产品 |
| [备份恢复演练](../runbooks/backup-restore.md) | P2 备份/恢复演练步骤与记录模板 | SRE |
| [商店内测分发](../runbooks/store-internal.md) | TestFlight / Play Internal 清单 | 客户端 / 产品 |
| [SLO 与告警](../runbooks/slo-alerts.md) | SLI 初值、合成探测、桌面推演 | SRE |
| [LiveKit stage](../runbooks/livekit-stage.md) | 连麦集群 env + 联调；真 RTC 边界 | 后端 / 客户端 |
| [Dogfood 队列](../runbooks/dogfood-cohort.md) | 20 主播 / 500 用户填表 | 运营 |

## 一页纸（English）

**AnyLive** is an overseas-first showroom live product. Stack: Rust modular monolith (Axum), Flutter mobile, Vue H5 + Vben admin, LiveKit + global CDN, Centrifugo for chat/gifts.

| Phase | Window | Exit |
|---|---|---|
| P0 Scaffold | W1–W2 | Monorepo, OpenAPI freeze v0, CI green |
| P1 MVP dogfood | ~W3–W12 | 20 streamers / 500 users, gifts+wallet safe |
| P2 Soft launch | ~W13–W20 | Closed beta, store TestFlight/Internal testing |
| P3 Co-host/PK | ~W21–W28 | ≤400ms interactive path live |
| P4 Growth | ~W24–W32 | Analytics, lite recsys, creator tools |
| P5 GA | ~M7–M9 | Public launch gates pass |
| P6 CN (optional) | post-GA | Provider swap, not blocking overseas |

**Non-negotiables:** no media plane in Rust; no giant WS in Axum; gifts via ledger + NATS + IM; public web ≠ Flutter Web; Media/Pay/KYC/Policy as ports.

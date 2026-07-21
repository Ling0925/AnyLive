# 架构总览（执行摘要）

完整裁定见 [技术评定与架构方案](../技术评定与架构方案.md)。  
开发节奏见 [产品规划索引](../product/README.md)。

## 一句话

**Rust 管业务，Centrifugo 管消息，LiveKit/SRS+CDN 管音视频，Flutter 管移动产品，Vue 管 H5 与运营后台。**

## 平面划分

| 平面 | 职责 | 默认实现（海外） |
|---|---|---|
| 业务控制面 | 账号、房间、钱包、礼物、审核、签发 token | Rust Axum 模块化单体 |
| 实时信令 | 聊天、礼物广播、在线、系统通知 | Centrifugo |
| 互动媒体 | 连麦/PK（台上小 N） | LiveKit |
| 广播媒体 | 秀场 1→N 推流与播放 | SRS（开发 origin）+ Cloudflare CDN/Stream |
| 客户端 | 观众/主播 App | Flutter iOS/Android |
| 公网 Web | 分享与观看 | Vue H5（非 Flutter Web） |
| 运营后台 | 监管与配置 | Vben Admin v5 |
| 数据 | 事务 / 缓存 / 事件 | Postgres + Redis + NATS JetStream |

## 硬约束

1. 媒体面（RTP/RTMP）**不进** Rust 业务进程  
2. 房间级大规模 WS **不进** Axum（用 Centrifugo）  
3. 礼物走 **账本事务 + NATS + IM**，不走视频轨  
4. 台下观众走 CDN，**全员不进 SFU**  
5. Media / Pay / KYC / Policy 全部 **Provider 端口**（CN 可后置替换）

## 拓扑（简图）

```
Flutter / Vue H5 / Vben Admin
   │ REST+JWT      │ IM/WS         │ AV
   ▼               ▼               ▼
 anylive-api    Centrifugo     LiveKit (互动)
 (Rust)              ▲         SRS/CDN (广播)
   │                 │ NATS
   └─ Postgres / Redis / NATS / Object Storage
```

## 与规划阶段的映射

| 阶段 | 架构焦点 |
|---|---|
| P0–P1 | 单体 API + SRS + Centrifugo + 账本 |
| P2 | 生产 CDN、支付 Webhook、观测与备份 |
| P3 | LiveKit 连麦/PK；台上/台下分流 |
| P4 | 分析管道、运营配置、审核增强 |
| P5 | 容量阶梯 L3、on-call、发布开关 |
| P6 | Provider 换 CN 实现（可选） |

## 仓库落位

见产品规划 [00-规划总览 §6](../product/00-规划总览.md) 与 ADR §8。

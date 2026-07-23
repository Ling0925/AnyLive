# P3 / P4 experimental surfaces（非 P1 退出条件）

> 对照：[06-P1进度评审与后续规划](./06-P1进度评审与后续规划.md) §8.1 · [mvp-scope](./mvp-scope.md)  
> 状态：工作区可能含控制面脚手架；**不得**作为 P1 dogfood 签字依据。

## 原则

1. P1 未签字前默认 **冻结新功能面**；P3/P4 代码可保留。
2. `FEATURE_PK` / `FEATURE_COHOST` **unset 时默认关闭**（见 `backend/crates/api/src/features.rs`）。
3. 文档与 Demo 主路径：登 → 刷 → 看 → 聊 → 送 → 管；**不含**连麦/PK/OAuth/真推送。

## 开关

| 变量 | 默认（unset） | 说明 |
|---|---|---|
| `FEATURE_PK` | **off** | PK start |
| `FEATURE_COHOST` | **off** | 连麦邀请 |
| `FEATURE_CLIENT_EVENTS` | on | 埋点批入（P4 脚手架） |
| `FEATURE_REAL_PAY` | on | 非 mock 通道建单；生产另有 mock 禁令 |
| `FEATURE_PUBLIC_REGISTER` | on | 与 `INVITE_ONLY` 配合 |

本地/测试栈：`deploy/.env.test` 显式 `FEATURE_PK=0` / `FEATURE_COHOST=0`。

## 已有脚手架（experimental）

| 域 | 代码位置（示意） | P1 口径 |
|---|---|---|
| 连麦 invite/respond | `routes/interactive` + Flutter 菜单 | 非退出条件；默认关 |
| PK 状态机 | `pk/start|end` | 非退出条件；默认关 |
| LiveKit join token | `rooms/.../livekit/join` | 控制面；真集群运维 |
| 埋点 / 创作者 | `events` / `me/creator` | 可保留；非签字项 |
| Push / OAuth | push-tokens / oauth exchange | 账号与密钥待外部 |

## 进入条件（后置）

- **P3：** P1 签字 + P2 基本稳定 + LiveKit stage 可用（非仅 token API）
- **P4：** 事件仓 / 看板 SLA / 真推送密钥就绪后再产品化

## 禁止

- 将 PK/连麦列入 dogfood 成功标准或对外 Demo 主路径  
- 在 P1 出口 PR 中与 A 批硬化混谈为「已完成连麦」  

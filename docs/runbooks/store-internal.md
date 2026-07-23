# 商店内测分发 Runbook（P2 · TestFlight / Play Internal）

> 工程侧已具备：年龄门、隐私/条款链接、账号导出/删除、举报、session 持久化。  
> 本页是**账号与提交流程**清单，不代替开发者控制台操作。

## 前置

- [ ] Apple Developer / Google Play Console 账号可用
- [ ] 包名 / Bundle ID 冻结（与 CI 签名一致）
- [ ] 隐私政策 + 用户协议 **公网 HTTPS** URL（替换 `anylive.example` 占位）
- [ ] 演示账号：邮箱 OTP（stage）或固定 TestFlight 审核账号
- [ ] `FEATURE_PUBLIC_REGISTER` / 邀请码策略与审核说明一致

## iOS — TestFlight

1. Xcode / CI 产出 `ipa`（或 Flutter `build ipa`）。
2. App Store Connect → 内测组；上传构建。
3. 填写出口合规、加密问卷；直播类目权限说明（相机/麦）。
4. 审核备注：演示路径（OTP 码或测试账号）、地理限制（若有）。
5. 内测通过后勾选：`docs/product/01-阶段与里程碑.md` → P2「TestFlight / Play Internal」。

## Android — Play Internal testing

1. 产出 AAB：`flutter build appbundle`。
2. Play Console → 内部测试轨上传。
3. 填写 Data safety、权限用途、虚拟货币说明（IAP vs 自建支付边界）。
4. 添加内部测试人员邮箱列表。

## 验收（内测出口）

| 项 | 通过标准 |
|---|---|
| 安装启动 | 冷启动无崩溃 |
| 登录 | OTP 或演示账号成功 |
| 看播 | HLS 可播（stage CDN/SRS） |
| 合规入口 | 隐私/条款可点；年龄勾选 |
| 账号 | 导出/删除入口可见 |

## 明确不做（本 Runbook）

- 公开商店链接（P5）
- 生产支付密钥上架审核材料终稿（法务）

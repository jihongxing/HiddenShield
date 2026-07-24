# RC1 双端 QA 总索引

更新时间：2026-07-04

本索引用于汇总 RC1 无外部依赖验收已经形成的桌面端、Android、云同步、报告、公开权利、L1/L2 视频和外部阻断证据。它不替代 `docs/封版收口计划.md` 的逐项结论，也不把 Android 证据等同于 iOS 证据。

## 0. RC1 验收包入口

| 项目 | 状态 | 证据 |
| --- | --- | --- |
| RC1 无外部依赖验收包 | READY_WITH_BLOCKED_ITEMS | `docs/RC1无外部依赖验收包.md` |
| Release owner 评审请求 | REVIEWED | `tmp-ui-qa/rc1-no-external-acceptance/20260704/release-owner-review-request-20260704.json`；人工版 `tmp-ui-qa/rc1-no-external-acceptance/20260704/release-owner-review-request-20260704.md`。 |
| Release owner 决策 | CONDITIONAL_GO_FOR_REVIEW / FINAL_SIGNOFF_NO_GO | `tmp-ui-qa/rc1-no-external-acceptance/20260704/release-owner-decision-20260704.json`；人工版 `tmp-ui-qa/rc1-no-external-acceptance/20260704/release-owner-decision-20260704.md`。 |
| Windows 桌面端断网拨测执行 | EXECUTION_BLOCKED_MISSING_ELEVATED_OR_NON_LOOPBACK_ENVIRONMENT | `tmp-ui-qa/rc1-no-external-acceptance/20260704/windows-desktop-os-network-drill/windows-desktop-os-network-drill-execution-20260704.json`；人工版 `tmp-ui-qa/rc1-no-external-acceptance/20260704/windows-desktop-os-network-drill/windows-desktop-os-network-drill-execution-20260704.md`。 |
| 机器摘要 | READY | `tmp-ui-qa/rc1-no-external-acceptance/20260704/rc1-no-external-acceptance-summary-20260704.json` |
| 人工摘要 | READY | `tmp-ui-qa/rc1-no-external-acceptance/20260704/rc1-no-external-acceptance-summary-20260704.md` |

## 1. 本轮新增数据模型收口

| 项目 | 状态 | 证据 |
| --- | --- | --- |
| `vault_records.file_type` 历史 backfill | READY | SQLite v18 migration：历史 `file_type='video'` 且扩展名可确定的图片 / 音频记录回填为 `image` / `audio`；L2 / L3 视频收据字段存在时保持 `video`。 |
| 新入库记录 `file_type` | READY | `insert_record` / `insert_record_tx` 显式写入 `infer_vault_record_file_type(record)`，不再依赖 schema 默认 `video`。 |
| 同步 `kind` 推断 | READY | 桌面 cloud event 与 desktop changes response 复用同一推断函数，避免图片 / 音频同步摘要回退为 `video`。 |
| 合同门禁 | READY | `npm run vault:file-type-backfill-contract`。 |

## 2. 自动化与合同门禁

| 范围 | 状态 | 证据 |
| --- | --- | --- |
| RC1 商业化自动化 | READY | 2026-07-10 `npm run commercial:ci` 完整复跑通过，最终输出 `HiddenShield commercial CI OK`；本轮已包含桌面云同步 `eventResults` 消费修复、`Vault file_type backfill contract`，并输出 `vault:file-type-backfill-contract OK`。 |
| `vault_records.file_type` 聚合门禁 | READY | `scripts/run-commercial-ci.mjs` 已串行纳入 `npm run vault:file-type-backfill-contract`，使本轮 v18 backfill / 新入库显式类型 / 同步 `kind` 推断进入 RC1 商业化聚合验收。 |
| Enterprise 运行态证据 | READY | 最新 `commercial:ci` 证据：`tmp-ui-qa/enterprise-gateway-dry-run-runtime/1783625469201/enterprise-gateway-dry-run-runtime-qa-1783625469201.json`、`tmp-ui-qa/enterprise-key-issuance-runtime/1783625482264/enterprise-key-issuance-runtime-qa-1783625482264.md`、`tmp-ui-qa/enterprise-public-rights-runtime/1783625490913/enterprise-public-rights-runtime-qa-1783625490913.md`。 |
| L3 release candidate 聚合证据 | READY | 最新 `commercial:ci` 证据：`tmp-ui-qa/l3-video-visual-cross-end-runtime/l3-video-visual-cross-end-runtime-qa-1783626639064.json`、`tmp-ui-qa/l3-video-visual-sellable-runtime/l3-video-visual-sellable-runtime-qa-1783626662534.json`、`tmp-ui-qa/l3-video-visual-production-ops/l3-video-visual-production-ops-runtime-qa-1783626712107.json`。 |
| 双端一致性合同 | READY | `npm run dual:contract`。 |
| 云同步可靠性合同 | READY | 2026-07-10 `npm run cloud:sync-reliability-contract` 通过；合同已检查桌面 flush 消费后端 `eventResults`，避免 `conflict_payload_changed` / `rejected_invalid_event` 被误清为 synced。 |
| 云同步专项 ready | READY | `tmp-ui-qa/cloud-sync-runtime-readiness/cloud-sync-runtime-qa-readiness-1783067156075.json`。 |
| L3 production readiness | BLOCKED | `tmp-ui-qa/l3-video-visual-production-readiness/l3-production-readiness-contract-1783113551653.json` 默认 blocked；缺真实告警平台、试点客户签字和真实样本 manifest。该 blocked artifact 是预期外部阻断，不计为 `commercial:ci` 失败。 |
| 公开权利 completion gate | BLOCKED | `public-rights:completion-gate` 默认 blocked；缺生产 C2PA/TSA、iOS QA、npm 发布、release 样本池和客户签字。 |

## 3. 桌面安装版页面级 QA

| 范围 | 状态 | 证据 |
| --- | --- | --- |
| 桌面安装版总核验 | READY | `tmp-ui-qa/desktop-batch2-qa/desktop-batch2-final-evidence-check-20260704.json`。 |
| 当前安装版 sanity | READY | `tmp-ui-qa/desktop-batch2-qa/97-desktop-batch2-current-sanity.png`。 |
| 图片 / 音频写入与验证 | READY | `tmp-ui-qa/desktop-batch2-qa/07-image-result-window.png`、`15-verify-image-result.png`、`13-audio-result.png`、`16-verify-audio-result.png`。 |
| 本地批量图片 / 音频 | READY | `tmp-ui-qa/desktop-batch2-qa/37-local-batch-after-manual-select-before-start.png`、`39-local-batch-final.png`。 |
| L1 视频音轨水印 | READY | `tmp-ui-qa/desktop-batch2-qa/56e-l1-video-verify-after-fix-result.png`。 |
| L2 视频指纹存证 | READY | `tmp-ui-qa/desktop-batch2-qa/48-l2-video-bundle-generated.png`、`49-l2-video-notary-submitted.png`。 |
| 公开权利 / 训练许可 / 公开元数据 | READY | `tmp-ui-qa/desktop-batch2-qa/desktop-batch2-public-rights-sync-cursor-summary-20260704.json`。 |
| 设置反馈 / 导出日志 / 关闭后端成熟错误 | READY | `tmp-ui-qa/desktop-batch2-qa/31-settings-feedback-log-section.png`、`33-settings-log-export-click.png`、`22-backend-off-error-visible.png`。 |

## 4. Android 页面级 QA

| 范围 | 状态 | 证据 |
| --- | --- | --- |
| Android Batch 2 剩余页面级 QA | READY | `tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa/1783106946906/android-batch2-page-qa-summary-1783106946906.json`。 |
| Android Batch 2 Markdown 摘要 | READY | `tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa/1783106946906/android-batch2-page-qa-summary-1783106946906.md`。 |
| Android 截图目录 | READY | `tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa/1783106946906/screenshots/`。 |
| L1 视频音轨验证 | READY | `tmp-ui-qa/desktop-batch2-qa/android-page-level-qa-summary.json`，截图 `android-page-qa-23-l1-verify-after-track-fix-result.png` / `android-page-qa-24-l1-verify-after-track-fix-result-details.png`。 |
| 公开权利 / 训练许可展示 | READY | `tmp-ui-qa/desktop-batch2-qa/android-page-qa-25-public-rights-image-verify-result.png`、`android-page-qa-26-public-rights-image-details.png`。 |
| 报告草稿隐私扫描 | READY | `tmp-ui-qa/desktop-batch2-qa/android-batch2-page-qa/1783106946906/pulled/android-batch2-formal-report-1783106946906.md`。 |

## 5. 云同步与后端

| 范围 | 状态 | 证据 |
| --- | --- | --- |
| 桌面安装版云同步专项 | READY | `tmp-ui-qa/cloud-sync-runtime/desktop-installer-sync-runtime-1783067038401.json`。 |
| Android 原生云同步专项 | READY | `tmp-ui-qa/cloud-sync-runtime/android-native-sync-runtime-1783067038401.json`。 |
| 网络恢复汇总 | READY | `tmp-ui-qa/cloud-sync-runtime/network-resume-sync-runtime-1783067038401.json`。 |
| 后端 event disposition | READY | `tmp-ui-qa/cloud-sync-runtime/event-disposition-sync-runtime-1783067038401.json`。 |
| 真实 OS 断网拨测 | PARTIAL_READY_DESKTOP_BLOCKED | Android 原生端已通过真实 `svc data/wifi disable` / restore 拨测，证据 `tmp-ui-qa/rc1-no-external-acceptance/20260704/android-os-network-disconnect-drill-20260704.json`，截图 `android-network-off-20260704.png` / `android-network-restored-20260704.png`；Windows 桌面端因本机安装版连接 `127.0.0.1:43188` 且当前会话无 firewall / proxy 提权仍 blocked，证据 `tmp-ui-qa/rc1-no-external-acceptance/20260704/desktop-os-network-disconnect-drill-20260704.json`，复跑安排 `tmp-ui-qa/rc1-no-external-acceptance/20260704/windows-desktop-os-network-drill-schedule-20260704.json`；聚合记录 `tmp-ui-qa/rc1-no-external-acceptance/20260704/os-network-disconnect-drill-record-20260704.json`。 |

## 6. PostgreSQL 迁移证据

| 范围 | 状态 | 证据 |
| --- | --- | --- |
| P2.2 migration smoke | READY | `tmp-ui-qa/postgres-migration/postgres-migrate-smoke-1783021160601.json`。 |
| P3.4 聚合门禁 | READY | `tmp-ui-qa/postgres-runtime-aggregate/cloud-postgres-runtime-qa-1783053449984.json`。 |
| P4 SQLite -> Postgres import smoke | READY | `tmp-ui-qa/postgres-import/postgres-import-smoke-1783053193204.json`。 |
| P5 production readiness | BLOCKED | `tmp-ui-qa/postgres-production-readiness/cloud-postgres-production-readiness-gate-1783053429272.json`。 |
| P6 SQLite shutdown | BLOCKED | `tmp-ui-qa/postgres-sqlite-shutdown/cloud-postgres-sqlite-shutdown-gate-1783053429239.json`。 |

## 7. 外部环境阻断

| 项目 | 状态 | 说明 |
| --- | --- | --- |
| iOS 页面级 QA | BLOCKED | 当前 Windows 环境缺 macOS + Xcode；官方 runner 已生成 blocked artifact `tmp-ui-qa/rc1-no-external-acceptance/20260704/ios-qa-blocked-20260704.json`，Android 证据不能替代 iOS。 |
| 真实微信支付 | BLOCKED | 缺真实商户参数、公网 HTTPS 回调和测试订单闭环。 |
| 生产 C2PA/TSA | BLOCKED | 当前公开元数据只能使用 QA / ephemeral signer 证据，不能宣称生产 trust chain。 |
| L3 可售 SLA | BLOCKED | 缺真实告警平台配置、试点客户签字和更大真实用户 MP4 样本 manifest。 |
| 生产 PostgreSQL 切换 | BLOCKED | disposable runtime QA 已通过，但缺真实 staging 压测、备份恢复、observability 和 release owner signoff。 |

当前推荐下一步：release owner 提供提权 Windows QA operator 或 LAN / staging backend endpoint 后重跑 Windows 桌面端断网拨测；通过后回写 RC1 包并复跑 `cloud:sync-reliability-contract`、`dual:contract` 和 `git diff --check`，否则最终 RC1 签字继续 NO-GO。

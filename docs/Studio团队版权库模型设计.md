# Studio 团队版权库模型设计

## 1. 目标

把 HiddenShield 的团队能力做成一个清晰、可扩展、可审计的共享版权库模型。

核心方向：

- 让 Studio 不是“Creator 的多人副本”，而是有独立协作语义的团队空间。
- 让桌面端和移动端共享同一套团队身份、权限和审计口径。
- 让团队协作只共享版权元数据，不共享原始媒体和本地路径。
- 让后续 Enterprise 私有化和 API 接入可以在同一模型上扩展。

## 2. 范围

本设计覆盖：

- workspace
- team member
- shared vault
- team audit log
- team 级订阅权益
- 客户端入口与门禁

不覆盖：

- 计费实现
- 支付渠道
- 私有化部署细节
- 团队原始媒体同步

## 3. 设计原则

1. 团队能力必须保持为 Studio 起开放，不影响 Creator。
2. 团队共享以元数据为主，不同步原始图片、音频、视频和本地路径。
3. 成员权限必须可审计、可回收、可降级。
4. 桌面端和移动端显示同一套团队术语。
5. 默认先只做“能看、能查、能审计”，不提前暴露复杂管理动作。

## 4. 核心实体

### 4.1 Workspace

团队空间，是版权协作的最小组织单元。

建议字段：

- `workspace_id`
- `account_id`
- `name`
- `workspace_type`：`personal` / `team`
- `status`：`active` / `suspended` / `archived`
- `created_at`
- `updated_at`

说明：

- 个人账户默认拥有一个 `personal` workspace。
- Studio 开通后可额外创建 `team` workspace。
- 未来 Enterprise 可以支持多个 workspace 和更复杂的组织树。

### 4.2 Team Member

团队成员是 workspace 下的成员关系，不等于账户本身。

建议字段：

- `member_id`
- `workspace_id`
- `account_id`
- `role`：`owner` / `admin` / `editor` / `viewer`
- `status`：`invited` / `active` / `removed`
- `invited_by`
- `joined_at`
- `updated_at`

说明：

- `owner` 拥有最高管理权。
- `admin` 管理成员与共享内容。
- `editor` 可写入团队共享版权库。
- `viewer` 只读查看和导出允许的摘要。

### 4.3 Shared Vault Record

团队共享版权库记录，本质上是权限范围扩展后的版权元数据。

建议字段：

- `shared_record_id`
- `workspace_id`
- `source_record_id`
- `watermark_uid`
- `revision`
- `record_type`
- `owner_creator_profile_id`
- `visible_to_roles`
- `sync_scope`
- `created_by`
- `created_at`
- `updated_at`

说明：

- `source_record_id` 指向原始版权库记录。
- `visible_to_roles` 控制哪些成员角色可见。
- `sync_scope` 只包含 metadata，不包含媒体文件。

### 4.4 Team Audit Log

所有团队关键动作必须落审计日志。

建议字段：

- `audit_id`
- `workspace_id`
- `actor_account_id`
- `actor_member_id`
- `action`
- `target_type`
- `target_id`
- `before_json`
- `after_json`
- `reason`
- `created_at`

建议记录的动作：

- 邀请成员
- 移除成员
- 修改角色
- 共享记录
- 收回共享
- 导出团队摘要
- 修改团队创作者身份

## 5. 权限模型

### 5.1 角色权限矩阵

| 能力 | owner | admin | editor | viewer |
| --- | --- | --- | --- | --- |
| 查看共享版权库 | yes | yes | yes | yes |
| 写入团队共享记录 | yes | yes | yes | no |
| 修改成员角色 | yes | yes | no | no |
| 邀请 / 移除成员 | yes | yes | no | no |
| 导出团队摘要 | yes | yes | yes | yes |
| 查看审计日志 | yes | yes | no | no |
| 修改团队名称 | yes | yes | no | no |

### 5.2 权益门禁

团队能力由 `team_workspace` 控制，建议和 Studio / Enterprise 绑定。

建议拆分：

- `team_workspace`: 团队空间入口
- `team_audit`: 团队审计日志
- `team_export`: 团队报告导出

其中首期可以先只实现 `team_workspace`，其余能力作为后续扩展字段预留。

## 6. 数据边界

团队共享只同步：

- 版权编号
- 版本次数
- 创作者档案摘要
- 写入时间
- 验证状态
- 摘要型报告
- 审计事件

团队不共享：

- 原始图片
- 加水印图片
- 原始音频
- 加水印音频
- 原始视频
- 加水印视频
- 本地文件路径
- 用户本地临时目录

## 7. 客户端体验边界

### 桌面端

建议入口：

- 版权库增加“团队空间”分区
- 设置页增加“Studio 团队”
- 订阅页显示 Studio / Enterprise 差异

### 移动端

建议入口：

- 版权库页增加“团队空间”卡片
- 设置页显示团队身份和成员数
- 不把团队能力包装成技术术语

### 统一文案

推荐使用：

- 团队空间
- 成员权限
- 共享版权库
- 团队审计

不建议使用：

- 协议同步
- 桥接层
- 共享通道
- 传输编排

## 8. 同步策略

团队空间只在云端共享以下内容：

- workspace
- member
- shared vault record
- audit log
- entitlement snapshot

同步原则：

1. 先写本地，再入同步队列。
2. 成功后才入账。
3. 冲突优先保留更高版本或 owner 决策。
4. 冲突必须留下审计记录，不静默覆盖。

## 9. 冲突处理

### 成员角色冲突

- 以云端最新权限为准。
- 本地展示只读降级状态。
- 需要 owner 重新确认后恢复编辑权限。

### 共享记录冲突

- 若同一 `watermark_uid` 出现重复共享，按 `revision` 和 `updated_at` 排序。
- 不删除历史版本，只标记当前可见版本。

### 审计冲突

- 审计日志不可覆盖。
- 发生冲突时追加系统审计事件。

## 10. 推荐接口草案

后续可从以下接口开始实现：

- `GET /v1/team/workspaces/current`
- `POST /v1/team/workspaces`
- `GET /v1/team/workspaces/:id/members`
- `POST /v1/team/workspaces/:id/members`
- `PATCH /v1/team/members/:id`
- `GET /v1/team/workspaces/:id/vault`
- `POST /v1/team/workspaces/:id/vault/share`
- `GET /v1/team/workspaces/:id/audit-logs`

## 11. 首期落地顺序

1. 先做 workspace / member 数据模型。
2. 再做 shared vault 只读展示。
3. 再做 audit log。
4. 最后再做成员管理和共享写入。

## 12. 验收标准

- Studio 能出现独立团队空间概念。
- Creator 不受影响。
- 团队共享只暴露元数据。
- 审计日志可追溯。
- 桌面端和移动端术语一致。


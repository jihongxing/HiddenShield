import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import '../../shared/widgets/tool_cards.dart';

class SettingsPage extends StatefulWidget {
  const SettingsPage({super.key, required this.bridge, required this.appState});

  final WatermarkBridge bridge;
  final MobileAppState appState;

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  late final TextEditingController _creatorController = TextEditingController(
    text: widget.appState.creatorLabel,
  );
  late final TextEditingController _accountController = TextEditingController(
    text: widget.appState.syncProfile.accountLabel ?? '',
  );

  @override
  void dispose() {
    _creatorController.dispose();
    _accountController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '设置',
      subtitle: '身份、同步、隐私与帮助',
      children: [
        AnimatedBuilder(
          animation: widget.appState,
          builder: (context, _) => _SettingsContent(
            appState: widget.appState,
            creatorController: _creatorController,
            accountController: _accountController,
          ),
        ),
      ],
    );
  }
}

class _SettingsContent extends StatelessWidget {
  const _SettingsContent({
    required this.appState,
    required this.creatorController,
    required this.accountController,
  });

  final MobileAppState appState;
  final TextEditingController creatorController;
  final TextEditingController accountController;

  @override
  Widget build(BuildContext context) {
    final profile = appState.syncProfile;
    final signedIn = appState.hasCloudAccount;

    return Column(
      children: [
        HsPanel(
          title: '创作者身份',
          icon: Icons.badge_outlined,
          child: Column(
            children: [
              TextField(
                controller: creatorController,
                decoration: const InputDecoration(
                  labelText: '创作者标识',
                  helperText: '继续使用账户并开启云同步后，创作者档案会在移动端和桌面端保持一致。',
                ),
                onSubmitted: appState.updateCreatorLabel,
              ),
              const SizedBox(height: 12),
              _DiagnosticRow(
                label: '档案同步',
                value: profile.creatorProfileSynced ? '随账户同步' : '仅保存在本机',
              ),
              const SizedBox(height: 12),
              Align(
                alignment: Alignment.centerRight,
                child: FilledButton.icon(
                  onPressed: () =>
                      appState.updateCreatorLabel(creatorController.text),
                  icon: const Icon(Icons.save_outlined),
                  label: const Text('保存身份'),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        HsPanel(
          title: '账户与权益',
          icon: Icons.account_circle_outlined,
          child: Column(
            children: [
              TextField(
                controller: accountController,
                decoration: const InputDecoration(
                  labelText: 'HiddenShield 账户',
                  hintText: 'name@example.com',
                  helperText: '输入邮箱或手机号后继续；新用户自动创建账户，老用户直接进入。',
                ),
              ),
              const SizedBox(height: 12),
              ListTile(
                contentPadding: EdgeInsets.zero,
                leading: const Icon(Icons.verified_user_outlined),
                title: Text(signedIn ? '已登录' : '未登录'),
                subtitle: Text(
                  signedIn
                      ? profile.accountLabel ?? 'HiddenShield 账户'
                      : '本地功能可直接使用，跨设备同步需要登录。',
                ),
                trailing: Chip(
                  label: Text(
                    entitlementStatusLabel(profile.entitlementStatus),
                  ),
                  materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  padding: EdgeInsets.zero,
                  backgroundColor: HsColors.chip,
                  side: BorderSide.none,
                ),
              ),
              const Divider(height: 1),
              const SizedBox(height: 12),
              _DiagnosticRow(
                label: '当前权益',
                value: '${profile.entitlementLabel} · 批量处理 / 云端视频处理按权益开放',
              ),
              _DiagnosticRow(
                label: '工作区',
                value: profile.workspaceName ?? '未创建',
              ),
              _DiagnosticRow(
                label: '设备',
                value: profile.deviceRegistered
                    ? profile.deviceName ?? '当前设备'
                    : '未加入账户',
              ),
              _DiagnosticRow(
                label: '权益模块',
                value: _enabledEntitlementSummary(profile.entitlementFeatures),
              ),
              const SizedBox(height: 12),
              Align(
                alignment: Alignment.centerRight,
                child: Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  alignment: WrapAlignment.end,
                  children: [
                    if (signedIn)
                      OutlinedButton.icon(
                        onPressed: appState.signOutCloud,
                        icon: const Icon(Icons.logout_outlined),
                        label: const Text('退出账户'),
                      )
                    else
                      FilledButton.icon(
                        onPressed: () =>
                            appState.continueWithAccountPlaceholder(
                              accountLabel: accountController.text,
                            ),
                        icon: const Icon(Icons.login_outlined),
                        label: const Text('继续'),
                      ),
                  ],
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        HsPanel(
          title: '云同步',
          icon: Icons.cloud_sync_outlined,
          child: Column(
            children: [
              SwitchListTile(
                value: appState.cloudSyncEnabled,
                onChanged: signedIn ? appState.setCloudSyncEnabled : null,
                title: const Text('开启云同步'),
                subtitle: const Text('同步版权库、取证记录、创作者档案和权益状态；不默认上传媒体文件。'),
                contentPadding: EdgeInsets.zero,
              ),
              _SyncHealthSummary(appState: appState),
              const Divider(height: 1),
              _SyncDiagnosticsPanel(
                appState: appState,
                onRecoverAccount: () {
                  final accountLabel = accountController.text.trim().isEmpty
                      ? appState.syncProfile.accountLabel ?? ''
                      : accountController.text.trim();
                  appState.continueWithAccountPlaceholder(
                    accountLabel: accountLabel,
                  );
                },
              ),
              const Divider(height: 1),
              _SyncResolutionSummary(resolutions: appState.syncResolutions),
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerRight,
                child: Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  alignment: WrapAlignment.end,
                  children: [
                    OutlinedButton.icon(
                      onPressed:
                          appState.isPullingRemoteChanges ||
                              appState.syncTransportMode ==
                                  SyncTransportMode.localOnly
                          ? null
                          : appState.pullRemoteChanges,
                      icon: appState.isPullingRemoteChanges
                          ? const SizedBox.square(
                              dimension: 18,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.download_outlined),
                      label: Text(
                        appState.isPullingRemoteChanges ? '正在拉取' : '拉取变更',
                      ),
                    ),
                    FilledButton.icon(
                      onPressed:
                          appState.isSyncing ||
                              appState.readySyncQueueCount == 0 ||
                              appState.syncTransportMode ==
                                  SyncTransportMode.localOnly
                          ? null
                          : appState.syncPendingQueue,
                      icon: appState.isSyncing
                          ? const SizedBox.square(
                              dimension: 18,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.cloud_upload_outlined),
                      label: Text(
                        appState.isSyncing
                            ? '正在同步'
                            : '${syncTransportModeLabel(appState.syncTransportMode)}同步',
                      ),
                    ),
                    OutlinedButton.icon(
                      onPressed:
                          appState.isSyncing ||
                              appState.failedSyncQueueCount == 0
                          ? null
                          : appState.retryFailedSyncQueue,
                      icon: const Icon(Icons.replay_outlined),
                      label: const Text('重试失败'),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        HsPanel(
          title: '隐私与权限',
          icon: Icons.lock_outline,
          child: Column(
            children: [
              SwitchListTile(
                value: appState.anonymousFeedbackEnabled,
                onChanged: appState.setAnonymousFeedbackEnabled,
                title: const Text('匿名反馈'),
                subtitle: const Text('仅记录功能结果和稳定性状态，不上传原始媒体。'),
                contentPadding: EdgeInsets.zero,
              ),
              const Divider(height: 1),
              const SizedBox(height: 12),
              const Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  '默认不同步原始图片、加水印后的图片、原始音频、加水印后的音频、原始视频、加水印后的视频和本地文件路径。',
                  style: TextStyle(color: Colors.white70),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _SyncDiagnosticsPanel extends StatelessWidget {
  const _SyncDiagnosticsPanel({
    required this.appState,
    required this.onRecoverAccount,
  });

  final MobileAppState appState;
  final VoidCallback onRecoverAccount;

  @override
  Widget build(BuildContext context) {
    final profile = appState.syncProfile;
    final recoverableError = _isRecoverableSyncError(profile.lastError);
    final hasProblem =
        recoverableError ||
        profile.lastError?.isNotEmpty == true ||
        appState.failedSyncQueueCount > 0;
    return ExpansionTile(
      tilePadding: EdgeInsets.zero,
      childrenPadding: EdgeInsets.zero,
      initiallyExpanded: hasProblem,
      leading: const Icon(Icons.manage_search_outlined),
      title: const Text('同步帮助'),
      subtitle: Text(hasProblem ? '有同步问题需要处理' : '同步正常时无需打开'),
      children: [
        Padding(
          padding: const EdgeInsets.only(bottom: 12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  const Spacer(),
                  IconButton(
                    tooltip: '复制同步信息',
                    onPressed: () async {
                      await Clipboard.setData(
                        ClipboardData(
                          text: _buildSyncDiagnosticsText(appState),
                        ),
                      );
                      if (context.mounted) {
                        ScaffoldMessenger.of(context).showSnackBar(
                          const SnackBar(content: Text('同步信息已复制')),
                        );
                      }
                    },
                    icon: const Icon(Icons.copy_outlined),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              _DiagnosticRow(
                label: '同步模式',
                value: syncTransportModeLabel(appState.syncTransportMode),
              ),
              _DiagnosticRow(label: '账户', value: profile.accountLabel ?? '未登录'),
              _DiagnosticRow(label: '工作区', value: profile.workspaceName ?? '无'),
              _DiagnosticRow(label: '设备', value: profile.deviceName ?? '当前设备'),
              _DiagnosticRow(
                label: '档案',
                value: profile.creatorProfileSynced ? '随账户同步' : '仅保存在本机',
              ),
              _DiagnosticRow(
                label: '云服务',
                value: profile.cloudBaseUrl.isEmpty
                    ? '由系统配置提供'
                    : profile.cloudBaseUrl,
              ),
              _DiagnosticRow(
                label: '连接状态',
                value: syncConnectionStatusLabel(profile.status),
              ),
              _DiagnosticRow(
                label: '同步位置',
                value: profile.lastRemotePullCursor ?? '尚未拉取',
              ),
              _DiagnosticRow(
                label: '最近尝试',
                value: _formatDateTime(profile.lastSyncAttemptAt),
              ),
              _DiagnosticRow(
                label: '最近成功',
                value: _formatDateTime(profile.lastSyncSuccessAt),
              ),
              _DiagnosticRow(
                label: '最近失败',
                value: _formatDateTime(profile.lastSyncFailureAt),
              ),
              _DiagnosticRow(
                label: '处理状态',
                value:
                    '待同步 ${appState.pendingSyncQueueCount} · 失败 ${appState.failedSyncQueueCount}',
              ),
              _DiagnosticRow(
                label: '下次自动重试',
                value: _mobileRetryDetail(appState),
              ),
              _DiagnosticRow(label: '最近错误', value: profile.lastError ?? '无'),
              if (recoverableError) ...[
                const SizedBox(height: 4),
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: HsColors.warningSurface,
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(color: HsColors.warning),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text(
                        '账户状态需要恢复',
                        style: TextStyle(fontWeight: FontWeight.w700),
                      ),
                      const SizedBox(height: 6),
                      const Text(
                        '当前账户、设备或工作区与云端不一致。重新继续账户会刷新授权、设备登记和工作区绑定。',
                        style: TextStyle(color: Colors.white70),
                      ),
                      const SizedBox(height: 10),
                      Align(
                        alignment: Alignment.centerRight,
                        child: FilledButton.icon(
                          onPressed: onRecoverAccount,
                          icon: const Icon(Icons.login_outlined),
                          label: const Text('重新继续账户'),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
              _DiagnosticRow(
                label: '解决记录',
                value: appState.syncResolutions.isEmpty
                    ? '无'
                    : '${appState.syncResolutions.length} 条',
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _SyncHealthSummary extends StatelessWidget {
  const _SyncHealthSummary({required this.appState});

  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    final health = _mobileSyncHealth(appState);
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: health.background,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: health.border),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(health.icon, color: health.iconColor),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  health.label,
                  style: const TextStyle(fontWeight: FontWeight.w700),
                ),
                const SizedBox(height: 4),
                Text(
                  health.detail,
                  style: const TextStyle(color: Colors.white70),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _DiagnosticRow extends StatelessWidget {
  const _DiagnosticRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 96,
            child: Text(label, style: const TextStyle(color: Colors.white70)),
          ),
          Expanded(child: SelectableText(value)),
        ],
      ),
    );
  }
}

String _enabledEntitlementSummary(Map<String, bool> features) {
  if (features.isEmpty) {
    return '未同步';
  }
  final enabled = features.entries
      .where((entry) => entry.value)
      .map((entry) => _entitlementFeatureLabel(entry.key))
      .toList(growable: false);
  if (enabled.isEmpty) {
    return '基础功能';
  }
  return enabled.join(' / ');
}

String _formatDateTime(DateTime? value) {
  if (value == null) {
    return '无';
  }
  return value.toLocal().toString().split('.').first;
}

String _mobileRetryDetail(MobileAppState appState) {
  if (appState.failedSyncQueueCount == 0) {
    return '无失败队列';
  }
  final readyCount = appState.readySyncQueueCount;
  if (readyCount > 0) {
    return '有 $readyCount 条可立即同步；点击同步或重试失败会立即处理。';
  }
  final exhausted = appState.retryExhaustedSyncQueueCount;
  if (exhausted == appState.failedSyncQueueCount) {
    return '已达自动重试上限；点击重试失败可手动再试。';
  }
  final nextRetryAt = appState.nextSyncQueueRetryAt;
  if (nextRetryAt != null) {
    return '下次自动重试：${_formatDateTime(nextRetryAt)}；点击重试失败可立即处理。';
  }
  return '等待手动重试；点击重试失败可立即处理。';
}

bool _isRecoverableSyncError(String? value) {
  if (value == null || value.isEmpty) {
    return false;
  }
  return value.contains('HTTP 401') ||
      value.contains('HTTP 403') ||
      value.contains('登录状态已失效') ||
      value.contains('设备未被当前账户授权') ||
      value.contains('工作区或设备与云端账户不匹配');
}

_SyncHealthState _mobileSyncHealth(MobileAppState appState) {
  final profile = appState.syncProfile;
  if (_isRecoverableSyncError(profile.lastError)) {
    return const _SyncHealthState(
      label: '需恢复账户',
      detail: '账户、设备或工作区授权不一致，请重新继续账户。',
      icon: Icons.warning_amber_outlined,
      iconColor: Color(0xFFFFC857),
      background: Color(0xFF2C2212),
      border: Color(0xFFFFC857),
    );
  }
  if (!appState.hasCloudAccount) {
    return const _SyncHealthState(
      label: '未连接',
      detail: '本地功能可直接使用，云同步需要继续账户。',
      icon: Icons.cloud_off_outlined,
      iconColor: Colors.white70,
      background: Color(0xFF182028),
      border: Color(0xFF2C3945),
    );
  }
  if (appState.failedSyncQueueCount > 0) {
    return _SyncHealthState(
      label: '有失败',
      detail: _mobileRetryDetail(appState),
      icon: Icons.error_outline,
      iconColor: const Color(0xFFFFC857),
      background: const Color(0xFF2A2118),
      border: const Color(0xFF6A4B20),
    );
  }
  if (appState.pendingSyncQueueCount > 0) {
    return _SyncHealthState(
      label: '有待同步',
      detail: '还有 ${appState.pendingSyncQueueCount} 条版权元数据等待上传。',
      icon: Icons.cloud_upload_outlined,
      iconColor: const Color(0xFF8BB8FF),
      background: const Color(0xFF172235),
      border: const Color(0xFF2B4D7A),
    );
  }
  return const _SyncHealthState(
    label: '正常',
    detail: '同步队列已清空，最近没有需要处理的同步问题。',
    icon: Icons.check_circle_outline,
    iconColor: Color(0xFF59D2C2),
    background: Color(0xFF132A25),
    border: Color(0xFF2A6B61),
  );
}

class _SyncHealthState {
  const _SyncHealthState({
    required this.label,
    required this.detail,
    required this.icon,
    required this.iconColor,
    required this.background,
    required this.border,
  });

  final String label;
  final String detail;
  final IconData icon;
  final Color iconColor;
  final Color background;
  final Color border;
}

String _buildSyncDiagnosticsText(MobileAppState appState) {
  final profile = appState.syncProfile;
  return [
    'HiddenShield 移动端同步信息',
    '生成时间: ${_formatDateTime(DateTime.now())}',
    '同步模式: ${syncTransportModeLabel(appState.syncTransportMode)}',
    '连接状态: ${syncConnectionStatusLabel(profile.status)}',
    '账户: ${profile.accountLabel ?? '未登录'}',
    '账户 ID: ${profile.accountId ?? '无'}',
    '工作区: ${profile.workspaceName ?? '无'}',
    '工作区 ID: ${profile.workspaceId ?? '无'}',
    '设备: ${profile.deviceName ?? '无'}',
    '设备 ID: ${profile.deviceId ?? '无'}',
    '设备平台: ${profile.devicePlatform ?? '无'}',
    '创作者档案: ${profile.creatorProfileSynced ? '已同步' : '本机'}',
    '权益: ${profile.entitlementLabel} / ${entitlementStatusLabel(profile.entitlementStatus)}',
    '权益模块: ${_enabledEntitlementSummary(profile.entitlementFeatures)}',
    '云服务: ${profile.cloudBaseUrl.isEmpty ? '由系统配置提供' : profile.cloudBaseUrl}',
    '上次游标: ${profile.lastRemotePullCursor ?? '尚未拉取'}',
    '最近尝试: ${_formatDateTime(profile.lastSyncAttemptAt)}',
    '最近成功: ${_formatDateTime(profile.lastSyncSuccessAt)}',
    '最近失败: ${_formatDateTime(profile.lastSyncFailureAt)}',
    '待同步: ${appState.pendingSyncQueueCount}',
    '失败记录: ${appState.failedSyncQueueCount}',
    '可立即同步记录: ${appState.readySyncQueueCount}',
    '已达自动重试上限: ${appState.retryExhaustedSyncQueueCount}',
    '下次自动重试: ${_mobileRetryDetail(appState)}',
    '最近错误: ${profile.lastError ?? '无'}',
    '同步处理记录: ${appState.syncResolutions.length}',
  ].join('\n');
}

String _entitlementFeatureLabel(String key) {
  return switch (key) {
    'batch_processing' => '批量处理',
    'cloud_video_processing' => '云端视频',
    'cloud_sync' => '云同步',
    'priority_queue' => '优先队列',
    'team_workspace' => '团队空间',
    _ => key,
  };
}

class _SyncResolutionSummary extends StatelessWidget {
  const _SyncResolutionSummary({required this.resolutions});

  final List<MobileSyncResolution> resolutions;

  @override
  Widget build(BuildContext context) {
    final latest = resolutions.isEmpty ? null : resolutions.first;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: const Icon(Icons.rule_folder_outlined),
      title: const Text('同步处理记录'),
      subtitle: Text(
        latest == null
            ? '还没有跨端同步处理记录。'
            : '${mobileSyncResolutionTypeLabel(latest.resolutionType)} · ${latest.watermarkUid} · v${latest.incomingRevision}',
      ),
      trailing: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          Text(
            '${resolutions.length}',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const Text(
            '累计',
            style: TextStyle(color: Colors.white70, fontSize: 12),
          ),
        ],
      ),
    );
  }
}

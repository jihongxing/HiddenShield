import 'package:flutter/material.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../shared/widgets/feature_page_scaffold.dart';

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
  late final TextEditingController _lanAddressController =
      TextEditingController(text: widget.appState.syncProfile.lanDebugAddress);
  late final TextEditingController _pairingCodeController =
      TextEditingController(
        text: widget.appState.syncProfile.lanDebugPairingCode,
      );

  @override
  void dispose() {
    _creatorController.dispose();
    _accountController.dispose();
    _lanAddressController.dispose();
    _pairingCodeController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '设置',
      subtitle: '身份、同步、隐私与帮助',
      bridge: widget.bridge,
      children: [
        AnimatedBuilder(
          animation: widget.appState,
          builder: (context, _) => _SettingsContent(
            appState: widget.appState,
            creatorController: _creatorController,
            accountController: _accountController,
            lanAddressController: _lanAddressController,
            pairingCodeController: _pairingCodeController,
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
    required this.lanAddressController,
    required this.pairingCodeController,
  });

  final MobileAppState appState;
  final TextEditingController creatorController;
  final TextEditingController accountController;
  final TextEditingController lanAddressController;
  final TextEditingController pairingCodeController;

  @override
  Widget build(BuildContext context) {
    final profile = appState.syncProfile;
    final signedIn = appState.hasCloudAccount;

    return Column(
      children: [
        _SectionCard(
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
              _DiagnosticRow(
                label: '档案 ID',
                value: profile.creatorProfileId ?? '未加入账户',
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
        _SectionCard(
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
                  backgroundColor: const Color(0xFF1A2730),
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
                    ? '${profile.deviceName ?? '当前设备'} · ${profile.devicePlatform ?? 'unknown'}'
                    : '未登记',
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
        _SectionCard(
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
              const Divider(height: 1),
              _SyncDiagnosticsPanel(appState: appState),
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
                              appState.pendingSyncQueueCount == 0 ||
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
        _SectionCard(
          title: '高级',
          icon: Icons.tune_outlined,
          child: ExpansionTile(
            tilePadding: EdgeInsets.zero,
            childrenPadding: EdgeInsets.zero,
            title: const Text('局域网调试同步'),
            subtitle: const Text('仅用于开发联调或临时迁移，不是正式同步路径。'),
            children: [
              TextField(
                controller: lanAddressController,
                decoration: const InputDecoration(
                  labelText: 'LAN 调试地址',
                  hintText: 'http://192.168.1.8:47219',
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: pairingCodeController,
                decoration: const InputDecoration(
                  labelText: '调试配对码',
                  hintText: '桌面端生成的一次性配对码',
                ),
              ),
              const SizedBox(height: 12),
              _LanDebugChecklist(appState: appState),
              const SizedBox(height: 12),
              Align(
                alignment: Alignment.centerRight,
                child: Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  alignment: WrapAlignment.end,
                  children: [
                    OutlinedButton.icon(
                      onPressed: () => appState.saveLanDebugPairing(
                        lanDebugAddress: lanAddressController.text,
                        pairingCode: pairingCodeController.text,
                      ),
                      icon: const Icon(Icons.link_outlined),
                      label: const Text('保存调试配置'),
                    ),
                    FilledButton.icon(
                      onPressed: appState.syncProfile.canConnectLanDebug
                          ? appState.testLanDebugConnection
                          : null,
                      icon:
                          appState.syncProfile.status ==
                              SyncConnectionStatus.connecting
                          ? const SizedBox.square(
                              dimension: 18,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.wifi_tethering_outlined),
                      label: Text(
                        appState.syncProfile.status ==
                                SyncConnectionStatus.connecting
                            ? '连接中'
                            : '测试连接',
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        _SectionCard(
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
  const _SyncDiagnosticsPanel({required this.appState});

  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    final latestResolution = appState.syncResolutions.isEmpty
        ? null
        : appState.syncResolutions.first;
    final profile = appState.syncProfile;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.health_and_safety_outlined),
              const SizedBox(width: 12),
              Text('同步诊断', style: Theme.of(context).textTheme.titleSmall),
            ],
          ),
          const SizedBox(height: 12),
          _DiagnosticRow(
            label: '同步模式',
            value: syncTransportModeLabel(appState.syncTransportMode),
          ),
          _DiagnosticRow(label: '账户', value: profile.accountLabel ?? '未登录'),
          _DiagnosticRow(label: '工作区', value: profile.workspaceId ?? '无'),
          _DiagnosticRow(label: '设备 ID', value: profile.deviceId ?? '无'),
          _DiagnosticRow(
            label: '创作者档案',
            value: profile.creatorProfileId ?? '无',
          ),
          _DiagnosticRow(
            label: '云服务',
            value: profile.cloudBaseUrl.isEmpty ? '由系统配置提供' : profile.cloudBaseUrl,
          ),
          _DiagnosticRow(
            label: '连接状态',
            value: syncConnectionStatusLabel(profile.status),
          ),
          _DiagnosticRow(
            label: '上次游标',
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
            label: '队列状态',
            value:
                '待同步 ${appState.pendingSyncQueueCount} · 失败 ${appState.failedSyncQueueCount}',
          ),
          _DiagnosticRow(label: '最近错误', value: profile.lastError ?? '无'),
          _DiagnosticRow(
            label: '最近解决',
            value: latestResolution == null
                ? '无'
                : '${mobileSyncResolutionTypeLabel(latestResolution.resolutionType)} · ${latestResolution.watermarkUid}',
          ),
        ],
      ),
    );
  }
}

class _LanDebugChecklist extends StatelessWidget {
  const _LanDebugChecklist({required this.appState});

  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    final profile = appState.syncProfile;
    final checks = [
      _ChecklistItem(
        label: '调试地址',
        ok: _isLikelyLanDebugAddress(profile.lanDebugAddress),
        detail: profile.lanDebugAddress.isEmpty
            ? '填写电脑局域网地址'
            : profile.lanDebugAddress,
      ),
      _ChecklistItem(
        label: '配对码',
        ok: profile.lanDebugPairingCode.isNotEmpty,
        detail: profile.lanDebugPairingCode.isEmpty ? '填写当前配对码' : '已保存',
      ),
      _ChecklistItem(
        label: '同步模式',
        ok: appState.syncTransportMode == SyncTransportMode.lanDebug,
        detail: syncTransportModeLabel(appState.syncTransportMode),
      ),
      _ChecklistItem(
        label: '最近错误',
        ok: profile.lastError == null,
        detail: profile.lastError ?? '无',
      ),
    ];
    final ready = checks.every((item) => item.ok);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(
                ready ? Icons.verified_outlined : Icons.fact_check_outlined,
                color: ready ? const Color(0xFF59D2C2) : Colors.white70,
              ),
              const SizedBox(width: 12),
              Text('联调检查', style: Theme.of(context).textTheme.titleSmall),
            ],
          ),
          const SizedBox(height: 12),
          ...checks.map((item) => _ChecklistLine(item: item)),
        ],
      ),
    );
  }
}

class _ChecklistLine extends StatelessWidget {
  const _ChecklistLine({required this.item});

  final _ChecklistItem item;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            item.ok ? Icons.check_circle_outline : Icons.error_outline,
            size: 18,
            color: item.ok ? const Color(0xFF59D2C2) : const Color(0xFFFFC857),
          ),
          const SizedBox(width: 8),
          SizedBox(
            width: 76,
            child: Text(
              item.label,
              style: const TextStyle(color: Colors.white70),
            ),
          ),
          Expanded(child: Text(item.detail)),
        ],
      ),
    );
  }
}

class _ChecklistItem {
  const _ChecklistItem({
    required this.label,
    required this.ok,
    required this.detail,
  });

  final String label;
  final bool ok;
  final String detail;
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

bool _isLikelyLanDebugAddress(String value) {
  final uri = Uri.tryParse(value.trim());
  return uri != null &&
      uri.scheme == 'http' &&
      uri.host.isNotEmpty &&
      uri.host != '127.0.0.1' &&
      uri.host != 'localhost' &&
      uri.port == 47219;
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
      title: const Text('自动解决审计'),
      subtitle: Text(
        latest == null
            ? '还没有云端或调试下行自动解决记录。'
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

class _SectionCard extends StatelessWidget {
  const _SectionCard({
    required this.title,
    required this.icon,
    required this.child,
  });

  final String title;
  final IconData icon;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Card(
      elevation: 0,
      color: const Color(0xFF141B22),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(icon, color: const Color(0xFF59D2C2)),
                const SizedBox(width: 12),
                Text(title, style: Theme.of(context).textTheme.titleMedium),
              ],
            ),
            const SizedBox(height: 12),
            child,
          ],
        ),
      ),
    );
  }
}

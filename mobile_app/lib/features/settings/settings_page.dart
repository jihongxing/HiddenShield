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
  late final TextEditingController _desktopAddressController =
      TextEditingController(
        text: widget.appState.pairingProfile.desktopAddress,
      );
  late final TextEditingController _pairingCodeController =
      TextEditingController(text: widget.appState.pairingProfile.pairingCode);

  @override
  void dispose() {
    _creatorController.dispose();
    _desktopAddressController.dispose();
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
            desktopAddressController: _desktopAddressController,
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
    required this.desktopAddressController,
    required this.pairingCodeController,
  });

  final MobileAppState appState;
  final TextEditingController creatorController;
  final TextEditingController desktopAddressController;
  final TextEditingController pairingCodeController;

  @override
  Widget build(BuildContext context) {
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
                  helperText: '用于本机生成水印身份，不会默认上传。',
                ),
                onSubmitted: appState.updateCreatorLabel,
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
          title: '同步与备份',
          icon: Icons.sync_outlined,
          child: Column(
            children: [
              SwitchListTile(
                value: appState.desktopSyncEnabled,
                onChanged: appState.setDesktopSyncEnabled,
                title: const Text('桌面端同步'),
                subtitle: Text(
                  appState.desktopSyncEnabled
                      ? '已允许后续连接桌面同步服务。'
                      : '当前仅保存在本机，待接入桌面配对。',
                ),
                contentPadding: EdgeInsets.zero,
              ),
              const Divider(height: 1),
              const SizedBox(height: 12),
              TextField(
                controller: desktopAddressController,
                decoration: const InputDecoration(
                  labelText: '桌面端地址',
                  hintText: 'http://192.168.1.8:47219',
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: pairingCodeController,
                decoration: const InputDecoration(
                  labelText: '配对码',
                  hintText: '桌面端生成的一次性配对码',
                ),
              ),
              const SizedBox(height: 12),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                alignment: WrapAlignment.end,
                children: [
                  OutlinedButton.icon(
                    onPressed: () => appState.saveDesktopPairing(
                      desktopAddress: desktopAddressController.text,
                      pairingCode: pairingCodeController.text,
                    ),
                    icon: const Icon(Icons.link_outlined),
                    label: const Text('保存配对'),
                  ),
                  FilledButton.icon(
                    onPressed: appState.pairingProfile.canConnect
                        ? appState.testDesktopConnection
                        : null,
                    icon:
                        appState.pairingProfile.status ==
                            DesktopPairingStatus.connecting
                        ? const SizedBox.square(
                            dimension: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.wifi_tethering_outlined),
                    label: Text(
                      appState.pairingProfile.status ==
                              DesktopPairingStatus.connecting
                          ? '连接中'
                          : '测试连接',
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              ListTile(
                contentPadding: EdgeInsets.zero,
                leading: const Icon(Icons.desktop_windows_outlined),
                title: const Text('配对状态'),
                subtitle: Text(
                  appState.pairingProfile.desktopAddress.isEmpty
                      ? '还没有绑定桌面设备。'
                      : appState.pairingProfile.desktopAddress,
                ),
                trailing: Chip(
                  label: Text(
                    desktopPairingStatusLabel(appState.pairingProfile.status),
                  ),
                  materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  padding: EdgeInsets.zero,
                  backgroundColor: const Color(0xFF1A2730),
                  side: BorderSide.none,
                ),
              ),
              const Divider(height: 1),
              _PairingChecklist(appState: appState),
              const Divider(height: 1),
              const SizedBox(height: 12),
              SegmentedButton<SyncTransportMode>(
                segments: const [
                  ButtonSegment(
                    value: SyncTransportMode.mock,
                    icon: Icon(Icons.science_outlined),
                    label: Text('本地模拟'),
                  ),
                  ButtonSegment(
                    value: SyncTransportMode.http,
                    icon: Icon(Icons.cloud_sync_outlined),
                    label: Text('桌面 HTTP'),
                  ),
                ],
                selected: {appState.syncTransportMode},
                onSelectionChanged: (value) =>
                    appState.setSyncTransportMode(value.single),
              ),
              if (!appState.canUseHttpSync) ...[
                const SizedBox(height: 8),
                const Align(
                  alignment: Alignment.centerLeft,
                  child: Text(
                    '保存桌面地址和配对码后可启用桌面 HTTP。',
                    style: TextStyle(color: Colors.white70),
                  ),
                ),
              ],
              const SizedBox(height: 12),
              ListTile(
                contentPadding: EdgeInsets.zero,
                leading: const Icon(Icons.pending_actions_outlined),
                title: const Text('待同步队列'),
                subtitle: Text(
                  appState.failedSyncQueueCount == 0
                      ? '写入和取证命中会先进入本地队列。'
                      : '${appState.failedSyncQueueCount} 条同步失败，可稍后重试。',
                ),
                trailing: Text('${appState.pendingSyncQueueCount}'),
              ),
              const Divider(height: 1),
              _SyncResolutionSummary(resolutions: appState.syncResolutions),
              const Divider(height: 1),
              _SyncDiagnosticsPanel(appState: appState),
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
                          appState.isPullingDesktopChanges ||
                              !appState.canUseHttpSync
                          ? null
                          : appState.pullDesktopChanges,
                      icon: appState.isPullingDesktopChanges
                          ? const SizedBox.square(
                              dimension: 18,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.download_outlined),
                      label: Text(
                        appState.isPullingDesktopChanges ? '正在拉取' : '拉取桌面变更',
                      ),
                    ),
                    FilledButton.icon(
                      onPressed:
                          appState.isSyncing ||
                              appState.pendingSyncQueueCount == 0
                          ? null
                          : appState.syncPendingQueue,
                      icon: appState.isSyncing
                          ? const SizedBox.square(
                              dimension: 18,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.cloud_sync_outlined),
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
          title: '隐私与权限',
          icon: Icons.lock_outline,
          child: SwitchListTile(
            value: appState.anonymousFeedbackEnabled,
            onChanged: appState.setAnonymousFeedbackEnabled,
            title: const Text('匿名反馈'),
            subtitle: const Text('仅记录功能结果和稳定性状态，不上传原始媒体。'),
            contentPadding: EdgeInsets.zero,
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
            label: '桌面地址',
            value: appState.pairingProfile.desktopAddress.isEmpty
                ? '未配置'
                : appState.pairingProfile.desktopAddress,
          ),
          _DiagnosticRow(
            label: '配对状态',
            value: desktopPairingStatusLabel(appState.pairingProfile.status),
          ),
          _DiagnosticRow(
            label: '同步通道',
            value: syncTransportModeLabel(appState.syncTransportMode),
          ),
          _DiagnosticRow(
            label: '上次拉取游标',
            value: appState.pairingProfile.lastDesktopPullSince ?? '尚未拉取',
          ),
          _DiagnosticRow(
            label: '队列状态',
            value:
                '待同步 ${appState.pendingSyncQueueCount} · 失败 ${appState.failedSyncQueueCount}',
          ),
          _DiagnosticRow(
            label: '最近错误',
            value: appState.pairingProfile.lastError ?? '无',
          ),
          _DiagnosticRow(
            label: '最近自动解决',
            value: latestResolution == null
                ? '无'
                : '${mobileSyncResolutionTypeLabel(latestResolution.resolutionType)} · ${latestResolution.watermarkUid}',
          ),
        ],
      ),
    );
  }
}

class _PairingChecklist extends StatelessWidget {
  const _PairingChecklist({required this.appState});

  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    final checks = [
      _ChecklistItem(
        label: '桌面地址',
        ok: _isLikelyDesktopAddress(appState.pairingProfile.desktopAddress),
        detail: appState.pairingProfile.desktopAddress.isEmpty
            ? '填写电脑局域网地址'
            : appState.pairingProfile.desktopAddress,
      ),
      _ChecklistItem(
        label: '配对码',
        ok: appState.pairingProfile.pairingCode.isNotEmpty,
        detail: appState.pairingProfile.pairingCode.isEmpty
            ? '填写桌面端当前配对码'
            : '已保存',
      ),
      _ChecklistItem(
        label: '同步通道',
        ok: appState.syncTransportMode == SyncTransportMode.http,
        detail: syncTransportModeLabel(appState.syncTransportMode),
      ),
      _ChecklistItem(
        label: '最近错误',
        ok: appState.pairingProfile.lastError == null,
        detail: appState.pairingProfile.lastError ?? '无',
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

bool _isLikelyDesktopAddress(String value) {
  final uri = Uri.tryParse(value.trim());
  return uri != null &&
      uri.scheme == 'http' &&
      uri.host.isNotEmpty &&
      uri.host != '127.0.0.1' &&
      uri.host != 'localhost' &&
      uri.port == 47219;
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
            ? '还没有桌面下行自动解决记录。'
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

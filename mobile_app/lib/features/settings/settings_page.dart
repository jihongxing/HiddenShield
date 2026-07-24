import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../licensing/offline_license_panel.dart';
import '../../shared/models/workspace_context.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import '../../shared/widgets/tool_cards.dart';
import '../../sync/cloud_account_client.dart' show AccountDevice;

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
  late final TextEditingController _passwordController =
      TextEditingController();

  @override
  void dispose() {
    _creatorController.dispose();
    _accountController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '设置',
      subtitle: '管理账户、创作者身份、同步和订阅。',
      icon: Icons.settings_outlined,
      contextData: HsWorkspaceContext(
        eyebrow: '设置上下文',
        title: '隐私、同步与诊断',
        summary: '设置页解释当前账户、云同步、匿名反馈、日志导出和商业权益的边界。',
        metrics: [
          HsContextMetric(
            label: '当前方案',
            value: widget.appState.effectiveEntitlementLabel,
            tone: HsContextTone.ok,
          ),
          HsContextMetric(
            label: '云同步',
            value: widget.appState.canUseCloudSync ? '正式可用' : 'Creator',
            tone: widget.appState.canUseCloudSync
                ? HsContextTone.ok
                : HsContextTone.warning,
          ),
          const HsContextMetric(
            label: '隐私边界',
            value: '不传媒体路径',
            tone: HsContextTone.ok,
          ),
        ],
      ),
      children: [
        AnimatedBuilder(
          animation: widget.appState,
          builder: (context, _) => _SettingsContent(
            appState: widget.appState,
            bridge: widget.bridge,
            creatorController: _creatorController,
            accountController: _accountController,
            passwordController: _passwordController,
          ),
        ),
      ],
    );
  }
}

class _SettingsContent extends StatefulWidget {
  const _SettingsContent({
    required this.appState,
    required this.bridge,
    required this.creatorController,
    required this.accountController,
    required this.passwordController,
  });

  final MobileAppState appState;
  final WatermarkBridge bridge;
  final TextEditingController creatorController;
  final TextEditingController accountController;
  final TextEditingController passwordController;

  @override
  State<_SettingsContent> createState() => _SettingsContentState();
}

class _SettingsContentState extends State<_SettingsContent> {
  final TextEditingController _verificationCodeController =
      TextEditingController();
  String? _challengeId;
  String _authMode = 'code';
  bool _sendingCode = false;

  @override
  void dispose() {
    _verificationCodeController.dispose();
    super.dispose();
  }

  Future<void> _sendCode() async {
    setState(() => _sendingCode = true);
    final challenge = await widget.appState.createAuthChallenge(
      accountLabel: widget.accountController.text,
    );
    if (!mounted) return;
    if (challenge != null) {
      setState(() {
        _challengeId = challenge.challengeId;
        _verificationCodeController.text = challenge.fixtureCode ?? '';
      });
    }
    setState(() => _sendingCode = false);
  }

  Future<void> _login() async {
    final ok = await widget.appState.continueWithAccountPlaceholder(
      accountLabel: widget.accountController.text,
      password: _authMode == 'password' ? widget.passwordController.text : '',
      challengeId: _authMode == 'code' ? _challengeId : null,
      verificationCode: _authMode == 'code'
          ? _verificationCodeController.text
          : '',
    );
    if (ok && mounted) {
      widget.passwordController.clear();
      _verificationCodeController.clear();
      setState(() => _challengeId = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    final appState = widget.appState;
    final bridge = widget.bridge;
    final creatorController = widget.creatorController;
    final accountController = widget.accountController;
    final passwordController = widget.passwordController;
    final profile = appState.syncProfile;
    final signedIn = appState.hasCloudAccount;
    final canUseCloudSync = appState.canUseCloudSync;

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
                  labelText: '创作者身份',
                  helperText: '会写入版权记录，并在登录同一账户后保持双端一致。',
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
        _ProtectionReadinessPanel(bridge: bridge, appState: appState),
        const SizedBox(height: 12),
        OfflineLicensePanel(appState: appState),
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
                  helperText: '这是登录账号，不会写入作品水印。',
                ),
              ),
              if (!signedIn) ...[
                const SizedBox(height: 12),
                SegmentedButton<String>(
                  segments: const [
                    ButtonSegment(
                      value: 'code',
                      label: Text('验证码'),
                      icon: Icon(Icons.pin_outlined),
                    ),
                    ButtonSegment(
                      value: 'password',
                      label: Text('密码'),
                      icon: Icon(Icons.password_outlined),
                    ),
                  ],
                  selected: {_authMode},
                  onSelectionChanged: (value) =>
                      setState(() => _authMode = value.first),
                ),
                const SizedBox(height: 12),
                if (_authMode == 'password')
                  TextField(
                    controller: passwordController,
                    obscureText: true,
                    decoration: const InputDecoration(
                      labelText: '密码',
                      hintText: '账户密码',
                      helperText: '用于已有密码的账户登录。',
                    ),
                  )
                else
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _verificationCodeController,
                          keyboardType: TextInputType.number,
                          decoration: const InputDecoration(
                            labelText: '验证码',
                            hintText: '6 位验证码',
                            helperText: '验证码只用于账户登录，不会写入水印。',
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Padding(
                        padding: const EdgeInsets.only(top: 8),
                        child: SizedBox(
                          width: 88,
                          child: OutlinedButton(
                            onPressed: _sendingCode ? null : _sendCode,
                            child: Text(_sendingCode ? '发送中' : '发送'),
                          ),
                        ),
                      ),
                    ],
                  ),
              ],
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
              _EntitlementOverviewCard(profile: profile),
              const SizedBox(height: 12),
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
              _UsageLedgerCard(summary: appState.usageSummary),
              const SizedBox(height: 12),
              _CommercialHealthCard(summary: appState.commercialHealthSummary),
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
                        onPressed: _login,
                        icon: const Icon(Icons.login_outlined),
                        label: const Text('登录'),
                      ),
                    OutlinedButton.icon(
                      onPressed: () =>
                          _showSubscriptionSheet(context, appState),
                      icon: const Icon(Icons.workspace_premium_outlined),
                      label: const Text('查看订阅方案'),
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
                onChanged: signedIn && canUseCloudSync
                    ? appState.setCloudSyncEnabled
                    : null,
                title: const Text('开启云同步'),
                subtitle: Text(
                  signedIn && !canUseCloudSync
                      ? 'Creator 起开放正式云同步；当前账户可继续本地使用。'
                      : '同步版权库、验证记录、创作者档案和权益状态；不默认上传媒体文件。',
                ),
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
                  accountController.text = accountLabel;
                  _login();
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
                              !appState.canUseCloudSync ||
                              appState.syncTransportMode !=
                                  SyncTransportMode.cloud
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
                              !appState.canUseCloudSync ||
                              appState.syncTransportMode !=
                                  SyncTransportMode.cloud
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
        _DeviceSessionsPanel(appState: appState),
        const SizedBox(height: 12),
        _TeamWorkspacePanel(appState: appState),
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
                subtitle: const Text('仅记录功能结果、错误码、耗时和桶化信息，不上传原始媒体、加水印媒体或本地路径。'),
                contentPadding: EdgeInsets.zero,
              ),
              const Divider(height: 1),
              SwitchListTile(
                value: appState.experienceImprovementEnabled,
                onChanged: appState.setExperienceImprovementEnabled,
                title: const Text('体验改进'),
                subtitle: const Text('用于汇总成功率、失败率和重复错误，只展示本机可确认的匿名统计。'),
                contentPadding: EdgeInsets.zero,
              ),
              const Divider(height: 1),
              const SizedBox(height: 12),
              const Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  '默认不同步原始媒体、加水印媒体和本地文件路径；本地路径、媒体文件、受保护副本路径不进入云同步或匿名反馈。',
                  style: TextStyle(color: HsColors.textMuted),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        _AnonymousFeedbackPanel(appState: appState),
        const SizedBox(height: 12),
        _ExperienceImprovementPanel(appState: appState),
        const SizedBox(height: 12),
        _DataUsagePanel(appState: appState),
        const SizedBox(height: 12),
        _SupportFeedbackPanel(appState: appState),
        const SizedBox(height: 12),
        const HsPanel(
          title: '条款与边界',
          icon: Icons.policy_outlined,
          child: Column(
            children: [
              _DiagnosticRow(
                label: '隐私政策',
                value: '默认不同步原始媒体、加水印媒体和本地文件路径；云同步只同步账户、权益、版权记录元数据和验证记录摘要。',
              ),
              _DiagnosticRow(
                label: '用户协议',
                value: '报告、时间戳和指纹存证是技术辅助材料，不构成法律意见、司法鉴定或诉讼结果承诺。',
              ),
              _DiagnosticRow(
                label: '支付订阅',
                value: '权益以云端状态为准；确认支付只触发查单或刷新，不会绕过后端直接开通订阅。',
              ),
              _DiagnosticRow(
                label: '视频存证',
                value:
                    'L1 是视频音轨水印，桌面端可生成本地视频保护副本，移动端可验证视频音轨；L2 是视频指纹存证，需要 Creator 云同步权益；当前是视频指纹存证，不是视频画面盲水印；L3 视频画面盲水印按 Studio / Enterprise release gate 进入受控创建与领取。',
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _TeamWorkspacePanel extends StatelessWidget {
  const _TeamWorkspacePanel({required this.appState});

  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    final profile = appState.syncProfile;
    final canUseTeamWorkspace = appState.canUseTeamWorkspace;
    return HsPanel(
      title: 'Studio 团队空间',
      icon: Icons.groups_2_outlined,
      child: Column(
        children: [
          ListTile(
            contentPadding: EdgeInsets.zero,
            title: const Text('团队空间预留'),
            subtitle: const Text(
              '真实共享版权库、成员权限和团队审计仍在建设中；未来只共享版权元数据，不共享媒体文件和本地路径。',
            ),
            trailing: Chip(
              label: Text(canUseTeamWorkspace ? '入口已预留' : 'Studio 预留'),
              materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
              padding: EdgeInsets.zero,
              backgroundColor: HsColors.chip,
              side: BorderSide.none,
            ),
          ),
          const Divider(height: 1),
          const SizedBox(height: 12),
          _DiagnosticRow(label: '当前空间', value: profile.workspaceName ?? '个人空间'),
          _DiagnosticRow(
            label: '团队版权库',
            value: canUseTeamWorkspace ? '入口已预留，真实共享操作建设中' : 'Studio 起预留',
          ),
          _DiagnosticRow(
            label: '成员权限',
            value: canUseTeamWorkspace ? '模型已预留，成员管理建设中' : 'Studio 起预留',
          ),
          _DiagnosticRow(
            label: '团队审计',
            value: canUseTeamWorkspace ? '模型已预留，审计流水建设中' : 'Studio 起预留',
          ),
          const SizedBox(height: 12),
          Align(
            alignment: Alignment.centerRight,
            child: OutlinedButton.icon(
              onPressed: () => _showSubscriptionSheet(context, appState),
              icon: const Icon(Icons.workspace_premium_outlined),
              label: const Text('查看 Studio'),
            ),
          ),
        ],
      ),
    );
  }
}

class _DeviceSessionsPanel extends StatelessWidget {
  const _DeviceSessionsPanel({required this.appState});

  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    final signedIn = appState.hasCloudAccount;
    final devices = appState.cloudDevices;
    return HsPanel(
      title: '设备与会话',
      icon: Icons.devices_other_outlined,
      child: Column(
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('账户设备', style: TextStyle(fontSize: 16)),
                    SizedBox(height: 4),
                    Text(
                      '撤销其他设备会关闭其会话；本机退出请使用账户退出。',
                      style: TextStyle(color: HsColors.textMuted),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 12),
              OutlinedButton.icon(
                onPressed: signedIn ? appState.refreshCloudDevices : null,
                style: OutlinedButton.styleFrom(
                  minimumSize: const Size(0, 44),
                  padding: const EdgeInsets.symmetric(horizontal: 14),
                ),
                icon: const Icon(Icons.refresh_outlined),
                label: const Text('刷新'),
              ),
            ],
          ),
          const SizedBox(height: 12),
          const Divider(height: 1),
          if (!signedIn)
            const Padding(
              padding: EdgeInsets.only(top: 12),
              child: Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  '登录账户后可查看和撤销其他设备。',
                  style: TextStyle(color: HsColors.textMuted),
                ),
              ),
            )
          else if (devices.isEmpty)
            const Padding(
              padding: EdgeInsets.only(top: 12),
              child: Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  '暂无设备记录，刷新后会从云端读取。',
                  style: TextStyle(color: HsColors.textMuted),
                ),
              ),
            )
          else
            ...devices.map(
              (device) => _DeviceTile(
                device: device,
                onRename: () => _renameDevice(context, device),
                onRevoke: device.isCurrent || !device.registered
                    ? null
                    : () => _revokeDevice(context, device),
              ),
            ),
        ],
      ),
    );
  }

  Future<void> _renameDevice(BuildContext context, AccountDevice device) async {
    final controller = TextEditingController(text: device.name);
    final name = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('重命名设备'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(labelText: '设备名称'),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(controller.text.trim()),
            child: const Text('保存'),
          ),
        ],
      ),
    );
    controller.dispose();
    if (name == null || name.isEmpty || name == device.name) return;
    await appState.renameCloudDevice(deviceId: device.id, name: name);
  }

  Future<void> _revokeDevice(BuildContext context, AccountDevice device) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('撤销设备'),
        content: Text('撤销“${device.name}”后，该设备需要重新登录才能继续同步。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('撤销'),
          ),
        ],
      ),
    );
    if (confirmed == true) {
      await appState.revokeCloudDevice(device.id);
    }
  }
}

class _DeviceTile extends StatelessWidget {
  const _DeviceTile({
    required this.device,
    required this.onRename,
    required this.onRevoke,
  });

  final AccountDevice device;
  final VoidCallback onRename;
  final VoidCallback? onRevoke;

  @override
  Widget build(BuildContext context) {
    final status = device.isCurrent
        ? '当前设备'
        : device.registered
        ? '已登录'
        : '已撤销';
    return Column(
      children: [
        ListTile(
          contentPadding: EdgeInsets.zero,
          leading: Icon(
            device.platform.toLowerCase().contains('android') ||
                    device.platform.toLowerCase().contains('ios')
                ? Icons.phone_android_outlined
                : Icons.desktop_windows_outlined,
          ),
          title: Text(device.name),
          subtitle: Text(
            '${device.platform} · ${device.appVersion}\n最近使用：${_formatDateTime(device.lastSeenAt)} · 活跃会话 ${device.activeSessionCount}',
          ),
          isThreeLine: true,
          trailing: Chip(
            label: Text(status),
            materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
            padding: EdgeInsets.zero,
            backgroundColor: device.registered
                ? HsColors.chip
                : HsColors.surface,
            side: BorderSide.none,
          ),
        ),
        Align(
          alignment: Alignment.centerRight,
          child: Wrap(
            spacing: 8,
            children: [
              OutlinedButton.icon(
                onPressed: device.registered ? onRename : null,
                icon: const Icon(Icons.edit_outlined),
                label: const Text('重命名'),
              ),
              OutlinedButton.icon(
                onPressed: onRevoke,
                icon: const Icon(Icons.block_outlined),
                label: const Text('撤销'),
              ),
            ],
          ),
        ),
        const Divider(height: 20),
      ],
    );
  }
}

class _AnonymousFeedbackPanel extends StatefulWidget {
  const _AnonymousFeedbackPanel({required this.appState});

  final MobileAppState appState;

  @override
  State<_AnonymousFeedbackPanel> createState() =>
      _AnonymousFeedbackPanelState();
}

class _AnonymousFeedbackPanelState extends State<_AnonymousFeedbackPanel> {
  bool _sending = false;

  Future<void> _sendFeedback() async {
    setState(() => _sending = true);
    final result = await widget.appState.flushAnonymousFeedbackQueue();
    if (!mounted) {
      return;
    }
    setState(() => _sending = false);
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(result.message)));
  }

  @override
  Widget build(BuildContext context) {
    final status = widget.appState.anonymousFeedbackStatus;
    return HsPanel(
      title: '匿名反馈',
      icon: Icons.feedback_outlined,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _DiagnosticRow(label: '待发送', value: '${status.queuedEvents} 条'),
          _DiagnosticRow(label: '队列大小', value: '${status.queuedBytes} B'),
          _DiagnosticRow(
            label: '失败次数',
            value: '${status.consecutiveFailures} 次',
          ),
          _DiagnosticRow(
            label: '下次重试',
            value: _formatDateTime(status.nextRetryAt),
          ),
          _DiagnosticRow(
            label: '最近尝试',
            value: _formatDateTime(status.lastAttemptAt),
          ),
          _DiagnosticRow(
            label: '最近成功',
            value: _formatDateTime(status.lastSuccessAt),
          ),
          _DiagnosticRow(label: '最后错误', value: status.lastFlushError ?? '无'),
          const SizedBox(height: 8),
          Text(
            status.endpointConfigured ? '已配置上报地址' : '未配置上报地址，队列仅本地保留',
            style: const TextStyle(color: HsColors.textMuted),
          ),
          const SizedBox(height: 12),
          Align(
            alignment: Alignment.centerRight,
            child: FilledButton.icon(
              onPressed: status.telemetryEnabled && !_sending
                  ? _sendFeedback
                  : null,
              icon: _sending
                  ? const SizedBox.square(
                      dimension: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.send_outlined),
              label: Text(_sending ? '发送中' : '发送反馈'),
            ),
          ),
          const SizedBox(height: 8),
          const Text(
            '发送的是当前匿名队列中的反馈事件，不包含文件名、路径、作品指纹或原始媒体内容。',
            style: TextStyle(color: HsColors.textSubtle),
          ),
        ],
      ),
    );
  }
}

class _ExperienceImprovementPanel extends StatelessWidget {
  const _ExperienceImprovementPanel({required this.appState});

  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    final snapshot = appState.experienceImprovementSnapshot;
    return HsPanel(
      title: '体验改进',
      icon: Icons.insights_outlined,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          ListTile(
            contentPadding: EdgeInsets.zero,
            title: Text(snapshot.riskLabel),
            subtitle: Text(snapshot.enabled ? '当前正在本机汇总匿名体验指标' : '体验改进已关闭'),
            trailing: HsStatusChip(label: snapshot.riskLabel),
          ),
          const Divider(height: 1),
          const SizedBox(height: 12),
          _DiagnosticRow(label: '总事件', value: '${snapshot.totalEvents} 条'),
          _DiagnosticRow(
            label: '启动 / 成功',
            value: '${snapshot.totalEvents} / ${snapshot.successEvents}',
          ),
          _DiagnosticRow(
            label: '失败 / 诊断',
            value: '${snapshot.failureEvents} / ${snapshot.diagnosticEvents}',
          ),
          _DiagnosticRow(
            label: '转化率',
            value: '${(snapshot.conversionRate * 100).round()}%',
          ),
          _DiagnosticRow(
            label: '失败率',
            value: '${(snapshot.failureRate * 100).round()}%',
          ),
          _DiagnosticRow(
            label: '重复错误',
            value: '${snapshot.repeatedErrorCount} 次',
          ),
          _DiagnosticRow(
            label: '最后事件',
            value: _formatDateTime(snapshot.lastEventAt),
          ),
          if (snapshot.reasons.isNotEmpty) ...[
            const SizedBox(height: 4),
            Text(
              '需要关注：${snapshot.reasons.join('；')}',
              style: const TextStyle(color: HsColors.textMuted),
            ),
          ],
        ],
      ),
    );
  }
}

class _DataUsagePanel extends StatelessWidget {
  const _DataUsagePanel({required this.appState});

  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    final usage = appState.dataUsageSnapshot;
    return HsPanel(
      title: '占用',
      icon: Icons.storage_outlined,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _DiagnosticRow(label: '版权库', value: '${usage.vaultRecords} 条记录'),
          _DiagnosticRow(label: '同步队列', value: '${usage.syncQueueItems} 条'),
          _DiagnosticRow(
            label: '本地批量',
            value: '${usage.localBatchJobs} 个队列 / ${usage.localBatchItems} 个项目',
          ),
          _DiagnosticRow(label: '使用流水', value: '${usage.usageEvents} 条'),
          _DiagnosticRow(
            label: '匿名反馈',
            value: '${usage.anonymousFeedbackEvents} 条',
          ),
          _DiagnosticRow(label: '本机记录估算', value: usage.estimatedSizeLabel),
          const SizedBox(height: 6),
          Text(usage.note, style: const TextStyle(color: HsColors.textSubtle)),
        ],
      ),
    );
  }
}

class _SupportFeedbackPanel extends StatelessWidget {
  const _SupportFeedbackPanel({required this.appState});

  final MobileAppState appState;

  Future<void> _copy(BuildContext context, String text, String message) async {
    await Clipboard.setData(ClipboardData(text: text));
    if (context.mounted) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(message)));
    }
  }

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      title: '问题反馈',
      icon: Icons.contact_support_outlined,
      child: Column(
        children: [
          ListTile(
            contentPadding: EdgeInsets.zero,
            leading: const Icon(Icons.wechat_outlined),
            title: const Text('微信'),
            subtitle: const SelectableText('Zoro998877'),
            trailing: IconButton(
              tooltip: '复制微信号',
              onPressed: () => _copy(context, 'Zoro998877', '微信号已复制'),
              icon: const Icon(Icons.copy_outlined),
            ),
          ),
          const Divider(height: 1),
          ListTile(
            contentPadding: EdgeInsets.zero,
            leading: const Icon(Icons.alternate_email_outlined),
            title: const Text('邮箱'),
            subtitle: const SelectableText('jhx800@163.com'),
            trailing: IconButton(
              tooltip: '复制邮箱',
              onPressed: () => _copy(context, 'jhx800@163.com', '邮箱已复制'),
              icon: const Icon(Icons.copy_outlined),
            ),
          ),
          const Divider(height: 1),
          const SizedBox(height: 12),
          Align(
            alignment: Alignment.centerRight,
            child: OutlinedButton.icon(
              onPressed: () => _copy(
                context,
                appState.exportSafeDiagnosticLog(),
                '日志已复制到剪贴板',
              ),
              icon: const Icon(Icons.ios_share_outlined),
              label: const Text('导出日志'),
            ),
          ),
          const SizedBox(height: 8),
          const Align(
            alignment: Alignment.centerLeft,
            child: Text(
              '导出的日志是安全诊断文本，不包含媒体文件、本地路径、文件名或完整作品指纹。',
              style: TextStyle(color: HsColors.textSubtle),
            ),
          ),
        ],
      ),
    );
  }
}

class _ProtectionReadinessPanel extends StatelessWidget {
  const _ProtectionReadinessPanel({
    required this.bridge,
    required this.appState,
  });

  final WatermarkBridge bridge;
  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    return FutureBuilder(
      future: bridge.status(),
      builder: (context, snapshot) {
        final status = snapshot.data;
        final supportedKinds =
            status?.capabilities.supportedKinds ?? const <WatermarkAssetKind>[];
        final imageReady = supportedKinds.contains(WatermarkAssetKind.image);
        final audioReady = supportedKinds.contains(WatermarkAssetKind.audio);
        return HsPanel(
          title: '保护前检查',
          icon: Icons.health_and_safety_outlined,
          child: Column(
            children: [
              _DiagnosticRow(
                label: '本机保护',
                value: status == null ? '正在检查' : '已就绪',
              ),
              _DiagnosticRow(label: '图片写入', value: imageReady ? '可用' : '不可用'),
              _DiagnosticRow(label: '音频写入', value: audioReady ? '可用' : '不可用'),
              const _DiagnosticRow(
                label: '视频能力',
                value:
                    '移动端提供 L1 视频音轨验证和 L2 视频指纹存证读取；当前是视频指纹存证，不是视频画面盲水印；L3 视频画面盲水印按 Studio / Enterprise release gate 进入受控创建与领取。',
              ),
              _DiagnosticRow(
                label: '版权库',
                value: appState.isLoaded ? '已就绪' : '正在准备',
              ),
              _DiagnosticRow(
                label: '云同步',
                value: appState.canUseCloudSync
                    ? '可同步版权记录'
                    : appState.hasCloudAccount
                    ? '当前权益未开放'
                    : '登录后可查看权益',
              ),
              const _DiagnosticRow(
                label: '保存位置',
                value: '由系统保存或分享面板管理；不默认同步媒体文件和本地路径',
              ),
            ],
          ),
        );
      },
    );
  }
}

void _showSubscriptionSheet(BuildContext context, MobileAppState appState) {
  showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    backgroundColor: HsColors.background,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(top: Radius.circular(HsRadii.sheet)),
    ),
    builder: (context) => _SubscriptionSheet(appState: appState),
  );
}

class _UsageLedgerCard extends StatelessWidget {
  const _UsageLedgerCard({required this.summary});

  final UsageLedgerSummary summary;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: HsColors.surfaceRaised,
        borderRadius: BorderRadius.circular(HsRadii.card),
        border: Border.all(color: HsColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.insights_outlined, color: HsColors.accent),
              const SizedBox(width: 8),
              Text('处理统计', style: Theme.of(context).textTheme.titleSmall),
            ],
          ),
          const SizedBox(height: 8),
          _DiagnosticRow(label: '累计完成', value: '${summary.totalUnits} 次'),
          _DiagnosticRow(
            label: '类型分布',
            value: '图片 ${summary.imageUnits} / 音频 ${summary.audioUnits}',
          ),
          _DiagnosticRow(
            label: '最近完成',
            value: summary.lastUsedAt == null
                ? '暂无记录'
                : _formatDateTime(summary.lastUsedAt!),
          ),
          _DiagnosticRow(
            label: '最近功能',
            value: _usageFeatureLabel(summary.lastFeatureName),
          ),
        ],
      ),
    );
  }
}

class _CommercialHealthCard extends StatelessWidget {
  const _CommercialHealthCard({required this.summary});

  final CommercialHealthSummary summary;

  @override
  Widget build(BuildContext context) {
    final recentReport = summary.reportExportUnits > 0 ? '最近已导出' : '暂无记录';
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: HsColors.surfaceRaised,
        borderRadius: BorderRadius.circular(HsRadii.card),
        border: Border.all(color: HsColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.insights_outlined, color: HsColors.accent),
              const SizedBox(width: 8),
              Text('商业健康摘要', style: Theme.of(context).textTheme.titleSmall),
              const Spacer(),
              HsStatusChip(label: summary.accountScope),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            '云端看板负责全局账户、支付会话和权益分布；这里展示当前设备可确认的权益、同步和处理使用情况。',
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: HsColors.textMuted),
          ),
          const SizedBox(height: 12),
          _MetricTileGrid(
            items: [
              _MetricTileData(
                label: '当前权益',
                value: summary.entitlementPlanName,
                detail: entitlementStatusLabel(summary.entitlementStatus),
              ),
              _MetricTileData(
                label: '本地批量',
                value: '${summary.localBatchJobs} 个队列',
                detail:
                    '验证 ${summary.verifiedBatchItems} / 失败 ${summary.failedBatchItems}',
              ),
              _MetricTileData(
                label: '正式报告',
                value: recentReport,
                detail: 'Creator 权益内导出',
              ),
              _MetricTileData(
                label: '云同步',
                value: '成功 ${summary.cloudAcceptedEvents}',
                detail: '失败 ${summary.cloudFailureEvents}',
              ),
              _MetricTileData(
                label: 'L2 视频存证',
                value: '${summary.l2VideoNotaryCount} 次',
                detail: '只统计存证次数',
              ),
              _MetricTileData(
                label: '支付会话',
                value: _paymentStatusLabel(summary.latestPaymentSessionStatus),
                detail: '正式状态以云端为准',
              ),
            ],
          ),
          const SizedBox(height: 10),
          Text(
            summary.privacyNote,
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: HsColors.textSubtle),
          ),
        ],
      ),
    );
  }
}

class _MetricTileGrid extends StatelessWidget {
  const _MetricTileGrid({required this.items});

  final List<_MetricTileData> items;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = constraints.maxWidth >= 520 ? 3 : 2;
        return GridView.count(
          crossAxisCount: columns,
          crossAxisSpacing: 8,
          mainAxisSpacing: 8,
          childAspectRatio: columns == 3 ? 1.75 : 1.45,
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          children: [
            for (final item in items)
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: HsColors.surface,
                  borderRadius: BorderRadius.circular(HsRadii.card),
                  border: Border.all(color: HsColors.border),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      item.label,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.labelSmall?.copyWith(
                        color: HsColors.textSubtle,
                      ),
                    ),
                    Text(
                      item.value,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleSmall,
                    ),
                    Text(
                      item.detail,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: HsColors.textSubtle,
                      ),
                    ),
                  ],
                ),
              ),
          ],
        );
      },
    );
  }
}

class _MetricTileData {
  const _MetricTileData({
    required this.label,
    required this.value,
    required this.detail,
  });

  final String label;
  final String value;
  final String detail;
}

class _EntitlementOverviewCard extends StatelessWidget {
  const _EntitlementOverviewCard({required this.profile});

  final SyncProfile profile;

  @override
  Widget build(BuildContext context) {
    final plan = _planInfo(profile.entitlementPlanCode);
    final enabled = _enabledEntitlementSummary(profile.entitlementFeatures);
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: HsColors.surfaceRaised,
        borderRadius: BorderRadius.circular(HsRadii.card),
        border: Border.all(color: HsColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      plan.name,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: 4),
                    Text(
                      entitlementStatusLabel(profile.entitlementStatus),
                      style: const TextStyle(color: HsColors.textMuted),
                    ),
                  ],
                ),
              ),
              HsStatusChip(label: profile.entitlementPlanCode.toUpperCase()),
            ],
          ),
          const SizedBox(height: 12),
          Text(plan.summary, style: const TextStyle(color: HsColors.textMuted)),
          const SizedBox(height: 12),
          _DiagnosticRow(label: '已开放', value: enabled),
          _DiagnosticRow(
            label: '批量处理',
            value: profile.entitlementFeatures['batch_processing'] == true
                ? '已开放'
                : 'Creator 起开放',
          ),
          _DiagnosticRow(label: '云端视频', value: 'L3 未来能力，按订阅和额度开放'),
        ],
      ),
    );
  }
}

class _SubscriptionSheet extends StatefulWidget {
  const _SubscriptionSheet({required this.appState});

  final MobileAppState appState;

  @override
  State<_SubscriptionSheet> createState() => _SubscriptionSheetState();
}

class _SubscriptionSheetState extends State<_SubscriptionSheet> {
  String? _loadingPlan;
  bool _confirmingPayment = false;

  Future<void> _startPayment(String planCode) async {
    setState(() => _loadingPlan = planCode);
    await widget.appState.createBillingPaymentSession(planCode: planCode);
    if (mounted) {
      setState(() => _loadingPlan = null);
    }
  }

  Future<void> _confirmPayment() async {
    setState(() => _confirmingPayment = true);
    await widget.appState.reconcileLatestPaymentSession();
    if (mounted) {
      setState(() => _confirmingPayment = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final profile = widget.appState.syncProfile;
    final plans = _subscriptionPlans;
    final paymentSession = widget.appState.latestPaymentSession;
    final paymentSessionStatus =
        widget.appState.latestPaymentSessionStatus ?? 'created';
    final paymentMessage = widget.appState.latestPaymentMessage;
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.9,
      minChildSize: 0.55,
      maxChildSize: 0.96,
      builder: (context, scrollController) => ListView(
        controller: scrollController,
        padding: const EdgeInsets.all(HsSpacing.lg),
        children: [
          Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      '订阅方案',
                      style: Theme.of(context).textTheme.headlineSmall,
                    ),
                    const SizedBox(height: 6),
                    const Text(
                      'Free / Creator / Studio / Enterprise',
                      style: TextStyle(color: HsColors.textMuted),
                    ),
                  ],
                ),
              ),
              IconButton(
                tooltip: '关闭',
                onPressed: () => Navigator.of(context).pop(),
                icon: const Icon(Icons.close),
              ),
            ],
          ),
          const SizedBox(height: 16),
          HsPanel(
            color: HsColors.surfaceRaised,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('当前权益', style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 10),
                _DiagnosticRow(
                  label: '方案',
                  value: _planInfo(profile.entitlementPlanCode).name,
                ),
                _DiagnosticRow(
                  label: '状态',
                  value: entitlementStatusLabel(profile.entitlementStatus),
                ),
                _DiagnosticRow(
                  label: '已开放',
                  value: _enabledEntitlementSummary(
                    profile.entitlementFeatures,
                  ),
                ),
              ],
            ),
          ),
          if (paymentMessage != null || paymentSession != null) ...[
            const SizedBox(height: 12),
            HsMessageCard(
              icon: paymentSession == null
                  ? Icons.info_outline
                  : Icons.qr_code_2_outlined,
              title: paymentSession == null ? '开通提示' : '支付会话已创建',
              detail: paymentSession == null
                  ? paymentMessage ?? ''
                  : '${paymentMessage ?? ''}\n订单号：${paymentSession.providerOrderId} · 状态 $paymentSessionStatus · 有效期至 ${paymentSession.expiresAt}',
            ),
            if (paymentSession != null) ...[
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerRight,
                child: FilledButton.icon(
                  onPressed: _confirmingPayment ? null : _confirmPayment,
                  icon: _confirmingPayment
                      ? const SizedBox.square(
                          dimension: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.verified_user_outlined),
                  label: Text(_confirmingPayment ? '确认中' : '确认支付'),
                ),
              ),
            ],
          ],
          const SizedBox(height: 12),
          ...plans.map(
            (plan) => Padding(
              padding: const EdgeInsets.only(bottom: 12),
              child: _PlanComparisonCard(
                plan: plan,
                isCurrent: plan.code == profile.entitlementPlanCode,
                isLoading: _loadingPlan == plan.code,
                onStartPayment: plan.code == 'creator' || plan.code == 'studio'
                    ? () => _startPayment(plan.code)
                    : null,
              ),
            ),
          ),
          const HsMessageCard(
            icon: Icons.info_outline,
            title: '说明',
            detail:
                '批量队列是 Creator 订阅权益；L1 是视频音轨水印，桌面端可生成本地视频保护副本，移动端可验证视频音轨；L2 是视频指纹存证，需要 Creator 云同步权益；当前是视频指纹存证，不是视频画面盲水印；L3 视频画面盲水印按 Studio / Enterprise release gate 进入受控创建与领取；报告是技术辅助材料，不构成法律意见或司法鉴定；确认支付只刷新云端订单状态。',
          ),
        ],
      ),
    );
  }
}

class _PlanComparisonCard extends StatelessWidget {
  const _PlanComparisonCard({
    required this.plan,
    required this.isCurrent,
    required this.isLoading,
    required this.onStartPayment,
  });

  final _SubscriptionPlan plan;
  final bool isCurrent;
  final bool isLoading;
  final VoidCallback? onStartPayment;

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      color: plan.code == 'creator'
          ? HsColors.accent.withValues(alpha: 0.12)
          : HsColors.surface,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      plan.tag,
                      style: const TextStyle(color: HsColors.textMuted),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      plan.name,
                      style: Theme.of(context).textTheme.titleLarge,
                    ),
                  ],
                ),
              ),
              if (isCurrent) const HsStatusChip(label: '当前'),
            ],
          ),
          const SizedBox(height: 8),
          Text(plan.summary, style: const TextStyle(color: HsColors.textMuted)),
          const SizedBox(height: 12),
          ...plan.items.map(
            (item) => Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Icon(Icons.check_circle_outline, size: 18),
                  const SizedBox(width: 8),
                  Expanded(child: Text(item)),
                ],
              ),
            ),
          ),
          const SizedBox(height: 8),
          Text(plan.note, style: const TextStyle(color: HsColors.textMuted)),
          if (onStartPayment != null) ...[
            const SizedBox(height: 12),
            FilledButton.icon(
              onPressed: isCurrent || isLoading ? null : onStartPayment,
              icon: const Icon(Icons.payments_outlined),
              label: Text(
                isLoading
                    ? '创建中'
                    : isCurrent
                    ? '当前方案'
                    : '开通 ${plan.name}',
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _SubscriptionPlan {
  const _SubscriptionPlan({
    required this.code,
    required this.name,
    required this.tag,
    required this.summary,
    required this.items,
    required this.note,
  });

  final String code;
  final String name;
  final String tag;
  final String summary;
  final List<String> items;
  final String note;
}

const _subscriptionPlans = [
  _SubscriptionPlan(
    code: 'free',
    name: 'Free',
    tag: '当前入口',
    summary: '适合偶发创作者，本地单文件处理保持可用。',
    items: ['单文件图片写入与验证', '单文件音频写入与验证', '本地版权库'],
    note: '未购买时不开放批量处理、订阅内正式报告和正式云同步；可按记录单份购买报告。',
  ),
  _SubscriptionPlan(
    code: 'creator',
    name: 'Creator',
    tag: '个人主线',
    summary: '适合个人创作者，把版权保护变成持续工作流。',
    items: ['批量队列', '桌面端与移动端云同步', '正式报告'],
    note: '批量队列是订阅权益，不按本地处理次数扣点。',
  ),
  _SubscriptionPlan(
    code: 'studio',
    name: 'Studio',
    tag: '团队能力',
    summary: '适合工作室、MCN 和小团队统一管理作品。',
    items: ['团队空间入口预留', '成员权限模型预留', '团队审计模型预留'],
    note: '真实团队管理、共享版权库操作和更高并发仍在建设中。',
  ),
  _SubscriptionPlan(
    code: 'enterprise',
    name: 'Enterprise',
    tag: '定制',
    summary: '适合平台、法务团队和深度集成客户。',
    items: ['定制接入', '私有化部署', '专属云端视频处理'],
    note: '云端视频属于未来高阶能力，L3 不作为当前可承诺能力。',
  ),
];

_SubscriptionPlan _planInfo(String code) {
  return _subscriptionPlans.firstWhere(
    (plan) => plan.code == code,
    orElse: () => _subscriptionPlans.first,
  );
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
    final hasPending = appState.pendingSyncQueueCount > 0;
    final hasProblem =
        recoverableError ||
        profile.lastError?.isNotEmpty == true ||
        appState.failedSyncQueueCount > 0 ||
        hasPending;
    return ExpansionTile(
      tilePadding: EdgeInsets.zero,
      childrenPadding: EdgeInsets.zero,
      initiallyExpanded: hasProblem,
      leading: const Icon(Icons.manage_search_outlined),
      title: const Text('同步状态'),
      subtitle: Text(
        hasPending && appState.failedSyncQueueCount == 0 && !recoverableError
            ? '有版权记录等待上传'
            : hasProblem
            ? '有同步问题需要处理'
            : '当前同步状态正常',
      ),
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
                    borderRadius: BorderRadius.circular(HsRadii.card),
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
                        '当前账户、设备或工作区与云端不一致。重新登录会刷新授权、设备登记和工作区绑定。',
                        style: TextStyle(color: HsColors.textMuted),
                      ),
                      const SizedBox(height: 10),
                      Align(
                        alignment: Alignment.centerRight,
                        child: FilledButton.icon(
                          onPressed: onRecoverAccount,
                          icon: const Icon(Icons.login_outlined),
                          label: const Text('重新登录'),
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
        borderRadius: BorderRadius.circular(HsRadii.card),
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
                  style: const TextStyle(color: HsColors.textMuted),
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
            child: Text(
              label,
              style: const TextStyle(color: HsColors.textMuted),
            ),
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
      detail: '账户、设备或工作区授权不一致，请重新登录。',
      icon: Icons.warning_amber_outlined,
      iconColor: HsColors.warning,
      background: HsColors.warningSurface,
      border: HsColors.warning,
    );
  }
  if (!appState.hasCloudAccount) {
    return const _SyncHealthState(
      label: '未连接',
      detail: '本地功能可直接使用，云同步需要登录账户。',
      icon: Icons.cloud_off_outlined,
      iconColor: HsColors.textMuted,
      background: HsColors.surfaceRaised,
      border: HsColors.border,
    );
  }
  if (appState.failedSyncQueueCount > 0) {
    return _SyncHealthState(
      label: '有失败',
      detail: _mobileRetryDetail(appState),
      icon: Icons.error_outline,
      iconColor: HsColors.warning,
      background: HsColors.warningSurface,
      border: HsColors.warning,
    );
  }
  if (appState.pendingSyncQueueCount > 0) {
    return _SyncHealthState(
      label: '有待同步',
      detail: '还有 ${appState.pendingSyncQueueCount} 条版权记录等待上传。',
      icon: Icons.cloud_upload_outlined,
      iconColor: HsColors.accent,
      background: HsColors.surfaceRaised,
      border: HsColors.border,
    );
  }
  return const _SyncHealthState(
    label: '正常',
    detail: '同步状态正常，最近没有需要处理的同步问题。',
    icon: Icons.check_circle_outline,
    iconColor: HsColors.accent,
    background: HsColors.surfaceRaised,
    border: HsColors.border,
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
    'report_export' => '正式报告',
    'cloud_batch_processing' => '云端批量',
    'cloud_video_processing' => '云端视频',
    'cloud_sync' => '云同步',
    'priority_queue' => '优先队列',
    'team_workspace' => '团队空间',
    'api_access' => '定制接入',
    _ => key,
  };
}

String _usageFeatureLabel(String? featureName) {
  return switch (featureName) {
    'watermark_image' => '图片写入',
    'watermark_audio' => '音频写入',
    'watermark_video' => '视频处理',
    'report_export' => '正式报告',
    null || '' => '暂无记录',
    _ => featureName,
  };
}

String _paymentStatusLabel(String? status) {
  return switch (status) {
    'created' => '待支付',
    'pending' => '确认中',
    'succeeded' => '已确认',
    'failed' => '失败',
    'expired' => '已过期',
    'closed' => '已关闭',
    null || '' => '暂无会话',
    _ => status,
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
            style: TextStyle(color: HsColors.textMuted, fontSize: 12),
          ),
        ],
      ),
    );
  }
}

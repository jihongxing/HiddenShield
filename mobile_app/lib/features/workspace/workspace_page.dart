import 'dart:async';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../shared/models/workspace_context.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/action_card.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import '../../shared/widgets/tool_cards.dart';
import 'adaptive_embed_page.dart';
import 'local_batch_page.dart';
import 'media_file_kind.dart';
import 'video_metadata.dart';

class WorkspacePage extends StatelessWidget {
  const WorkspacePage({
    super.key,
    required this.bridge,
    required this.appState,
    required this.onOpenVault,
    required this.onOpenSettings,
  });

  final WatermarkBridge bridge;
  final MobileAppState appState;
  final VoidCallback onOpenVault;
  final Future<void> Function() onOpenSettings;

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '工作台',
      subtitle: '从这里开始处理图片、音频、视频音轨和批量队列。',
      icon: Icons.dashboard_customize_outlined,
      contextData: HsWorkspaceContext(
        eyebrow: '移动上下文',
        title: '创作者工作区',
        summary: '移动端使用同一产品模型：主流程单列展示，桌面右侧上下文转译为底部 sheet。',
        metrics: [
          HsContextMetric(
            label: '当前方案',
            value: appState.effectiveEntitlementLabel,
            tone: HsContextTone.ok,
          ),
          HsContextMetric(
            label: '批量队列',
            value: appState.canUseLocalBatchProcessing ? '可用' : 'Creator',
            tone: appState.canUseLocalBatchProcessing
                ? HsContextTone.ok
                : HsContextTone.warning,
          ),
          HsContextMetric(
            label: 'L3',
            value: '不开放',
            tone: HsContextTone.danger,
          ),
        ],
      ),
      children: [
        AnimatedBuilder(
          animation: appState,
          builder: (context, _) => _WorkspaceOverview(appState: appState),
        ),
        AnimatedBuilder(
          animation: appState,
          builder: (context, _) => _WorkspaceAccountRecoveryNotice(
            appState: appState,
            onOpenSettings: onOpenSettings,
          ),
        ),
        const SizedBox(height: HsSpacing.md),
        ActionCard(
          title: '作品写入',
          icon: Icons.upload_file_outlined,
          description: '选择图片或音频，系统自动识别类型并生成保护副本。',
          onTap: () => Navigator.of(context).push(
            MaterialPageRoute<void>(
              builder: (_) => AdaptiveEmbedPage(
                bridge: bridge,
                appState: appState,
                onOpenVault: onOpenVault,
              ),
            ),
          ),
        ),
        ActionCard(
          title: '批量队列',
          icon: Icons.queue_outlined,
          description: '连续处理多份作品，完成后逐个验证。',
          onTap: () => Navigator.of(context).push(
            MaterialPageRoute<void>(
              builder: (_) =>
                  LocalBatchPage(bridge: bridge, appState: appState),
            ),
          ),
        ),
        AnimatedBuilder(
          animation: appState,
          builder: (context, _) => _VideoTrackCard(appState: appState),
        ),
        AnimatedBuilder(
          animation: appState,
          builder: (context, _) => _CloudVideoFutureCard(
            appState: appState,
            onOpenVault: onOpenVault,
          ),
        ),
        AnimatedBuilder(
          animation: appState,
          builder: (context, _) =>
              _RecentTaskCard(records: appState.recentRecords),
        ),
      ],
    );
  }
}

class _WorkspaceAccountRecoveryNotice extends StatelessWidget {
  const _WorkspaceAccountRecoveryNotice({
    required this.appState,
    required this.onOpenSettings,
  });

  final MobileAppState appState;
  final Future<void> Function() onOpenSettings;

  @override
  Widget build(BuildContext context) {
    final error = appState.syncProfile.lastError?.trim() ?? '';
    if (!_isRecoverableWorkspaceSyncError(error)) {
      return const SizedBox.shrink();
    }
    return Padding(
      padding: const EdgeInsets.only(top: HsSpacing.md),
      child: HsMessageCard(
        icon: Icons.warning_amber_outlined,
        title: '账户状态需要恢复',
        detail: '',
        iconColor: HsColors.warning,
        detailWidget: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '$error\n当前设备可能已被撤销，请重新登录后再同步。',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: HsColors.textMuted,
                height: 1.35,
              ),
            ),
            const SizedBox(height: HsSpacing.md),
            Align(
              alignment: Alignment.centerRight,
              child: FilledButton.icon(
                onPressed: () => unawaited(onOpenSettings()),
                icon: const Icon(Icons.login_outlined),
                label: const Text('重新登录'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _WorkspaceOverview extends StatelessWidget {
  const _WorkspaceOverview({required this.appState});

  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    final recordCount = appState.records.length;
    final synced = appState.hasCloudAccount ? '已登录' : '未登录';
    final plan = appState.effectiveEntitlementLabel;
    final canUseBatch = appState.canUseLocalBatchProcessing;
    final latestBatch = appState.latestLocalBatchJob;
    final batchLabel = latestBatch == null
        ? (canUseBatch ? '可创建' : 'Creator')
        : '${latestBatch.items.length} 项';
    return HsPanel(
      color: HsColors.surfaceRaised,
      padding: const EdgeInsets.all(18),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(
                Icons.workspace_premium_outlined,
                color: HsColors.accent,
              ),
              const SizedBox(width: HsSpacing.md),
              Expanded(
                child: Text(
                  '版权保护工具箱',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w800,
                  ),
                ),
              ),
              HsStatusChip(
                label: synced,
                foregroundColor: appState.hasCloudAccount
                    ? HsColors.accent
                    : HsColors.textMuted,
                backgroundColor: appState.hasCloudAccount
                    ? HsColors.accent.withValues(alpha: 0.12)
                    : HsColors.chip,
              ),
            ],
          ),
          const SizedBox(height: HsSpacing.lg),
          GridView.count(
            crossAxisCount: 2,
            childAspectRatio: 1.95,
            crossAxisSpacing: HsSpacing.sm,
            mainAxisSpacing: HsSpacing.sm,
            physics: const NeverScrollableScrollPhysics(),
            shrinkWrap: true,
            children: [
              _MetricTile(label: '版权记录', value: '$recordCount'),
              _MetricTile(label: '当前权益', value: plan),
              _MetricTile(label: '批量队列', value: batchLabel),
              _MetricTile(label: '当前身份', value: appState.creatorLabel),
            ],
          ),
        ],
      ),
    );
  }
}

class _MetricTile extends StatelessWidget {
  const _MetricTile({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(HsSpacing.md),
      decoration: BoxDecoration(
        color: HsColors.surfaceMuted,
        borderRadius: BorderRadius.circular(HsRadii.card),
        border: Border.all(color: HsColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: Theme.of(
              context,
            ).textTheme.labelMedium?.copyWith(color: HsColors.textSubtle),
          ),
          const SizedBox(height: HsSpacing.xs),
          Text(
            value,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: Theme.of(
              context,
            ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w800),
          ),
        ],
      ),
    );
  }
}

class _CloudVideoFutureCard extends StatefulWidget {
  const _CloudVideoFutureCard({
    required this.appState,
    required this.onOpenVault,
  });

  final MobileAppState appState;
  final VoidCallback onOpenVault;

  @override
  State<_CloudVideoFutureCard> createState() => _CloudVideoFutureCardState();
}

class _CloudVideoFutureCardState extends State<_CloudVideoFutureCard> {
  final TextEditingController _taskController = TextEditingController();
  final TextEditingController _durationController = TextEditingController();
  bool _busy = false;
  String? _status;
  String? _error;
  String? _l2Status;
  String? _l2Error;
  String? _selectedFileName;
  List<int>? _selectedFileBytes;
  VideoMetadata? _selectedVideoMetadata;
  L3VideoVisualUploadTaskResult? _uploadResult;
  VaultRecord? _savedRecord;
  VaultRecord? _l2SavedRecord;

  @override
  void dispose() {
    _taskController.dispose();
    _durationController.dispose();
    super.dispose();
  }

  Future<void> _pickUploadSource() async {
    try {
      final result = await FilePicker.pickFiles(
        type: FileType.custom,
        allowedExtensions: const ['mp4'],
        withData: true,
      );
      final file = result?.files.single;
      if (file == null) return;
      if (!file.name.toLowerCase().endsWith('.mp4')) {
        setState(() {
          _error = 'L3 正式创建上传入口当前只接收 MP4；其他容器待 worker 转码入口放开后再承诺';
          _status = 'L3 创建上传任务失败';
        });
        return;
      }
      final bytes = file.bytes;
      if (bytes == null || bytes.isEmpty) {
        setState(() {
          _error = '请选择可读取字节的 MP4 文件；移动端不会把本地路径写入任务或同步字段';
          _status = 'L3 创建上传任务失败';
        });
        return;
      }
      setState(() {
        _selectedFileName = file.name;
        _selectedFileBytes = bytes;
        _selectedVideoMetadata = inspectVideoMetadata(
          Uint8List.fromList(bytes),
          fileName: file.name,
        );
        if (_selectedVideoMetadata != null) {
          _durationController.text = _selectedVideoMetadata!.durationSeconds
              .toStringAsFixed(2);
        }
        _error = null;
        _status = _selectedVideoMetadata == null
            ? '已选择 MP4；未读到可信尺寸 / 帧率，请人工确认时长后创建上传任务'
            : '已选择 MP4，可信视频探测完成，可继续创建上传任务';
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _error = mobileUserFacingErrorMessage(error, action: '选择 L3 MP4');
        _status = 'L3 创建上传任务失败';
      });
    }
  }

  Future<void> _pickL2Source() async {
    try {
      final result = await FilePicker.pickFiles(
        type: FileType.custom,
        allowedExtensions: supportedVideoExtensions,
        withData: true,
      );
      final file = result?.files.single;
      if (file == null) return;
      final extension = fileNameExtension(file.name);
      if (!supportedVideoExtensions.contains(extension)) {
        setState(() {
          _l2Error = 'L2 视频指纹存证当前支持 MP4 / MOV / MKV / WebM';
          _l2Status = 'L2 提交存证失败';
        });
        return;
      }
      final bytes = file.bytes;
      if (bytes == null || bytes.isEmpty) {
        setState(() {
          _l2Error = '请选择可读取字节的视频文件；移动端不会把本地路径写入任务或同步字段';
          _l2Status = 'L2 提交存证失败';
        });
        return;
      }
      final metadata = inspectVideoMetadata(
        Uint8List.fromList(bytes),
        fileName: file.name,
      );
      setState(() {
        _selectedFileName = file.name;
        _selectedFileBytes = bytes;
        _selectedVideoMetadata = metadata;
        if (metadata != null) {
          _durationController.text = metadata.durationSeconds.toStringAsFixed(
            2,
          );
        }
        _l2Error = null;
        _l2Status = metadata == null
            ? '已选择视频；未读到可信尺寸 / 帧率，请人工确认时长后提交 L2 存证'
            : '已选择视频，可信 metadata 探测完成，可提交 L2 存证';
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _l2Error = mobileUserFacingErrorMessage(error, action: '选择 L2 视频');
        _l2Status = 'L2 提交存证失败';
      });
    }
  }

  Future<void> _submitL2Notary() async {
    final bytes = _selectedFileBytes;
    final fileName = _selectedFileName;
    final metadata = _selectedVideoMetadata;
    final durationSeconds =
        metadata?.durationSeconds ??
        double.tryParse(_durationController.text.trim());
    if (bytes == null || fileName == null) {
      setState(() {
        _l2Status = '请先选择 MP4 / MOV / MKV / WebM 视频';
        _l2Error = null;
      });
      return;
    }
    if (durationSeconds == null || durationSeconds <= 0) {
      setState(() {
        _l2Status = 'L2 提交存证需要填写可确认的视频时长';
        _l2Error = null;
      });
      return;
    }
    setState(() {
      _busy = true;
      _l2Status = '正在生成不可逆 metadata 指纹并提交云端 notary';
      _l2Error = null;
      _l2SavedRecord = null;
    });
    try {
      final record = await widget.appState
          .createL2VideoFingerprintNotaryFromBytes(
            bytes: bytes,
            fileName: fileName,
            durationMs: (durationSeconds * 1000).round(),
            width: metadata?.width,
            height: metadata?.height,
            frameCount: metadata?.frameCount,
          );
      if (!mounted) return;
      setState(() {
        _l2SavedRecord = record;
        _l2Status = 'L2 视频指纹存证已保存到版权库：${record.videoNotaryId}';
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _l2Error = mobileUserFacingErrorMessage(error, action: '提交 L2 视频指纹存证');
        _l2Status = 'L2 提交存证失败';
      });
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _createUploadTask() async {
    final bytes = _selectedFileBytes;
    final fileName = _selectedFileName;
    final metadata = _selectedVideoMetadata;
    final durationSeconds =
        metadata?.durationSeconds ??
        double.tryParse(_durationController.text.trim());
    if (bytes == null || fileName == null) {
      setState(() {
        _status = '请先选择 MP4 视频';
        _error = null;
      });
      return;
    }
    if (durationSeconds == null || durationSeconds <= 0) {
      setState(() {
        _status = 'L3 创建任务需要填写可确认的视频时长';
        _error = null;
      });
      return;
    }
    setState(() {
      _busy = true;
      _status = '步骤 1/4 准备上传并校验权益';
      _error = null;
      _uploadResult = null;
    });
    try {
      setState(() => _status = '步骤 2/4 上传受控对象并回读哈希');
      final result = await widget.appState
          .createL3VideoVisualUploadTaskFromBytes(
            bytes: bytes,
            fileName: fileName,
            durationMs: (durationSeconds * 1000).round(),
            width: metadata?.width,
            height: metadata?.height,
            frameCount: metadata?.frameCount,
          );
      if (!mounted) return;
      setState(() {
        _uploadResult = result;
        _taskController.text = result.task.taskId;
        _status =
            '步骤 4/4 已创建 L3 任务 ${result.task.taskId}，等待 trusted worker 完成后再领取入库';
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _error = mobileUserFacingErrorMessage(error, action: '创建 L3 上传任务');
        _status = 'L3 创建上传任务失败';
      });
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _saveTask() async {
    final taskId = _taskController.text.trim();
    if (taskId.isEmpty) {
      setState(() {
        _status = '请输入已 succeeded 的 L3 taskId';
        _error = null;
      });
      return;
    }
    setState(() {
      _busy = true;
      _status = '正在下载 L3 MP4 成品并复核哈希';
      _error = null;
      _savedRecord = null;
    });
    try {
      final record = await widget.appState.saveL3VideoVisualTaskToVault(
        taskId: taskId,
        title: 'L3 视频画面盲水印成品',
      );
      if (!mounted) return;
      setState(() {
        _savedRecord = record;
        _status = 'L3 成品已写入版权库：${record.title}';
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _error = mobileUserFacingErrorMessage(error, action: '领取 L3 视频成品');
        _status = 'L3 成品领取失败';
      });
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final canUseCloudVideo =
        widget
            .appState
            .syncProfile
            .entitlementFeatures['cloud_video_processing'] ==
        true;
    final planCode = widget.appState.syncProfile.entitlementPlanCode;
    final canUseControlledL3 = planCode == 'studio' || planCode == 'enterprise';
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: HsPanel(
        radius: HsRadii.panel,
        color: HsColors.surfaceRaised,
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Icon(Icons.video_settings_outlined, color: HsColors.accent),
            const SizedBox(width: HsSpacing.md),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          '视频指纹存证与 L3 对象上传入口',
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                      ),
                      HsStatusChip(
                        label: canUseControlledL3
                            ? 'L3 可领取'
                            : canUseCloudVideo
                            ? '可查看'
                            : '未开放',
                      ),
                    ],
                  ),
                  const SizedBox(height: HsSpacing.sm),
                  Text(
                    canUseCloudVideo
                        ? '当前可提交 L2 视频指纹存证并查看同步来的视频指纹存证记录；L1 视频音轨水印可在移动端验证，带水印视频保护副本由桌面端生成。本机不做视频画面水印，L3 视频画面盲水印在 Studio / Enterprise release gate 中创建上传 MP4 到对象上传入口，等待 trusted worker succeeded 后领取成品。'
                        : '当前移动端可查看已同步的视频指纹存证记录；L1 视频音轨水印可在移动端验证，带水印视频保护副本由桌面端生成，本机不做视频画面水印，L2 提交需 Creator 云同步权益，L3 视频画面盲水印需 Studio / Enterprise 对象上传入口。',
                    style: const TextStyle(
                      color: HsColors.textMuted,
                      height: 1.45,
                    ),
                  ),
                  const SizedBox(height: HsSpacing.sm),
                  Text(
                    canUseControlledL3
                        ? '创建向导：可信视频探测、容量预检、准备上传、上传受控对象、创建云端 L3 任务、等待 trusted worker；同步和报告只保存 videoVisual* 收据元数据，不保存本地路径、对象 ref 或签名 URL，也不保存媒体字节。'
                        : 'L2 提交存证只生成不可逆 metadata 指纹包并调用云端 notary，不上传原始视频、不保存本地路径；升级到 Studio / Enterprise 后才可进入 L3 对象上传队列。',
                    style: const TextStyle(
                      color: HsColors.textMuted,
                      height: 1.45,
                    ),
                  ),
                  const SizedBox(height: HsSpacing.sm),
                  const Text(
                    '失败归因：权益 / 登录 / MP4 类型 / 可信尺寸 / 帧率 / 时长 / 上传授权 / 哈希回读 / 任务创建 / strategy_invalid 容量不足 / self_check_failed / worker_receipt_invalid。隐私边界：signed_object_upload_only_no_local_path_no_raw_video_sync。',
                    style: TextStyle(color: HsColors.textMuted, height: 1.45),
                  ),
                  const SizedBox(height: HsSpacing.md),
                  TextField(
                    controller: _durationController,
                    enabled: canUseControlledL3 && canUseCloudVideo && !_busy,
                    keyboardType: const TextInputType.numberWithOptions(
                      decimal: true,
                    ),
                    decoration: const InputDecoration(
                      labelText: '视频时长（秒）',
                      hintText: '优先使用可信视频探测；未读到时需人工确认',
                    ),
                  ),
                  if (_selectedFileName != null) ...[
                    const SizedBox(height: HsSpacing.sm),
                    Text(
                      '已选择：$_selectedFileName / ${_selectedFileBytes?.length ?? 0} bytes',
                      style: const TextStyle(
                        color: HsColors.textMuted,
                        height: 1.35,
                      ),
                    ),
                  ],
                  if (_selectedVideoMetadata != null) ...[
                    const SizedBox(height: HsSpacing.sm),
                    Text(
                      '可信视频探测：${_selectedVideoMetadata!.width}x${_selectedVideoMetadata!.height} / ${_selectedVideoMetadata!.frameRate.toStringAsFixed(2)}fps / ${_selectedVideoMetadata!.frameCount} 帧 / ${_selectedVideoMetadata!.durationSeconds.toStringAsFixed(2)}s / ${_selectedVideoMetadata!.probeSchema}',
                      style: const TextStyle(
                        color: HsColors.textMuted,
                        height: 1.35,
                      ),
                    ),
                  ],
                  if (_uploadResult != null) ...[
                    const SizedBox(height: HsSpacing.sm),
                    Text(
                      '任务 ${_uploadResult!.task.taskId}（${_uploadResult!.task.status}）/ UID ${_uploadResult!.watermarkUid} / ${_uploadResult!.sourceSha256}',
                      style: const TextStyle(
                        color: HsColors.textMuted,
                        height: 1.35,
                      ),
                    ),
                  ],
                  const SizedBox(height: HsSpacing.sm),
                  Wrap(
                    spacing: HsSpacing.sm,
                    runSpacing: HsSpacing.sm,
                    children: [
                      OutlinedButton.icon(
                        onPressed:
                            widget.appState.hasCloudSyncEntitlement && !_busy
                            ? () => unawaited(_pickL2Source())
                            : null,
                        icon: const Icon(Icons.video_file_outlined),
                        label: const Text('选择 L2 视频'),
                      ),
                      FilledButton.icon(
                        onPressed:
                            widget.appState.hasCloudSyncEntitlement && !_busy
                            ? () => unawaited(_submitL2Notary())
                            : null,
                        icon: _busy
                            ? const SizedBox(
                                width: 16,
                                height: 16,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                ),
                              )
                            : const Icon(Icons.verified_outlined),
                        label: Text(_busy ? '提交中' : '提交 L2 指纹存证'),
                      ),
                      if (_l2SavedRecord != null)
                        OutlinedButton.icon(
                          onPressed: widget.onOpenVault,
                          icon: const Icon(Icons.folder_shared_outlined),
                          label: const Text('查看 L2 记录'),
                        ),
                    ],
                  ),
                  if (_l2Status != null) ...[
                    const SizedBox(height: HsSpacing.sm),
                    Text(
                      _l2Status!,
                      style: const TextStyle(
                        color: HsColors.textMuted,
                        height: 1.35,
                      ),
                    ),
                  ],
                  if (_l2Error != null) ...[
                    const SizedBox(height: HsSpacing.sm),
                    Text(
                      _l2Error!,
                      style: const TextStyle(
                        color: HsColors.danger,
                        height: 1.35,
                      ),
                    ),
                  ],
                  const SizedBox(height: HsSpacing.sm),
                  TextField(
                    controller: _taskController,
                    enabled: canUseControlledL3 && canUseCloudVideo && !_busy,
                    decoration: const InputDecoration(
                      labelText: 'L3 taskId',
                      hintText: 'trusted worker succeeded 后输入或使用上方 taskId',
                    ),
                  ),
                  const SizedBox(height: HsSpacing.sm),
                  Wrap(
                    spacing: HsSpacing.sm,
                    runSpacing: HsSpacing.sm,
                    children: [
                      OutlinedButton.icon(
                        onPressed:
                            canUseControlledL3 && canUseCloudVideo && !_busy
                            ? () => unawaited(_pickUploadSource())
                            : null,
                        icon: const Icon(Icons.upload_file_outlined),
                        label: const Text('选择 MP4'),
                      ),
                      FilledButton.icon(
                        onPressed:
                            canUseControlledL3 && canUseCloudVideo && !_busy
                            ? () => unawaited(_createUploadTask())
                            : null,
                        icon: _busy
                            ? const SizedBox(
                                width: 16,
                                height: 16,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                ),
                              )
                            : const Icon(Icons.cloud_upload_outlined),
                        label: Text(_busy ? '创建中' : '创建并上传 L3 任务'),
                      ),
                      FilledButton.icon(
                        onPressed:
                            canUseControlledL3 && canUseCloudVideo && !_busy
                            ? () => unawaited(_saveTask())
                            : null,
                        icon: _busy
                            ? const SizedBox(
                                width: 16,
                                height: 16,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                ),
                              )
                            : const Icon(Icons.download_done_outlined),
                        label: Text(_busy ? '领取中' : '下载并保存版权库'),
                      ),
                      if (_savedRecord != null)
                        OutlinedButton.icon(
                          onPressed: widget.onOpenVault,
                          icon: const Icon(Icons.folder_shared_outlined),
                          label: const Text('查看版权库'),
                        ),
                    ],
                  ),
                  if (_status != null) ...[
                    const SizedBox(height: HsSpacing.sm),
                    Text(
                      _status!,
                      style: const TextStyle(
                        color: HsColors.textMuted,
                        height: 1.35,
                      ),
                    ),
                  ],
                  if (_error != null) ...[
                    const SizedBox(height: HsSpacing.sm),
                    Text(
                      _error!,
                      style: const TextStyle(
                        color: HsColors.danger,
                        height: 1.35,
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

bool _isRecoverableWorkspaceSyncError(String error) {
  final value = error.trim();
  if (value.isEmpty) {
    return false;
  }
  return value.contains('重新登录') ||
      value.contains('登录状态已失效') ||
      value.contains('设备未被当前账户授权') ||
      value.contains('工作区或设备与云端账户不匹配') ||
      value.contains('账户、设备或工作区授权不一致');
}

class _VideoTrackCard extends StatelessWidget {
  const _VideoTrackCard({required this.appState});

  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    final canUseCloudVideo =
        appState.syncProfile.entitlementFeatures['cloud_video_processing'] ==
        true;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: HsPanel(
        radius: HsRadii.panel,
        color: HsColors.surfaceRaised,
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Icon(Icons.graphic_eq_outlined, color: HsColors.accent),
            const SizedBox(width: HsSpacing.md),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          '视频音轨水印',
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                      ),
                      const HsStatusChip(label: 'L1'),
                    ],
                  ),
                  const SizedBox(height: HsSpacing.sm),
                  Text(
                    '这一层直接处理视频中的音轨，和视频指纹存证是两层不同能力。移动端可验证 L1 视频音轨水印；带水印视频保护副本由桌面端本地生成，不需要 Creator。',
                    style: const TextStyle(
                      color: HsColors.textMuted,
                      height: 1.45,
                    ),
                  ),
                  const SizedBox(height: HsSpacing.sm),
                  Text(
                    canUseCloudVideo
                        ? 'L2 视频指纹存证在下方单独展示，当前可继续验证 L1 视频音轨。'
                        : 'L2 视频指纹存证在下方单独展示，当前只是不开放提交，不影响 L1 视频音轨验证。',
                    style: const TextStyle(
                      color: HsColors.textMuted,
                      height: 1.45,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _RecentTaskCard extends StatelessWidget {
  const _RecentTaskCard({required this.records});

  final List<VaultRecord> records;

  @override
  Widget build(BuildContext context) {
    if (records.isEmpty) {
      return const ActionCard(
        title: '最近任务',
        icon: Icons.history_outlined,
        description: '完成保护副本或验证后，这里会显示最近结果和版本记录。',
      );
    }

    final latest = records.first;
    return HsPanel(
      radius: HsRadii.panel,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.history_outlined, color: HsColors.accent),
              const SizedBox(width: 12),
              Text('最近任务', style: Theme.of(context).textTheme.titleMedium),
            ],
          ),
          const SizedBox(height: 12),
          Text(
            '${vaultRecordSourceLabel(latest.source)} · ${_kindLabel(latest.kind)} · ${latest.title}',
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          const SizedBox(height: 6),
          Text(
            '版权编号: ${latest.watermarkUid}',
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(color: HsColors.textMuted),
          ),
        ],
      ),
    );
  }
}

String _kindLabel(WatermarkAssetKind kind) {
  return switch (kind) {
    WatermarkAssetKind.image => '图片',
    WatermarkAssetKind.audio => '音频',
    WatermarkAssetKind.video => '视频',
  };
}

import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../shared/models/workspace_context.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import '../../shared/widgets/tool_cards.dart';
import 'audio_metadata.dart';
import 'batch_file_reader.dart';
import 'watermark_payload_seed.dart';

class LocalBatchPage extends StatefulWidget {
  const LocalBatchPage({
    super.key,
    required this.bridge,
    required this.appState,
    this.showAppBar = true,
  });

  final WatermarkBridge bridge;
  final MobileAppState appState;
  final bool showAppBar;

  @override
  State<LocalBatchPage> createState() => _LocalBatchPageState();
}

class _LocalBatchPageState extends State<LocalBatchPage> {
  bool _processingMedia = false;

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.appState,
      builder: (context, _) {
        final canUseBatch = widget.appState.canUseLocalBatchProcessing;
        final page = FeaturePageScaffold(
          title: '批量队列',
          subtitle: '连续生成保护副本，完成后逐个验证版权编号',
          showBackButton: widget.showAppBar,
          contextData: HsWorkspaceContext(
            eyebrow: '队列上下文',
            title: '批量任务',
            summary: '批量队列从 Creator 起开放，失败项保留在队列里等待重试，不会被表现成成功入库。',
            metrics: [
              HsContextMetric(
                label: '权益',
                value: canUseBatch ? '可创建队列' : 'Creator',
                tone: canUseBatch ? HsContextTone.ok : HsContextTone.warning,
              ),
              HsContextMetric(
                label: '当前队列',
                value: widget.appState.latestLocalBatchJob == null
                    ? '暂无'
                    : '${widget.appState.latestLocalBatchJob!.items.length} 项',
                tone: HsContextTone.muted,
              ),
              const HsContextMetric(
                label: '计费',
                value: '本地不扣点',
                tone: HsContextTone.ok,
              ),
            ],
          ),
          children: [
            _EntitlementCard(
              planName: widget.appState.effectiveEntitlementLabel,
              canUseBatch: canUseBatch,
            ),
            const SizedBox(height: HsSpacing.md),
            if (canUseBatch)
              _BatchQueue(
                job: widget.appState.latestLocalBatchJob,
                onPickFiles: _pickFiles,
                onPause: _pauseJob,
                onResume: _resumeJob,
                onCancel: _cancelJob,
                onRetryFailed: _retryFailedItems,
                onProcessMedia: _processQueuedMedia,
                processingMedia: _processingMedia,
              )
            else
              const _BatchGateCard(),
          ],
        );
        if (!widget.showAppBar) {
          return page;
        }
        return SafeArea(child: page);
      },
    );
  }

  Future<void> _pickFiles() async {
    if (!await _authorizeBatchExecution()) return;
    final result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowMultiple: true,
      withData: false,
      allowedExtensions: const [
        'jpg',
        'jpeg',
        'png',
        'bmp',
        'webp',
        'tiff',
        'wav',
        'mp3',
        'aac',
        'flac',
        'ogg',
        'm4a',
      ],
    );
    final files = result?.files;
    if (files == null || files.isEmpty) {
      return;
    }
    final now = DateTime.now();
    final jobId = 'batch-${now.microsecondsSinceEpoch}';
    final job = LocalBatchJob(
      id: jobId,
      status: BatchJobStatus.queued,
      createdAt: now,
      updatedAt: now,
      entitlementPlanCode: widget.appState.syncProfile.entitlementPlanCode,
      entitlementStatus: widget.appState.syncProfile.entitlementStatus,
      items: [
        for (var index = 0; index < files.length; index++)
          _buildItem(files[index], jobId, now, index),
      ],
    );
    await widget.appState.saveLocalBatchJob(job);
  }

  LocalBatchItem _buildItem(
    PlatformFile file,
    String jobId,
    DateTime now,
    int index,
  ) {
    final inputRef = file.path ?? file.name;
    final mediaKind = _mediaKindFromName(file.name);
    final supported = mediaKind != BatchMediaKind.unsupported;
    return LocalBatchItem(
      id: 'batch-item-${now.microsecondsSinceEpoch}-$index',
      jobId: jobId,
      inputRef: inputRef,
      fileName: file.name,
      mediaKind: mediaKind,
      status: supported ? BatchItemStatus.queued : BatchItemStatus.failed,
      attempts: 0,
      createdAt: now,
      updatedAt: now,
      lastError: supported ? null : '仅支持图片和音频文件',
    );
  }

  void _pauseJob() {
    final job = widget.appState.latestLocalBatchJob;
    if (job == null || job.status != BatchJobStatus.queued) {
      return;
    }
    widget.appState.saveLocalBatchJob(
      job.copyWith(status: BatchJobStatus.paused, updatedAt: DateTime.now()),
    );
  }

  void _resumeJob() {
    final job = widget.appState.latestLocalBatchJob;
    if (job == null || job.status != BatchJobStatus.paused) {
      return;
    }
    widget.appState.saveLocalBatchJob(
      job.copyWith(status: BatchJobStatus.queued, updatedAt: DateTime.now()),
    );
  }

  void _cancelJob() {
    final job = widget.appState.latestLocalBatchJob;
    if (job == null || job.status == BatchJobStatus.cancelled) {
      return;
    }
    final now = DateTime.now();
    widget.appState.saveLocalBatchJob(
      job.copyWith(
        status: BatchJobStatus.cancelled,
        updatedAt: now,
        items: [
          for (final item in job.items)
            item.status == BatchItemStatus.queued ||
                    item.status == BatchItemStatus.running
                ? item.copyWith(
                    status: BatchItemStatus.cancelled,
                    updatedAt: now,
                  )
                : item,
        ],
      ),
    );
  }

  void _retryFailedItems() {
    final job = widget.appState.latestLocalBatchJob;
    if (job == null) {
      return;
    }
    final now = DateTime.now();
    widget.appState.saveLocalBatchJob(
      job.copyWith(
        status: BatchJobStatus.queued,
        updatedAt: now,
        items: [
          for (final item in job.items)
            item.status == BatchItemStatus.failed
                ? item.copyWith(
                    status: BatchItemStatus.queued,
                    attempts: item.attempts + 1,
                    updatedAt: now,
                    clearLastError: true,
                    clearOutputRef: true,
                    clearVaultRecordId: true,
                    clearWriteVerificationStatus: true,
                    clearWriteVerificationMessage: true,
                  )
                : item,
        ],
      ),
    );
  }

  Future<void> _processQueuedMedia() async {
    if (_processingMedia) {
      return;
    }
    if (!await _authorizeBatchExecution()) return;
    var job = widget.appState.latestLocalBatchJob;
    if (job == null || job.status != BatchJobStatus.queued) {
      return;
    }
    final hasQueuedMedia = job.items.any(
      (item) =>
          item.mediaKind != BatchMediaKind.unsupported &&
          _canProcessMediaItem(item),
    );
    if (!hasQueuedMedia) {
      return;
    }

    setState(() => _processingMedia = true);
    try {
      await _recoverInterruptedRunningItems(job);
      while (mounted) {
        job = widget.appState.latestLocalBatchJob;
        if (job == null || job.status != BatchJobStatus.queued) {
          break;
        }
        final nextItem = _nextQueuedMedia(job);
        if (nextItem == null) {
          break;
        }
        await _processMediaItem(job, nextItem);
      }
    } finally {
      if (mounted) {
        setState(() => _processingMedia = false);
      }
    }
  }

  Future<bool> _authorizeBatchExecution() async {
    final authorization = await widget.appState.authorizeLocalExecution(
      'batch_processing',
    );
    if (authorization.allowed) return true;
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('本地批量处理需要有效 Creator 云权益或本机离线许可证。')),
      );
    }
    return false;
  }

  Future<void> _recoverInterruptedRunningItems(LocalBatchJob job) async {
    if (!job.items.any((item) => item.status == BatchItemStatus.running)) {
      return;
    }
    final now = DateTime.now();
    await widget.appState.saveLocalBatchJob(
      job.copyWith(
        updatedAt: now,
        items: [
          for (final item in job.items)
            item.status == BatchItemStatus.running
                ? item.copyWith(
                    status: BatchItemStatus.failed,
                    updatedAt: now,
                    lastError: '上次处理未完成，已转为可重试状态',
                    clearWriteVerificationStatus: true,
                    clearWriteVerificationMessage: true,
                  )
                : item,
        ],
      ),
    );
  }

  LocalBatchItem? _nextQueuedMedia(LocalBatchJob job) {
    for (final item in job.items) {
      if (item.mediaKind != BatchMediaKind.unsupported &&
          _canProcessMediaItem(item)) {
        return item;
      }
    }
    return null;
  }

  Future<void> _processMediaItem(LocalBatchJob job, LocalBatchItem item) async {
    final startedAt = DateTime.now();
    await widget.appState.saveLocalBatchJob(
      _replaceBatchItem(
        job,
        item.copyWith(
          status: BatchItemStatus.running,
          updatedAt: startedAt,
          clearLastError: true,
          clearOutputRef: true,
          clearVaultRecordId: true,
          clearWriteVerificationStatus: true,
          clearWriteVerificationMessage: true,
        ),
      ),
    );

    try {
      final bytes = await readBatchFileBytes(item.inputRef);
      final durationError = _audioDurationRuleError(item, bytes);
      if (durationError != null) {
        throw _BatchUserFacingError(durationError);
      }
      final kind = _watermarkKindForBatchItem(item);
      final result = await widget.bridge.write(
        WatermarkWriteRequest(
          kind: kind,
          bytes: bytes,
          seed: buildPayloadSeed(bytes, widget.appState),
        ),
      );
      final trustedTimeAttestation = await widget.appState
          .requestTrustedTimeAttestation();
      final record = widget.appState.addWriteResult(
        result: result,
        fileName: item.fileName,
        allowRewrite: false,
        trustedTimeAttestation: trustedTimeAttestation,
      );
      final verified = result.verification.verified;
      if (verified) {
        await widget.appState.appendUsageForWriteResult(
          result: result,
          vaultRecordId: record.id,
          pipelineId: '${job.id}/${item.id}',
        );
      }
      final finishedAt = DateTime.now();
      final currentJob = widget.appState.latestLocalBatchJob ?? job;
      await widget.appState.saveLocalBatchJob(
        _replaceBatchItem(
          currentJob,
          item.copyWith(
            status: verified
                ? BatchItemStatus.verified
                : BatchItemStatus.failed,
            attempts: item.attempts + 1,
            updatedAt: finishedAt,
            lastError: verified ? null : result.verification.message,
            clearLastError: verified,
            vaultRecordId: record.id,
            writeVerificationStatus: verified
                ? WriteVerificationStatus.verified
                : WriteVerificationStatus.failed,
            writeVerificationMessage: result.verification.message,
          ),
        ),
      );
    } catch (error) {
      final currentJob = widget.appState.latestLocalBatchJob ?? job;
      await widget.appState.saveLocalBatchJob(
        _replaceBatchItem(
          currentJob,
          item.copyWith(
            status: BatchItemStatus.failed,
            attempts: item.attempts + 1,
            updatedAt: DateTime.now(),
            lastError: _friendlyBatchError(error),
            clearOutputRef: true,
            clearVaultRecordId: true,
            clearWriteVerificationStatus: true,
            clearWriteVerificationMessage: true,
          ),
        ),
      );
    }
  }
}

class _EntitlementCard extends StatelessWidget {
  const _EntitlementCard({required this.planName, required this.canUseBatch});

  final String planName;
  final bool canUseBatch;

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      child: Row(
        children: [
          Icon(
            canUseBatch ? Icons.playlist_add_check : Icons.lock_outline,
            color: canUseBatch ? HsColors.accent : HsColors.warning,
          ),
          const SizedBox(width: HsSpacing.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(planName, style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: HsSpacing.xs),
                Text(
                  canUseBatch ? '批量队列已开放' : 'Creator 起开放批量队列',
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

class _BatchGateCard extends StatelessWidget {
  const _BatchGateCard();

  @override
  Widget build(BuildContext context) {
    return HsMessageCard(
      icon: Icons.workspace_premium_outlined,
      iconColor: HsColors.warning,
      title: 'Free 可使用单文件写入',
      detail: '批量队列是 Creator 订阅权益。Free 不进入文件选择，也不会创建批量队列。',
      detailWidget: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            '批量队列是 Creator 订阅权益。Free 不进入文件选择，也不会创建批量队列。',
            style: TextStyle(color: HsColors.textMuted),
          ),
          const SizedBox(height: HsSpacing.md),
          FilledButton.icon(
            onPressed: () => Navigator.of(context).pop(),
            icon: const Icon(Icons.arrow_back),
            label: const Text('返回工作台'),
          ),
        ],
      ),
    );
  }
}

class _BatchQueue extends StatelessWidget {
  const _BatchQueue({
    required this.job,
    required this.onPickFiles,
    required this.onPause,
    required this.onResume,
    required this.onCancel,
    required this.onRetryFailed,
    required this.onProcessMedia,
    required this.processingMedia,
  });

  final LocalBatchJob? job;
  final VoidCallback onPickFiles;
  final VoidCallback onPause;
  final VoidCallback onResume;
  final VoidCallback onCancel;
  final VoidCallback onRetryFailed;
  final VoidCallback onProcessMedia;
  final bool processingMedia;

  @override
  Widget build(BuildContext context) {
    final job = this.job;
    final canPause = job?.status == BatchJobStatus.queued;
    final canCancel = job != null && job.status != BatchJobStatus.cancelled;
    final canProcessMedia =
        job != null &&
        job.status == BatchJobStatus.queued &&
        job.items.any(
          (item) =>
              item.mediaKind != BatchMediaKind.unsupported &&
              _canProcessMediaItem(item),
        );
    return HsPanel(
      title: '批量队列',
      icon: Icons.queue_outlined,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          HsPreviewBox(
            height: 132,
            child: Center(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  const Icon(Icons.upload_file_outlined),
                  const SizedBox(height: HsSpacing.sm),
                  Text(
                    job == null ? '选择批量文件' : '重新选择文件',
                    style: const TextStyle(color: HsColors.textMuted),
                  ),
                  const SizedBox(height: HsSpacing.xs),
                  const Text(
                    '支持图片和音频。音频需满足 30 秒以上规则。',
                    style: TextStyle(color: HsColors.textMuted, fontSize: 12),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: HsSpacing.lg),
          Wrap(
            spacing: HsSpacing.sm,
            runSpacing: HsSpacing.sm,
            children: const [
              HsStatusChip(label: '逐个写入'),
              HsStatusChip(label: '完成后验证'),
              HsStatusChip(label: '失败可重试'),
              HsStatusChip(label: '本地执行'),
            ],
          ),
          const SizedBox(height: HsSpacing.lg),
          Wrap(
            spacing: HsSpacing.sm,
            runSpacing: HsSpacing.sm,
            children: [
              FilledButton.icon(
                onPressed: onPickFiles,
                icon: const Icon(Icons.add),
                label: const Text('创建队列'),
              ),
              FilledButton.icon(
                onPressed: canProcessMedia && !processingMedia
                    ? onProcessMedia
                    : null,
                icon: Icon(
                  processingMedia ? Icons.hourglass_top : Icons.play_arrow,
                ),
                label: Text(processingMedia ? '正在处理队列' : '开始处理队列'),
              ),
              if (job?.status == BatchJobStatus.paused)
                OutlinedButton.icon(
                  onPressed: onResume,
                  icon: const Icon(Icons.play_arrow),
                  label: const Text('继续队列'),
                )
              else
                OutlinedButton.icon(
                  onPressed: canPause ? onPause : null,
                  icon: const Icon(Icons.pause),
                  label: const Text('暂停全部'),
                ),
              OutlinedButton.icon(
                onPressed: canCancel ? onCancel : null,
                icon: const Icon(Icons.close),
                label: const Text('取消队列'),
              ),
            ],
          ),
          if (job != null) ...[
            const SizedBox(height: HsSpacing.lg),
            _BatchSummary(job: job, onRetryFailed: onRetryFailed),
            const SizedBox(height: HsSpacing.md),
            for (final item in job.items) ...[
              _BatchItemTile(item: item),
              const SizedBox(height: HsSpacing.sm),
            ],
          ],
        ],
      ),
    );
  }
}

class _BatchSummary extends StatelessWidget {
  const _BatchSummary({required this.job, required this.onRetryFailed});

  final LocalBatchJob job;
  final VoidCallback onRetryFailed;

  @override
  Widget build(BuildContext context) {
    final failed = job.items
        .where((item) => item.status == BatchItemStatus.failed)
        .length;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Wrap(
          spacing: HsSpacing.sm,
          runSpacing: HsSpacing.sm,
          children: [
            HsStatusChip(label: _jobStatusLabel(job.status)),
            HsStatusChip(label: '总数 ${job.items.length}'),
            HsStatusChip(label: '待处理 ${_count(job, BatchItemStatus.queued)}'),
            HsStatusChip(label: '写入中 ${_count(job, BatchItemStatus.running)}'),
            HsStatusChip(label: '已验证 ${_count(job, BatchItemStatus.verified)}'),
            HsStatusChip(label: '需处理 $failed'),
            HsStatusChip(
              label: '已取消 ${_count(job, BatchItemStatus.cancelled)}',
            ),
          ],
        ),
        if (failed > 0) ...[
          const SizedBox(height: HsSpacing.md),
          OutlinedButton.icon(
            onPressed: onRetryFailed,
            icon: const Icon(Icons.refresh),
            label: const Text('重试失败项'),
          ),
        ],
      ],
    );
  }
}

class _BatchItemTile extends StatelessWidget {
  const _BatchItemTile({required this.item});

  final LocalBatchItem item;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: item.status == BatchItemStatus.failed
            ? HsColors.warningSurface
            : HsColors.surfaceRaised,
        borderRadius: BorderRadius.circular(HsRadii.preview),
        border: Border.all(color: HsColors.border),
      ),
      child: ListTile(
        leading: Icon(_mediaIcon(item.mediaKind), color: HsColors.accent),
        title: Text(
          item.fileName,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        subtitle: Text(
          _itemSubtitle(item),
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
        ),
        isThreeLine: _batchFriendlyHint(item) != null,
        trailing: HsStatusChip(label: _itemStatusLabel(item.status)),
      ),
    );
  }
}

BatchMediaKind _mediaKindFromName(String name) {
  final ext = name.split('.').last.toLowerCase();
  if (['jpg', 'jpeg', 'png', 'bmp', 'webp', 'tiff'].contains(ext)) {
    return BatchMediaKind.image;
  }
  if (['wav', 'mp3', 'aac', 'flac', 'ogg', 'm4a'].contains(ext)) {
    return BatchMediaKind.audio;
  }
  return BatchMediaKind.unsupported;
}

IconData _mediaIcon(BatchMediaKind kind) {
  return switch (kind) {
    BatchMediaKind.image => Icons.image_outlined,
    BatchMediaKind.audio => Icons.graphic_eq_outlined,
    BatchMediaKind.unsupported => Icons.error_outline,
  };
}

int _count(LocalBatchJob job, BatchItemStatus status) {
  return job.items.where((item) => item.status == status).length;
}

bool _canProcessMediaItem(LocalBatchItem item) {
  return item.status == BatchItemStatus.queued;
}

WatermarkAssetKind _watermarkKindForBatchItem(LocalBatchItem item) {
  return switch (item.mediaKind) {
    BatchMediaKind.image => WatermarkAssetKind.image,
    BatchMediaKind.audio => WatermarkAssetKind.audio,
    BatchMediaKind.unsupported => throw UnsupportedError('不支持的批量文件类型'),
  };
}

String _jobStatusLabel(BatchJobStatus status) {
  return switch (status) {
    BatchJobStatus.draft => '草稿',
    BatchJobStatus.queued => '已建队列',
    BatchJobStatus.paused => '已暂停',
    BatchJobStatus.cancelled => '已取消',
  };
}

String _itemStatusLabel(BatchItemStatus status) {
  return switch (status) {
    BatchItemStatus.queued => '待处理',
    BatchItemStatus.running => '写入中',
    BatchItemStatus.verified => '已验证',
    BatchItemStatus.failed => '需处理',
    BatchItemStatus.cancelled => '已取消',
  };
}

String _itemDetailLabel(LocalBatchItem item) {
  return switch (item.status) {
    BatchItemStatus.queued => '等待开始处理',
    BatchItemStatus.running => '正在生成保护副本并验证',
    BatchItemStatus.verified => '完成后验证已通过，版权记录已保存',
    BatchItemStatus.failed => '处理失败，可重试',
    BatchItemStatus.cancelled => '已取消',
  };
}

String _itemSubtitle(LocalBatchItem item) {
  final hint = _batchFriendlyHint(item);
  if (hint != null) {
    return '${hint.title}。${hint.action}';
  }
  return item.lastError ??
      item.writeVerificationMessage ??
      _itemDetailLabel(item);
}

_BatchFriendlyHint? _batchFriendlyHint(LocalBatchItem item) {
  final error = item.lastError ?? '';
  if (error.contains('短于 30 秒')) {
    return const _BatchFriendlyHint(
      title: '音频时长不足 30 秒，未生成保护副本',
      action: '请选择 30 秒以上的完整音频作品后重试',
    );
  }
  if (error.contains('无法确认音频时长')) {
    return const _BatchFriendlyHint(
      title: '无法确认音频时长，未生成保护副本',
      action: '请更换可识别时长的完整音频文件后重试',
    );
  }
  return null;
}

LocalBatchJob _replaceBatchItem(LocalBatchJob job, LocalBatchItem nextItem) {
  return job.copyWith(
    updatedAt: DateTime.now(),
    items: [
      for (final item in job.items) item.id == nextItem.id ? nextItem : item,
    ],
  );
}

String _friendlyBatchError(Object error) {
  if (error is _BatchUserFacingError) {
    return error.message;
  }
  final message = '$error';
  if (message.contains('PathNotFound') || message.contains('No such file')) {
    return '文件不存在或已被移动';
  }
  return message;
}

String? _audioDurationRuleError(LocalBatchItem item, List<int> bytes) {
  if (item.mediaKind != BatchMediaKind.audio) {
    return null;
  }
  final metadata = inspectAudioMetadata(
    bytes is Uint8List ? bytes : Uint8List.fromList(bytes),
    fileName: item.fileName,
  );
  final duration = metadata.durationSeconds;
  if (duration == null) {
    return '无法确认音频时长，暂不生成保护副本';
  }
  if (duration < 30) {
    return '当前音频短于 30 秒，暂不生成保护副本';
  }
  return null;
}

class _BatchUserFacingError implements Exception {
  const _BatchUserFacingError(this.message);

  final String message;

  @override
  String toString() => message;
}

class _BatchFriendlyHint {
  const _BatchFriendlyHint({required this.title, required this.action});

  final String title;
  final String action;
}

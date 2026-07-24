import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import '../../shared/widgets/tool_cards.dart';
import 'protected_copy_share.dart';
import 'rewrite_preflight.dart';
import 'watermark_payload_seed.dart';
import 'work_declaration_panel.dart';

class ImageEmbedPage extends StatefulWidget {
  const ImageEmbedPage({
    super.key,
    required this.bridge,
    required this.appState,
    required this.onOpenVault,
    this.initialBytes,
    this.initialFileName,
  });

  final WatermarkBridge bridge;
  final MobileAppState appState;
  final VoidCallback onOpenVault;
  final Uint8List? initialBytes;
  final String? initialFileName;

  @override
  State<ImageEmbedPage> createState() => _ImageEmbedPageState();
}

class _ImageEmbedPageState extends State<ImageEmbedPage> {
  Uint8List? _selectedBytes;
  String? _fileName;
  bool _allowRewrite = false;
  bool _isProcessing = false;
  bool _isInspecting = false;
  WatermarkWriteResult? _result;
  VaultRecord? _savedRecord;
  RewritePreflightResult? _preflight;
  WorkDeclaration _workDeclaration = const WorkDeclaration();
  String? _errorText;
  int _preflightRequestId = 0;

  @override
  void initState() {
    super.initState();
    final initialBytes = widget.initialBytes;
    if (initialBytes != null) {
      _selectedBytes = initialBytes;
      _fileName = widget.initialFileName;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) {
          _inspectSelected(initialBytes);
        }
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final selectedBytes = _selectedBytes;
    final productionReady = widget.bridge.supportsProductionWatermark;
    final blocksInitialWrite =
        _preflight?.shouldBlockInitialWrite(allowRewrite: _allowRewrite) ??
        false;
    return SafeArea(
      child: FeaturePageScaffold(
        title: '图片写入',
        subtitle: '为图片生成可验证的保护副本，并保存版权记录。',
        icon: Icons.image_outlined,
        showBackButton: true,
        children: [
          if (!productionReady) ...[
            const HsMessageCard(
              icon: Icons.info_outline,
              title: 'Web 预览模式',
              detail: '当前浏览器预览只用于界面体验，不生成可被桌面端验证的正式盲水印。请使用移动端原生运行进行真实写入。',
            ),
            const SizedBox(height: HsSpacing.md),
          ],
          HsPanel(
            title: '作品',
            icon: Icons.add_photo_alternate_outlined,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (selectedBytes == null)
                  const _EmptyPreview()
                else
                  _ImagePreview(bytes: selectedBytes, fileName: _fileName),
                const SizedBox(height: 12),
                FilledButton.icon(
                  onPressed: _isProcessing ? null : _pickImage,
                  icon: const Icon(Icons.upload_file_outlined),
                  label: Text(selectedBytes == null ? '选择作品' : '更换作品'),
                ),
              ],
            ),
          ),
          const SizedBox(height: HsSpacing.md),
          HsPanel(
            title: '写入方式',
            icon: Icons.tune_outlined,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SwitchListTile(
                  value: _allowRewrite,
                  onChanged: _isProcessing
                      ? null
                      : (value) => setState(() => _allowRewrite = value),
                  title: const Text('作为新版写入'),
                  subtitle: const Text('用于已保护作品的再次发布，会记录新的版本次数。'),
                  contentPadding: EdgeInsets.zero,
                ),
                const SizedBox(height: HsSpacing.sm),
                const Wrap(
                  spacing: HsSpacing.sm,
                  runSpacing: HsSpacing.sm,
                  children: [
                    HsStatusChip(label: 'JPG / PNG / WebP'),
                    HsStatusChip(label: '完成后验证'),
                    HsStatusChip(label: '版本留痕'),
                  ],
                ),
                const SizedBox(height: HsSpacing.md),
                _PreflightStatusCard(
                  isInspecting: _isInspecting,
                  result: _preflight,
                ),
              ],
            ),
          ),
          const SizedBox(height: HsSpacing.md),
          WorkDeclarationPanel(
            value: _workDeclaration,
            onChanged: (next) => setState(() => _workDeclaration = next),
          ),
          const SizedBox(height: HsSpacing.md),
          FilledButton.icon(
            onPressed:
                selectedBytes == null ||
                    _isProcessing ||
                    blocksInitialWrite ||
                    !productionReady
                ? null
                : _embedImage,
            icon: _isProcessing
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.shield_outlined),
            label: Text(_isProcessing ? '正在处理' : '生成保护副本'),
          ),
          if (_errorText != null) ...[
            const SizedBox(height: 12),
            HsMessageCard(
              icon: Icons.error_outline,
              title: '处理失败',
              detail: _errorText!,
            ),
          ],
          if (_result != null) ...[
            const SizedBox(height: 12),
            _ResultCard(
              result: _result!,
              record: _savedRecord,
              appState: widget.appState,
              onOpenVault: widget.onOpenVault,
            ),
          ],
        ],
      ),
    );
  }

  Future<void> _pickImage() async {
    final result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: const ['jpg', 'jpeg', 'png', 'bmp', 'tiff', 'webp'],
      withData: true,
    );
    final file = result?.files.single;
    final bytes = file?.bytes;
    if (file == null || bytes == null) {
      return;
    }

    setState(() {
      _selectedBytes = bytes;
      _fileName = file.name;
      _result = null;
      _savedRecord = null;
      _preflight = null;
      _errorText = null;
    });
    await _inspectSelected(bytes);
  }

  Future<void> _embedImage() async {
    final bytes = _selectedBytes;
    if (bytes == null) {
      return;
    }

    final canContinue = await _ensureRewritePreflightBeforeWrite(bytes);
    if (!canContinue) {
      return;
    }

    setState(() {
      _isProcessing = true;
      _errorText = null;
      _result = null;
      _savedRecord = null;
    });

    try {
      final parent = _allowRewrite
          ? (_preflight?.readResult ?? await _readParentWatermark(bytes))
          : null;
      final revision = _allowRewrite
          ? (_preflight?.hasWatermark == true
                ? _preflight!.nextRevision
                : parent == null
                ? 2
                : parent.revision + 1)
          : 1;
      final originalHash = widget.appState.sha256HexForBytes(bytes);
      final reservedRegistry = await widget.appState.reserveWatermarkIdForWrite(
        kind: WatermarkAssetKind.image,
        originalHash: originalHash,
        parentWatermarkUid: parent?.watermarkUid,
        revision: revision,
      );
      final result = await widget.bridge.write(
        WatermarkWriteRequest(
          kind: WatermarkAssetKind.image,
          bytes: bytes,
          seed: buildPayloadSeed(bytes, widget.appState),
          allowRewrite: _allowRewrite,
          rewriteReason: _allowRewrite ? '移动端确认更新版本' : null,
          parentWatermarkUid: parent?.watermarkUid,
          revision: revision,
          registryDraft: reservedRegistry?.toDraft(),
        ),
      );
      final displayResult = result.copyWithOutputArtifact(
        outputFileName: _protectedCopyName(_fileName, 'png'),
        outputLocationLabel: '已生成保护副本，可通过系统分享面板保存到相册、文件或其他应用。',
        outputActionLabel: '保存或分享保护副本',
      );
      final registryResult = await widget.appState.confirmWatermarkIdForWrite(
        result: displayResult,
        originalHash: originalHash,
        reserved: reservedRegistry,
      );
      final trustedTimeAttestation = await widget.appState
          .requestTrustedTimeAttestation();
      if (!mounted) return;
      final record = widget.appState.addWriteResult(
        result: displayResult,
        fileName: _fileName,
        allowRewrite: _allowRewrite,
        rewriteReason: _allowRewrite ? '移动端确认更新版本' : null,
        parentWatermarkUid: parent?.watermarkUid,
        revision: revision,
        trustedTimeAttestation: trustedTimeAttestation,
        declaration: _workDeclaration,
        registryResult: registryResult,
      );
      if (result.verification.verified) {
        await widget.appState.appendUsageForWriteResult(
          result: displayResult,
          vaultRecordId: record.id,
          pipelineId: null,
        );
      }
      setState(() {
        _result = displayResult;
        _savedRecord = record;
      });
    } catch (error) {
      if (!mounted) return;
      setState(() => _errorText = mobileWatermarkWriteErrorMessage(error));
    } finally {
      if (mounted) {
        setState(() => _isProcessing = false);
      }
    }
  }

  Future<bool> _ensureRewritePreflightBeforeWrite(List<int> bytes) async {
    RewritePreflightResult? result = _preflight;
    if (result == null || _isInspecting) {
      setState(() => _isInspecting = true);
      result = await inspectMobileRewriteTarget(
        bridge: widget.bridge,
        appState: widget.appState,
        kind: WatermarkAssetKind.image,
        bytes: bytes,
      );
      if (!mounted) {
        return false;
      }
      setState(() {
        _preflight = result;
        _isInspecting = false;
      });
    }

    final RewritePreflightResult current = result;
    if (current.shouldBlockInitialWrite(allowRewrite: _allowRewrite)) {
      setState(() {
        _errorText = existingWatermarkRewriteBlockedMessage(
          current.watermarkUid,
        );
      });
      return false;
    }
    return true;
  }

  Future<WatermarkReadResult?> _readParentWatermark(List<int> bytes) async {
    if (!_allowRewrite) {
      return null;
    }
    try {
      return await widget.bridge.read(
        WatermarkReadRequest(kind: WatermarkAssetKind.image, bytes: bytes),
      );
    } catch (_) {
      return null;
    }
  }

  Future<void> _inspectSelected(List<int> bytes) async {
    final requestId = ++_preflightRequestId;
    setState(() => _isInspecting = true);
    final result = await inspectMobileRewriteTarget(
      bridge: widget.bridge,
      appState: widget.appState,
      kind: WatermarkAssetKind.image,
      bytes: bytes,
    );
    if (!mounted || requestId != _preflightRequestId) {
      return;
    }
    setState(() {
      _preflight = result;
      _isInspecting = false;
    });
  }
}

String _protectedCopyName(String? fileName, String outputExtension) {
  final trimmed = fileName?.trim();
  final baseName = (trimmed == null || trimmed.isEmpty)
      ? '未命名图片'
      : trimmed.replaceFirst(RegExp(r'\.[^.]+$'), '');
  return '${baseName}_protected.$outputExtension';
}

class _EmptyPreview extends StatelessWidget {
  const _EmptyPreview();

  @override
  Widget build(BuildContext context) {
    return HsPreviewBox(
      height: 180,
      child: const Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(Icons.image_outlined, size: 42, color: HsColors.iconMuted),
          SizedBox(height: 8),
          Text('选择一张图片作品'),
        ],
      ),
    );
  }
}

class _ImagePreview extends StatelessWidget {
  const _ImagePreview({required this.bytes, required this.fileName});

  final Uint8List bytes;
  final String? fileName;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        ClipRRect(
          borderRadius: BorderRadius.circular(8),
          child: Image.memory(
            bytes,
            height: 220,
            width: double.infinity,
            fit: BoxFit.cover,
          ),
        ),
        const SizedBox(height: 8),
        Text(
          fileName ?? '未命名图片',
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: const TextStyle(color: HsColors.textMuted),
        ),
      ],
    );
  }
}

class _ResultCard extends StatelessWidget {
  const _ResultCard({
    required this.result,
    required this.record,
    required this.appState,
    required this.onOpenVault,
  });

  final WatermarkWriteResult result;
  final VaultRecord? record;
  final MobileAppState appState;
  final VoidCallback onOpenVault;

  @override
  Widget build(BuildContext context) {
    final shaPreview = result.sha256.length > 16
        ? '${result.sha256.substring(0, 16)}...'
        : result.sha256;
    final savedRecord = record;
    final revision = savedRecord?.revision ?? result.revision;
    final parent = savedRecord?.parentWatermarkUid;
    return HsPrimaryResultCard(
      icon: Icons.verified_outlined,
      title: '保护副本已生成',
      statusLabel: result.verification.verified ? '完成后验证已通过' : '完成后验证未通过',
      statusColor: result.verification.verified
          ? HsColors.accent
          : HsColors.warning,
      children: [
        _WriteVerificationDetail(
          result: result,
          verification: result.verification,
          watermarkUid: result.watermarkUid,
          revision: revision,
          parentWatermarkUid: parent,
          shaPreview: shaPreview,
          record: savedRecord,
          appState: appState,
          onOpenVault: onOpenVault,
        ),
      ],
    );
  }
}

class _WriteVerificationDetail extends StatelessWidget {
  const _WriteVerificationDetail({
    required this.result,
    required this.verification,
    required this.watermarkUid,
    required this.revision,
    required this.parentWatermarkUid,
    required this.shaPreview,
    required this.record,
    required this.appState,
    required this.onOpenVault,
  });

  final WatermarkWriteResult result;
  final WatermarkWriteVerification verification;
  final String watermarkUid;
  final int revision;
  final String? parentWatermarkUid;
  final String shaPreview;
  final VaultRecord? record;
  final MobileAppState appState;
  final VoidCallback onOpenVault;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        HsInfoRow(label: '版权编号', value: watermarkUid),
        HsInfoRow(label: '版本次数', value: '第 $revision 次'),
        HsInfoRow(
          label: '处理耗时',
          value: _formatDurationMs(result.processTimeMs),
        ),
        if (parentWatermarkUid != null)
          HsInfoRow(label: '上一版', value: parentWatermarkUid!),
        HsInfoRow(label: '作品指纹', value: shaPreview),
        HsInfoRow(label: '保护副本名称', value: result.outputFileName ?? '未记录'),
        HsInfoRow(
          label: 'Payload 协议',
          value:
              'V${record?.payloadProtocolVersion ?? 2} / ${record?.payloadBytesLength ?? 119} bytes',
        ),
        HsInfoRow(
          label: '编号签发',
          value: record?.watermarkIdIssueMode ?? 'offline_generated',
        ),
        HsInfoRow(
          label: '登记状态',
          value: record?.watermarkIdRegistryStatus ?? 'pending_registration',
        ),
        HsInfoRow(
          label: 'Payload 认证',
          value:
              record?.payloadAuthStatus ??
              (verification.verified ? 'verified' : 'failed'),
        ),
        HsInfoRow(
          label: '保存方式',
          value:
              result.outputLocationLabel ?? '已生成保护副本，可通过系统分享面板保存到相册、文件或其他应用。',
        ),
        if (parentWatermarkUid != null || revision > 1) ...[
          const SizedBox(height: HsSpacing.sm),
          _VersionSummaryTile(
            revision: revision,
            watermarkUid: watermarkUid,
            parentWatermarkUid: parentWatermarkUid,
          ),
        ],
        if (!verification.verified) ...[
          const SizedBox(height: HsSpacing.sm),
          _FailureRecoveryBlock(message: verification.message),
        ],
        const SizedBox(height: HsSpacing.sm),
        Wrap(
          spacing: HsSpacing.sm,
          runSpacing: HsSpacing.sm,
          children: [
            OutlinedButton.icon(
              onPressed: record == null
                  ? null
                  : () async {
                      await Clipboard.setData(
                        ClipboardData(
                          text: appState.buildCopyrightSummary(record!),
                        ),
                      );
                      if (!context.mounted) return;
                      ScaffoldMessenger.of(
                        context,
                      ).showSnackBar(const SnackBar(content: Text('已复制存证摘要')));
                    },
              icon: const Icon(Icons.copy_all_outlined),
              label: const Text('复制存证摘要'),
            ),
            OutlinedButton.icon(
              onPressed: () => shareProtectedCopy(
                context: context,
                result: result,
                fallbackFileName: 'hiddenshield_protected.png',
                mimeType: 'image/png',
              ),
              icon: const Icon(Icons.ios_share_outlined),
              label: Text(result.outputActionLabel ?? '保存或分享保护副本'),
            ),
            OutlinedButton.icon(
              onPressed: () {
                Navigator.of(context).popUntil((route) => route.isFirst);
                onOpenVault();
              },
              icon: const Icon(Icons.folder_outlined),
              label: const Text('查看版权库'),
            ),
          ],
        ),
      ],
    );
  }
}

String _formatDurationMs(int ms) {
  if (ms < 1000) return '${ms}ms';
  return '${(ms / 1000).toStringAsFixed(1)}s';
}

class _FailureRecoveryBlock extends StatelessWidget {
  const _FailureRecoveryBlock({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: HsColors.warningSurface,
        borderRadius: BorderRadius.circular(HsRadii.preview),
        border: Border.all(color: HsColors.warning.withValues(alpha: 0.28)),
      ),
      child: Padding(
        padding: const EdgeInsets.all(HsSpacing.md),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              '完成后验证未通过',
              style: TextStyle(
                color: HsColors.warning,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: HsSpacing.sm),
            const Text(
              '建议先重新生成保护副本；如果需要排查，可查看失败原因。',
              style: TextStyle(color: HsColors.textMuted, fontSize: 12),
            ),
            const SizedBox(height: HsSpacing.sm),
            Wrap(
              spacing: HsSpacing.sm,
              runSpacing: HsSpacing.sm,
              children: [
                FilledButton(
                  onPressed: () => Navigator.of(context).maybePop(),
                  child: const Text('重新生成保护副本'),
                ),
                OutlinedButton(
                  onPressed: () {
                    showDialog<void>(
                      context: context,
                      builder: (context) => AlertDialog(
                        title: const Text('失败原因'),
                        content: Text(message),
                      ),
                    );
                  },
                  child: const Text('查看原因'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _VersionSummaryTile extends StatelessWidget {
  const _VersionSummaryTile({
    required this.revision,
    required this.watermarkUid,
    required this.parentWatermarkUid,
  });

  final int revision;
  final String watermarkUid;
  final String? parentWatermarkUid;

  @override
  Widget build(BuildContext context) {
    return Theme(
      data: Theme.of(context).copyWith(dividerColor: Colors.transparent),
      child: ExpansionTile(
        tilePadding: EdgeInsets.zero,
        childrenPadding: EdgeInsets.zero,
        dense: true,
        initiallyExpanded: true,
        title: Text('版本记录', style: Theme.of(context).textTheme.labelLarge),
        subtitle: Text(
          '第 $revision 次',
          style: const TextStyle(color: HsColors.textMuted, fontSize: 12),
        ),
        children: [
          HsInfoRow(label: '版权编号', value: watermarkUid),
          HsInfoRow(label: '版本次数', value: '第 $revision 次'),
          if (parentWatermarkUid != null)
            HsInfoRow(label: '上一版', value: parentWatermarkUid!),
          const HsInfoRow(label: '说明', value: '确认更新版本'),
        ],
      ),
    );
  }
}

class _PreflightStatusCard extends StatelessWidget {
  const _PreflightStatusCard({
    required this.isInspecting,
    required this.result,
  });

  final bool isInspecting;
  final RewritePreflightResult? result;

  @override
  Widget build(BuildContext context) {
    if (isInspecting) {
      return const HsMessageCard(
        icon: Icons.search_outlined,
        title: '正在检查版本',
        detail: '正在确认这张图片是否需要作为新版写入。',
      );
    }
    final result = this.result;
    if (result == null) {
      return const HsMessageCard(
        icon: Icons.info_outline,
        title: '写入提示',
        detail: '选择作品后会自动确认是否需要作为新版写入。',
      );
    }
    return HsMessageCard(
      icon: result.hasWatermark
          ? Icons.warning_amber_outlined
          : Icons.check_circle_outline,
      title: preflightSummaryLabel(result),
      detail: preflightActionLabel(result),
      detailWidget: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            preflightActionLabel(result),
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: HsColors.textMuted,
              height: 1.35,
            ),
          ),
          const SizedBox(height: HsSpacing.xs),
          ExpansionTile(
            tilePadding: EdgeInsets.zero,
            childrenPadding: EdgeInsets.zero,
            dense: true,
            initiallyExpanded: false,
            title: Text(
              '查看详情',
              style: Theme.of(context).textTheme.labelMedium?.copyWith(
                color: HsColors.textMuted,
                fontWeight: FontWeight.w700,
              ),
            ),
            children: [
              ...preflightEvidenceLines(result).map(
                (line) => Padding(
                  padding: const EdgeInsets.only(bottom: HsSpacing.xs),
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: Text(
                      line,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: HsColors.textMuted,
                        height: 1.35,
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

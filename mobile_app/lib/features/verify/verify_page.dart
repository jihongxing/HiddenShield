import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../shared/models/workspace_context.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import '../../shared/widgets/tool_cards.dart';
import '../public_rights/public_rights_scanner.dart';
import '../vault/rights_evidence_pack_saf_bridge.dart';
import '../vault/rights_evidence_pack_verifier.dart';
import '../workspace/media_file_kind.dart';
import 'mobile_verify_reason.dart';
import 'mobile_verification_result.dart';

class VerifyPage extends StatefulWidget {
  const VerifyPage({
    super.key,
    required this.bridge,
    required this.appState,
    this.pickRightsEvidencePackDirectory,
    this.rightsEvidencePackVerifier = const RightsEvidencePackVerifier(),
    this.rightsEvidencePackSafBridge = const RightsEvidencePackSafBridge(),
    this.onRightsEvidencePackVerified,
    this.onRightsEvidencePackAccessFailure,
  });

  final WatermarkBridge bridge;
  final MobileAppState appState;
  final Future<String?> Function()? pickRightsEvidencePackDirectory;
  final RightsEvidencePackVerifier rightsEvidencePackVerifier;
  final RightsEvidencePackSafBridge rightsEvidencePackSafBridge;
  final ValueChanged<RightsEvidencePackVerificationResult>?
  onRightsEvidencePackVerified;
  final ValueChanged<RightsEvidencePackAccessException>?
  onRightsEvidencePackAccessFailure;

  @override
  State<VerifyPage> createState() => _VerifyPageState();
}

class _VerifyPageState extends State<VerifyPage> {
  WatermarkAssetKind? _kind;
  Uint8List? _selectedBytes;
  String? _fileName;
  bool _isProcessing = false;
  MobileVerificationResult? _result;
  MobileVerifyReason? _reason;
  String? _errorText;
  int? _lastDurationMs;
  bool _verifyingRightsEvidencePack = false;
  String? _rightsEvidencePackDir;
  String? _rightsEvidencePackError;
  RightsEvidencePackVerificationResult? _rightsEvidencePackResult;
  SafRightsEvidencePackDirectory? _authorizedSafDirectory;

  bool get _usesAndroidSaf =>
      !kIsWeb &&
      defaultTargetPlatform == TargetPlatform.android &&
      widget.pickRightsEvidencePackDirectory == null;

  @override
  void initState() {
    super.initState();
    if (_usesAndroidSaf) {
      _loadPersistedSafDirectory();
    }
  }

  @override
  Widget build(BuildContext context) {
    final productionReady = widget.bridge.supportsProductionWatermark;
    return FeaturePageScaffold(
      title: '验证记录',
      subtitle: '导入疑似样本，检查是否保留 HiddenShield 版权记录。',
      icon: Icons.document_scanner_outlined,
      contextData: HsWorkspaceContext(
        eyebrow: '验证上下文',
        title: '结果、证据与报告',
        summary: '验证页先给结论，再展示置信度、匹配记录、证据摘要和后续报告动作。',
        metrics: [
          const HsContextMetric(
            label: '结果顺序',
            value: '结论优先',
            tone: HsContextTone.ok,
          ),
          HsContextMetric(
            label: '正式报告',
            value: widget.appState.canExportFormalReports
                ? 'Creator 可导出'
                : '需授权',
            tone: widget.appState.canExportFormalReports
                ? HsContextTone.ok
                : HsContextTone.warning,
          ),
          const HsContextMetric(
            label: 'L2 边界',
            value: '不是画面水印',
            tone: HsContextTone.muted,
          ),
        ],
      ),
      children: [
        if (!productionReady) ...[
          const HsMessageCard(
            icon: Icons.info_outline,
            title: 'Web 预览模式',
            detail: '当前浏览器预览只用于界面体验，不能验证桌面端或原生移动端生成的正式盲水印。请使用移动端原生运行进行真实验证。',
          ),
          const SizedBox(height: 12),
        ],
        HsPanel(
          title: '选择样本',
          icon: Icons.upload_file_outlined,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _SelectedFileSummary(
                kind: _kind,
                bytes: _selectedBytes,
                fileName: _fileName,
              ),
              const SizedBox(height: 12),
              const _DetectionScopeNotes(),
              const SizedBox(height: 12),
              FilledButton.icon(
                onPressed: _isProcessing || !productionReady ? null : _pickFile,
                icon: const Icon(Icons.upload_file_outlined),
                label: Text(_selectedBytes == null ? '选择文件' : '重新选择'),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        FilledButton.icon(
          onPressed: _selectedBytes == null || _isProcessing || !productionReady
              ? null
              : _verify,
          icon: _isProcessing
              ? const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.document_scanner_outlined),
          label: Text(_isProcessing ? '正在检查' : '开始检查'),
        ),
        if (_errorText != null) ...[
          const SizedBox(height: 12),
          HsMessageCard(
            icon: Icons.error_outline,
            title: '未找到对应记录',
            detail: _errorText!,
            detailWidget: _MessageDetail(detail: _errorText!, reason: _reason),
          ),
        ],
        if (_result != null) ...[
          const SizedBox(height: 12),
          _ResultCard(
            result: _result!,
            reason: _reason,
            durationMs: _lastDurationMs,
            appState: widget.appState,
            canExportFormalReports: widget.appState.canExportFormalReports,
          ),
        ],
        const SizedBox(height: 12),
        _RightsEvidencePackPanel(
          enabled: !kIsWeb || widget.pickRightsEvidencePackDirectory != null,
          verifying: _verifyingRightsEvidencePack,
          caseDir: _rightsEvidencePackDir,
          error: _rightsEvidencePackError,
          result: _rightsEvidencePackResult,
          hasAuthorizedDirectory: _authorizedSafDirectory != null,
          onPickAndVerify: _pickAndVerifyRightsEvidencePack,
          onVerifyAuthorized: _authorizedSafDirectory == null
              ? null
              : _verifyAuthorizedRightsEvidencePack,
        ),
      ],
    );
  }

  Future<void> _pickAndVerifyRightsEvidencePack() async {
    if (_usesAndroidSaf) {
      final directory = await widget.rightsEvidencePackSafBridge
          .pickDirectory();
      if (directory == null || !mounted) return;
      setState(() => _authorizedSafDirectory = directory);
      await _verifySafDirectory(directory);
      return;
    }
    final picker = widget.pickRightsEvidencePackDirectory;
    final caseDir = picker == null
        ? await FilePicker.getDirectoryPath(
            dialogTitle: '选择 HiddenShield 维权证据包目录',
          )
        : await picker();
    if (caseDir == null || !mounted) return;
    setState(() {
      _verifyingRightsEvidencePack = true;
      _rightsEvidencePackDir = caseDir;
      _rightsEvidencePackError = null;
      _rightsEvidencePackResult = null;
    });
    try {
      final result = await widget.rightsEvidencePackVerifier.verify(caseDir);
      if (!mounted) return;
      setState(() => _rightsEvidencePackResult = result);
      widget.onRightsEvidencePackVerified?.call(result);
    } catch (error) {
      if (!mounted) return;
      setState(() => _rightsEvidencePackError = '案件包校验失败：$error');
    } finally {
      if (mounted) {
        setState(() => _verifyingRightsEvidencePack = false);
      }
    }
  }

  Future<void> _loadPersistedSafDirectory() async {
    try {
      final directory = await widget.rightsEvidencePackSafBridge
          .getPersistedDirectory();
      if (directory == null || !mounted) return;
      setState(() {
        _authorizedSafDirectory = directory;
        _rightsEvidencePackDir = '已授权目录：${directory.displayName}';
      });
    } on MissingPluginException {
      return;
    } on PlatformException {
      return;
    }
  }

  Future<void> _verifyAuthorizedRightsEvidencePack() async {
    final directory = _authorizedSafDirectory;
    if (directory == null) return;
    await _verifySafDirectory(directory);
  }

  Future<void> _verifySafDirectory(
    SafRightsEvidencePackDirectory directory,
  ) async {
    setState(() {
      _verifyingRightsEvidencePack = true;
      _rightsEvidencePackDir = '已授权目录：${directory.displayName}';
      _rightsEvidencePackError = null;
      _rightsEvidencePackResult = null;
    });
    final verifier = RightsEvidencePackVerifier(
      readBytes: (_, relativePath) => widget.rightsEvidencePackSafBridge
          .readBytes(directory.treeUri, relativePath),
      readDirectory: (_) =>
          widget.rightsEvidencePackSafBridge.listDirectory(directory.treeUri),
    );
    try {
      final result = await verifier.verify(directory.treeUri);
      if (!mounted) return;
      setState(() => _rightsEvidencePackResult = result);
      widget.onRightsEvidencePackVerified?.call(result);
    } on RightsEvidencePackAccessException catch (error) {
      if (!mounted) return;
      setState(() => _rightsEvidencePackError = error.userMessage);
      widget.onRightsEvidencePackAccessFailure?.call(error);
    } catch (error) {
      if (!mounted) return;
      setState(() => _rightsEvidencePackError = '案件包校验失败：$error');
    } finally {
      if (mounted) {
        setState(() => _verifyingRightsEvidencePack = false);
      }
    }
  }

  Future<void> _pickFile() async {
    final result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: supportedMediaExtensions,
      withData: true,
    );
    final file = result?.files.single;
    final bytes = file?.bytes;
    if (file == null || bytes == null) {
      return;
    }

    final kind = mediaKindForFileName(file.name);
    if (kind == null) {
      setState(() {
        _kind = null;
        _selectedBytes = null;
        _fileName = file.name;
        _result = null;
        _reason = null;
        _errorText = '请选择受支持的图片、音频或 L1 视频音轨样本。';
        _lastDurationMs = null;
      });
      return;
    }

    setState(() {
      _kind = kind;
      _selectedBytes = bytes;
      _fileName = file.name;
      _result = null;
      _reason = null;
      _errorText = null;
      _lastDurationMs = null;
    });
    await _verify();
  }

  Future<void> _verify() async {
    final bytes = _selectedBytes;
    final kind = _kind;
    if (bytes == null || kind == null) {
      return;
    }

    setState(() {
      _isProcessing = true;
      _result = null;
      _reason = null;
      _errorText = null;
      _lastDurationMs = null;
    });

    try {
      final startedAt = DateTime.now();
      final result = await widget.bridge.read(
        WatermarkReadRequest(kind: kind, bytes: bytes),
      );
      final durationMs = DateTime.now().difference(startedAt).inMilliseconds;
      if (!mounted) return;
      MobileVerificationResult? verification;
      if (result != null) {
        verification = buildMobileVerificationResult(
          readResult: result,
          records: widget.appState.records,
        );
        widget.appState.addReadResult(result: result, fileName: _fileName);
      }
      setState(() {
        _result = verification;
        _reason = verification == null
            ? MobileVerifyReason.noWatermark()
            : verification.reason;
        _errorText = result == null ? '没有找到可验证的隐盾版权记录。' : null;
        _lastDurationMs = durationMs;
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _reason = MobileVerifyReason.forError(error.toString());
        _errorText = '检查过程未完成。';
        _lastDurationMs = null;
      });
    } finally {
      if (mounted) {
        setState(() => _isProcessing = false);
      }
    }
  }
}

class _RightsEvidencePackPanel extends StatelessWidget {
  const _RightsEvidencePackPanel({
    required this.enabled,
    required this.verifying,
    required this.caseDir,
    required this.error,
    required this.result,
    required this.hasAuthorizedDirectory,
    required this.onPickAndVerify,
    required this.onVerifyAuthorized,
  });

  final bool enabled;
  final bool verifying;
  final String? caseDir;
  final String? error;
  final RightsEvidencePackVerificationResult? result;
  final bool hasAuthorizedDirectory;
  final VoidCallback onPickAndVerify;
  final VoidCallback? onVerifyAuthorized;

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      title: '维权证据包完整性',
      icon: Icons.folder_zip_outlined,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const HsStatusChip(
            label: 'Phase R4 · 只读校验',
            icon: Icons.fact_check_outlined,
            foregroundColor: HsColors.copper,
          ),
          const SizedBox(height: HsSpacing.md),
          Text(
            caseDir ??
                '选择包含 case.json、case-manifest.json 与 attachments/ 的案件包目录。',
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: HsColors.textMuted,
              height: 1.45,
            ),
          ),
          const SizedBox(height: HsSpacing.md),
          if (hasAuthorizedDirectory) ...[
            FilledButton.icon(
              key: const ValueKey(
                'verify-authorized-rights-evidence-pack-button',
              ),
              onPressed: enabled && !verifying ? onVerifyAuthorized : null,
              icon: verifying
                  ? const SizedBox.square(
                      dimension: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.fact_check_outlined),
              label: Text(verifying ? '校验中…' : '校验已授权目录'),
            ),
            const SizedBox(height: HsSpacing.sm),
            OutlinedButton.icon(
              key: const ValueKey('verify-rights-evidence-pack-button'),
              onPressed: enabled && !verifying ? onPickAndVerify : null,
              icon: const Icon(Icons.drive_folder_upload_outlined),
              label: const Text('重新选择案件包目录'),
            ),
          ] else
            FilledButton.icon(
              key: const ValueKey('verify-rights-evidence-pack-button'),
              onPressed: enabled && !verifying ? onPickAndVerify : null,
              icon: verifying
                  ? const SizedBox.square(
                      dimension: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.create_new_folder_outlined),
              label: Text(verifying ? '校验中…' : '选择案件包目录'),
            ),
          if (!enabled) ...[
            const SizedBox(height: HsSpacing.sm),
            const Text(
              'Web 预览不能读取本机案件包目录，请使用 Android、iOS 或桌面原生运行。',
              style: TextStyle(color: HsColors.textMuted, fontSize: 12),
            ),
          ],
          if (error != null) ...[
            const SizedBox(height: HsSpacing.md),
            Text(
              error!,
              key: const ValueKey('rights-evidence-pack-error'),
              style: const TextStyle(color: HsColors.danger, height: 1.4),
            ),
          ],
          if (result != null) ...[
            const SizedBox(height: HsSpacing.lg),
            _RightsEvidenceStatusGrid(result: result!),
            const SizedBox(height: HsSpacing.md),
            HsInfoRow(label: '案件编号', value: result!.caseId ?? '未记录'),
            HsInfoRow(label: '证据包编号', value: result!.packId ?? '未记录'),
            HsInfoRow(
              label: '声明 root digest',
              value: result!.declaredRootDigest ?? '未记录',
            ),
            HsInfoRow(
              label: '复算 root digest',
              value: result!.computedRootDigest ?? '未完成',
            ),
            HsInfoRow(
              label: '附件结果',
              value:
                  '${result!.attachments.where((item) => item.status == 'matched').length}'
                  ' / ${result!.attachments.length} 匹配',
            ),
            const SizedBox(height: HsSpacing.sm),
            Text(
              result!.message,
              key: const ValueKey('rights-evidence-pack-message'),
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: HsColors.textMuted,
                height: 1.45,
              ),
            ),
          ],
          const SizedBox(height: HsSpacing.md),
          const Text(
            '校验只复算目录、文件与摘要链，不读取媒体水印，不判断侵权成立、签发主体可信或采集时间可信。',
            key: ValueKey('rights-evidence-pack-boundary'),
            style: TextStyle(
              color: HsColors.textSubtle,
              fontSize: 12,
              height: 1.45,
            ),
          ),
        ],
      ),
    );
  }
}

class _RightsEvidenceStatusGrid extends StatelessWidget {
  const _RightsEvidenceStatusGrid({required this.result});

  final RightsEvidencePackVerificationResult result;

  @override
  Widget build(BuildContext context) {
    final items = [
      (key: 'directory', label: '目录合同', status: result.directoryContractStatus),
      (
        key: 'attachments',
        label: '附件完整性',
        status: result.attachmentIntegrityStatus,
      ),
      (key: 'events', label: '采集事件链', status: result.eventChainStatus),
      (
        key: 'attachment-chain',
        label: '附件链',
        status: result.attachmentChainStatus,
      ),
      (key: 'signature', label: '数字签名', status: result.signatureStatus),
      (key: 'trusted-time', label: '可信时间', status: result.trustedTimeStatus),
    ];
    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = constraints.maxWidth >= 360 ? 2 : 1;
        final width =
            (constraints.maxWidth - HsSpacing.sm * (columns - 1)) / columns;
        return Wrap(
          spacing: HsSpacing.sm,
          runSpacing: HsSpacing.sm,
          children: items
              .map(
                (item) => SizedBox(
                  width: width,
                  child: _RightsEvidenceStatusCard(
                    key: ValueKey('rights-evidence-status-${item.key}'),
                    label: item.label,
                    status: item.status,
                  ),
                ),
              )
              .toList(),
        );
      },
    );
  }
}

class _RightsEvidenceStatusCard extends StatelessWidget {
  const _RightsEvidenceStatusCard({
    super.key,
    required this.label,
    required this.status,
  });

  final String label;
  final String status;

  @override
  Widget build(BuildContext context) {
    final matched = status == 'matched';
    final boundary =
        status == 'not_signed' ||
        status == 'not_timestamped' ||
        status == 'present_unverified';
    final color = matched
        ? HsColors.accent
        : boundary
        ? HsColors.warning
        : HsColors.danger;
    return Container(
      padding: const EdgeInsets.all(HsSpacing.md),
      decoration: BoxDecoration(
        color: HsColors.surfaceMuted,
        borderRadius: BorderRadius.circular(HsRadii.card),
        border: Border.all(color: color.withValues(alpha: 0.35)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: const TextStyle(color: HsColors.textMuted, fontSize: 12),
          ),
          const SizedBox(height: HsSpacing.xs),
          Text(
            _rightsEvidenceStatusLabel(status),
            style: TextStyle(color: color, fontWeight: FontWeight.w800),
          ),
        ],
      ),
    );
  }
}

String _rightsEvidenceStatusLabel(String status) => switch (status) {
  'matched' => '匹配',
  'mismatch' => '不匹配',
  'not_signed' => '未签名',
  'not_timestamped' => '未加盖',
  'present_unverified' => '存在但未验证',
  _ => status,
};

class _DetectionScopeNotes extends StatelessWidget {
  const _DetectionScopeNotes();

  @override
  Widget build(BuildContext context) {
    return Theme(
      data: Theme.of(context).copyWith(dividerColor: Colors.transparent),
      child: const ExpansionTile(
        tilePadding: EdgeInsets.zero,
        childrenPadding: EdgeInsets.zero,
        dense: true,
        title: Text('检测范围'),
        subtitle: Text(
          '支持常见压缩、二次保存和轻度改动后的记录检查。',
          style: TextStyle(color: HsColors.textMuted),
        ),
        children: [
          _DetectionScopeRow(
            icon: Icons.search_outlined,
            title: '默认检测',
            detail: MobileVerifyReason.defaultDetectionScope,
          ),
          SizedBox(height: HsSpacing.sm),
          _DetectionScopeRow(
            icon: Icons.inventory_2_outlined,
            title: '版权库深度检测',
            detail: MobileVerifyReason.vaultDeepDetectionScope,
          ),
        ],
      ),
    );
  }
}

class _DetectionScopeRow extends StatelessWidget {
  const _DetectionScopeRow({
    required this.icon,
    required this.title,
    required this.detail,
  });

  final IconData icon;
  final String title;
  final String detail;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, color: HsColors.iconMuted, size: 18),
        const SizedBox(width: HsSpacing.sm),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: Theme.of(context).textTheme.labelLarge),
              const SizedBox(height: 3),
              Text(
                detail,
                style: const TextStyle(
                  color: HsColors.textMuted,
                  fontSize: 12,
                  height: 1.45,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _SelectedFileSummary extends StatelessWidget {
  const _SelectedFileSummary({
    required this.kind,
    required this.bytes,
    required this.fileName,
  });

  final WatermarkAssetKind? kind;
  final Uint8List? bytes;
  final String? fileName;

  @override
  Widget build(BuildContext context) {
    const emptyText = '选择需要验证的图片、音频或 L1 视频音轨样本';
    final detail = bytes == null
        ? emptyText
        : '${_kindLabel(kind)} · ${(bytes!.length / 1024).toStringAsFixed(1)} KB';
    return HsPreviewBox(
      height: 150,
      child: Row(
        children: [
          Icon(
            kind == WatermarkAssetKind.audio
                ? Icons.graphic_eq_outlined
                : kind == WatermarkAssetKind.image
                ? Icons.image_search_outlined
                : Icons.upload_file_outlined,
            size: 42,
            color: HsColors.iconMuted,
          ),
          const SizedBox(width: 16),
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  fileName ?? '未选择文件',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                Text(detail, style: const TextStyle(color: HsColors.textMuted)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

String _kindLabel(WatermarkAssetKind? kind) {
  if (kind == WatermarkAssetKind.audio) return '音频';
  if (kind == WatermarkAssetKind.video) return '视频音轨';
  return '图片';
}

class _ResultCard extends StatelessWidget {
  const _ResultCard({
    required this.result,
    required this.reason,
    required this.durationMs,
    required this.appState,
    required this.canExportFormalReports,
  });

  final MobileVerificationResult result;
  final MobileVerifyReason? reason;
  final int? durationMs;
  final MobileAppState appState;
  final bool canExportFormalReports;

  @override
  Widget build(BuildContext context) {
    final read = result.readResult;
    final matchedRecord = result.matchedRecord;
    final parentWatermarkUid =
        matchedRecord?.parentWatermarkUid ?? read.parentWatermarkUid;
    final rewriteReason = matchedRecord?.rewriteReason ?? read.rewriteReason;
    final versionValue = read.payloadProtocolVersion == 3 && read.revision == 0
        ? 'registry 解析'
        : '第 ${matchedRecord?.revision ?? read.revision} 次';
    return HsPrimaryResultCard(
      icon: Icons.fact_check_outlined,
      title: result.matched ? '已匹配本机版权记录' : '检测到有效水印',
      statusLabel: result.matched ? '版权库已命中' : '可作为验证线索',
      children: [
        _MessageDetail(
          rows: [
            HsInfoRow(label: '版权编号', value: read.watermarkUid),
            HsInfoRow(label: '版本次数', value: versionValue),
            if (parentWatermarkUid != null)
              HsInfoRow(label: '上一版', value: parentWatermarkUid),
            if (rewriteReason != null)
              HsInfoRow(label: '说明', value: rewriteReason),
            HsInfoRow(
              label: 'Payload 协议',
              value:
                  'V${read.payloadProtocolVersion} / ${read.payloadBytesLength} bytes',
            ),
            HsInfoRow(
              label: '签发模式',
              value: _issueModeLabel(read.watermarkIdIssueMode),
            ),
            HsInfoRow(
              label: 'Payload 认证',
              value: read.payloadAuthStatus == 'verified' ? '已通过' : '未通过',
            ),
            HsInfoRow(
              label: '匹配状态',
              value: result.matched ? '已命中本机版权库' : '未命中本机版权库',
            ),
            HsInfoRow(
              label: '可信度',
              value: '${(result.confidence * 100).toStringAsFixed(0)}%',
            ),
            if (durationMs != null)
              HsInfoRow(label: '验证耗时', value: _formatDurationMs(durationMs!)),
            HsInfoRow(label: '作品指纹', value: read.fileHashHex),
            _PublicRightsRows(
              watermarkUid: read.watermarkUid,
              appState: appState,
            ),
          ],
          reason: reason,
          action: _FormalReportAction(
            canExportFormalReports: canExportFormalReports,
          ),
        ),
      ],
    );
  }
}

String _issueModeLabel(String value) => switch (value) {
  'server_reserved' => '后端预签发',
  'server_confirmed' => '后端已确认',
  'server_reissued' => '后端重签发',
  'offline_generated' => '离线生成',
  'registry_resolved' => 'registry 解析',
  _ => value,
};

String _formatDurationMs(int ms) {
  if (ms < 1000) return '${ms}ms';
  return '${(ms / 1000).toStringAsFixed(1)}s';
}

class _PublicRightsRows extends StatelessWidget {
  const _PublicRightsRows({required this.watermarkUid, required this.appState});

  final String watermarkUid;
  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    if (!appState.canQueryPublicRightsRegistry) {
      return const HsInfoRow(label: '公开权利信号', value: '未连接公开 registry');
    }
    return FutureBuilder<PublicRightsSdkResult>(
      future: PublicRightsScanner(appState: appState).scanOne(watermarkUid),
      builder: (context, snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return const HsInfoRow(label: '公开权利信号', value: '正在查询');
        }
        if (snapshot.hasError || snapshot.data == null) {
          return HsInfoRow(
            label: '公开权利信号',
            value: '查询失败: ${snapshot.error ?? '暂无结果'}',
          );
        }
        final result = snapshot.data!;
        final rights = result.scan;
        if (rights == null) {
          return HsInfoRow(label: '公开权利信号', value: result.message);
        }
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            HsInfoRow(
              label: '公开权利信号',
              value: publicRightsScanStatusLabel(rights.scanStatus),
            ),
            HsInfoRow(label: '训练许可', value: rights.trainingPermission.label),
            HsInfoRow(
              label: '锚点协议',
              value: publicRightsAnchorProtocolLabel(
                rights.registry.anchorProtocol,
              ),
            ),
            HsInfoRow(
              label: 'Manifest',
              value: rights.rightsManifest == null
                  ? '待回填'
                  : 'v${rights.rightsManifest!.manifestVersion}',
            ),
            HsInfoRow(label: '边界', value: result.message),
          ],
        );
      },
    );
  }
}

class _MessageDetail extends StatelessWidget {
  const _MessageDetail({
    this.detail,
    this.rows = const [],
    this.reason,
    this.action,
  });

  final String? detail;
  final List<Widget> rows;
  final MobileVerifyReason? reason;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        if (rows.isNotEmpty) ...rows else if (detail != null) Text(detail!),
        if (reason != null) ...[
          const SizedBox(height: HsSpacing.sm),
          Text(
            reason!.detail,
            style: const TextStyle(color: HsColors.textMuted, fontSize: 12),
          ),
          if (reason!.checklist.isNotEmpty) ...[
            const SizedBox(height: HsSpacing.sm),
            const Text(
              '建议检查',
              style: TextStyle(color: HsColors.textMuted, fontSize: 12),
            ),
            const SizedBox(height: 4),
            ...reason!.checklist.map(
              (item) => Padding(
                padding: const EdgeInsets.only(bottom: 3),
                child: Text(
                  '• $item',
                  style: const TextStyle(
                    color: HsColors.textMuted,
                    fontSize: 12,
                  ),
                ),
              ),
            ),
          ],
        ],
        if (action != null) ...[const SizedBox(height: HsSpacing.md), action!],
      ],
    );
  }
}

class _FormalReportAction extends StatelessWidget {
  const _FormalReportAction({required this.canExportFormalReports});

  final bool canExportFormalReports;

  @override
  Widget build(BuildContext context) {
    return OutlinedButton.icon(
      onPressed: () {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              canExportFormalReports ? '请到版权库记录详情中导出正式报告。' : 'Creator 起开放正式报告。',
            ),
          ),
        );
      },
      icon: Icon(
        canExportFormalReports
            ? Icons.picture_as_pdf_outlined
            : Icons.workspace_premium_outlined,
      ),
      label: Text(canExportFormalReports ? '到版权库导出报告' : 'Creator 导出正式报告'),
    );
  }
}

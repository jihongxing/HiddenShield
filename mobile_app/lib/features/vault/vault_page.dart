import 'dart:convert';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:share_plus/share_plus.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../shared/models/workspace_context.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import '../../shared/widgets/tool_cards.dart';
import '../public_rights/public_metadata_embedder.dart';
import '../public_rights/public_rights_scanner.dart';
import 'report_bundle_verifier.dart';
import 'report_handoff_bundle.dart';

class VaultPage extends StatefulWidget {
  const VaultPage({super.key, required this.bridge, required this.appState});

  final WatermarkBridge bridge;
  final MobileAppState appState;

  @override
  State<VaultPage> createState() => _VaultPageState();
}

class _VaultPageState extends State<VaultPage> {
  final TextEditingController _searchController = TextEditingController();
  WatermarkAssetKind? _kindFilter;
  VaultRecordSource? _sourceFilter;
  SyncStatus? _syncStatusFilter;
  bool _verifyingReportBundle = false;

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _setKindFilter(WatermarkAssetKind? value) {
    setState(() => _kindFilter = _kindFilter == value ? null : value);
  }

  void _setSourceFilter(VaultRecordSource? value) {
    setState(() => _sourceFilter = _sourceFilter == value ? null : value);
  }

  void _setSyncStatusFilter(SyncStatus? value) {
    setState(
      () => _syncStatusFilter = _syncStatusFilter == value ? null : value,
    );
  }

  void _clearFilters() {
    _searchController.clear();
    setState(() {
      _kindFilter = null;
      _sourceFilter = null;
      _syncStatusFilter = null;
    });
  }

  Future<void> _pickAndVerifyReportBundle() async {
    final reportDir = await FilePicker.getDirectoryPath(
      dialogTitle: '选择 HiddenShield 报告包目录',
    );
    if (reportDir == null || !mounted) return;
    setState(() => _verifyingReportBundle = true);
    try {
      final result = await verifyMobileReportBundle(reportDir);
      if (!mounted) return;
      await showDialog<void>(
        context: context,
        builder: (context) => _ReportBundleVerificationDialog(result: result),
      );
    } catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('报告包校验失败：$error')));
    } finally {
      if (mounted) setState(() => _verifyingReportBundle = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '版权库',
      subtitle: '集中管理作品记录、版本次数和验证结果。',
      icon: Icons.inventory_2_outlined,
      contextData: HsWorkspaceContext(
        eyebrow: '记录上下文',
        title: '版权记录详情',
        summary: '移动端版权库使用列表加详情页承载桌面右侧记录详情、报告状态、同步状态和时间线。',
        metrics: [
          HsContextMetric(
            label: '记录数',
            value: '${widget.appState.records.length}',
            tone: HsContextTone.ok,
          ),
          HsContextMetric(
            label: '正式报告',
            value: widget.appState.canExportFormalReports ? '可导出' : '购买或升级',
            tone: widget.appState.canExportFormalReports
                ? HsContextTone.ok
                : HsContextTone.warning,
          ),
          HsContextMetric(
            label: '云同步',
            value: widget.appState.canUseCloudSync ? '可同步' : '本地优先',
            tone: widget.appState.canUseCloudSync
                ? HsContextTone.ok
                : HsContextTone.muted,
          ),
        ],
      ),
      children: [
        HsPanel(
          radius: HsRadii.panel,
          color: HsColors.surfaceRaised,
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Icon(Icons.fact_check_outlined, color: HsColors.accent),
              const SizedBox(width: HsSpacing.md),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      '校验桌面报告包',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: HsSpacing.sm),
                    const Text(
                      '选择包含 report.pdf、report.json 和 manifest.json 的目录。移动端只读复算文件摘要与 Manifest 链，不读取原始媒体，也不判断签名可信。',
                      style: TextStyle(color: HsColors.textMuted, height: 1.45),
                    ),
                    const SizedBox(height: HsSpacing.md),
                    OutlinedButton.icon(
                      key: const ValueKey('verify-report-bundle-button'),
                      onPressed: _verifyingReportBundle
                          ? null
                          : _pickAndVerifyReportBundle,
                      icon: const Icon(Icons.folder_open_outlined),
                      label: Text(_verifyingReportBundle ? '校验中…' : '选择报告包并校验'),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        AnimatedBuilder(
          animation: widget.appState,
          builder: (context, _) => _VaultSummary(
            appState: widget.appState,
            searchController: _searchController,
            kindFilter: _kindFilter,
            sourceFilter: _sourceFilter,
            syncStatusFilter: _syncStatusFilter,
            onSearchChanged: (_) => setState(() {}),
            onKindFilterChanged: _setKindFilter,
            onSourceFilterChanged: _setSourceFilter,
            onSyncStatusFilterChanged: _setSyncStatusFilter,
            onClearFilters: _clearFilters,
          ),
        ),
      ],
    );
  }
}

class _ReportBundleVerificationDialog extends StatelessWidget {
  const _ReportBundleVerificationDialog({required this.result});

  final MobileReportBundleVerificationResult result;

  @override
  Widget build(BuildContext context) {
    final matched = result.isIntegrityMatched;
    return AlertDialog(
      title: Row(
        children: [
          Icon(
            matched ? Icons.verified_outlined : Icons.warning_amber_outlined,
            color: matched ? HsColors.accent : HsColors.warning,
          ),
          const SizedBox(width: 10),
          Expanded(child: Text(matched ? '报告包文件匹配' : '报告包校验失败')),
        ],
      ),
      content: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(result.message),
            const SizedBox(height: 16),
            _ReportVerificationRow(label: '报告编号', value: result.reportId),
            _ReportVerificationRow(
              label: '报告版本',
              value: '第 ${result.bundleVersion} 版',
            ),
            _ReportVerificationRow(
              label: '文件完整性',
              value: matched ? '文件匹配' : '不匹配',
            ),
            _ReportVerificationRow(
              label: 'Manifest 链',
              value: result.manifestChainStatus == 'matched' ? '匹配' : '不匹配',
            ),
            _ReportVerificationRow(
              label: 'report.json 合同',
              value: result.documentContractStatus == 'matched' ? '匹配' : '不匹配',
            ),
            _ReportVerificationRow(
              label: '数字签名',
              value: result.signatureStatus == 'not_signed' ? '未签名' : '存在但未验证',
            ),
            _ReportVerificationRow(
              label: '报告包可信时间',
              value: result.trustedTimeStatus == 'not_timestamped'
                  ? '未加盖'
                  : '存在但未验证',
            ),
            const SizedBox(height: 12),
            ...result.files.map(
              (file) => Text(
                '${file.path}：${file.status == 'matched' ? '匹配' : file.status}',
                style: TextStyle(
                  color: file.status == 'matched'
                      ? HsColors.accent
                      : HsColors.warning,
                ),
              ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('完成'),
        ),
      ],
    );
  }
}

class _ReportVerificationRow extends StatelessWidget {
  const _ReportVerificationRow({required this.label, required this.value});

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
            width: 116,
            child: Text(
              label,
              style: const TextStyle(color: HsColors.textMuted),
            ),
          ),
          Expanded(child: Text(value)),
        ],
      ),
    );
  }
}

class _VaultSummary extends StatelessWidget {
  const _VaultSummary({
    required this.appState,
    required this.searchController,
    required this.kindFilter,
    required this.sourceFilter,
    required this.syncStatusFilter,
    required this.onSearchChanged,
    required this.onKindFilterChanged,
    required this.onSourceFilterChanged,
    required this.onSyncStatusFilterChanged,
    required this.onClearFilters,
  });

  final MobileAppState appState;
  final TextEditingController searchController;
  final WatermarkAssetKind? kindFilter;
  final VaultRecordSource? sourceFilter;
  final SyncStatus? syncStatusFilter;
  final ValueChanged<String> onSearchChanged;
  final ValueChanged<WatermarkAssetKind?> onKindFilterChanged;
  final ValueChanged<VaultRecordSource?> onSourceFilterChanged;
  final ValueChanged<SyncStatus?> onSyncStatusFilterChanged;
  final VoidCallback onClearFilters;

  @override
  Widget build(BuildContext context) {
    final records = appState.records;
    if (records.isEmpty) {
      return const _EmptyVaultCard();
    }
    final filteredRecords = _filterRecords(
      records: records,
      query: searchController.text,
      kindFilter: kindFilter,
      sourceFilter: sourceFilter,
      syncStatusFilter: syncStatusFilter,
    );
    final hasActiveFilters =
        searchController.text.trim().isNotEmpty ||
        kindFilter != null ||
        sourceFilter != null ||
        syncStatusFilter != null;

    return Column(
      children: [
        _StatsCard(
          total: records.length,
          pendingSync: appState.pendingSyncCount,
        ),
        const SizedBox(height: 12),
        _TeamWorkspaceCard(appState: appState),
        const SizedBox(height: 12),
        _VaultFilterPanel(
          searchController: searchController,
          kindFilter: kindFilter,
          sourceFilter: sourceFilter,
          syncStatusFilter: syncStatusFilter,
          filteredCount: filteredRecords.length,
          totalCount: records.length,
          hasActiveFilters: hasActiveFilters,
          onSearchChanged: onSearchChanged,
          onKindFilterChanged: onKindFilterChanged,
          onSourceFilterChanged: onSourceFilterChanged,
          onSyncStatusFilterChanged: onSyncStatusFilterChanged,
          onClearFilters: onClearFilters,
        ),
        const SizedBox(height: 12),
        if (filteredRecords.isEmpty)
          _EmptyFilterResultCard(onClearFilters: onClearFilters)
        else
          ...filteredRecords.map(
            (record) => _VaultRecordCard(
              record: record,
              appState: appState,
              canExportFormalReports: appState.canExportFormalReports,
            ),
          ),
      ],
    );
  }
}

class _TeamWorkspaceCard extends StatelessWidget {
  const _TeamWorkspaceCard({required this.appState});

  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    final canUseTeamWorkspace = appState.canUseTeamWorkspace;
    return HsPanel(
      color: HsColors.surfaceRaised,
      radius: HsRadii.panel,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(Icons.groups_2_outlined, color: HsColors.accent),
          const SizedBox(width: HsSpacing.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        '团队空间',
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                    ),
                    HsStatusChip(
                      label: canUseTeamWorkspace ? '权益已包含' : 'Studio',
                    ),
                  ],
                ),
                const SizedBox(height: HsSpacing.sm),
                Text(
                  canUseTeamWorkspace
                      ? '当前权益已包含团队空间入口。共享版权库、成员权限和团队审计模型仍在建设中。'
                      : 'Studio 起预留共享版权库、成员权限和团队审计模型；当前记录保存在个人版权库。',
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
    );
  }
}

class _EmptyVaultCard extends StatelessWidget {
  const _EmptyVaultCard();

  @override
  Widget build(BuildContext context) {
    return const HsPanel(
      radius: HsRadii.panel,
      padding: EdgeInsets.all(HsSpacing.xl),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.folder_open_outlined, color: HsColors.accent),
          SizedBox(height: 12),
          Text('还没有作品记录'),
          SizedBox(height: 8),
          Text(
            '完成图片或音频处理后，记录会自动进入这里。验证命中也会保存为本机记录。',
            style: TextStyle(color: HsColors.textMuted),
          ),
        ],
      ),
    );
  }
}

class _StatsCard extends StatelessWidget {
  const _StatsCard({required this.total, required this.pendingSync});

  final int total;
  final int pendingSync;

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      color: HsColors.surfaceRaised,
      radius: HsRadii.panel,
      child: Row(
        children: [
          Expanded(
            child: _Metric(label: '记录', value: '$total'),
          ),
          Expanded(
            child: _Metric(label: '待同步', value: '$pendingSync'),
          ),
        ],
      ),
    );
  }
}

class _VaultFilterPanel extends StatelessWidget {
  const _VaultFilterPanel({
    required this.searchController,
    required this.kindFilter,
    required this.sourceFilter,
    required this.syncStatusFilter,
    required this.filteredCount,
    required this.totalCount,
    required this.hasActiveFilters,
    required this.onSearchChanged,
    required this.onKindFilterChanged,
    required this.onSourceFilterChanged,
    required this.onSyncStatusFilterChanged,
    required this.onClearFilters,
  });

  final TextEditingController searchController;
  final WatermarkAssetKind? kindFilter;
  final VaultRecordSource? sourceFilter;
  final SyncStatus? syncStatusFilter;
  final int filteredCount;
  final int totalCount;
  final bool hasActiveFilters;
  final ValueChanged<String> onSearchChanged;
  final ValueChanged<WatermarkAssetKind?> onKindFilterChanged;
  final ValueChanged<VaultRecordSource?> onSourceFilterChanged;
  final ValueChanged<SyncStatus?> onSyncStatusFilterChanged;
  final VoidCallback onClearFilters;

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      radius: HsRadii.panel,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          TextField(
            controller: searchController,
            onChanged: onSearchChanged,
            decoration: InputDecoration(
              prefixIcon: const Icon(Icons.search_outlined),
              suffixIcon: searchController.text.trim().isEmpty
                  ? null
                  : IconButton(
                      tooltip: '清空搜索',
                      onPressed: onClearFilters,
                      icon: const Icon(Icons.close_outlined),
                    ),
              labelText: '搜索作品记录',
              hintText: '标题、版权编号、作品指纹',
            ),
          ),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              _FilterChipItem(
                label: '图片',
                icon: Icons.image_outlined,
                selected: kindFilter == WatermarkAssetKind.image,
                onSelected: () => onKindFilterChanged(WatermarkAssetKind.image),
              ),
              _FilterChipItem(
                label: '音频',
                icon: Icons.graphic_eq_outlined,
                selected: kindFilter == WatermarkAssetKind.audio,
                onSelected: () => onKindFilterChanged(WatermarkAssetKind.audio),
              ),
              _FilterChipItem(
                label: '视频',
                icon: Icons.video_file_outlined,
                selected: kindFilter == WatermarkAssetKind.video,
                onSelected: () => onKindFilterChanged(WatermarkAssetKind.video),
              ),
              _FilterChipItem(
                label: '写入',
                icon: Icons.edit_note_outlined,
                selected: sourceFilter == VaultRecordSource.write,
                onSelected: () =>
                    onSourceFilterChanged(VaultRecordSource.write),
              ),
              _FilterChipItem(
                label: '验证',
                icon: Icons.search_outlined,
                selected: sourceFilter == VaultRecordSource.verify,
                onSelected: () =>
                    onSourceFilterChanged(VaultRecordSource.verify),
              ),
              _FilterChipItem(
                label: '待同步',
                icon: Icons.pending_actions_outlined,
                selected: syncStatusFilter == SyncStatus.pending,
                onSelected: () => onSyncStatusFilterChanged(SyncStatus.pending),
              ),
              _FilterChipItem(
                label: '已同步',
                icon: Icons.cloud_done_outlined,
                selected: syncStatusFilter == SyncStatus.synced,
                onSelected: () => onSyncStatusFilterChanged(SyncStatus.synced),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Row(
            children: [
              Expanded(
                child: Text(
                  hasActiveFilters
                      ? '显示 $filteredCount / $totalCount 条记录'
                      : '显示全部 $totalCount 条记录',
                  style: const TextStyle(color: HsColors.textMuted),
                ),
              ),
              TextButton.icon(
                onPressed: hasActiveFilters ? onClearFilters : null,
                icon: const Icon(Icons.filter_alt_off_outlined),
                label: const Text('重置'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _FilterChipItem extends StatelessWidget {
  const _FilterChipItem({
    required this.label,
    required this.icon,
    required this.selected,
    required this.onSelected,
  });

  final String label;
  final IconData icon;
  final bool selected;
  final VoidCallback onSelected;

  @override
  Widget build(BuildContext context) {
    return FilterChip(
      key: ValueKey('vault-filter-$label'),
      selected: selected,
      onSelected: (_) => onSelected(),
      avatar: Icon(icon, size: 18),
      label: Text(label),
      showCheckmark: false,
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
      backgroundColor: HsColors.chip,
      selectedColor: HsColors.accentSeed,
      side: BorderSide.none,
    );
  }
}

class _EmptyFilterResultCard extends StatelessWidget {
  const _EmptyFilterResultCard({required this.onClearFilters});

  final VoidCallback onClearFilters;

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      radius: HsRadii.panel,
      padding: const EdgeInsets.all(HsSpacing.xl),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(Icons.manage_search_outlined, color: HsColors.accent),
          const SizedBox(height: 12),
          const Text('没有匹配的作品记录'),
          const SizedBox(height: 8),
          const Text(
            '换一个标题、版权编号或作品指纹，或者重置筛选条件。',
            style: TextStyle(color: HsColors.textMuted),
          ),
          const SizedBox(height: 12),
          OutlinedButton.icon(
            onPressed: onClearFilters,
            icon: const Icon(Icons.filter_alt_off_outlined),
            label: const Text('重置筛选'),
          ),
        ],
      ),
    );
  }
}

class _Metric extends StatelessWidget {
  const _Metric({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(value, style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 4),
        Text(label, style: const TextStyle(color: HsColors.textMuted)),
      ],
    );
  }
}

class _VaultRecordCard extends StatelessWidget {
  const _VaultRecordCard({
    required this.record,
    required this.appState,
    required this.canExportFormalReports,
  });

  final VaultRecord record;
  final MobileAppState appState;
  final bool canExportFormalReports;

  @override
  Widget build(BuildContext context) {
    final sha = record.sha256;
    final verificationLabel = writeVerificationStatusLabel(
      record.writeVerificationStatus,
    );
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Card(
        key: ValueKey('vault-record-${record.id}'),
        elevation: 0,
        color: HsColors.surface,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(HsRadii.panel),
        ),
        child: ListTile(
          onTap: () => _showVaultRecordDetails(
            context,
            record,
            appState: appState,
            canExportFormalReports: canExportFormalReports,
          ),
          leading: Icon(_kindIcon(record.kind), color: HsColors.accent),
          title: Text(
            record.title,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          subtitle: Text(
            [
              '${vaultRecordSourceLabel(record.source)} · ${_kindLabel(record.kind)} · 第 ${record.revision} 次',
              '版权编号: ${record.watermarkUid}',
              '创作者身份: ${_displayValue(record.creatorDisplayName)}',
              '第三方验证: ${_displayValue(record.thirdPartyVerificationStatus)}',
              '可信时间: ${_displayValue(record.trustedTimeStatus)}',
              if (record.parentWatermarkUid != null) '包含上一版本记录',
              if (record.rewriteReason != null) '说明: ${record.rewriteReason}',
              if (record.writeVerificationStatus != null)
                '完成后验证: $verificationLabel',
              if (record.videoNotaryId != null)
                '视频指纹存证: ${record.videoNotaryId}',
              if (sha != null) '作品指纹: ${_shorten(sha)}',
            ].join('\n'),
          ),
          trailing: _RecordStatusColumn(record: record),
        ),
      ),
    );
  }
}

class _RecordStatusColumn extends StatelessWidget {
  const _RecordStatusColumn({required this.record});

  final VaultRecord record;

  @override
  Widget build(BuildContext context) {
    return HsStatusChip(label: syncStatusLabel(record.syncStatus));
  }
}

void _showVaultRecordDetails(
  BuildContext context,
  VaultRecord record, {
  required MobileAppState appState,
  required bool canExportFormalReports,
}) {
  showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    backgroundColor: HsColors.appBar,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(top: Radius.circular(HsRadii.sheet)),
    ),
    builder: (context) => _VaultRecordDetailsSheet(
      record: record,
      appState: appState,
      canExportFormalReports: canExportFormalReports,
    ),
  );
}

class _VaultRecordDetailsSheet extends StatelessWidget {
  const _VaultRecordDetailsSheet({
    required this.record,
    required this.appState,
    required this.canExportFormalReports,
  });

  final VaultRecord record;
  final MobileAppState appState;
  final bool canExportFormalReports;

  @override
  Widget build(BuildContext context) {
    final canExportRecordReport = appState.canExportFormalReportForRecord(
      record,
    );
    return SizedBox(
      height: MediaQuery.sizeOf(context).height * 0.86,
      child: ListView(
        padding: const EdgeInsets.fromLTRB(20, 16, 20, 24),
        children: [
          Center(
            child: Container(
              width: 36,
              height: 4,
              decoration: BoxDecoration(
                color: HsColors.border,
                borderRadius: BorderRadius.circular(HsRadii.pill),
              ),
            ),
          ),
          const SizedBox(height: 20),
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(_kindIcon(record.kind), color: HsColors.accent),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      record.title,
                      style: Theme.of(context).textTheme.titleLarge,
                    ),
                    const SizedBox(height: 6),
                    Text(
                      '${vaultRecordSourceLabel(record.source)} · ${_kindLabel(record.kind)} · ${syncStatusLabel(record.syncStatus)}',
                      style: const TextStyle(color: HsColors.textMuted),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 20),
          if (_needsRegistryAction(record)) ...[
            _RegistryArbitrationCard(record: record, appState: appState),
            const SizedBox(height: 12),
          ],
          _DetailGroup(
            title: '版权信息',
            rows: [
              _DetailRow(label: '版权编号', value: record.watermarkUid),
              _DetailRow(label: '版本次数', value: '第 ${record.revision} 次'),
              _DetailRow(label: '创作者身份', value: record.creatorDisplayName),
              _DetailRow(label: '上一版本', value: record.parentWatermarkUid),
              _DetailRow(label: '更新说明', value: record.rewriteReason),
            ],
          ),
          const SizedBox(height: 12),
          _DetailGroup(
            title: '第三方验证 / 可信时间',
            rows: [
              _DetailRow(
                label: '第三方验证',
                value: record.thirdPartyVerificationStatus,
              ),
              _DetailRow(
                label: '验证服务',
                value: record.thirdPartyVerificationProvider,
              ),
              _DetailRow(
                label: '验证路径',
                value: record.thirdPartyVerificationPath,
              ),
              _DetailRow(label: '可信时间', value: record.trustedTimeStatus),
              _DetailRow(label: '时间来源', value: record.trustedTimeSource),
              _DetailRow(
                label: '记录时间',
                value: record.trustedTimeAt == null
                    ? null
                    : _formatDateTime(record.trustedTimeAt!),
              ),
            ],
          ),
          const SizedBox(height: 12),
          _DetailGroup(
            title: '文件指纹',
            rows: [
              _DetailRow(label: '作品指纹', value: record.sha256),
              _DetailRow(label: '记录片段', value: record.extractedFileHashHex),
            ],
          ),
          const SizedBox(height: 12),
          _DetailGroup(
            title: '保护副本',
            rows: [
              _DetailRow(label: '保护副本名称', value: record.protectedCopyName),
              _DetailRow(label: '保护副本摘要', value: record.protectedCopyHash),
              _DetailRow(
                label: '输出策略',
                value: _outputStrategyLabel(record.outputStrategy),
              ),
            ],
          ),
          const SizedBox(height: 12),
          _DetailGroup(
            title: 'Payload 与登记',
            rows: [
              _DetailRow(
                label: 'Payload 协议',
                value:
                    'V${record.payloadProtocolVersion} / ${record.payloadBytesLength} bytes',
              ),
              _DetailRow(
                label: '编号签发',
                value: _watermarkIssueModeLabel(record.watermarkIdIssueMode),
              ),
              _DetailRow(
                label: '登记状态',
                value: _registryStatusLabel(record.watermarkIdRegistryStatus),
              ),
              _DetailRow(
                label: '登记收据',
                value: record.watermarkIdRegistryReceipt,
              ),
              _DetailRow(
                label: 'Payload 认证',
                value: _payloadAuthStatusLabel(record.payloadAuthStatus),
              ),
            ],
          ),
          const SizedBox(height: 12),
          _DetailGroup(
            title: '作品声明与授权策略',
            rows: [
              _DetailRow(
                label: '作品来源声明',
                value: _workSourceDeclarationLabel(
                  record.workSourceDeclaration,
                ),
              ),
              _DetailRow(
                label: '训练许可声明',
                value: _trainingPermissionLabel(
                  record.trainingPermissionDeclaration,
                ),
              ),
              _DetailRow(
                label: '创作方式声明',
                value: record.creationMethodDeclaration,
              ),
              _DetailRow(
                label: '人工编辑声明',
                value: record.humanEditLevelDeclaration,
              ),
              _DetailRow(
                label: '真实性声明',
                value: _authenticityClaimLabel(
                  record.authenticityClaimDeclaration,
                ),
              ),
              _DetailRow(label: '自定义版权声明', value: record.customRightsStatement),
            ],
          ),
          const SizedBox(height: 12),
          _PublicRightsRegistryCard(
            watermarkUid: record.watermarkUid,
            appState: appState,
            canExportEmbeddedImage: record.kind == WatermarkAssetKind.image,
          ),
          if (record.videoNotaryId != null) ...[
            const SizedBox(height: 12),
            _DetailGroup(
              title: '视频指纹存证',
              rows: [
                _DetailRow(label: '存证编号', value: record.videoNotaryId),
                _DetailRow(
                  label: '存证时间',
                  value: record.videoNotaryAt == null
                      ? null
                      : _formatDateTime(record.videoNotaryAt!),
                ),
                _DetailRow(label: '指纹根', value: record.videoFingerprintRoot),
                _DetailRow(label: '指纹包摘要', value: record.videoBundleSha256),
                _DetailRow(
                  label: '采样帧',
                  value: record.videoBundleSceneCount?.toString(),
                ),
                _DetailRow(
                  label: '生成耗时',
                  value: record.videoBundleElapsedMs == null
                      ? null
                      : '${(record.videoBundleElapsedMs! / 1000).toStringAsFixed(1)}s',
                ),
                _DetailRow(label: '采样策略', value: record.videoFrameSamplePolicy),
              ],
            ),
          ],
          if (record.videoVisualTaskId != null ||
              record.videoVisualMediaHash != null) ...[
            const SizedBox(height: 12),
            _DetailGroup(
              title: 'L3 视频画面盲水印',
              rows: [
                _DetailRow(label: '任务编号', value: record.videoVisualTaskId),
                _DetailRow(
                  label: '完成时间',
                  value: record.videoVisualCompletedAt == null
                      ? null
                      : _formatDateTime(record.videoVisualCompletedAt!),
                ),
                _DetailRow(
                  label: '策略摘要',
                  value: record.videoVisualStrategyDigest,
                ),
                _DetailRow(
                  label: '自检置信度',
                  value: record.videoVisualSelfCheckConfidence?.toStringAsFixed(
                    6,
                  ),
                ),
                _DetailRow(
                  label: '自检阈值',
                  value: record.videoVisualSelfCheckThreshold?.toStringAsFixed(
                    6,
                  ),
                ),
                _DetailRow(
                  label: '检查帧数',
                  value: record.videoVisualCheckedFrames?.toString(),
                ),
                _DetailRow(label: '成品摘要', value: record.videoVisualMediaHash),
                _DetailRow(
                  label: 'Worker 收据',
                  value: record.videoVisualReceiptHash,
                ),
                _DetailRow(
                  label: '成品字节数',
                  value: record.videoVisualOutputBytes?.toString(),
                ),
                _DetailRow(
                  label: '成品内容类型',
                  value: record.videoVisualOutputContentType,
                ),
              ],
            ),
          ],
          const SizedBox(height: 12),
          _DetailGroup(
            title: '完成后验证',
            rows: [
              _DetailRow(
                label: '验证状态',
                value: writeVerificationStatusLabel(
                  record.writeVerificationStatus,
                ),
              ),
              _DetailRow(
                label: '验证时间',
                value: record.writeVerificationAt == null
                    ? null
                    : _formatDateTime(record.writeVerificationAt!),
              ),
              _DetailRow(label: '验证说明', value: record.writeVerificationMessage),
            ],
          ),
          const SizedBox(height: 12),
          _DetailGroup(
            title: record.source == VaultRecordSource.write
                ? '写入后验证信息'
                : '验证提取信息',
            rows: _verificationInfoRows(record),
          ),
          const SizedBox(height: 12),
          _DetailGroup(
            title: '本地记录',
            rows: [
              _DetailRow(label: '记录编号', value: record.id),
              _DetailRow(
                label: '创建时间',
                value: _formatDateTime(record.createdAt),
              ),
              _DetailRow(
                label: '同步状态',
                value: syncStatusLabel(record.syncStatus),
              ),
              _DetailRow(
                label: '来源',
                value: vaultRecordSourceLabel(record.source),
              ),
            ],
          ),
          const SizedBox(height: 12),
          OutlinedButton.icon(
            onPressed: () async {
              await Clipboard.setData(
                ClipboardData(text: appState.buildCopyrightSummary(record)),
              );
              if (!context.mounted) return;
              ScaffoldMessenger.of(
                context,
              ).showSnackBar(const SnackBar(content: Text('已复制存证摘要')));
            },
            icon: const Icon(Icons.copy_all_outlined),
            label: const Text('复制存证摘要'),
          ),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            onPressed: () async {
              if (!canExportRecordReport) {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('可购买单份版权详细报告，或开通 Creator。')),
                );
                return;
              }
              try {
                final draft = await appState.buildFormalReportDraft(record);
                if (!context.mounted) return;
                final bundle = buildMobileReportHandoffBundle(
                  record: record,
                  draft: draft,
                );
                final box = context.findRenderObject() as RenderBox?;
                final sharePositionOrigin = box == null
                    ? null
                    : box.localToGlobal(Offset.zero) & box.size;
                await SharePlus.instance.share(
                  ShareParams(
                    files: [
                      XFile.fromData(
                        bundle.reportJsonBytes,
                        mimeType: 'application/json',
                        name: 'report.json',
                      ),
                      XFile.fromData(
                        bundle.manifestJsonBytes,
                        mimeType: 'application/json',
                        name: 'manifest.json',
                      ),
                    ],
                    subject: 'HiddenShield 桌面签发交接包',
                    text: '该交接包尚未生成 PDF，也未完成数字签名或报告包可信时间。',
                    fileNameOverrides: const ['report.json', 'manifest.json'],
                    sharePositionOrigin: sharePositionOrigin,
                  ),
                );
                if (!context.mounted) return;
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text('桌面签发交接包已打开分享面板：${draft.reportId}')),
                );
              } catch (error) {
                if (!context.mounted) return;
                ScaffoldMessenger.of(
                  context,
                ).showSnackBar(SnackBar(content: Text(error.toString())));
              }
            },
            icon: Icon(
              canExportRecordReport
                  ? Icons.drive_folder_upload_outlined
                  : Icons.workspace_premium_outlined,
            ),
            label: Text(canExportRecordReport ? '生成桌面签发交接包' : 'Creator 正式报告'),
          ),
          if (!canExportFormalReports) ...[
            const SizedBox(height: 8),
            OutlinedButton.icon(
              onPressed: () => _buySingleReport(
                context,
                appState,
                record,
                'copyright_report_single',
              ),
              icon: const Icon(Icons.description_outlined),
              label: const Text('购买版权详细报告 · 19.9 元'),
            ),
            const SizedBox(height: 8),
            OutlinedButton.icon(
              onPressed: () => _buySingleReport(
                context,
                appState,
                record,
                'rights_evidence_pack_single',
              ),
              icon: const Icon(Icons.fact_check_outlined),
              label: const Text('购买维权证据包 · 49.9 元'),
            ),
          ],
        ],
      ),
    );
  }
}

Future<void> _buySingleReport(
  BuildContext context,
  MobileAppState appState,
  VaultRecord record,
  String productCode,
) async {
  final messenger = ScaffoldMessenger.of(context);
  final session = await appState.createReportPurchaseSession(
    record: record,
    productCode: productCode,
  );
  if (session == null) {
    messenger.showSnackBar(
      SnackBar(content: Text(appState.latestPaymentMessage ?? '暂不能创建报告购买会话')),
    );
    return;
  }
  final granted = await appState.reconcileReportPurchaseSession(
    paymentSessionId: session.paymentSessionId,
  );
  if (!context.mounted) return;
  if (!granted) {
    messenger.showSnackBar(
      SnackBar(content: Text(appState.latestPaymentMessage ?? '暂未确认支付完成')),
    );
    return;
  }
  final draft = await appState.buildFormalReportDraft(record);
  await Clipboard.setData(ClipboardData(text: draft.markdown));
  if (!context.mounted) return;
  messenger.showSnackBar(
    SnackBar(content: Text('已购买并复制正式报告草稿：${draft.reportId}')),
  );
}

List<_DetailRow> _verificationInfoRows(VaultRecord record) {
  if (record.source == VaultRecordSource.write) {
    return [
      _DetailRow(
        label: '回读时间',
        value: record.writeVerificationAt == null
            ? null
            : _formatDateTime(record.writeVerificationAt!),
      ),
      _DetailRow(label: '来源设备', value: record.extractedDeviceIdHex),
      _DetailRow(label: '记录片段', value: record.extractedFileHashHex),
    ];
  }
  return [
    _DetailRow(
      label: '记录时间',
      value: record.extractedTimestamp == null
          ? null
          : _formatUnixSeconds(record.extractedTimestamp!),
    ),
    _DetailRow(label: '来源设备', value: record.extractedDeviceIdHex),
  ];
}

String _outputStrategyLabel(String value) {
  return value == 'minimal_required_change' || value.isEmpty ? '最小必要变更' : value;
}

String _workSourceDeclarationLabel(String value) {
  return switch (value) {
    'human_created' => '人工创作',
    'ai_assisted' => 'AI 辅助',
    'ai_generated' => 'AI 生成',
    _ => '未声明',
  };
}

String _trainingPermissionLabel(String value) {
  return switch (value) {
    'separate_authorization_required' => '需单独授权',
    'non_commercial_allowed' => '允许非商业训练',
    'commercial_allowed' => '允许商业训练',
    'unspecified' => '未声明',
    _ => '禁止模型训练',
  };
}

String _authenticityClaimLabel(String value) {
  return switch (value) {
    'synthetic' => '虚构或合成',
    'based_on_reality' => '基于真实',
    'creator_claimed_authentic' || 'authentic' => '创作者声明真实',
    _ => '未声明',
  };
}

class _DetailGroup extends StatelessWidget {
  const _DetailGroup({required this.title, required this.rows});

  final String title;
  final List<_DetailRow> rows;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: HsColors.surface,
        borderRadius: BorderRadius.circular(HsRadii.panel),
        border: Border.all(color: HsColors.border),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 12),
            ...rows.map((row) => _DetailLine(row: row)),
          ],
        ),
      ),
    );
  }
}

class _DetailLine extends StatelessWidget {
  const _DetailLine({required this.row});

  final _DetailRow row;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 96,
            child: Text(
              row.label,
              style: const TextStyle(color: HsColors.textMuted),
            ),
          ),
          Expanded(
            child: SelectableText(
              row.displayValue,
              style: const TextStyle(color: HsColors.text),
            ),
          ),
        ],
      ),
    );
  }
}

class _PublicRightsRegistryCard extends StatefulWidget {
  const _PublicRightsRegistryCard({
    required this.watermarkUid,
    required this.appState,
    required this.canExportEmbeddedImage,
  });

  final String watermarkUid;
  final MobileAppState appState;
  final bool canExportEmbeddedImage;

  @override
  State<_PublicRightsRegistryCard> createState() =>
      _PublicRightsRegistryCardState();
}

class _PublicRightsRegistryCardState extends State<_PublicRightsRegistryCard> {
  bool _exporting = false;
  bool _exportingEmbeddedImage = false;

  Future<void> _shareMetadataJson() async {
    setState(() => _exporting = true);
    final box = context.findRenderObject() as RenderBox?;
    final sharePositionOrigin = box == null
        ? null
        : box.localToGlobal(Offset.zero) & box.size;
    try {
      final metadata = await widget.appState.fetchPublicRightsMetadata(
        widget.watermarkUid,
      );
      final jsonText = const JsonEncoder.withIndent('  ').convert(metadata);
      final fileName =
          'hiddenshield-public-rights-${_safeFilePart(widget.watermarkUid)}.json';
      await SharePlus.instance.share(
        ShareParams(
          files: [
            XFile.fromData(
              Uint8List.fromList(utf8.encode('$jsonText\n')),
              mimeType: 'application/json',
              name: fileName,
            ),
          ],
          subject: 'HiddenShield 公开权利元数据',
          text: 'HiddenShield 公开权利元数据 JSON',
          fileNameOverrides: [fileName],
          sharePositionOrigin: sharePositionOrigin,
        ),
      );
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('公开元数据 JSON 已打开分享面板')));
    } catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('导出公开元数据失败：$error')));
    } finally {
      if (mounted) setState(() => _exporting = false);
    }
  }

  Future<void> _selectAndShareEmbeddedImage() async {
    setState(() => _exportingEmbeddedImage = true);
    final box = context.findRenderObject() as RenderBox?;
    final sharePositionOrigin = box == null
        ? null
        : box.localToGlobal(Offset.zero) & box.size;
    try {
      final picked = await FilePicker.pickFiles(
        type: FileType.custom,
        allowedExtensions: const ['png', 'jpg', 'jpeg'],
        withData: true,
      );
      final file = picked?.files.single;
      final sourceBytes = file?.bytes;
      if (file == null || sourceBytes == null) {
        return;
      }
      final format = detectPublicMetadataImageFormat(sourceBytes);
      final metadata = await widget.appState.fetchPublicRightsMetadata(
        widget.watermarkUid,
      );
      final metadataUid = metadata['watermarkUid']?.toString().trim() ?? '';
      if (metadataUid != widget.watermarkUid) {
        throw StateError('公开元数据 watermarkUid 与本地版权记录不一致，已阻断嵌入导出。');
      }
      final embedded = embedPublicRightsMetadataInImage(
        sourceBytes: sourceBytes,
        metadata: metadata,
        format: format,
      );
      final manifestHash = metadata['manifestHash']?.toString() ?? '';
      final checks = checkEmbeddedPublicMetadataBytes(
        bytes: embedded.bytes,
        format: embedded.format,
        watermarkUid: widget.watermarkUid,
        manifestHash: manifestHash,
      );
      if (!checks.pass) {
        throw StateError('嵌入副本字节检查未通过，请重新选择 PNG / JPEG 保护副本。');
      }
      final extension = embedded.format == PublicMetadataImageFormat.jpeg
          ? 'jpg'
          : 'png';
      final fileName =
          'hiddenshield-public-rights-${_safeFilePart(widget.watermarkUid)}-embedded.$extension';
      await SharePlus.instance.share(
        ShareParams(
          files: [
            XFile.fromData(
              embedded.bytes,
              mimeType: embedded.format == PublicMetadataImageFormat.jpeg
                  ? 'image/jpeg'
                  : 'image/png',
              name: fileName,
              length: embedded.bytes.length,
            ),
          ],
          subject: 'HiddenShield 嵌入公开元数据图片副本',
          text: 'HiddenShield 公开权利元数据图片副本',
          fileNameOverrides: [fileName],
          sharePositionOrigin: sharePositionOrigin,
        ),
      );
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('嵌入公开元数据图片副本已打开分享面板')));
    } catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('导出嵌入元数据图片副本失败：$error')));
    } finally {
      if (mounted) setState(() => _exportingEmbeddedImage = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    if (!widget.appState.canQueryPublicRightsRegistry) {
      return const _DetailGroup(
        title: '公开权利信号',
        rows: [
          _DetailRow(label: '状态', value: '未连接公开 registry'),
          _DetailRow(label: '说明', value: '当前仅显示本机版权库声明。'),
        ],
      );
    }
    return FutureBuilder<PublicRightsSdkResult>(
      future: PublicRightsScanner(
        appState: widget.appState,
      ).scanOne(widget.watermarkUid),
      builder: (context, snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return const _DetailGroup(
            title: '公开权利信号',
            rows: [_DetailRow(label: '状态', value: '正在查询')],
          );
        }
        if (snapshot.hasError || snapshot.data == null) {
          return _DetailGroup(
            title: '公开权利信号',
            rows: [
              _DetailRow(label: '状态', value: '查询失败'),
              _DetailRow(label: '说明', value: '${snapshot.error ?? '暂无结果'}'),
            ],
          );
        }
        final result = snapshot.data!;
        final rights = result.scan;
        if (rights == null) {
          return _DetailGroup(
            title: '公开权利信号',
            rows: [
              const _DetailRow(label: '状态', value: '查询失败'),
              _DetailRow(label: '说明', value: result.message),
            ],
          );
        }
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _DetailGroup(
              title: '公开权利信号',
              rows: [
                _DetailRow(
                  label: '扫描状态',
                  value: publicRightsScanStatusLabel(rights.scanStatus),
                ),
                _DetailRow(
                  label: '训练许可',
                  value: rights.trainingPermission.label,
                ),
                _DetailRow(
                  label: '锚点协议',
                  value: publicRightsAnchorProtocolLabel(
                    rights.registry.anchorProtocol,
                  ),
                ),
                _DetailRow(
                  label: 'Manifest',
                  value: rights.rightsManifest == null
                      ? '待回填'
                      : 'v${rights.rightsManifest!.manifestVersion}',
                ),
                _DetailRow(
                  label: '元数据一致性',
                  value: rights.publicMetadata.consistency,
                ),
                _DetailRow(
                  label: '法律结论',
                  value: rights.trainingPermission.legalConclusion ? '是' : '否',
                ),
                if (rights.warnings.isNotEmpty)
                  _DetailRow(label: '提示', value: rights.warnings.join(' / ')),
                _DetailRow(label: '边界', value: result.message),
                const _DetailRow(
                  label: '嵌入导出',
                  value: publicRightsEmbeddedImageExportUnavailableMessage,
                ),
              ],
            ),
            const SizedBox(height: 8),
            Align(
              alignment: Alignment.centerLeft,
              child: Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  OutlinedButton.icon(
                    onPressed: _exporting ? null : _shareMetadataJson,
                    icon: const Icon(Icons.ios_share_outlined),
                    label: Text(
                      _exporting ? '导出中' : publicRightsMetadataJsonExportLabel,
                    ),
                  ),
                  if (widget.canExportEmbeddedImage)
                    OutlinedButton.icon(
                      onPressed: _exportingEmbeddedImage
                          ? null
                          : _selectAndShareEmbeddedImage,
                      icon: const Icon(Icons.add_photo_alternate_outlined),
                      label: Text(
                        _exportingEmbeddedImage
                            ? '导出中'
                            : publicRightsEmbeddedImageExportLabel,
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

String _safeFilePart(String value) {
  final safe = value.trim().replaceAll(RegExp(r'[^a-zA-Z0-9._-]+'), '_');
  return safe.isEmpty ? 'unknown' : safe;
}

class _DetailRow {
  const _DetailRow({required this.label, required this.value});

  final String label;
  final String? value;

  String get displayValue {
    final trimmed = value?.trim();
    return trimmed == null || trimmed.isEmpty ? '未记录' : trimmed;
  }
}

String _displayValue(String? value) {
  final trimmed = value?.trim();
  return trimmed == null || trimmed.isEmpty ? '未记录' : trimmed;
}

IconData _kindIcon(WatermarkAssetKind kind) {
  return switch (kind) {
    WatermarkAssetKind.image => Icons.image_outlined,
    WatermarkAssetKind.audio => Icons.graphic_eq_outlined,
    WatermarkAssetKind.video => Icons.video_file_outlined,
  };
}

List<VaultRecord> _filterRecords({
  required List<VaultRecord> records,
  required String query,
  required WatermarkAssetKind? kindFilter,
  required VaultRecordSource? sourceFilter,
  required SyncStatus? syncStatusFilter,
}) {
  final normalizedQuery = query.trim().toLowerCase();
  return records
      .where((record) {
        if (kindFilter != null && record.kind != kindFilter) {
          return false;
        }
        if (sourceFilter != null && record.source != sourceFilter) {
          return false;
        }
        if (syncStatusFilter != null && record.syncStatus != syncStatusFilter) {
          return false;
        }
        if (normalizedQuery.isEmpty) {
          return true;
        }
        return _recordSearchText(record).contains(normalizedQuery);
      })
      .toList(growable: false);
}

String _recordSearchText(VaultRecord record) {
  return [
    record.id,
    record.title,
    record.watermarkUid,
    record.creatorDisplayName,
    record.trustedTimeStatus,
    record.trustedTimeSource,
    record.thirdPartyVerificationStatus,
    record.thirdPartyVerificationProvider,
    record.thirdPartyVerificationPath,
    record.revision.toString(),
    record.sha256,
    record.parentWatermarkUid,
    record.rewriteReason,
    record.extractedTimestamp?.toString(),
    record.extractedDeviceIdHex,
    record.extractedFileHashHex,
    writeVerificationStatusLabel(record.writeVerificationStatus),
    record.writeVerificationMessage,
    _watermarkIssueModeLabel(record.watermarkIdIssueMode),
    _registryStatusLabel(record.watermarkIdRegistryStatus),
    record.watermarkIdRegistryReceipt,
    _payloadAuthStatusLabel(record.payloadAuthStatus),
    record.videoNotaryId,
    record.videoFingerprintRoot,
    record.videoBundleSha256,
    record.videoFrameSamplePolicy,
    vaultRecordSourceLabel(record.source),
    syncStatusLabel(record.syncStatus),
    _kindLabel(record.kind),
  ].whereType<String>().join('\n').toLowerCase();
}

String writeVerificationStatusLabel(WriteVerificationStatus? status) {
  return switch (status) {
    WriteVerificationStatus.verified => '完成后验证已通过',
    WriteVerificationStatus.failed => '完成后验证未通过',
    null => '未记录',
  };
}

String _watermarkIssueModeLabel(String value) {
  return switch (value) {
    'server_reserved' => '后端预签发',
    'server_confirmed' => '后端已确认',
    'server_reissued' => '后端重新签发',
    _ => '本地离线生成',
  };
}

String _registryStatusLabel(String value) {
  return switch (value) {
    'reserved' => '已预留，等待写入确认',
    'server_confirmed' => '后端已确认',
    'offline_confirmed' => '离线编号已补登记',
    'conflict' => '编号冲突',
    'reissue_required' => '需要重新签发',
    'pending_registry_reconcile' => '待登记仲裁',
    _ => '等待联网登记',
  };
}

String _payloadAuthStatusLabel(String value) {
  return switch (value) {
    'verified' => '已验证',
    'failed' => '验证失败',
    'pending_repair' => '待修复',
    _ => '未验证',
  };
}

bool _needsRegistryAction(VaultRecord record) {
  return record.watermarkIdRegistryStatus == 'pending_registry_reconcile' ||
      record.watermarkIdRegistryStatus == 'conflict' ||
      record.watermarkIdRegistryStatus == 'reissue_required';
}

class _RegistryArbitrationCard extends StatefulWidget {
  const _RegistryArbitrationCard({
    required this.record,
    required this.appState,
  });

  final VaultRecord record;
  final MobileAppState appState;

  @override
  State<_RegistryArbitrationCard> createState() =>
      _RegistryArbitrationCardState();
}

class _RegistryArbitrationCardState extends State<_RegistryArbitrationCard> {
  bool _isRepairing = false;

  Future<void> _requestReissue() async {
    setState(() => _isRepairing = true);
    try {
      final message = await widget.appState.requestWatermarkReissueForRecord(
        widget.record,
      );
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(message)));
    } catch (error) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(error.toString())));
    } finally {
      if (mounted) {
        setState(() => _isRepairing = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.policy_outlined, color: HsColors.warning),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  '登记仲裁',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          const Text(
            '该记录的版权编号需要后端仲裁或重新签发。移动端会先创建重签任务并保存状态；完成保护副本 payload 修复时，需要重新选择原作品或保护副本。',
            style: TextStyle(color: HsColors.textMuted, height: 1.45),
          ),
          const SizedBox(height: 12),
          SizedBox(
            width: double.infinity,
            child: FilledButton.icon(
              onPressed: _isRepairing ? null : _requestReissue,
              icon: const Icon(Icons.autorenew_outlined),
              label: Text(_isRepairing ? '申请中' : '申请重新签发'),
            ),
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

String _shorten(String value) {
  return value.length > 16 ? '${value.substring(0, 16)}...' : value;
}

String _formatDateTime(DateTime value) {
  final local = value.toLocal();
  String twoDigits(int input) => input.toString().padLeft(2, '0');
  return '${local.year}-${twoDigits(local.month)}-${twoDigits(local.day)} '
      '${twoDigits(local.hour)}:${twoDigits(local.minute)}:${twoDigits(local.second)}';
}

String _formatUnixSeconds(int value) {
  return _formatDateTime(
    DateTime.fromMillisecondsSinceEpoch(value * 1000, isUtc: true),
  );
}

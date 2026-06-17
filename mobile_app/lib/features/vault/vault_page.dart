import 'package:flutter/material.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import '../../shared/widgets/tool_cards.dart';

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

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '版权库',
      subtitle: '查看作品记录、写入次数和同步状态',
      children: [
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
          ...filteredRecords.map((record) => _VaultRecordCard(record: record)),
      ],
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
          Text('还没有版权记录'),
          SizedBox(height: 8),
          Text(
            '完成图片或 WAV 写入后，记录会自动进入这里。取证命中也会保存为本机证据线索。',
            style: TextStyle(color: Colors.white70),
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
              labelText: '搜索版权记录',
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
                label: 'WAV',
                icon: Icons.graphic_eq_outlined,
                selected: kindFilter == WatermarkAssetKind.audio,
                onSelected: () => onKindFilterChanged(WatermarkAssetKind.audio),
              ),
              _FilterChipItem(
                label: '写入',
                icon: Icons.edit_note_outlined,
                selected: sourceFilter == VaultRecordSource.write,
                onSelected: () =>
                    onSourceFilterChanged(VaultRecordSource.write),
              ),
              _FilterChipItem(
                label: '取证',
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
                  style: const TextStyle(color: Colors.white70),
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
          const Text('没有匹配的版权记录'),
          const SizedBox(height: 8),
          const Text(
            '换一个标题、版权编号或作品指纹，或者重置筛选条件。',
            style: TextStyle(color: Colors.white70),
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
        Text(label, style: const TextStyle(color: Colors.white70)),
      ],
    );
  }
}

class _VaultRecordCard extends StatelessWidget {
  const _VaultRecordCard({required this.record});

  final VaultRecord record;

  @override
  Widget build(BuildContext context) {
    final sha = record.sha256;
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
          onTap: () => _showVaultRecordDetails(context, record),
          leading: Icon(_kindIcon(record.kind), color: HsColors.accent),
          title: Text(
            record.title,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          subtitle: Text(
            [
              '${vaultRecordSourceLabel(record.source)} · ${_kindLabel(record.kind)} · 第 ${record.revision} 次写入',
              '版权编号: ${record.watermarkUid}',
              if (record.parentWatermarkUid != null) '包含上一版本记录',
              if (record.rewriteReason != null) '原因: ${record.rewriteReason}',
              if (sha != null) '作品指纹: ${_shorten(sha)}',
            ].join('\n'),
          ),
          trailing: HsStatusChip(label: syncStatusLabel(record.syncStatus)),
        ),
      ),
    );
  }
}

void _showVaultRecordDetails(BuildContext context, VaultRecord record) {
  showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    backgroundColor: HsColors.appBar,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(top: Radius.circular(24)),
    ),
    builder: (context) => _VaultRecordDetailsSheet(record: record),
  );
}

class _VaultRecordDetailsSheet extends StatelessWidget {
  const _VaultRecordDetailsSheet({required this.record});

  final VaultRecord record;

  @override
  Widget build(BuildContext context) {
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
                color: Colors.white24,
                borderRadius: BorderRadius.circular(999),
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
                      style: const TextStyle(color: Colors.white70),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 20),
          _DetailGroup(
            title: '水印信息',
            rows: [
              _DetailRow(label: '版权编号', value: record.watermarkUid),
              _DetailRow(label: '写入次数', value: '第 ${record.revision} 次'),
              _DetailRow(label: '上一版本', value: record.parentWatermarkUid),
              _DetailRow(label: '重写原因', value: record.rewriteReason),
            ],
          ),
          const SizedBox(height: 12),
          _DetailGroup(
            title: '文件指纹',
            rows: [
              _DetailRow(label: '作品指纹', value: record.sha256),
              _DetailRow(label: '命中片段', value: record.extractedFileHashHex),
            ],
          ),
          const SizedBox(height: 12),
          _DetailGroup(
            title: '取证信息',
            rows: [
              _DetailRow(
                label: '写入时间',
                value: record.extractedTimestamp?.toString(),
              ),
              _DetailRow(label: '来源设备', value: record.extractedDeviceIdHex),
            ],
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
        ],
      ),
    );
  }
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
        border: Border.all(color: Colors.white10),
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
              style: const TextStyle(color: Colors.white70),
            ),
          ),
          Expanded(
            child: SelectableText(
              row.displayValue,
              style: const TextStyle(color: Colors.white),
            ),
          ),
        ],
      ),
    );
  }
}

class _DetailRow {
  const _DetailRow({required this.label, required this.value});

  final String label;
  final String? value;

  String get displayValue {
    final trimmed = value?.trim();
    return trimmed == null || trimmed.isEmpty ? '无' : trimmed;
  }
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
    record.revision.toString(),
    record.sha256,
    record.parentWatermarkUid,
    record.rewriteReason,
    record.extractedTimestamp?.toString(),
    record.extractedDeviceIdHex,
    record.extractedFileHashHex,
    vaultRecordSourceLabel(record.source),
    syncStatusLabel(record.syncStatus),
    _kindLabel(record.kind),
  ].whereType<String>().join('\n').toLowerCase();
}

String _kindLabel(WatermarkAssetKind kind) {
  return switch (kind) {
    WatermarkAssetKind.image => '图片',
    WatermarkAssetKind.audio => 'WAV',
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

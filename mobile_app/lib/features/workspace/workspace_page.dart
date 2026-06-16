import 'package:flutter/material.dart';

import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';
import '../../shared/widgets/action_card.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import 'audio_embed_page.dart';
import 'image_embed_page.dart';

class WorkspacePage extends StatelessWidget {
  const WorkspacePage({
    super.key,
    required this.bridge,
    required this.appState,
  });

  final WatermarkBridge bridge;
  final MobileAppState appState;

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '工作台',
      subtitle: '图片和 WAV 音频的本地确权入口',
      bridge: bridge,
      children: [
        ActionCard(
          title: '图片嵌入',
          icon: Icons.image_outlined,
          description: '导入图片，生成带水印副本并写入版权库。',
          onTap: () => Navigator.of(context).push(
            MaterialPageRoute<void>(
              builder: (_) =>
                  ImageEmbedPage(bridge: bridge, appState: appState),
            ),
          ),
        ),
        ActionCard(
          title: '音频嵌入',
          icon: Icons.graphic_eq_outlined,
          description: '导入 WAV 音频，完成本地盲水印写入。',
          onTap: () => Navigator.of(context).push(
            MaterialPageRoute<void>(
              builder: (_) =>
                  AudioEmbedPage(bridge: bridge, appState: appState),
            ),
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

class _RecentTaskCard extends StatelessWidget {
  const _RecentTaskCard({required this.records});

  final List<VaultRecord> records;

  @override
  Widget build(BuildContext context) {
    if (records.isEmpty) {
      return const ActionCard(
        title: '最近任务',
        icon: Icons.history_outlined,
        description: '完成写入或取证后，这里会显示最近结果和重写链路。',
      );
    }

    final latest = records.first;
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
                const Icon(Icons.history_outlined, color: Color(0xFF59D2C2)),
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
              'UID: ${latest.watermarkUid}',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(color: Colors.white70),
            ),
          ],
        ),
      ),
    );
  }
}

String _kindLabel(WatermarkAssetKind kind) {
  return switch (kind) {
    WatermarkAssetKind.image => '图片',
    WatermarkAssetKind.audio => 'WAV',
    WatermarkAssetKind.video => '视频',
  };
}

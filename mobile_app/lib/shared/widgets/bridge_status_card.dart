import 'package:flutter/material.dart';

import '../../bridge/watermark_bridge.dart';
import '../../bridge/watermark_models.dart';

class BridgeStatusCard extends StatefulWidget {
  const BridgeStatusCard({super.key, required this.bridge});

  final WatermarkBridge bridge;

  @override
  State<BridgeStatusCard> createState() => _BridgeStatusCardState();
}

class _BridgeStatusCardState extends State<BridgeStatusCard> {
  late final Future<BridgeStatus> _statusFuture = widget.bridge.status();

  @override
  Widget build(BuildContext context) {
    return Card(
      elevation: 0,
      color: const Color(0xFF162028),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: FutureBuilder<BridgeStatus>(
          future: _statusFuture,
          builder: (context, snapshot) {
            final status = snapshot.data;
            final capabilities = status?.capabilities;
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  status?.label ?? '本地优先 · 未配对桌面',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                Text(
                  status?.detail ?? '先完成图片和音频的本地确权，再接入桌面同步。',
                  style: const TextStyle(color: Colors.white70),
                ),
                if (capabilities != null) ...[
                  const SizedBox(height: 12),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      ...capabilities.supportedKinds.map(
                        (kind) => _CapabilityChip(label: _kindLabel(kind)),
                      ),
                      _CapabilityChip(
                        label: capabilities.supportsDesktopSync
                            ? '支持桌面同步'
                            : '暂不支持桌面同步',
                      ),
                      _CapabilityChip(
                        label: capabilities.supportsLocalVideo
                            ? '本地视频已开放'
                            : '本地视频由桌面端处理',
                      ),
                    ],
                  ),
                ],
              ],
            );
          },
        ),
      ),
    );
  }
}

class _CapabilityChip extends StatelessWidget {
  const _CapabilityChip({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return Chip(
      label: Text(label),
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
      padding: EdgeInsets.zero,
      backgroundColor: const Color(0xFF1A2730),
      side: BorderSide.none,
      labelStyle: Theme.of(context).textTheme.labelMedium,
    );
  }
}

String _kindLabel(WatermarkAssetKind kind) {
  switch (kind) {
    case WatermarkAssetKind.image:
      return '图片';
    case WatermarkAssetKind.audio:
      return '音频';
    case WatermarkAssetKind.video:
      return '视频';
  }
}

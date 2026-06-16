import 'package:flutter/material.dart';

import '../../bridge/watermark_bridge.dart';
import '../../shared/widgets/action_card.dart';
import '../../shared/widgets/feature_page_scaffold.dart';

class VaultPage extends StatelessWidget {
  const VaultPage({super.key, required this.bridge});

  final WatermarkBridge bridge;

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '版权库',
      subtitle: '时间线、详情和派生链',
      bridge: bridge,
      children: const [
        ActionCard(
          title: '时间线',
          icon: Icons.timeline_outlined,
          description: '浏览记录，按时间查看每次写入。',
        ),
        ActionCard(
          title: '链路详情',
          icon: Icons.device_hub_outlined,
          description: '查看 parent UID、revision 和 rewrite_reason。',
        ),
      ],
    );
  }
}

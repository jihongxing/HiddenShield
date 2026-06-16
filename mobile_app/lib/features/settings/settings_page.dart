import 'package:flutter/material.dart';

import '../../bridge/watermark_bridge.dart';
import '../../shared/widgets/action_card.dart';
import '../../shared/widgets/feature_page_scaffold.dart';

class SettingsPage extends StatelessWidget {
  const SettingsPage({super.key, required this.bridge});

  final WatermarkBridge bridge;

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '设置',
      subtitle: '身份、同步、隐私与帮助',
      bridge: bridge,
      children: const [
        ActionCard(
          title: '创作者身份',
          icon: Icons.badge_outlined,
          description: '后续接入创作者身份包和桌面配对。',
        ),
        ActionCard(
          title: '同步与备份',
          icon: Icons.sync_outlined,
          description: '后续接入桌面端同步状态和冲突处理。',
        ),
        ActionCard(
          title: '隐私与权限',
          icon: Icons.lock_outline,
          description: '管理相册、文件、相机和通知权限。',
        ),
      ],
    );
  }
}

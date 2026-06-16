import 'package:flutter/material.dart';

import '../../bridge/watermark_bridge.dart';
import '../../shared/widgets/action_card.dart';
import '../../shared/widgets/feature_page_scaffold.dart';

class VerifyPage extends StatelessWidget {
  const VerifyPage({super.key, required this.bridge});

  final WatermarkBridge bridge;

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '取证',
      subtitle: '检测疑似侵权图片或音频，展示命中和链路',
      bridge: bridge,
      children: const [
        ActionCard(
          title: '文件提取',
          icon: Icons.document_scanner_outlined,
          description: '选择文件后自动提取水印并匹配版权库。',
        ),
        ActionCard(
          title: '结果摘要',
          icon: Icons.fact_check_outlined,
          description: '展示 UID、revision、父级 UID 和重写原因。',
        ),
      ],
    );
  }
}

import 'package:flutter/material.dart';

import '../../bridge/watermark_bridge.dart';
import '../../shared/widgets/action_card.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import 'audio_embed_page.dart';
import 'image_embed_page.dart';

class WorkspacePage extends StatelessWidget {
  const WorkspacePage({super.key, required this.bridge});

  final WatermarkBridge bridge;

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
              builder: (_) => ImageEmbedPage(bridge: bridge),
            ),
          ),
        ),
        ActionCard(
          title: '音频嵌入',
          icon: Icons.graphic_eq_outlined,
          description: '导入 WAV 音频，完成本地盲水印写入。',
          onTap: () => Navigator.of(context).push(
            MaterialPageRoute<void>(
              builder: (_) => AudioEmbedPage(bridge: bridge),
            ),
          ),
        ),
        ActionCard(
          title: '最近任务',
          icon: Icons.history_outlined,
          description: '这里会显示最近的处理结果和重写链路。',
        ),
      ],
    );
  }
}

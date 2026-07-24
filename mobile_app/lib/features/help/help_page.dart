import 'package:flutter/material.dart';

import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/feature_page_scaffold.dart';
import '../../shared/widgets/tool_cards.dart';

class HelpPage extends StatelessWidget {
  const HelpPage({super.key});

  @override
  Widget build(BuildContext context) {
    return FeaturePageScaffold(
      title: '帮助',
      subtitle: '了解写入流程、同步边界和正式报告用途。',
      icon: Icons.help_outline,
      children: const [
        HsPanel(
          title: '快速上手',
          icon: Icons.route_outlined,
          child: Column(
            children: [
              _HelpStep(
                number: '1',
                title: '登录账户',
                detail: '首次使用先登录账户。新账户会自动创建，已有账户会直接进入。',
              ),
              _HelpStep(
                number: '2',
                title: '完成基础设置',
                detail: '设置创作者身份后再开始处理作品。创作者身份会写入版权记录。',
              ),
              _HelpStep(
                number: '3',
                title: '生成保护副本',
                detail: '在工作台选择作品，系统会自动识别图片或 30 秒以上音频，完成前会自动验证版权编号。',
              ),
              _HelpStep(
                number: '4',
                title: '验证与留档',
                detail: '导入疑似样本，匹配版权库并保存验证记录。',
              ),
            ],
          ),
        ),
        SizedBox(height: 12),
        HsPanel(
          title: '能力边界',
          icon: Icons.verified_outlined,
          child: Column(
            children: [
              _HelpItem(
                title: '作品写入',
                detail: '支持常见图片和完整音频本地生成保护副本，并自动进行完成后验证。',
              ),
              _HelpItem(
                title: '音频边界',
                detail: '只保护 30 秒以上且能确认时长的完整音频；短片段不会生成保护副本。',
              ),
              _HelpItem(
                title: '视频',
                detail:
                    'L1 是视频音轨水印，桌面端可生成本地视频保护副本，移动端可验证视频音轨。L2 是视频指纹存证，需要 Creator 云同步权益。L3 视频画面盲水印按 Studio / Enterprise release gate 进入受控创建与领取。',
              ),
              _HelpItem(
                title: '云同步',
                detail: '默认不同步原始媒体、保护副本和本地路径，只同步账户、版权记录和验证摘要。',
              ),
            ],
          ),
        ),
        SizedBox(height: 12),
        HsPanel(
          title: '常见问题',
          icon: Icons.help_outline,
          child: Column(
            children: [
              _HelpItem(
                title: '为什么要填写创作者身份？',
                detail: '创作者身份会显示在版权记录中，并在登录同一账户后保持双端一致。',
              ),
              _HelpItem(
                title: '验证失败代表什么？',
                detail: '可能是样本未经过保护、被严重裁剪压缩，或当前版权库没有对应记录。先更新云端记录再验证。',
              ),
              _HelpItem(
                title: '正式报告能作为法律意见吗？',
                detail: '不能。正式报告、时间回执和指纹存证是技术辅助材料，不构成法律意见或司法鉴定。',
              ),
              _HelpItem(
                title: '遇到问题怎么反馈？',
                detail: '在设置中开启匿名反馈，或联系作者。反馈不包含原始媒体、保护副本、文件名、本地路径或完整作品指纹。',
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _HelpStep extends StatelessWidget {
  const _HelpStep({
    required this.number,
    required this.title,
    required this.detail,
  });

  final String number;
  final String title;
  final String detail;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 28,
            height: 28,
            alignment: Alignment.center,
            decoration: BoxDecoration(
              color: HsColors.accent,
              borderRadius: BorderRadius.circular(HsRadii.card),
            ),
            child: Text(
              number,
              style: const TextStyle(
                color: HsColors.background,
                fontWeight: FontWeight.w700,
              ),
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: _HelpText(title: title, detail: detail),
          ),
        ],
      ),
    );
  }
}

class _HelpItem extends StatelessWidget {
  const _HelpItem({required this.title, required this.detail});

  final String title;
  final String detail;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: _HelpText(title: title, detail: detail),
    );
  }
}

class _HelpText extends StatelessWidget {
  const _HelpText({required this.title, required this.detail});

  final String title;
  final String detail;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(title, style: Theme.of(context).textTheme.titleSmall),
        const SizedBox(height: 4),
        Text(
          detail,
          style: const TextStyle(color: HsColors.textMuted, height: 1.45),
        ),
      ],
    );
  }
}

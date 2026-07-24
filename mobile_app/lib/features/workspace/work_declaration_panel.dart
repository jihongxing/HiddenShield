import 'package:flutter/material.dart';

import '../../app/mobile_app_state.dart';
import '../../shared/theme/design_tokens.dart';
import '../../shared/widgets/tool_cards.dart';

class WorkDeclarationPanel extends StatelessWidget {
  const WorkDeclarationPanel({
    super.key,
    required this.value,
    required this.onChanged,
  });

  final WorkDeclaration value;
  final ValueChanged<WorkDeclaration> onChanged;

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      title: '作品声明与授权策略',
      icon: Icons.assignment_outlined,
      child: ExpansionTile(
        tilePadding: EdgeInsets.zero,
        childrenPadding: EdgeInsets.zero,
        title: const Text('记录创作者声明'),
        subtitle: const Text('HiddenShield 只记录声明，不检测 AI，也不自动判断训练许可。'),
        children: [
          const SizedBox(height: HsSpacing.sm),
          _ChoiceField(
            label: '作品来源声明',
            value: value.workSourceDeclaration,
            options: const [
              _DeclarationOption('unspecified', '未声明'),
              _DeclarationOption('human_created', '人工创作'),
              _DeclarationOption('ai_assisted', 'AI 辅助'),
              _DeclarationOption('ai_generated', 'AI 生成'),
            ],
            onChanged: (next) =>
                onChanged(value.copyWith(workSourceDeclaration: next)),
          ),
          _ChoiceField(
            label: '训练许可声明',
            value: value.trainingPermissionDeclaration,
            options: const [
              _DeclarationOption('prohibited', '禁止模型训练'),
              _DeclarationOption('separate_authorization_required', '需单独授权'),
              _DeclarationOption('non_commercial_allowed', '允许非商业训练'),
              _DeclarationOption('commercial_allowed', '允许商业训练'),
              _DeclarationOption('unspecified', '未声明'),
            ],
            onChanged: (next) =>
                onChanged(value.copyWith(trainingPermissionDeclaration: next)),
          ),
          _ChoiceField(
            label: '真实性声明',
            value: value.authenticityClaimDeclaration,
            options: const [
              _DeclarationOption('unspecified', '未声明'),
              _DeclarationOption('synthetic', '虚构或合成'),
              _DeclarationOption('based_on_reality', '基于真实'),
              _DeclarationOption('creator_claimed_authentic', '创作者声明真实'),
            ],
            onChanged: (next) =>
                onChanged(value.copyWith(authenticityClaimDeclaration: next)),
          ),
          const SizedBox(height: HsSpacing.sm),
          TextFormField(
            initialValue: value.customRightsStatement ?? '',
            maxLines: 3,
            decoration: const InputDecoration(
              labelText: '自定义版权声明',
              hintText: '可选填写授权边界、禁止用途或合同说明',
            ),
            onChanged: (next) => onChanged(
              value.copyWith(
                customRightsStatement: next.trim().isEmpty ? null : next,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ChoiceField extends StatelessWidget {
  const _ChoiceField({
    required this.label,
    required this.value,
    required this.options,
    required this.onChanged,
  });

  final String label;
  final String value;
  final List<_DeclarationOption> options;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: HsSpacing.md),
      child: DropdownButtonFormField<String>(
        initialValue: value,
        decoration: InputDecoration(labelText: label),
        dropdownColor: HsColors.surfaceRaised,
        items: options
            .map(
              (option) => DropdownMenuItem<String>(
                value: option.value,
                child: Text(option.label),
              ),
            )
            .toList(),
        onChanged: (next) {
          if (next != null) {
            onChanged(next);
          }
        },
      ),
    );
  }
}

class _DeclarationOption {
  const _DeclarationOption(this.value, this.label);

  final String value;
  final String label;
}

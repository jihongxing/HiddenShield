import 'package:flutter/material.dart';

import '../models/workspace_context.dart';
import '../theme/design_tokens.dart';

Future<void> showHsContextSheet(
  BuildContext context, {
  required HsWorkspaceContext workspaceContext,
}) {
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    backgroundColor: HsColors.surface,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(top: Radius.circular(HsRadii.sheet)),
    ),
    builder: (context) => HsContextSheet(contextData: workspaceContext),
  );
}

class HsContextSheet extends StatelessWidget {
  const HsContextSheet({super.key, required this.contextData});

  final HsWorkspaceContext contextData;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: DraggableScrollableSheet(
        expand: false,
        initialChildSize: 0.58,
        minChildSize: 0.32,
        maxChildSize: 0.92,
        builder: (context, controller) {
          return ListView(
            controller: controller,
            padding: const EdgeInsets.fromLTRB(20, 12, 20, 24),
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
              const SizedBox(height: HsSpacing.lg),
              Text(
                contextData.eyebrow,
                style: Theme.of(context).textTheme.labelMedium?.copyWith(
                  color: HsColors.accent,
                  fontWeight: FontWeight.w800,
                ),
              ),
              const SizedBox(height: HsSpacing.xs),
              Text(
                contextData.title,
                style: Theme.of(context).textTheme.titleLarge?.copyWith(
                  color: HsColors.text,
                  fontWeight: FontWeight.w800,
                ),
              ),
              const SizedBox(height: HsSpacing.sm),
              Text(
                contextData.summary,
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: HsColors.textMuted,
                  height: 1.45,
                ),
              ),
              const SizedBox(height: HsSpacing.lg),
              Wrap(
                spacing: HsSpacing.sm,
                runSpacing: HsSpacing.sm,
                children: contextData.metrics
                    .map((metric) => _MetricPill(metric: metric))
                    .toList(),
              ),
              if (contextData.actions.isNotEmpty) ...[
                const SizedBox(height: HsSpacing.xl),
                ...contextData.actions.map(
                  (action) => Padding(
                    padding: const EdgeInsets.only(bottom: HsSpacing.sm),
                    child: FilledButton.icon(
                      onPressed: () {
                        Navigator.of(context).pop();
                        action.onPressed?.call();
                      },
                      icon: Icon(action.icon),
                      label: Text(action.label),
                      style: FilledButton.styleFrom(
                        backgroundColor: action.primary
                            ? HsColors.accent
                            : HsColors.surfaceMuted,
                        foregroundColor: action.primary
                            ? HsColors.background
                            : HsColors.text,
                        minimumSize: const Size.fromHeight(44),
                      ),
                    ),
                  ),
                ),
              ],
            ],
          );
        },
      ),
    );
  }
}

class _MetricPill extends StatelessWidget {
  const _MetricPill({required this.metric});

  final HsContextMetric metric;

  @override
  Widget build(BuildContext context) {
    final color = switch (metric.tone) {
      HsContextTone.ok => HsColors.accent,
      HsContextTone.warning => HsColors.warning,
      HsContextTone.danger => HsColors.danger,
      HsContextTone.muted => HsColors.textMuted,
    };
    return Container(
      constraints: const BoxConstraints(minWidth: 126),
      padding: const EdgeInsets.all(HsSpacing.md),
      decoration: BoxDecoration(
        color: HsColors.surfaceRaised,
        borderRadius: BorderRadius.circular(HsRadii.card),
        border: Border.all(color: HsColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            metric.label,
            style: Theme.of(
              context,
            ).textTheme.labelSmall?.copyWith(color: HsColors.textMuted),
          ),
          const SizedBox(height: HsSpacing.xs),
          Text(
            metric.value,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: color,
              fontWeight: FontWeight.w800,
            ),
          ),
        ],
      ),
    );
  }
}

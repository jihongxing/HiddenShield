import 'package:flutter/material.dart';

import '../theme/design_tokens.dart';

class HsPanel extends StatelessWidget {
  const HsPanel({
    super.key,
    this.title,
    this.icon,
    required this.child,
    this.padding = const EdgeInsets.all(HsSpacing.lg),
    this.color = HsColors.surface,
    this.radius = HsRadii.card,
  });

  final String? title;
  final IconData? icon;
  final Widget child;
  final EdgeInsetsGeometry padding;
  final Color color;
  final double radius;

  @override
  Widget build(BuildContext context) {
    final title = this.title;
    return Card(
      color: color,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(radius),
        side: const BorderSide(color: HsColors.border),
      ),
      child: Padding(
        padding: padding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (title != null) ...[
              Row(
                children: [
                  if (icon != null) ...[
                    Container(
                      width: 34,
                      height: 34,
                      decoration: BoxDecoration(
                        color: HsColors.surfaceMuted,
                        borderRadius: BorderRadius.circular(HsRadii.card),
                        border: Border.all(color: HsColors.border),
                      ),
                      child: Icon(icon, color: HsColors.accent, size: 20),
                    ),
                    const SizedBox(width: HsSpacing.md),
                  ],
                  Expanded(
                    child: Text(
                      title,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        color: HsColors.text,
                        fontWeight: FontWeight.w800,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: HsSpacing.md),
            ],
            child,
          ],
        ),
      ),
    );
  }
}

class HsMessageCard extends StatelessWidget {
  const HsMessageCard({
    super.key,
    required this.icon,
    required this.title,
    required this.detail,
    this.detailWidget,
    this.iconColor = HsColors.accent,
  });

  final IconData icon;
  final String title;
  final String detail;
  final Widget? detailWidget;
  final Color iconColor;

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      color: HsColors.surfaceRaised,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, color: iconColor),
          const SizedBox(width: HsSpacing.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: Theme.of(
                    context,
                  ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w700),
                ),
                const SizedBox(height: HsSpacing.xs),
                detailWidget ??
                    Text(
                      detail,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: HsColors.textMuted,
                        height: 1.35,
                      ),
                    ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class HsPreviewBox extends StatelessWidget {
  const HsPreviewBox({
    super.key,
    required this.child,
    this.height = 160,
    this.padding = const EdgeInsets.all(HsSpacing.lg),
  });

  final Widget child;
  final double height;
  final EdgeInsetsGeometry padding;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: height,
      padding: padding,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(HsRadii.preview),
        border: Border.all(color: HsColors.border),
        color: HsColors.surfaceMuted,
      ),
      child: child,
    );
  }
}

class HsStatusChip extends StatelessWidget {
  const HsStatusChip({
    super.key,
    required this.label,
    this.icon,
    this.foregroundColor = HsColors.textMuted,
    this.backgroundColor = HsColors.chip,
  });

  final String label;
  final IconData? icon;
  final Color foregroundColor;
  final Color backgroundColor;

  @override
  Widget build(BuildContext context) {
    return Chip(
      avatar: icon == null
          ? null
          : Icon(icon, size: 16, color: foregroundColor),
      label: Text(label),
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
      padding: const EdgeInsets.symmetric(horizontal: 2),
      backgroundColor: backgroundColor,
      side: const BorderSide(color: HsColors.border),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(HsRadii.pill),
      ),
      labelStyle: Theme.of(context).textTheme.labelMedium?.copyWith(
        color: foregroundColor,
        fontWeight: FontWeight.w700,
      ),
    );
  }
}

class HsInfoRow extends StatelessWidget {
  const HsInfoRow({super.key, required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: HsSpacing.xs),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 76,
            child: Text(
              label,
              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                color: HsColors.textSubtle,
                fontWeight: FontWeight.w700,
              ),
            ),
          ),
          Expanded(
            child: SelectableText(
              value,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: HsColors.text,
                height: 1.35,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class HsPrimaryResultCard extends StatelessWidget {
  const HsPrimaryResultCard({
    super.key,
    required this.icon,
    required this.title,
    required this.statusLabel,
    required this.children,
    this.statusColor = HsColors.accent,
    this.actions = const [],
  });

  final IconData icon;
  final String title;
  final String statusLabel;
  final Color statusColor;
  final List<Widget> children;
  final List<Widget> actions;

  @override
  Widget build(BuildContext context) {
    return HsPanel(
      color: HsColors.surfaceRaised,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Container(
                width: 38,
                height: 38,
                decoration: BoxDecoration(
                  color: statusColor.withValues(alpha: 0.14),
                  borderRadius: BorderRadius.circular(HsRadii.card),
                  border: Border.all(
                    color: statusColor.withValues(alpha: 0.24),
                  ),
                ),
                child: Icon(icon, color: statusColor, size: 22),
              ),
              const SizedBox(width: HsSpacing.md),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      title,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w800,
                      ),
                    ),
                    const SizedBox(height: HsSpacing.xs),
                    HsStatusChip(
                      label: statusLabel,
                      foregroundColor: statusColor,
                      backgroundColor: statusColor.withValues(alpha: 0.12),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: HsSpacing.md),
          ...children,
          if (actions.isNotEmpty) ...[
            const SizedBox(height: HsSpacing.md),
            Wrap(
              spacing: HsSpacing.sm,
              runSpacing: HsSpacing.sm,
              children: actions,
            ),
          ],
        ],
      ),
    );
  }
}

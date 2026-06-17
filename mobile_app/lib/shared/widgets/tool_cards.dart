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
      elevation: 0,
      color: color,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(radius),
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
                    Icon(icon, color: HsColors.accent),
                    const SizedBox(width: HsSpacing.md),
                  ],
                  Expanded(
                    child: Text(
                      title,
                      style: Theme.of(context).textTheme.titleMedium,
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
      child: ListTile(
        contentPadding: EdgeInsets.zero,
        leading: Icon(icon, color: iconColor),
        title: Text(title),
        subtitle: detailWidget ?? Text(detail),
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
  const HsStatusChip({super.key, required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return Chip(
      label: Text(label),
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
      padding: EdgeInsets.zero,
      backgroundColor: HsColors.chip,
      side: BorderSide.none,
      labelStyle: Theme.of(context).textTheme.labelMedium,
    );
  }
}

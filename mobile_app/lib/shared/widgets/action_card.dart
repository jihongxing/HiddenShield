import 'package:flutter/material.dart';

import '../theme/design_tokens.dart';

class ActionCard extends StatelessWidget {
  const ActionCard({
    super.key,
    required this.title,
    required this.icon,
    required this.description,
    this.onTap,
  });

  final String title;
  final IconData icon;
  final String description;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: HsSpacing.md),
      child: Card(
        color: HsColors.surfaceRaised,
        child: InkWell(
          borderRadius: BorderRadius.circular(HsRadii.panel),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.all(HsSpacing.lg),
            child: Row(
              children: [
                Container(
                  width: 42,
                  height: 42,
                  decoration: BoxDecoration(
                    color: HsColors.surfaceMuted,
                    borderRadius: BorderRadius.circular(HsRadii.card),
                    border: Border.all(color: HsColors.border),
                  ),
                  child: Icon(icon, color: HsColors.accent),
                ),
                const SizedBox(width: HsSpacing.lg),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        title,
                        style: Theme.of(context).textTheme.titleMedium
                            ?.copyWith(
                              color: HsColors.text,
                              fontWeight: FontWeight.w800,
                            ),
                      ),
                      const SizedBox(height: HsSpacing.xs),
                      Text(
                        description,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: HsColors.textMuted,
                          height: 1.35,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: HsSpacing.sm),
                Icon(
                  Icons.chevron_right,
                  color: onTap == null
                      ? HsColors.textSubtle
                      : HsColors.iconMuted,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

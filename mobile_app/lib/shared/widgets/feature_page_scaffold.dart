import 'package:flutter/material.dart';

import '../models/workspace_context.dart';
import '../theme/design_tokens.dart';
import 'context_sheet.dart';

class FeaturePageScaffold extends StatelessWidget {
  const FeaturePageScaffold({
    super.key,
    required this.title,
    required this.subtitle,
    required this.children,
    this.icon,
    this.trailing,
    this.showBackButton = false,
    this.contextData,
  });

  final String title;
  final String subtitle;
  final List<Widget> children;
  final IconData? icon;
  final Widget? trailing;
  final bool showBackButton;
  final HsWorkspaceContext? contextData;

  @override
  Widget build(BuildContext context) {
    final width = MediaQuery.sizeOf(context).width;
    final horizontalPadding = width < 390 ? 16.0 : 24.0;
    return ListView(
      padding: EdgeInsets.fromLTRB(
        horizontalPadding,
        18,
        horizontalPadding,
        28,
      ),
      children: [
        HsPageHeader(
          title: title,
          subtitle: subtitle,
          icon: icon,
          trailing: trailing,
          showBackButton: showBackButton,
          contextData: contextData,
        ),
        const SizedBox(height: HsSpacing.xxl),
        ...children,
      ],
    );
  }
}

class HsPageHeader extends StatelessWidget {
  const HsPageHeader({
    super.key,
    required this.title,
    required this.subtitle,
    this.icon,
    this.trailing,
    this.showBackButton = false,
    this.contextData,
  });

  final String title;
  final String subtitle;
  final IconData? icon;
  final Widget? trailing;
  final bool showBackButton;
  final HsWorkspaceContext? contextData;

  @override
  Widget build(BuildContext context) {
    final icon = this.icon;
    final titleBlock = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          title,
          style: Theme.of(context).textTheme.headlineSmall?.copyWith(
            fontWeight: FontWeight.w800,
            letterSpacing: 0,
            color: HsColors.text,
          ),
        ),
        const SizedBox(height: HsSpacing.xs),
        Text(
          subtitle,
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
            color: HsColors.textMuted,
            height: 1.35,
          ),
        ),
      ],
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < 360;
        final leading = icon == null
            ? null
            : Container(
                width: 44,
                height: 44,
                decoration: BoxDecoration(
                  color: HsColors.surfaceMuted,
                  borderRadius: BorderRadius.circular(HsRadii.card),
                  border: Border.all(color: HsColors.border),
                ),
                child: Icon(icon, color: HsColors.accent),
              );
        final backButton = showBackButton && Navigator.of(context).canPop()
            ? IconButton(
                tooltip: '返回',
                onPressed: () => Navigator.of(context).maybePop(),
                icon: const Icon(Icons.arrow_back_outlined),
              )
            : null;
        final trailingGroup = Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (backButton != null) backButton,
            if (trailing != null) ...[
              if (backButton != null) const SizedBox(width: HsSpacing.xs),
              trailing!,
            ],
          ],
        );
        final contextButton = contextData == null
            ? null
            : IconButton(
                tooltip: '上下文',
                onPressed: () =>
                    showHsContextSheet(context, workspaceContext: contextData!),
                icon: const Icon(Icons.info_outline),
              );
        final hasTrailing =
            backButton != null || trailing != null || contextButton != null;
        if (compact) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  if (leading != null) ...[
                    leading,
                    const SizedBox(width: HsSpacing.md),
                  ],
                  if (leading != null && hasTrailing) const Spacer(),
                  if (hasTrailing)
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        trailingGroup,
                        if (contextButton != null) contextButton,
                      ],
                    ),
                ],
              ),
              const SizedBox(height: HsSpacing.md),
              titleBlock,
            ],
          );
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (leading != null) ...[
              leading,
              const SizedBox(width: HsSpacing.md),
            ],
            Expanded(child: titleBlock),
            if (hasTrailing) ...[
              const SizedBox(width: HsSpacing.md),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  trailingGroup,
                  if (contextButton != null) contextButton,
                ],
              ),
            ],
          ],
        );
      },
    );
  }
}

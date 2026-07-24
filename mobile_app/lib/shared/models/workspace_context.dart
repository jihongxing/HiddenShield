import 'package:flutter/material.dart';

enum HsContextTone { ok, warning, danger, muted }

class HsContextMetric {
  const HsContextMetric({
    required this.label,
    required this.value,
    this.tone = HsContextTone.muted,
  });

  final String label;
  final String value;
  final HsContextTone tone;
}

class HsContextAction {
  const HsContextAction({
    required this.label,
    this.icon = Icons.arrow_forward_outlined,
    this.primary = false,
    this.onPressed,
  });

  final String label;
  final IconData icon;
  final bool primary;
  final VoidCallback? onPressed;
}

class HsWorkspaceContext {
  const HsWorkspaceContext({
    required this.eyebrow,
    required this.title,
    required this.summary,
    this.metrics = const [],
    this.actions = const [],
  });

  final String eyebrow;
  final String title;
  final String summary;
  final List<HsContextMetric> metrics;
  final List<HsContextAction> actions;
}

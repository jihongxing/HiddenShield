import 'package:flutter/material.dart';

import '../../bridge/watermark_bridge.dart';
import 'bridge_status_card.dart';

class FeaturePageScaffold extends StatelessWidget {
  const FeaturePageScaffold({
    super.key,
    required this.title,
    required this.subtitle,
    required this.bridge,
    required this.children,
  });

  final String title;
  final String subtitle;
  final WatermarkBridge bridge;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(
          title,
          style: Theme.of(
            context,
          ).textTheme.headlineMedium?.copyWith(fontWeight: FontWeight.w700),
        ),
        const SizedBox(height: 8),
        Text(
          subtitle,
          style: Theme.of(
            context,
          ).textTheme.bodyMedium?.copyWith(color: Colors.white70),
        ),
        const SizedBox(height: 16),
        BridgeStatusCard(bridge: bridge),
        const SizedBox(height: 16),
        ...children,
      ],
    );
  }
}

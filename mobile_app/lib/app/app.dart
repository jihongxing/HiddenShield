import 'package:flutter/material.dart';

import '../bridge/local_preview_watermark_bridge.dart';
import '../bridge/watermark_bridge.dart';
import 'mobile_shell.dart';
import 'theme.dart';

class HiddenShieldApp extends StatelessWidget {
  const HiddenShieldApp({
    super.key,
    this.bridge = const PreviewWatermarkBridge(),
  });

  final WatermarkBridge bridge;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'HiddenShield',
      theme: buildHiddenShieldTheme(),
      home: MobileShell(bridge: bridge),
    );
  }
}

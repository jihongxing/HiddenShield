import 'package:flutter/material.dart';

import '../bridge/local_preview_watermark_bridge.dart';
import '../bridge/watermark_bridge.dart';
import 'mobile_app_state.dart';
import 'mobile_shell.dart';
import 'theme.dart';

class HiddenShieldApp extends StatefulWidget {
  const HiddenShieldApp({
    super.key,
    this.bridge = const PreviewWatermarkBridge(),
    this.appState,
  });

  final WatermarkBridge bridge;
  final MobileAppState? appState;

  @override
  State<HiddenShieldApp> createState() => _HiddenShieldAppState();
}

class _HiddenShieldAppState extends State<HiddenShieldApp> {
  late final MobileAppState _appState = widget.appState ?? MobileAppState();

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'HiddenShield',
      theme: buildHiddenShieldTheme(),
      home: MobileShell(bridge: widget.bridge, appState: _appState),
    );
  }
}

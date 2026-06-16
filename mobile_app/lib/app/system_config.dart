import 'dart:convert';

import 'package:flutter/services.dart';

class HiddenShieldSystemConfig {
  const HiddenShieldSystemConfig({
    required this.cloudBaseUrl,
    required this.lanDebugPort,
  });

  final String cloudBaseUrl;
  final int lanDebugPort;

  static const fallback = HiddenShieldSystemConfig(
    cloudBaseUrl: 'http://127.0.0.1:43188',
    lanDebugPort: 47219,
  );

  static Future<HiddenShieldSystemConfig> load() async {
    try {
      final raw = await rootBundle.loadString('assets/hiddenshield.system.json');
      final json = jsonDecode(raw) as Map<String, Object?>;
      return HiddenShieldSystemConfig(
        cloudBaseUrl:
            json['cloudBaseUrl'] as String? ?? fallback.cloudBaseUrl,
        lanDebugPort:
            (json['lanDebugPort'] as num?)?.toInt() ?? fallback.lanDebugPort,
      );
    } catch (_) {
      return fallback;
    }
  }
}

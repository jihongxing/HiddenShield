import 'package:flutter/material.dart';

import '../shared/theme/design_tokens.dart';

ThemeData buildHiddenShieldTheme() {
  return ThemeData(
    useMaterial3: true,
    colorScheme: ColorScheme.fromSeed(
      seedColor: HsColors.accentSeed,
      brightness: Brightness.dark,
    ),
    scaffoldBackgroundColor: HsColors.background,
  );
}

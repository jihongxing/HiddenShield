import 'package:flutter/material.dart';

ThemeData buildHiddenShieldTheme() {
  return ThemeData(
    useMaterial3: true,
    colorScheme: ColorScheme.fromSeed(
      seedColor: const Color(0xFF1E6F66),
      brightness: Brightness.dark,
    ),
    scaffoldBackgroundColor: const Color(0xFF0C1116),
  );
}

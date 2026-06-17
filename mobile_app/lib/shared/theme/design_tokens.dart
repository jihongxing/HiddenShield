import 'package:flutter/material.dart';

abstract final class HsColors {
  static const background = Color(0xFF0C1116);
  static const appBar = Color(0xFF0F151B);
  static const navigation = Color(0xFF121920);
  static const surface = Color(0xFF141B22);
  static const surfaceRaised = Color(0xFF162028);
  static const surfaceMuted = Color(0xFF0F151B);
  static const chip = Color(0xFF1A2730);
  static const accent = Color(0xFF59D2C2);
  static const accentSeed = Color(0xFF1E6F66);
  static const textMuted = Colors.white70;
  static const iconMuted = Colors.white54;
  static const border = Colors.white12;
  static const warning = Color(0xFFFFC857);
  static const warningSurface = Color(0xFF2C2212);
}

abstract final class HsRadii {
  static const preview = 8.0;
  static const card = 12.0;
  static const panel = 16.0;
  static const sheet = 24.0;
  static const pill = 999.0;
}

abstract final class HsSpacing {
  static const xs = 4.0;
  static const sm = 8.0;
  static const md = 12.0;
  static const lg = 16.0;
  static const xl = 20.0;
  static const xxl = 24.0;
}

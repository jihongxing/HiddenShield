import 'package:flutter/material.dart';

import '../shared/theme/design_tokens.dart';

const _hiddenShieldFontFallback = [
  'HiddenShieldCjk',
  'Microsoft YaHei',
  'PingFang SC',
  'Noto Sans CJK SC',
  'Noto Sans SC',
  'Arial Unicode MS',
  'sans-serif',
];

ThemeData buildHiddenShieldTheme() {
  final base = ThemeData(
    useMaterial3: true,
    colorScheme: ColorScheme.fromSeed(
      seedColor: HsColors.accentSeed,
      brightness: Brightness.dark,
    ),
  );
  return ThemeData(
    useMaterial3: true,
    colorScheme: base.colorScheme.copyWith(
      primary: HsColors.accent,
      secondary: HsColors.copper,
      surface: HsColors.surface,
      surfaceContainerHighest: HsColors.surfaceRaised,
    ),
    scaffoldBackgroundColor: HsColors.background,
    fontFamily: 'Roboto',
    fontFamilyFallback: _hiddenShieldFontFallback,
    textTheme: base.textTheme.apply(
      fontFamilyFallback: _hiddenShieldFontFallback,
      bodyColor: HsColors.text,
      displayColor: HsColors.text,
    ),
    appBarTheme: const AppBarTheme(
      elevation: 0,
      scrolledUnderElevation: 0,
      backgroundColor: HsColors.appBar,
      foregroundColor: HsColors.text,
      titleTextStyle: TextStyle(
        color: HsColors.text,
        fontSize: 18,
        fontWeight: FontWeight.w700,
        fontFamilyFallback: _hiddenShieldFontFallback,
      ),
    ),
    cardTheme: CardThemeData(
      elevation: 0,
      margin: EdgeInsets.zero,
      color: HsColors.surface,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(HsRadii.panel),
        side: const BorderSide(color: HsColors.border),
      ),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: HsColors.surfaceMuted,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(HsRadii.card),
        borderSide: const BorderSide(color: HsColors.border),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(HsRadii.card),
        borderSide: const BorderSide(color: HsColors.border),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(HsRadii.card),
        borderSide: const BorderSide(color: HsColors.accent, width: 1.4),
      ),
      labelStyle: const TextStyle(color: HsColors.textMuted),
      hintStyle: const TextStyle(color: HsColors.textSubtle),
      helperStyle: const TextStyle(color: HsColors.textSubtle, height: 1.35),
      contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        minimumSize: const Size.fromHeight(48),
        backgroundColor: HsColors.accent,
        foregroundColor: HsColors.textInverse,
        disabledBackgroundColor: HsColors.surfaceRaised,
        disabledForegroundColor: HsColors.textSubtle,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(HsRadii.card),
        ),
        textStyle: const TextStyle(
          fontWeight: FontWeight.w700,
          fontFamilyFallback: _hiddenShieldFontFallback,
        ),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: HsColors.accent,
        textStyle: const TextStyle(
          fontWeight: FontWeight.w700,
          fontFamilyFallback: _hiddenShieldFontFallback,
        ),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        minimumSize: const Size.fromHeight(44),
        foregroundColor: HsColors.text,
        side: const BorderSide(color: HsColors.border),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(HsRadii.card),
        ),
        textStyle: const TextStyle(
          fontWeight: FontWeight.w700,
          fontFamilyFallback: _hiddenShieldFontFallback,
        ),
      ),
    ),
    iconButtonTheme: IconButtonThemeData(
      style: IconButton.styleFrom(
        foregroundColor: HsColors.iconMuted,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(HsRadii.card),
        ),
      ),
    ),
    bottomSheetTheme: const BottomSheetThemeData(
      backgroundColor: HsColors.appBar,
      surfaceTintColor: Colors.transparent,
      modalBackgroundColor: HsColors.appBar,
      modalBarrierColor: HsColors.scrim,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(
          top: Radius.circular(HsRadii.sheet),
        ),
      ),
    ),
    snackBarTheme: SnackBarThemeData(
      backgroundColor: HsColors.surfaceRaised,
      contentTextStyle: const TextStyle(
        color: HsColors.text,
        fontFamilyFallback: _hiddenShieldFontFallback,
      ),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(HsRadii.card),
        side: const BorderSide(color: HsColors.border),
      ),
      behavior: SnackBarBehavior.floating,
    ),
    navigationBarTheme: NavigationBarThemeData(
      height: 68,
      backgroundColor: HsColors.navigation,
      indicatorColor: HsColors.chip,
      labelTextStyle: WidgetStateProperty.resolveWith(
        (states) => TextStyle(
          color: states.contains(WidgetState.selected)
              ? HsColors.text
              : HsColors.textSubtle,
          fontSize: 12,
          fontWeight: states.contains(WidgetState.selected)
              ? FontWeight.w700
              : FontWeight.w500,
          fontFamilyFallback: _hiddenShieldFontFallback,
        ),
      ),
      iconTheme: WidgetStateProperty.resolveWith(
        (states) => IconThemeData(
          color: states.contains(WidgetState.selected)
              ? HsColors.accent
              : HsColors.iconMuted,
          size: 22,
        ),
      ),
    ),
    chipTheme: ChipThemeData(
      backgroundColor: HsColors.chip,
      selectedColor: HsColors.surfaceRaised,
      disabledColor: HsColors.surfaceMuted,
      side: const BorderSide(color: HsColors.border),
      labelStyle: const TextStyle(
        color: HsColors.textMuted,
        fontWeight: FontWeight.w700,
        fontFamilyFallback: _hiddenShieldFontFallback,
      ),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(HsRadii.pill),
      ),
    ),
    dividerTheme: const DividerThemeData(color: HsColors.border, thickness: 1),
  );
}

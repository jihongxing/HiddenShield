import 'package:flutter_test/flutter_test.dart';

import 'package:hidden_shield_mobile/app/app.dart';

void main() {
  testWidgets('renders the four main tabs', (WidgetTester tester) async {
    await tester.pumpWidget(const HiddenShieldApp());
    await tester.pumpAndSettle();

    expect(find.text('工作台'), findsWidgets);
    expect(find.text('取证'), findsWidgets);
    expect(find.text('版权库'), findsWidgets);
    expect(find.text('设置'), findsWidgets);
    expect(find.text('桥接层已接入'), findsOneWidget);
  });

  testWidgets('opens the image embed flow', (WidgetTester tester) async {
    await tester.pumpWidget(const HiddenShieldApp());
    await tester.pumpAndSettle();

    await tester.tap(find.text('图片嵌入'));
    await tester.pumpAndSettle();

    expect(find.text('选择图片'), findsOneWidget);
    expect(find.text('允许重写已有隐盾水印'), findsOneWidget);
    expect(find.text('写入盲水印'), findsOneWidget);
  });

  testWidgets('opens the audio embed flow', (WidgetTester tester) async {
    await tester.pumpWidget(const HiddenShieldApp());
    await tester.pumpAndSettle();

    await tester.tap(find.text('音频嵌入'));
    await tester.pumpAndSettle();

    expect(find.text('选择 WAV'), findsOneWidget);
    expect(find.text('允许重写已有隐盾水印'), findsOneWidget);
    expect(find.text('写入盲水印'), findsOneWidget);
  });
}

import 'package:flutter_test/flutter_test.dart';

import 'package:hidden_shield_mobile/main.dart';

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
}

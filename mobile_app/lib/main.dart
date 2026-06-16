import 'package:flutter/material.dart';

import 'app/app.dart';
import 'app/bootstrap.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final bridge = await createDefaultWatermarkBridge();
  runApp(HiddenShieldApp(bridge: bridge));
}

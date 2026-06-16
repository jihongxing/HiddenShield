import 'package:flutter/material.dart';

import 'app/app.dart';
import 'app/bootstrap.dart';
import 'app/mobile_app_state.dart';
import 'storage/vault_store.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final bridge = await createDefaultWatermarkBridge();
  final appState = MobileAppState(vaultStore: await SQLiteVaultStore.open());
  await appState.load();
  runApp(HiddenShieldApp(bridge: bridge, appState: appState));
}

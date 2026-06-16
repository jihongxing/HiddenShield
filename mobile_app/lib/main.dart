import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import 'app/app.dart';
import 'app/bootstrap.dart';
import 'app/mobile_app_state.dart';
import 'storage/vault_store.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final bridge = await createDefaultWatermarkBridge();
  final vaultStore = kIsWeb
      ? MemoryVaultStore()
      : await SQLiteVaultStore.open();
  final appState = MobileAppState(vaultStore: vaultStore);
  await appState.load();
  runApp(HiddenShieldApp(bridge: bridge, appState: appState));
}

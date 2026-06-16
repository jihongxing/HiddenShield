import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import 'app/app.dart';
import 'app/bootstrap.dart';
import 'app/mobile_app_state.dart';
import 'app/system_config.dart';
import 'sync/cloud_account_client.dart';
import 'storage/vault_store.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final bridge = await createDefaultWatermarkBridge();
  final systemConfig = await HiddenShieldSystemConfig.load();
  final vaultStore = kIsWeb
      ? MemoryVaultStore()
      : await SQLiteVaultStore.open();
  final appState = MobileAppState(
    vaultStore: vaultStore,
    cloudAccountClient: CloudAccountClient(
      baseUrl: systemConfig.cloudBaseUrl,
    ),
  );
  await appState.load();
  runApp(HiddenShieldApp(bridge: bridge, appState: appState));
}

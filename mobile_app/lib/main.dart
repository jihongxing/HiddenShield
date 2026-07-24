import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:package_info_plus/package_info_plus.dart';

import 'app/app.dart';
import 'app/bootstrap.dart';
import 'app/mobile_app_state.dart';
import 'app/system_config.dart';
import 'licensing/offline_license_manager.dart';
import 'storage/vault_store_factory.dart';
import 'sync/cloud_account_client.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final bridge = await createDefaultWatermarkBridge();
  final systemConfig = await HiddenShieldSystemConfig.load();
  final vaultStore = await openDefaultVaultStore();
  final packageInfo = await PackageInfo.fromPlatform();
  final appState = MobileAppState(
    vaultStore: vaultStore,
    cloudAccountClient: CloudAccountClient(baseUrl: systemConfig.cloudBaseUrl),
    offlineLicenseManager: OfflineLicenseManager(
      secureStore: PlatformOfflineLicenseSecureStore(),
      platform: switch (defaultTargetPlatform) {
        TargetPlatform.android => 'android',
        TargetPlatform.iOS => 'ios',
        _ => 'unsupported',
      },
      appVersion: packageInfo.version,
    ),
  );
  await appState.load();
  runApp(HiddenShieldApp(bridge: bridge, appState: appState));
}

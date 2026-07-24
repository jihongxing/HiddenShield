import 'vault_store.dart';
import 'vault_store_factory_io.dart'
    if (dart.library.html) 'vault_store_factory_web.dart'
    as platform;

Future<VaultStore> openDefaultVaultStore() {
  return platform.openDefaultVaultStore();
}

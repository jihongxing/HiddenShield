import 'vault_store.dart';
import 'web_profile_vault_store.dart';

Future<VaultStore> openDefaultVaultStore() {
  return WebProfileVaultStore.open();
}

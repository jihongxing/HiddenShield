import 'vault_store.dart';

Future<VaultStore> openDefaultVaultStore() {
  return SQLiteVaultStore.open();
}

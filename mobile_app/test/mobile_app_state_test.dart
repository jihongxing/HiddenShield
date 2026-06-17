import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';
import 'package:hidden_shield_mobile/sync/cloud_account_client.dart';
import 'package:hidden_shield_mobile/sync/sync_transport.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  test('loads existing vault records from the store', () async {
    final store = MemoryVaultStore();
    await store.upsertRecord(
      VaultRecord(
        id: 'existing-record',
        kind: WatermarkAssetKind.image,
        title: 'existing.png',
        watermarkUid: 'uid-existing',
        revision: 1,
        source: VaultRecordSource.write,
        syncStatus: SyncStatus.pending,
        createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
      ),
    );

    final state = MobileAppState(vaultStore: store);
    await state.load();

    expect(state.isLoaded, isTrue);
    expect(state.records, hasLength(1));
    expect(state.records.single.watermarkUid, 'uid-existing');
    expect(state.syncQueue, isEmpty);
  });

  test('persists write results and queues them for desktop sync', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();

    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.audio,
        bytes: [1, 2, 3],
        watermarkUid: 'uid-audio',
        revision: 1,
        sha256: 'abc123',
        verification: WatermarkWriteVerification(
          verified: true,
          watermarkUid: 'verified-uid',
          revision: 1,
          message: '已回读验证版权编号，保护副本可取证。',
        ),
      ),
      fileName: 'song.wav',
      allowRewrite: false,
    );

    final persisted = await store.loadRecords();
    expect(persisted, hasLength(1));
    expect(persisted.single.title, 'song.wav');
    expect(persisted.single.syncStatus, SyncStatus.pending);

    final queue = await store.loadSyncQueue();
    expect(queue, hasLength(1));
    expect(queue.single.recordId, persisted.single.id);
    expect(queue.single.operation, SyncQueueOperation.upsertVaultRecord);
    expect(queue.single.status, SyncQueueItemStatus.pending);
    expect(state.pendingSyncQueueCount, 1);
  });

  test('persists mobile rewrite lineage for write results', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();

    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.image,
        bytes: [1, 2, 3],
        watermarkUid: 'uid-rewrite',
        revision: 2,
        sha256: 'hash-rewrite',
        verification: WatermarkWriteVerification(
          verified: true,
          watermarkUid: 'verified-uid',
          revision: 1,
          message: '已回读验证版权编号，保护副本可取证。',
        ),
      ),
      fileName: 'rewrite.png',
      allowRewrite: true,
      rewriteReason: 'mobile explicit rewrite',
      parentWatermarkUid: 'uid-parent',
      revision: 4,
    );

    final persisted = await store.loadRecords();
    expect(persisted.single.revision, 4);
    expect(persisted.single.parentWatermarkUid, 'uid-parent');
    expect(persisted.single.rewriteReason, 'mobile explicit rewrite');

    final queue = await store.loadSyncQueue();
    expect(queue.single.payloadJson, contains('"revision":4'));
    expect(queue.single.payloadJson, contains('uid-parent'));
    expect(queue.single.payloadJson, contains('mobile explicit rewrite'));
  });

  test('persists verify results and queues evidence records', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();

    state.addReadResult(
      result: const WatermarkReadResult(
        kind: WatermarkAssetKind.image,
        watermarkUid: 'uid-image',
        revision: 2,
        timestamp: 123,
        deviceIdHex: 'device',
        fileHashHex: 'hash',
        parentWatermarkUid: 'uid-parent',
        rewriteReason: 'authorized rewrite',
      ),
      fileName: 'suspect.png',
    );

    final persisted = await store.loadRecords();
    expect(persisted, hasLength(1));
    expect(persisted.single.source, VaultRecordSource.verify);
    expect(persisted.single.syncStatus, SyncStatus.pending);
    expect(persisted.single.extractedTimestamp, 123);
    expect(persisted.single.extractedDeviceIdHex, 'device');
    expect(persisted.single.extractedFileHashHex, 'hash');

    final queue = await store.loadSyncQueue();
    expect(queue, hasLength(1));
    expect(queue.single.operation, SyncQueueOperation.upsertEvidenceRecord);
    expect(queue.single.payloadJson, contains('uid-parent'));
    expect(queue.single.payloadJson, contains('extracted_timestamp'));
    expect(queue.single.payloadJson, contains('device'));
    expect(queue.single.payloadJson, contains('hash'));
  });

  test('syncs pending queue items with the local mock transport', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(
      vaultStore: store,
      syncTransport: const LocalMockSyncTransport(),
    );
    await state.load();

    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.image,
        bytes: [1, 2, 3],
        watermarkUid: 'uid-sync',
        revision: 1,
        sha256: 'hash',
        verification: WatermarkWriteVerification(
          verified: true,
          watermarkUid: 'verified-uid',
          revision: 1,
          message: '已回读验证版权编号，保护副本可取证。',
        ),
      ),
      fileName: 'work.png',
      allowRewrite: false,
    );

    await state.syncPendingQueue();

    final queue = await store.loadSyncQueue();
    expect(queue.single.status, SyncQueueItemStatus.synced);
    expect(queue.single.attempts, 1);
    expect(queue.single.lastError, isNull);
    expect(state.pendingSyncQueueCount, 0);
    expect(state.syncProfile.lastSyncAttemptAt, isNotNull);
    expect(state.syncProfile.lastSyncSuccessAt, isNotNull);
    expect(state.syncProfile.lastSyncFailureAt, isNull);
    expect(state.syncProfile.status, SyncConnectionStatus.connected);
  });

  test('marks failed sync attempts and keeps retry metadata', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(
      vaultStore: store,
      syncTransport: const LocalMockSyncTransport(shouldFail: true),
    );
    await state.load();

    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.audio,
        bytes: [1, 2, 3],
        watermarkUid: 'uid-fail',
        revision: 1,
        sha256: 'hash',
        verification: WatermarkWriteVerification(
          verified: true,
          watermarkUid: 'verified-uid',
          revision: 1,
          message: '已回读验证版权编号，保护副本可取证。',
        ),
      ),
      fileName: 'fail.wav',
      allowRewrite: false,
    );

    await state.syncPendingQueue();

    final queue = await store.loadSyncQueue();
    expect(queue.single.status, SyncQueueItemStatus.failed);
    expect(queue.single.attempts, 1);
    expect(queue.single.lastError, 'local mock sync failed');
    expect(queue.single.nextRetryAt, isNotNull);
    expect(queue.single.nextRetryAt!.isAfter(DateTime.now()), isTrue);
    expect(state.failedSyncQueueCount, 1);
    expect(state.syncProfile.lastSyncAttemptAt, isNotNull);
    expect(state.syncProfile.lastSyncSuccessAt, isNull);
    expect(state.syncProfile.lastSyncFailureAt, isNotNull);
    expect(state.syncProfile.status, SyncConnectionStatus.failed);
    expect(state.syncProfile.lastError, 'local mock sync failed');
  });

  test('retries failed sync queue items', () async {
    final store = MemoryVaultStore();
    final failedItem = _syncQueueItem('queue-failed').copyWith(
      status: SyncQueueItemStatus.failed,
      attempts: 1,
      lastError: 'network failed',
    );
    await store.enqueueSyncItem(failedItem);
    final state = MobileAppState(
      vaultStore: store,
      syncTransport: const LocalMockSyncTransport(),
    );
    await state.load();

    await state.retryFailedSyncQueue();

    final queue = await store.loadSyncQueue();
    expect(queue.single.status, SyncQueueItemStatus.synced);
    expect(queue.single.attempts, 2);
    expect(queue.single.lastError, isNull);
    expect(queue.single.nextRetryAt, isNull);
    expect(state.failedSyncQueueCount, 0);
  });

  test('skips failed sync queue items before retry backoff expires', () async {
    final store = MemoryVaultStore();
    final failedItem = _syncQueueItem('queue-backoff').copyWith(
      status: SyncQueueItemStatus.failed,
      attempts: 1,
      lastError: 'network failed',
      nextRetryAt: DateTime.now().add(const Duration(minutes: 5)),
    );
    await store.enqueueSyncItem(failedItem);
    final transport = _RecordingBatchTransport();
    final state = MobileAppState(vaultStore: store, syncTransport: transport);
    await state.load();

    await state.syncPendingQueue();

    final queue = await store.loadSyncQueue();
    expect(transport.batchCalls, 0);
    expect(queue.single.status, SyncQueueItemStatus.failed);
    expect(queue.single.attempts, 1);
  });

  test('manually retries failed queue items even during backoff', () async {
    final store = MemoryVaultStore();
    final failedItem = _syncQueueItem('queue-manual-backoff').copyWith(
      status: SyncQueueItemStatus.failed,
      attempts: 1,
      lastError: 'network failed',
      nextRetryAt: DateTime.now().add(const Duration(hours: 1)),
    );
    await store.enqueueSyncItem(failedItem);
    final state = MobileAppState(
      vaultStore: store,
      syncTransport: const LocalMockSyncTransport(),
    );
    await state.load();

    await state.retryFailedSyncQueue();

    final queue = await store.loadSyncQueue();
    expect(queue.single.status, SyncQueueItemStatus.synced);
    expect(queue.single.attempts, 2);
    expect(queue.single.nextRetryAt, isNull);
  });

  test('stops automatic sync after max failed attempts', () async {
    final store = MemoryVaultStore();
    final failedItem = _syncQueueItem('queue-max-attempts').copyWith(
      status: SyncQueueItemStatus.failed,
      attempts: MobileAppState.syncQueueMaxAttempts,
      lastError: 'network failed',
    );
    await store.enqueueSyncItem(failedItem);
    final transport = _RecordingBatchTransport();
    final state = MobileAppState(vaultStore: store, syncTransport: transport);
    await state.load();

    await state.syncPendingQueue();

    final queue = await store.loadSyncQueue();
    expect(transport.batchCalls, 0);
    expect(queue.single.status, SyncQueueItemStatus.failed);
    expect(queue.single.attempts, MobileAppState.syncQueueMaxAttempts);
  });

  test('saves and loads LAN debug sync profile', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();

    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: '123456',
    );

    expect(state.cloudSyncEnabled, isFalse);
    expect(state.syncTransportMode, SyncTransportMode.lanDebug);
    expect(state.syncProfile.status, SyncConnectionStatus.connected);
    expect(state.syncProfile.lanDebugAddress, 'http://127.0.0.1:47219');

    final reloaded = MobileAppState(vaultStore: store);
    await reloaded.load();

    expect(reloaded.cloudSyncEnabled, isFalse);
    expect(reloaded.syncProfile.lanDebugPairingCode, '123456');
  });

  test(
    'continue with account creates or loads account identity contract',
    () async {
      final store = MemoryVaultStore();
      final state = MobileAppState(vaultStore: store);
      await state.load();
      state.updateCreatorLabel('Alice Creator');

      await state.continueWithAccountPlaceholder(
        accountLabel: 'alice@example.com',
      );

      expect(state.cloudSyncEnabled, isTrue);
      expect(state.syncProfile.accountId, startsWith('acct_'));
      expect(state.syncProfile.workspaceId, startsWith('ws_'));
      expect(state.syncProfile.workspaceName, '个人空间');
      expect(state.syncProfile.deviceId, startsWith('dev_'));
      expect(state.syncProfile.deviceRegistered, isTrue);
      expect(state.syncProfile.creatorProfileId, startsWith('creator_'));
      expect(state.syncProfile.creatorDisplayName, 'Alice Creator');
      expect(state.syncProfile.creatorProfileSynced, isTrue);
      expect(state.syncProfile.entitlementId, startsWith('ent_'));
      expect(state.syncProfile.entitlementPlanCode, 'free');
      expect(state.syncProfile.entitlementFeatures['cloud_sync'], isTrue);

      final reloaded = MobileAppState(vaultStore: store);
      await reloaded.load();

      expect(reloaded.cloudSyncEnabled, isTrue);
      expect(reloaded.syncProfile.accountLabel, 'alice@example.com');
      expect(reloaded.syncProfile.workspaceName, '个人空间');
      expect(reloaded.syncProfile.deviceRegistered, isTrue);
      expect(reloaded.syncProfile.creatorDisplayName, 'Alice Creator');
      expect(reloaded.syncProfile.entitlementFeatures['cloud_sync'], isTrue);
    },
  );

  test('continue with the same account is idempotent', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();

    await state.continueWithAccountPlaceholder(
      accountLabel: 'alice@example.com',
    );
    final firstAccountId = state.syncProfile.accountId;
    final firstWorkspaceId = state.syncProfile.workspaceId;
    final firstCreatorId = state.syncProfile.creatorProfileId;

    await state.signOutCloud();
    await state.continueWithAccountPlaceholder(
      accountLabel: 'alice@example.com',
    );

    expect(state.syncProfile.accountId, firstAccountId);
    expect(state.syncProfile.workspaceId, firstWorkspaceId);
    expect(state.syncProfile.creatorProfileId, firstCreatorId);
  });

  test(
    'continue with account can apply cloud auth continue response',
    () async {
      final store = MemoryVaultStore();
      final cloudClient = CloudAccountClient(
        baseUrl: 'https://api.hiddenshield.test',
        client: MockClient((request) async {
          return http.Response.bytes(
            utf8.encode(
              jsonEncode({
                'accessToken': 'access-token',
                'refreshToken': 'refresh-token',
                'account': {
                  'id': 'acct-cloud',
                  'displayName': 'alice@example.com',
                },
                'workspace': {'id': 'ws-cloud', 'name': '个人空间'},
                'device': {'id': 'device-cloud', 'registered': true},
                'creatorProfile': {
                  'id': 'creator-cloud',
                  'displayName': 'Alice Creator',
                  'isDefault': true,
                },
                'entitlement': {
                  'id': 'ent-cloud',
                  'planName': '免费版',
                  'planCode': 'free',
                  'status': 'free',
                  'features': {'cloud_sync': true},
                },
              }),
            ),
            200,
            headers: const {'content-type': 'application/json; charset=utf-8'},
          );
        }),
      );
      final state = MobileAppState(
        vaultStore: store,
        cloudAccountClient: cloudClient,
      );
      await state.load();
      state.updateCreatorLabel('Alice Creator');

      await state.continueWithAccountPlaceholder(
        accountLabel: 'alice@example.com',
      );

      expect(state.syncProfile.accountId, 'acct-cloud');
      expect(state.syncProfile.authToken, 'access-token');
      expect(state.syncProfile.refreshToken, 'refresh-token');
      expect(state.syncProfile.workspaceId, 'ws-cloud');
      expect(state.syncProfile.deviceId, 'device-cloud');
      expect(state.syncProfile.creatorProfileId, 'creator-cloud');
      expect(state.syncProfile.entitlementId, 'ent-cloud');
    },
  );

  test('test desktop connection returns paired status', () async {
    final state = MobileAppState(vaultStore: MemoryVaultStore());
    await state.load();
    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );

    await state.testLanDebugConnection();

    expect(state.syncProfile.status, SyncConnectionStatus.connected);
    expect(state.syncProfile.lastError, isNull);
  });

  test('enables LAN debug sync mode only after debug pairing', () async {
    final state = MobileAppState(vaultStore: MemoryVaultStore());
    await state.load();

    state.setSyncTransportMode(SyncTransportMode.lanDebug);
    expect(state.syncTransportMode, SyncTransportMode.localOnly);

    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );
    state.setSyncTransportMode(SyncTransportMode.lanDebug);

    expect(state.syncTransportMode, SyncTransportMode.lanDebug);
  });

  test('uses selected transport mode when syncing', () async {
    final usedModes = <SyncTransportMode>[];
    final state = MobileAppState(
      vaultStore: MemoryVaultStore(),
      syncTransportFactory: (mode, syncProfile) {
        usedModes.add(mode);
        return const LocalMockSyncTransport();
      },
    );
    await state.load();
    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );
    state.setSyncTransportMode(SyncTransportMode.lanDebug);

    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.image,
        bytes: [1, 2, 3],
        watermarkUid: 'uid-http',
        revision: 1,
        sha256: 'hash',
        verification: WatermarkWriteVerification(
          verified: true,
          watermarkUid: 'verified-uid',
          revision: 1,
          message: '已回读验证版权编号，保护副本可取证。',
        ),
      ),
      fileName: 'http.png',
      allowRewrite: false,
    );

    await state.syncPendingQueue();

    expect(usedModes, contains(SyncTransportMode.lanDebug));
    expect(state.pendingSyncQueueCount, 0);
  });

  test('syncs multiple pending items in one batch transport call', () async {
    final store = MemoryVaultStore();
    await store.enqueueSyncItem(_syncQueueItem('queue-batch-1'));
    await store.enqueueSyncItem(_syncQueueItem('queue-batch-2'));

    final transport = _RecordingBatchTransport();
    final state = MobileAppState(vaultStore: store, syncTransport: transport);
    await state.load();

    await state.syncPendingQueue();

    expect(transport.batchCalls, 1);
    expect(transport.batchSizes.single, 2);
    expect(state.pendingSyncQueueCount, 0);
  });

  test('pulls desktop changes into the local vault', () async {
    final store = MemoryVaultStore();
    final transport = _DesktopChangesTransport();
    final state = MobileAppState(vaultStore: store, syncTransport: transport);
    await state.load();
    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );

    await state.pullRemoteChanges();

    final records = await store.loadRecords();
    expect(transport.fetchCalls, 1);
    expect(records, hasLength(2));
    final desktopRecord = records.firstWhere(
      (record) => record.watermarkUid == 'uid-desktop',
    );
    final evidenceRecord = records.firstWhere(
      (record) => record.watermarkUid == 'uid-evidence',
    );
    expect(desktopRecord.id, 'lan:desktop-1');
    expect(desktopRecord.syncStatus, SyncStatus.synced);
    expect(evidenceRecord.source, VaultRecordSource.verify);
    expect(evidenceRecord.extractedTimestamp, 123);
    expect(evidenceRecord.extractedDeviceIdHex, 'device');
    expect(evidenceRecord.extractedFileHashHex, 'hash');
    expect(state.syncProfile.lastError, isNull);
    expect(state.syncProfile.lastRemotePullCursor, '2026-06-16T12:00:00.000Z');
    expect(state.syncProfile.lastSyncAttemptAt, isNotNull);
    expect(state.syncProfile.lastSyncSuccessAt, isNotNull);
    expect(state.syncProfile.lastSyncFailureAt, isNull);

    final reloaded = MobileAppState(
      vaultStore: store,
      syncTransport: transport,
    );
    await reloaded.load();
    expect(
      reloaded.syncProfile.lastRemotePullCursor,
      '2026-06-16T12:00:00.000Z',
    );

    await reloaded.pullRemoteChanges();
    expect(transport.lastSince, '2026-06-16T12:00:00.000Z');
  });

  test(
    'deduplicates desktop pull records with the same uid hash and revision',
    () async {
      final store = MemoryVaultStore();
      await store.upsertRecord(
        _vaultRecord(
          id: 'local-existing',
          watermarkUid: 'uid-dup',
          revision: 2,
          sha256: 'hash-dup',
        ),
      );
      final state = MobileAppState(
        vaultStore: store,
        syncTransport: _StaticChangesTransport(
          changes: [
            _desktopChange(
              id: 'desktop-duplicate',
              watermarkUid: 'uid-dup',
              revision: 2,
              sha256: 'hash-dup',
            ),
          ],
        ),
      );
      await state.load();
      await state.saveLanDebugPairing(
        lanDebugAddress: 'http://127.0.0.1:47219',
        pairingCode: 'abcdef',
      );

      await state.pullRemoteChanges();

      final records = await store.loadRecords();
      final resolutions = await store.loadSyncResolutions();
      expect(records, hasLength(1));
      expect(records.single.id, 'local-existing');
      expect(
        resolutions.single.resolutionType,
        MobileSyncResolutionType.duplicateIgnored,
      );
      expect(resolutions.single.existingRecordId, 'local-existing');
    },
  );

  test('upgrades same uid and hash to the highest pulled revision', () async {
    final store = MemoryVaultStore();
    await store.upsertRecord(
      _vaultRecord(
        id: 'local-existing',
        title: 'old.png',
        watermarkUid: 'uid-upgrade',
        revision: 1,
        sha256: 'hash-upgrade',
      ),
    );
    final state = MobileAppState(
      vaultStore: store,
      syncTransport: _StaticChangesTransport(
        changes: [
          _desktopChange(
            id: 'desktop-upgrade',
            title: 'new.png',
            watermarkUid: 'uid-upgrade',
            revision: 3,
            sha256: 'hash-upgrade',
          ),
        ],
      ),
    );
    await state.load();
    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );

    await state.pullRemoteChanges();

    final records = await store.loadRecords();
    final resolutions = await store.loadSyncResolutions();
    expect(records, hasLength(1));
    expect(records.single.id, 'local-existing');
    expect(records.single.title, 'new.png');
    expect(records.single.revision, 3);
    expect(
      resolutions.single.resolutionType,
      MobileSyncResolutionType.revisionUpgraded,
    );
  });

  test('ignores stale desktop revisions for the same uid and hash', () async {
    final store = MemoryVaultStore();
    await store.upsertRecord(
      _vaultRecord(
        id: 'local-existing',
        watermarkUid: 'uid-stale',
        revision: 5,
        sha256: 'hash-stale',
      ),
    );
    final state = MobileAppState(
      vaultStore: store,
      syncTransport: _StaticChangesTransport(
        changes: [
          _desktopChange(
            id: 'desktop-stale',
            watermarkUid: 'uid-stale',
            revision: 3,
            sha256: 'hash-stale',
          ),
        ],
      ),
    );
    await state.load();
    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );

    await state.pullRemoteChanges();

    final records = await store.loadRecords();
    final resolutions = await store.loadSyncResolutions();
    expect(records, hasLength(1));
    expect(records.single.revision, 5);
    expect(
      resolutions.single.resolutionType,
      MobileSyncResolutionType.staleRevisionIgnored,
    );
  });

  test(
    'accepts desktop variants with the same uid and different hash',
    () async {
      final store = MemoryVaultStore();
      await store.upsertRecord(
        _vaultRecord(
          id: 'local-existing',
          watermarkUid: 'uid-variant',
          revision: 2,
          sha256: 'hash-a',
        ),
      );
      final state = MobileAppState(
        vaultStore: store,
        syncTransport: _StaticChangesTransport(
          changes: [
            _desktopChange(
              id: 'desktop-variant',
              watermarkUid: 'uid-variant',
              revision: 2,
              sha256: 'hash-b',
            ),
          ],
        ),
      );
      await state.load();
      await state.saveLanDebugPairing(
        lanDebugAddress: 'http://127.0.0.1:47219',
        pairingCode: 'abcdef',
      );

      await state.pullRemoteChanges();

      final records = await store.loadRecords();
      final resolutions = await store.loadSyncResolutions();
      expect(records, hasLength(2));
      expect(
        records.map((record) => record.sha256),
        containsAll(['hash-a', 'hash-b']),
      );
      expect(
        resolutions.single.resolutionType,
        MobileSyncResolutionType.variantAccepted,
      );
      expect(resolutions.single.insertedRecordId, 'lan:desktop-variant');
    },
  );
}

VaultRecord _vaultRecord({
  required String id,
  required String watermarkUid,
  required int revision,
  required String sha256,
  String title = 'work.png',
}) {
  return VaultRecord(
    id: id,
    kind: WatermarkAssetKind.image,
    title: title,
    watermarkUid: watermarkUid,
    revision: revision,
    sha256: sha256,
    source: VaultRecordSource.write,
    syncStatus: SyncStatus.synced,
    createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
  );
}

RemoteSyncChange _desktopChange({
  required String id,
  required String watermarkUid,
  required int revision,
  required String sha256,
  String title = 'desktop.png',
}) {
  return RemoteSyncChange(
    id: id,
    kind: 'image',
    title: title,
    watermarkUid: watermarkUid,
    revision: revision,
    sha256: sha256,
    sourceDevice: 'lanDebug',
    createdAt: '2026-06-16T12:00:00.000Z',
  );
}

SyncQueueItem _syncQueueItem(String id) {
  return SyncQueueItem(
    id: id,
    recordId: '$id-record',
    operation: SyncQueueOperation.upsertVaultRecord,
    payloadType: 'vault_record',
    payloadJson: '{"id":"$id-record","kind":"image","watermark_uid":"$id-uid"}',
    status: SyncQueueItemStatus.pending,
    attempts: 0,
    createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
  );
}

class _StaticChangesTransport implements SyncTransport {
  const _StaticChangesTransport({required this.changes});

  final List<RemoteSyncChange> changes;

  @override
  Future<SyncSendResult> send(SyncQueueItem item) async {
    return const SyncSendResult.success();
  }

  @override
  Future<SyncBatchSendResult> sendBatch(List<SyncQueueItem> items) async {
    return SyncBatchSendResult({
      for (final item in items) item.id: const SyncSendResult.success(),
    });
  }

  @override
  Future<SyncChangesResult> fetchChanges({String? since}) async {
    return SyncChangesResult.success(
      changes: changes,
      nextSince: '2026-06-16T12:00:00.000Z',
    );
  }
}

class _RecordingBatchTransport implements SyncTransport {
  int batchCalls = 0;
  final List<int> batchSizes = [];

  @override
  Future<SyncSendResult> send(SyncQueueItem item) async {
    throw StateError('single-item send should not be used');
  }

  @override
  Future<SyncBatchSendResult> sendBatch(List<SyncQueueItem> items) async {
    batchCalls += 1;
    batchSizes.add(items.length);
    return SyncBatchSendResult({
      for (final item in items) item.id: const SyncSendResult.success(),
    });
  }

  @override
  Future<SyncChangesResult> fetchChanges({String? since}) async {
    return const SyncChangesResult.success(changes: [], nextSince: '');
  }
}

class _DesktopChangesTransport implements SyncTransport {
  int fetchCalls = 0;
  String? lastSince;

  @override
  Future<SyncSendResult> send(SyncQueueItem item) async {
    return const SyncSendResult.success();
  }

  @override
  Future<SyncBatchSendResult> sendBatch(List<SyncQueueItem> items) async {
    return SyncBatchSendResult({
      for (final item in items) item.id: const SyncSendResult.success(),
    });
  }

  @override
  Future<SyncChangesResult> fetchChanges({String? since}) async {
    fetchCalls += 1;
    lastSince = since;
    return const SyncChangesResult.success(
      nextSince: '2026-06-16T12:00:00.000Z',
      changes: [
        RemoteSyncChange(
          id: 'desktop-1',
          kind: 'image',
          title: 'desktop.png',
          watermarkUid: 'uid-desktop',
          revision: 2,
          sha256: 'hash-desktop',
          sourceDevice: 'lanDebug',
          createdAt: '2026-06-16T12:00:00.000Z',
        ),
        RemoteSyncChange(
          id: 'desktop-evidence-1',
          kind: 'audio',
          title: 'suspect.wav',
          watermarkUid: 'uid-evidence',
          revision: 3,
          source: 'verify',
          extractedTimestamp: 123,
          extractedDeviceIdHex: 'device',
          extractedFileHashHex: 'hash',
          sourceDevice: 'lanDebug',
          createdAt: '2026-06-16T12:00:01.000Z',
        ),
      ],
    );
  }
}

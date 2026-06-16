import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';
import 'package:hidden_shield_mobile/sync/sync_transport.dart';

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
    final state = MobileAppState(vaultStore: store);
    await state.load();

    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.image,
        bytes: [1, 2, 3],
        watermarkUid: 'uid-sync',
        revision: 1,
        sha256: 'hash',
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
      ),
      fileName: 'fail.wav',
      allowRewrite: false,
    );

    await state.syncPendingQueue();

    final queue = await store.loadSyncQueue();
    expect(queue.single.status, SyncQueueItemStatus.failed);
    expect(queue.single.attempts, 1);
    expect(queue.single.lastError, 'local mock sync failed');
    expect(state.failedSyncQueueCount, 1);
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
    expect(state.failedSyncQueueCount, 0);
  });

  test('saves and loads desktop pairing profile', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();

    await state.saveDesktopPairing(
      desktopAddress: 'http://127.0.0.1:47219',
      pairingCode: '123456',
    );

    expect(state.desktopSyncEnabled, isTrue);
    expect(state.pairingProfile.status, DesktopPairingStatus.paired);
    expect(state.pairingProfile.desktopAddress, 'http://127.0.0.1:47219');

    final reloaded = MobileAppState(vaultStore: store);
    await reloaded.load();

    expect(reloaded.desktopSyncEnabled, isTrue);
    expect(reloaded.pairingProfile.pairingCode, '123456');
  });

  test('test desktop connection returns paired status', () async {
    final state = MobileAppState(vaultStore: MemoryVaultStore());
    await state.load();
    await state.saveDesktopPairing(
      desktopAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );

    await state.testDesktopConnection();

    expect(state.pairingProfile.status, DesktopPairingStatus.paired);
    expect(state.pairingProfile.lastError, isNull);
  });

  test('enables http sync mode only after desktop pairing', () async {
    final state = MobileAppState(vaultStore: MemoryVaultStore());
    await state.load();

    state.setSyncTransportMode(SyncTransportMode.http);
    expect(state.syncTransportMode, SyncTransportMode.mock);

    await state.saveDesktopPairing(
      desktopAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );
    state.setSyncTransportMode(SyncTransportMode.http);

    expect(state.syncTransportMode, SyncTransportMode.http);
  });

  test('uses selected transport mode when syncing', () async {
    final usedModes = <SyncTransportMode>[];
    final state = MobileAppState(
      vaultStore: MemoryVaultStore(),
      syncTransportFactory: (mode, pairingProfile) {
        usedModes.add(mode);
        return const LocalMockSyncTransport();
      },
    );
    await state.load();
    await state.saveDesktopPairing(
      desktopAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );
    state.setSyncTransportMode(SyncTransportMode.http);

    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.image,
        bytes: [1, 2, 3],
        watermarkUid: 'uid-http',
        revision: 1,
        sha256: 'hash',
      ),
      fileName: 'http.png',
      allowRewrite: false,
    );

    await state.syncPendingQueue();

    expect(usedModes, contains(SyncTransportMode.http));
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
    await state.saveDesktopPairing(
      desktopAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );

    await state.pullDesktopChanges();

    final records = await store.loadRecords();
    expect(transport.fetchCalls, 1);
    expect(records, hasLength(2));
    final desktopRecord = records.firstWhere(
      (record) => record.watermarkUid == 'uid-desktop',
    );
    final evidenceRecord = records.firstWhere(
      (record) => record.watermarkUid == 'uid-evidence',
    );
    expect(desktopRecord.id, 'desktop:desktop-1');
    expect(desktopRecord.syncStatus, SyncStatus.synced);
    expect(evidenceRecord.source, VaultRecordSource.verify);
    expect(evidenceRecord.extractedTimestamp, 123);
    expect(evidenceRecord.extractedDeviceIdHex, 'device');
    expect(evidenceRecord.extractedFileHashHex, 'hash');
    expect(state.pairingProfile.lastError, isNull);
    expect(
      state.pairingProfile.lastDesktopPullSince,
      '2026-06-16T12:00:00.000Z',
    );

    final reloaded = MobileAppState(
      vaultStore: store,
      syncTransport: transport,
    );
    await reloaded.load();
    expect(
      reloaded.pairingProfile.lastDesktopPullSince,
      '2026-06-16T12:00:00.000Z',
    );

    await reloaded.pullDesktopChanges();
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
      await state.saveDesktopPairing(
        desktopAddress: 'http://127.0.0.1:47219',
        pairingCode: 'abcdef',
      );

      await state.pullDesktopChanges();

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
    await state.saveDesktopPairing(
      desktopAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );

    await state.pullDesktopChanges();

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
    await state.saveDesktopPairing(
      desktopAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );

    await state.pullDesktopChanges();

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
      await state.saveDesktopPairing(
        desktopAddress: 'http://127.0.0.1:47219',
        pairingCode: 'abcdef',
      );

      await state.pullDesktopChanges();

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
      expect(resolutions.single.insertedRecordId, 'desktop:desktop-variant');
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

DesktopSyncChange _desktopChange({
  required String id,
  required String watermarkUid,
  required int revision,
  required String sha256,
  String title = 'desktop.png',
}) {
  return DesktopSyncChange(
    id: id,
    kind: 'image',
    title: title,
    watermarkUid: watermarkUid,
    revision: revision,
    sha256: sha256,
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

  final List<DesktopSyncChange> changes;

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
        DesktopSyncChange(
          id: 'desktop-1',
          kind: 'image',
          title: 'desktop.png',
          watermarkUid: 'uid-desktop',
          revision: 2,
          sha256: 'hash-desktop',
          createdAt: '2026-06-16T12:00:00.000Z',
        ),
        DesktopSyncChange(
          id: 'desktop-evidence-1',
          kind: 'audio',
          title: 'suspect.wav',
          watermarkUid: 'uid-evidence',
          revision: 3,
          source: 'verify',
          extractedTimestamp: 123,
          extractedDeviceIdHex: 'device',
          extractedFileHashHex: 'hash',
          createdAt: '2026-06-16T12:00:01.000Z',
        ),
      ],
    );
  }
}

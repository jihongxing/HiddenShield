import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';

import '../bridge/watermark_models.dart';
import '../storage/vault_store.dart';
import '../sync/sync_transport.dart';

class MobileAppState extends ChangeNotifier {
  MobileAppState({
    VaultStore? vaultStore,
    SyncTransport? syncTransport,
    SyncTransportFactory? syncTransportFactory,
  }) : _vaultStore = vaultStore ?? MemoryVaultStore(),
       _syncTransportFactory =
           syncTransportFactory ?? _defaultSyncTransportFactory,
       _transportOverride = syncTransport;

  final VaultStore _vaultStore;
  final SyncTransportFactory _syncTransportFactory;
  final SyncTransport? _transportOverride;
  final List<VaultRecord> _records = [];
  final List<SyncQueueItem> _syncQueue = [];
  final List<MobileSyncResolution> _syncResolutions = [];

  String _creatorLabel = '本机创作者';
  bool _desktopSyncEnabled = false;
  bool _anonymousFeedbackEnabled = false;
  DesktopPairingProfile _pairingProfile = DesktopPairingProfile.unpaired();
  SyncTransportMode _syncTransportMode = SyncTransportMode.mock;
  bool _isLoaded = false;
  bool _isSyncing = false;
  bool _isPullingDesktopChanges = false;

  bool get isLoaded => _isLoaded;

  bool get isSyncing => _isSyncing;

  bool get isPullingDesktopChanges => _isPullingDesktopChanges;

  String get creatorLabel => _creatorLabel;

  bool get desktopSyncEnabled => _desktopSyncEnabled;

  bool get anonymousFeedbackEnabled => _anonymousFeedbackEnabled;

  DesktopPairingProfile get pairingProfile => _pairingProfile;

  SyncTransportMode get syncTransportMode => _syncTransportMode;

  List<VaultRecord> get records => List.unmodifiable(_records);

  List<SyncQueueItem> get syncQueue => List.unmodifiable(_syncQueue);

  List<MobileSyncResolution> get syncResolutions =>
      List.unmodifiable(_syncResolutions);

  List<VaultRecord> get recentRecords => records.take(3).toList();

  int get pendingSyncCount => _records
      .where((record) => record.syncStatus == SyncStatus.pending)
      .length;

  int get pendingSyncQueueCount => _syncQueue
      .where((item) => item.status == SyncQueueItemStatus.pending)
      .length;

  int get failedSyncQueueCount => _syncQueue
      .where((item) => item.status == SyncQueueItemStatus.failed)
      .length;

  bool get canUseHttpSync =>
      _pairingProfile.desktopAddress.isNotEmpty &&
      _pairingProfile.pairingCode.isNotEmpty &&
      _pairingProfile.status != DesktopPairingStatus.unpaired;

  Future<void> load() async {
    final records = await _vaultStore.loadRecords();
    final syncQueue = await _vaultStore.loadSyncQueue();
    final syncResolutions = await _vaultStore.loadSyncResolutions();
    final pairingProfile = await _vaultStore.loadPairingProfile();
    _records
      ..clear()
      ..addAll(records);
    _syncQueue
      ..clear()
      ..addAll(syncQueue);
    _syncResolutions
      ..clear()
      ..addAll(syncResolutions);
    _pairingProfile = pairingProfile;
    _desktopSyncEnabled =
        pairingProfile.status != DesktopPairingStatus.unpaired;
    _isLoaded = true;
    notifyListeners();
  }

  VaultRecord addWriteResult({
    required WatermarkWriteResult result,
    required String? fileName,
    required bool allowRewrite,
    String? rewriteReason,
  }) {
    final record = VaultRecord(
      id: _newRecordId(),
      kind: result.kind,
      title: fileName?.isNotEmpty == true
          ? fileName!
          : _fallbackTitle(result.kind),
      watermarkUid: result.watermarkUid,
      revision: result.revision,
      sha256: result.sha256,
      source: VaultRecordSource.write,
      syncStatus: SyncStatus.pending,
      createdAt: DateTime.now(),
      rewriteReason: allowRewrite ? rewriteReason : null,
    );
    _records.insert(0, record);
    final queueItem = _newSyncQueueItem(
      record,
      SyncQueueOperation.upsertVaultRecord,
    );
    _syncQueue.insert(0, queueItem);
    _persistRecordAndQueueItem(record, queueItem);
    notifyListeners();
    return record;
  }

  VaultRecord addReadResult({
    required WatermarkReadResult result,
    required String? fileName,
  }) {
    final record = VaultRecord(
      id: _newRecordId(),
      kind: result.kind,
      title: fileName?.isNotEmpty == true
          ? fileName!
          : _fallbackTitle(result.kind),
      watermarkUid: result.watermarkUid,
      revision: result.revision,
      parentWatermarkUid: result.parentWatermarkUid,
      rewriteReason: result.rewriteReason,
      extractedTimestamp: result.timestamp,
      extractedDeviceIdHex: result.deviceIdHex,
      extractedFileHashHex: result.fileHashHex,
      source: VaultRecordSource.verify,
      syncStatus: SyncStatus.pending,
      createdAt: DateTime.now(),
    );
    _records.insert(0, record);
    final queueItem = _newSyncQueueItem(
      record,
      SyncQueueOperation.upsertEvidenceRecord,
    );
    _syncQueue.insert(0, queueItem);
    _persistRecordAndQueueItem(record, queueItem);
    notifyListeners();
    return record;
  }

  void updateCreatorLabel(String value) {
    final next = value.trim();
    if (next.isEmpty || next == _creatorLabel) {
      return;
    }
    _creatorLabel = next;
    notifyListeners();
  }

  void setDesktopSyncEnabled(bool value) {
    if (value == _desktopSyncEnabled) {
      return;
    }
    _desktopSyncEnabled = value;
    if (!value) {
      _pairingProfile = DesktopPairingProfile.unpaired();
      unawaited(_vaultStore.savePairingProfile(_pairingProfile));
    }
    notifyListeners();
  }

  void setAnonymousFeedbackEnabled(bool value) {
    if (value == _anonymousFeedbackEnabled) {
      return;
    }
    _anonymousFeedbackEnabled = value;
    notifyListeners();
  }

  void setSyncTransportMode(SyncTransportMode mode) {
    if (mode == SyncTransportMode.http && !canUseHttpSync) {
      return;
    }
    if (mode == _syncTransportMode) {
      return;
    }
    _syncTransportMode = mode;
    notifyListeners();
  }

  Future<void> saveDesktopPairing({
    required String desktopAddress,
    required String pairingCode,
  }) async {
    final address = desktopAddress.trim();
    final code = pairingCode.trim();
    if (address.isEmpty || code.isEmpty) {
      _pairingProfile = DesktopPairingProfile.unpaired();
      _desktopSyncEnabled = false;
      _syncTransportMode = SyncTransportMode.mock;
    } else {
      _pairingProfile = DesktopPairingProfile(
        desktopAddress: address,
        pairingCode: code,
        status: DesktopPairingStatus.paired,
        updatedAt: DateTime.now(),
      );
      _desktopSyncEnabled = true;
    }
    await _vaultStore.savePairingProfile(_pairingProfile);
    notifyListeners();
  }

  Future<void> testDesktopConnection() async {
    if (!_pairingProfile.canConnect) {
      return;
    }
    _pairingProfile = _pairingProfile.copyWith(
      status: DesktopPairingStatus.connecting,
      updatedAt: DateTime.now(),
      clearLastError: true,
    );
    notifyListeners();
    await _vaultStore.savePairingProfile(_pairingProfile);

    await Future<void>.delayed(const Duration(milliseconds: 250));
    _pairingProfile = _pairingProfile.copyWith(
      status: DesktopPairingStatus.paired,
      updatedAt: DateTime.now(),
      clearLastError: true,
    );
    await _vaultStore.savePairingProfile(_pairingProfile);
    notifyListeners();
  }

  Future<void> syncPendingQueue() async {
    if (_isSyncing) {
      return;
    }

    final pendingItems = _syncQueue
        .where((item) => item.status == SyncQueueItemStatus.pending)
        .toList(growable: false);
    if (pendingItems.isEmpty) {
      return;
    }

    _isSyncing = true;
    notifyListeners();

    try {
      final syncingItems = <SyncQueueItem>[];
      for (final item in pendingItems) {
        final current = _updateQueueItem(
          item.copyWith(
            status: SyncQueueItemStatus.syncing,
            attempts: item.attempts + 1,
            clearLastError: true,
          ),
        );
        await _vaultStore.updateSyncItem(current);
        syncingItems.add(current);
        notifyListeners();
      }

      final batchResult = await _activeSyncTransport().sendBatch(syncingItems);
      for (final current in syncingItems) {
        final result = batchResult.resultFor(current.id);
        final next = _updateQueueItem(
          current.copyWith(
            status: result.isSuccess
                ? SyncQueueItemStatus.synced
                : SyncQueueItemStatus.failed,
            lastError: result.error,
            clearLastError: result.isSuccess,
          ),
        );
        await _vaultStore.updateSyncItem(next);
        notifyListeners();
      }
    } finally {
      _isSyncing = false;
      notifyListeners();
    }
  }

  Future<void> retryFailedSyncQueue() async {
    if (_isSyncing || failedSyncQueueCount == 0) {
      return;
    }

    final failedItems = _syncQueue
        .where((item) => item.status == SyncQueueItemStatus.failed)
        .toList(growable: false);
    for (final item in failedItems) {
      final next = _updateQueueItem(
        item.copyWith(
          status: SyncQueueItemStatus.pending,
          clearLastError: true,
        ),
      );
      await _vaultStore.updateSyncItem(next);
    }
    notifyListeners();
    await syncPendingQueue();
  }

  Future<void> pullDesktopChanges() async {
    if (_isPullingDesktopChanges || !canUseHttpSync) {
      return;
    }

    _isPullingDesktopChanges = true;
    notifyListeners();

    try {
      final result = await _activeSyncTransport().fetchChanges(
        since: _pairingProfile.lastDesktopPullSince,
      );
      if (!result.isSuccess) {
        _pairingProfile = _pairingProfile.copyWith(
          status: DesktopPairingStatus.failed,
          lastError: result.error,
          updatedAt: DateTime.now(),
        );
        await _vaultStore.savePairingProfile(_pairingProfile);
        return;
      }

      for (final change in result.changes) {
        final record = change.toVaultRecord();
        final mergeResult = _mergeDesktopRecord(record);
        if (mergeResult.record != null) {
          await _vaultStore.upsertRecord(mergeResult.record!);
        }
        await _vaultStore.recordSyncResolution(mergeResult.resolution);
        _syncResolutions.insert(0, mergeResult.resolution);
      }
      if (result.nextSince.isNotEmpty) {
        _pairingProfile = _pairingProfile.copyWith(
          lastDesktopPullSince: result.nextSince,
        );
      }
      _pairingProfile = _pairingProfile.copyWith(
        status: DesktopPairingStatus.paired,
        updatedAt: DateTime.now(),
        clearLastError: true,
      );
      await _vaultStore.savePairingProfile(_pairingProfile);
    } finally {
      _isPullingDesktopChanges = false;
      notifyListeners();
    }
  }

  String _newRecordId() => DateTime.now().microsecondsSinceEpoch.toString();

  SyncQueueItem _newSyncQueueItem(
    VaultRecord record,
    SyncQueueOperation operation,
  ) {
    return SyncQueueItem(
      id: '${record.id}-${operation.name}',
      recordId: record.id,
      operation: operation,
      payloadType: 'vault_record',
      payloadJson: jsonEncode(record.toSyncPayload()),
      status: SyncQueueItemStatus.pending,
      attempts: 0,
      createdAt: DateTime.now(),
    );
  }

  SyncQueueItem _updateQueueItem(SyncQueueItem item) {
    final index = _syncQueue.indexWhere((queued) => queued.id == item.id);
    if (index != -1) {
      _syncQueue[index] = item;
    }
    return item;
  }

  void _persistRecordAndQueueItem(VaultRecord record, SyncQueueItem queueItem) {
    unawaited(_persistRecordAndQueueItemAsync(record, queueItem));
  }

  Future<void> _persistRecordAndQueueItemAsync(
    VaultRecord record,
    SyncQueueItem queueItem,
  ) async {
    try {
      await _vaultStore.upsertRecord(record);
      await _vaultStore.enqueueSyncItem(queueItem);
    } catch (error) {
      debugPrint('Failed to persist vault record sync event: $error');
    }
  }

  String _fallbackTitle(WatermarkAssetKind kind) {
    return switch (kind) {
      WatermarkAssetKind.image => '未命名图片',
      WatermarkAssetKind.audio => '未命名 WAV',
      WatermarkAssetKind.video => '未命名视频',
    };
  }

  SyncTransport _activeSyncTransport() {
    if (_transportOverride != null) {
      return _transportOverride;
    }
    return _syncTransportFactory(_syncTransportMode, _pairingProfile);
  }

  _DesktopMergeResult _mergeDesktopRecord(VaultRecord incoming) {
    final exactMatchIndex = _records.indexWhere(
      (item) => item.id == incoming.id,
    );
    if (exactMatchIndex != -1) {
      final current = _records[exactMatchIndex];
      _records[exactMatchIndex] = incoming;
      return _DesktopMergeResult(
        record: incoming,
        resolution: MobileSyncResolution(
          id: _newResolutionId(incoming, 'record-replaced'),
          resolvedAt: DateTime.now(),
          resolutionType: MobileSyncResolutionType.recordReplaced,
          reason: 'desktop record refreshed by stable id',
          incomingRecordId: incoming.id,
          existingRecordId: current.id,
          watermarkUid: incoming.watermarkUid,
          existingHash: _recordFingerprint(current),
          incomingHash: _recordFingerprint(incoming),
          existingRevision: current.revision,
          incomingRevision: incoming.revision,
          insertedRecordId: incoming.id,
        ),
      );
    }

    final incomingFingerprint = _recordFingerprint(incoming);
    final sameUidMatches = _records
        .where((item) => item.watermarkUid == incoming.watermarkUid)
        .toList(growable: false);
    final sameFingerprintMatches = incomingFingerprint == null
        ? const <VaultRecord>[]
        : sameUidMatches
              .where((item) => _recordFingerprint(item) == incomingFingerprint)
              .toList(growable: false);

    if (sameFingerprintMatches.isNotEmpty) {
      final current = sameFingerprintMatches.reduce(
        (a, b) => a.revision >= b.revision ? a : b,
      );
      if (incoming.revision > current.revision) {
        final updated = current.copyWith(
          kind: incoming.kind,
          title: incoming.title,
          watermarkUid: incoming.watermarkUid,
          revision: incoming.revision,
          sha256: incoming.sha256,
          parentWatermarkUid: incoming.parentWatermarkUid,
          rewriteReason: incoming.rewriteReason,
          extractedTimestamp: incoming.extractedTimestamp,
          extractedDeviceIdHex: incoming.extractedDeviceIdHex,
          extractedFileHashHex: incoming.extractedFileHashHex,
          source: incoming.source,
          syncStatus: incoming.syncStatus,
          createdAt: incoming.createdAt,
        );
        final index = _records.indexOf(current);
        if (index != -1) {
          _records[index] = updated;
        }
        return _DesktopMergeResult(
          record: updated,
          resolution: MobileSyncResolution(
            id: _newResolutionId(incoming, 'revision-upgraded'),
            resolvedAt: DateTime.now(),
            resolutionType: MobileSyncResolutionType.revisionUpgraded,
            reason: 'higher revision replaced existing same-hash record',
            incomingRecordId: incoming.id,
            existingRecordId: current.id,
            watermarkUid: incoming.watermarkUid,
            existingHash: _recordFingerprint(current),
            incomingHash: incomingFingerprint,
            existingRevision: current.revision,
            incomingRevision: incoming.revision,
            insertedRecordId: updated.id,
          ),
        );
      }
      if (incoming.revision < current.revision) {
        return _DesktopMergeResult(
          record: null,
          resolution: MobileSyncResolution(
            id: _newResolutionId(incoming, 'stale-ignored'),
            resolvedAt: DateTime.now(),
            resolutionType: MobileSyncResolutionType.staleRevisionIgnored,
            reason: 'incoming revision is older than local record',
            incomingRecordId: incoming.id,
            existingRecordId: current.id,
            watermarkUid: incoming.watermarkUid,
            existingHash: _recordFingerprint(current),
            incomingHash: incomingFingerprint,
            existingRevision: current.revision,
            incomingRevision: incoming.revision,
          ),
        );
      }
      return _DesktopMergeResult(
        record: null,
        resolution: MobileSyncResolution(
          id: _newResolutionId(incoming, 'duplicate-ignored'),
          resolvedAt: DateTime.now(),
          resolutionType: MobileSyncResolutionType.duplicateIgnored,
          reason: 'same uid, hash and revision already exist locally',
          incomingRecordId: incoming.id,
          existingRecordId: current.id,
          watermarkUid: incoming.watermarkUid,
          existingHash: _recordFingerprint(current),
          incomingHash: incomingFingerprint,
          existingRevision: current.revision,
          incomingRevision: incoming.revision,
        ),
      );
    }

    if (sameUidMatches.isNotEmpty) {
      final current = sameUidMatches.reduce(
        (a, b) => a.revision >= b.revision ? a : b,
      );
      _records.insert(0, incoming);
      return _DesktopMergeResult(
        record: incoming,
        resolution: MobileSyncResolution(
          id: _newResolutionId(incoming, 'variant-accepted'),
          resolvedAt: DateTime.now(),
          resolutionType: MobileSyncResolutionType.variantAccepted,
          reason: 'same watermark uid but different asset fingerprint',
          incomingRecordId: incoming.id,
          existingRecordId: current.id,
          watermarkUid: incoming.watermarkUid,
          existingHash: _recordFingerprint(current),
          incomingHash: incomingFingerprint,
          existingRevision: current.revision,
          incomingRevision: incoming.revision,
          insertedRecordId: incoming.id,
        ),
      );
    }

    _records.insert(0, incoming);
    return _DesktopMergeResult(
      record: incoming,
      resolution: MobileSyncResolution(
        id: _newResolutionId(incoming, 'record-inserted'),
        resolvedAt: DateTime.now(),
        resolutionType: MobileSyncResolutionType.recordInserted,
        reason: 'desktop record added to local vault',
        incomingRecordId: incoming.id,
        watermarkUid: incoming.watermarkUid,
        incomingHash: incomingFingerprint,
        incomingRevision: incoming.revision,
        insertedRecordId: incoming.id,
      ),
    );
  }

  String _newResolutionId(VaultRecord record, String suffix) {
    return '${record.id}-$suffix-${DateTime.now().microsecondsSinceEpoch}';
  }

  String? _recordFingerprint(VaultRecord record) {
    if (record.sha256?.isNotEmpty == true) {
      return record.sha256;
    }
    if (record.extractedFileHashHex?.isNotEmpty == true) {
      return record.extractedFileHashHex;
    }
    return null;
  }
}

extension on DesktopSyncChange {
  VaultRecord toVaultRecord() {
    return VaultRecord(
      id: id.startsWith('desktop:') ? id : 'desktop:$id',
      kind: switch (kind) {
        'audio' => WatermarkAssetKind.audio,
        'video' => WatermarkAssetKind.video,
        _ => WatermarkAssetKind.image,
      },
      title: title,
      watermarkUid: watermarkUid,
      revision: revision,
      sha256: sha256,
      parentWatermarkUid: parentWatermarkUid,
      rewriteReason: rewriteReason,
      extractedTimestamp: extractedTimestamp,
      extractedDeviceIdHex: extractedDeviceIdHex,
      extractedFileHashHex: extractedFileHashHex,
      source: source == 'verify'
          ? VaultRecordSource.verify
          : VaultRecordSource.write,
      syncStatus: SyncStatus.synced,
      createdAt: DateTime.tryParse(createdAt) ?? DateTime.now(),
    );
  }
}

class VaultRecord {
  const VaultRecord({
    required this.id,
    required this.kind,
    required this.title,
    required this.watermarkUid,
    required this.revision,
    required this.source,
    required this.syncStatus,
    required this.createdAt,
    this.sha256,
    this.parentWatermarkUid,
    this.rewriteReason,
    this.extractedTimestamp,
    this.extractedDeviceIdHex,
    this.extractedFileHashHex,
  });

  final String id;
  final WatermarkAssetKind kind;
  final String title;
  final String watermarkUid;
  final int revision;
  final String? sha256;
  final String? parentWatermarkUid;
  final String? rewriteReason;
  final int? extractedTimestamp;
  final String? extractedDeviceIdHex;
  final String? extractedFileHashHex;
  final VaultRecordSource source;
  final SyncStatus syncStatus;
  final DateTime createdAt;

  VaultRecord copyWith({
    String? id,
    WatermarkAssetKind? kind,
    String? title,
    String? watermarkUid,
    int? revision,
    String? sha256,
    String? parentWatermarkUid,
    String? rewriteReason,
    int? extractedTimestamp,
    String? extractedDeviceIdHex,
    String? extractedFileHashHex,
    VaultRecordSource? source,
    SyncStatus? syncStatus,
    DateTime? createdAt,
  }) {
    return VaultRecord(
      id: id ?? this.id,
      kind: kind ?? this.kind,
      title: title ?? this.title,
      watermarkUid: watermarkUid ?? this.watermarkUid,
      revision: revision ?? this.revision,
      sha256: sha256 ?? this.sha256,
      parentWatermarkUid: parentWatermarkUid ?? this.parentWatermarkUid,
      rewriteReason: rewriteReason ?? this.rewriteReason,
      extractedTimestamp: extractedTimestamp ?? this.extractedTimestamp,
      extractedDeviceIdHex: extractedDeviceIdHex ?? this.extractedDeviceIdHex,
      extractedFileHashHex: extractedFileHashHex ?? this.extractedFileHashHex,
      source: source ?? this.source,
      syncStatus: syncStatus ?? this.syncStatus,
      createdAt: createdAt ?? this.createdAt,
    );
  }

  Map<String, Object?> toSyncPayload() {
    return {
      'id': id,
      'kind': kind.name,
      'title': title,
      'watermark_uid': watermarkUid,
      'revision': revision,
      'sha256': sha256,
      'parent_watermark_uid': parentWatermarkUid,
      'rewrite_reason': rewriteReason,
      'extracted_timestamp': extractedTimestamp,
      'extracted_device_id_hex': extractedDeviceIdHex,
      'extracted_file_hash_hex': extractedFileHashHex,
      'source': source.name,
      'sync_status': syncStatus.name,
      'created_at': createdAt.toIso8601String(),
    };
  }
}

enum VaultRecordSource { write, verify }

enum SyncStatus { pending, synced, localOnly, conflict }

class MobileSyncResolution {
  const MobileSyncResolution({
    required this.id,
    required this.resolvedAt,
    required this.resolutionType,
    required this.reason,
    required this.incomingRecordId,
    required this.watermarkUid,
    required this.incomingRevision,
    this.existingRecordId,
    this.existingHash,
    this.incomingHash,
    this.existingRevision,
    this.insertedRecordId,
  });

  final String id;
  final DateTime resolvedAt;
  final MobileSyncResolutionType resolutionType;
  final String reason;
  final String incomingRecordId;
  final String? existingRecordId;
  final String watermarkUid;
  final String? existingHash;
  final String? incomingHash;
  final int? existingRevision;
  final int incomingRevision;
  final String? insertedRecordId;
}

enum MobileSyncResolutionType {
  recordInserted,
  recordReplaced,
  duplicateIgnored,
  variantAccepted,
  revisionUpgraded,
  staleRevisionIgnored,
}

class _DesktopMergeResult {
  const _DesktopMergeResult({required this.record, required this.resolution});

  final VaultRecord? record;
  final MobileSyncResolution resolution;
}

class SyncQueueItem {
  const SyncQueueItem({
    required this.id,
    required this.recordId,
    required this.operation,
    required this.payloadType,
    required this.payloadJson,
    required this.status,
    required this.attempts,
    required this.createdAt,
    this.lastError,
  });

  final String id;
  final String recordId;
  final SyncQueueOperation operation;
  final String payloadType;
  final String payloadJson;
  final SyncQueueItemStatus status;
  final int attempts;
  final DateTime createdAt;
  final String? lastError;

  SyncQueueItem copyWith({
    SyncQueueItemStatus? status,
    int? attempts,
    String? lastError,
    bool clearLastError = false,
  }) {
    return SyncQueueItem(
      id: id,
      recordId: recordId,
      operation: operation,
      payloadType: payloadType,
      payloadJson: payloadJson,
      status: status ?? this.status,
      attempts: attempts ?? this.attempts,
      createdAt: createdAt,
      lastError: clearLastError ? null : lastError ?? this.lastError,
    );
  }
}

enum SyncQueueOperation { upsertVaultRecord, upsertEvidenceRecord }

enum SyncQueueItemStatus { pending, syncing, synced, failed }

class DesktopPairingProfile {
  const DesktopPairingProfile({
    required this.desktopAddress,
    required this.pairingCode,
    required this.status,
    required this.updatedAt,
    this.lastError,
    this.lastDesktopPullSince,
  });

  factory DesktopPairingProfile.unpaired() {
    return DesktopPairingProfile(
      desktopAddress: '',
      pairingCode: '',
      status: DesktopPairingStatus.unpaired,
      updatedAt: DateTime.fromMillisecondsSinceEpoch(0),
    );
  }

  final String desktopAddress;
  final String pairingCode;
  final DesktopPairingStatus status;
  final DateTime updatedAt;
  final String? lastError;
  final String? lastDesktopPullSince;

  bool get canConnect =>
      desktopAddress.isNotEmpty &&
      pairingCode.isNotEmpty &&
      status != DesktopPairingStatus.connecting;

  DesktopPairingProfile copyWith({
    String? desktopAddress,
    String? pairingCode,
    DesktopPairingStatus? status,
    DateTime? updatedAt,
    String? lastError,
    String? lastDesktopPullSince,
    bool clearLastError = false,
  }) {
    return DesktopPairingProfile(
      desktopAddress: desktopAddress ?? this.desktopAddress,
      pairingCode: pairingCode ?? this.pairingCode,
      status: status ?? this.status,
      updatedAt: updatedAt ?? this.updatedAt,
      lastError: clearLastError ? null : lastError ?? this.lastError,
      lastDesktopPullSince: lastDesktopPullSince ?? this.lastDesktopPullSince,
    );
  }
}

enum DesktopPairingStatus { unpaired, paired, connecting, failed }

enum SyncTransportMode { mock, http }

String vaultRecordSourceLabel(VaultRecordSource source) {
  return switch (source) {
    VaultRecordSource.write => '写入',
    VaultRecordSource.verify => '取证',
  };
}

String syncStatusLabel(SyncStatus status) {
  return switch (status) {
    SyncStatus.pending => '待同步',
    SyncStatus.synced => '已同步',
    SyncStatus.localOnly => '仅本机',
    SyncStatus.conflict => '冲突',
  };
}

String syncQueueOperationLabel(SyncQueueOperation operation) {
  return switch (operation) {
    SyncQueueOperation.upsertVaultRecord => '版权记录',
    SyncQueueOperation.upsertEvidenceRecord => '取证记录',
  };
}

String mobileSyncResolutionTypeLabel(MobileSyncResolutionType type) {
  return switch (type) {
    MobileSyncResolutionType.recordInserted => '新增记录',
    MobileSyncResolutionType.recordReplaced => '刷新记录',
    MobileSyncResolutionType.duplicateIgnored => '忽略重复',
    MobileSyncResolutionType.variantAccepted => '接收变体',
    MobileSyncResolutionType.revisionUpgraded => '升级版本',
    MobileSyncResolutionType.staleRevisionIgnored => '忽略旧版本',
  };
}

String desktopPairingStatusLabel(DesktopPairingStatus status) {
  return switch (status) {
    DesktopPairingStatus.unpaired => '未配对',
    DesktopPairingStatus.paired => '已配对',
    DesktopPairingStatus.connecting => '连接中',
    DesktopPairingStatus.failed => '连接失败',
  };
}

String syncTransportModeLabel(SyncTransportMode mode) {
  return switch (mode) {
    SyncTransportMode.mock => '本地模拟',
    SyncTransportMode.http => '桌面 HTTP',
  };
}

typedef SyncTransportFactory =
    SyncTransport Function(
      SyncTransportMode mode,
      DesktopPairingProfile pairingProfile,
    );

SyncTransport _defaultSyncTransportFactory(
  SyncTransportMode mode,
  DesktopPairingProfile pairingProfile,
) {
  return switch (mode) {
    SyncTransportMode.mock => const LocalMockSyncTransport(),
    SyncTransportMode.http => DesktopHttpSyncTransport(
      desktopAddress: pairingProfile.desktopAddress,
      pairingCode: pairingProfile.pairingCode,
    ),
  };
}

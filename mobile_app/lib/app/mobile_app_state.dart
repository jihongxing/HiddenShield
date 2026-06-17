import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';

import '../bridge/watermark_models.dart';
import '../sync/cloud_account_client.dart';
import '../storage/vault_store.dart';
import '../sync/sync_transport.dart';
import 'system_config.dart';

class MobileAppState extends ChangeNotifier {
  MobileAppState({
    VaultStore? vaultStore,
    SyncTransport? syncTransport,
    SyncTransportFactory? syncTransportFactory,
    CloudAccountClient? cloudAccountClient,
  }) : _vaultStore = vaultStore ?? MemoryVaultStore(),
       _syncTransportFactory =
           syncTransportFactory ?? _defaultSyncTransportFactory,
       _transportOverride = syncTransport,
       _cloudAccountClient = cloudAccountClient;

  static const int syncQueueMaxAttempts = 5;
  static const List<Duration> _syncQueueRetryBackoff = [
    Duration(minutes: 1),
    Duration(minutes: 5),
    Duration(minutes: 15),
    Duration(hours: 1),
  ];

  final VaultStore _vaultStore;
  final SyncTransportFactory _syncTransportFactory;
  final SyncTransport? _transportOverride;
  final CloudAccountClient? _cloudAccountClient;
  final List<VaultRecord> _records = [];
  final List<SyncQueueItem> _syncQueue = [];
  final List<MobileSyncResolution> _syncResolutions = [];

  String _creatorLabel = '本机创作者';
  bool _anonymousFeedbackEnabled = false;
  SyncProfile _syncProfile = SyncProfile.localOnly();
  SyncTransportMode _syncTransportMode = SyncTransportMode.localOnly;
  bool _isLoaded = false;
  bool _isSyncing = false;
  bool _isPullingRemoteChanges = false;

  bool get isLoaded => _isLoaded;

  bool get isSyncing => _isSyncing;

  bool get isPullingRemoteChanges => _isPullingRemoteChanges;

  String get creatorLabel => _creatorLabel;

  bool get cloudSyncEnabled => _syncTransportMode == SyncTransportMode.cloud;

  bool get anonymousFeedbackEnabled => _anonymousFeedbackEnabled;

  SyncProfile get syncProfile => _syncProfile;

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

  int get retryExhaustedSyncQueueCount => _syncQueue
      .where(
        (item) =>
            item.status == SyncQueueItemStatus.failed &&
            item.attempts >= syncQueueMaxAttempts,
      )
      .length;

  int get readySyncQueueCount {
    final now = DateTime.now();
    return _syncQueue
        .where((item) => _canSyncQueueItem(item, now, manualRetry: false))
        .length;
  }

  DateTime? get nextSyncQueueRetryAt {
    final retryTimes = _syncQueue
        .where(
          (item) =>
              item.status == SyncQueueItemStatus.failed &&
              item.attempts < syncQueueMaxAttempts,
        )
        .map((item) => item.nextRetryAt)
        .whereType<DateTime>()
        .toList(growable: false);
    if (retryTimes.isEmpty) {
      return null;
    }
    retryTimes.sort();
    return retryTimes.first;
  }

  bool get canUseLanDebugSync =>
      _syncProfile.lanDebugAddress.isNotEmpty &&
      _syncProfile.lanDebugPairingCode.isNotEmpty &&
      _syncProfile.status != SyncConnectionStatus.unconfigured;

  bool get hasCloudAccount =>
      _syncProfile.accountId?.isNotEmpty == true &&
      _syncProfile.authToken?.isNotEmpty == true;

  bool get canUseCloudSync =>
      hasCloudAccount && _syncProfile.cloudBaseUrl.isNotEmpty;

  Future<void> load() async {
    final records = await _vaultStore.loadRecords();
    final syncQueue = await _vaultStore.loadSyncQueue();
    final syncResolutions = await _vaultStore.loadSyncResolutions();
    final syncProfile = await _vaultStore.loadSyncProfile();
    _records
      ..clear()
      ..addAll(records);
    _syncQueue
      ..clear()
      ..addAll(syncQueue);
    _syncResolutions
      ..clear()
      ..addAll(syncResolutions);
    _syncProfile = syncProfile;
    _syncTransportMode = syncProfile.mode;
    _isLoaded = true;
    notifyListeners();
  }

  VaultRecord addWriteResult({
    required WatermarkWriteResult result,
    required String? fileName,
    required bool allowRewrite,
    String? rewriteReason,
    String? parentWatermarkUid,
    int? revision,
  }) {
    final record = VaultRecord(
      id: _newRecordId(),
      kind: result.kind,
      title: fileName?.isNotEmpty == true
          ? fileName!
          : _fallbackTitle(result.kind),
      watermarkUid: result.watermarkUid,
      revision: revision ?? result.revision,
      sha256: result.sha256,
      parentWatermarkUid: allowRewrite ? parentWatermarkUid : null,
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
    if (canUseCloudSync) {
      _syncProfile = _syncProfile.copyWith(
        creatorDisplayName: next,
        creatorProfileSynced: false,
        updatedAt: DateTime.now(),
      );
      unawaited(_vaultStore.saveSyncProfile(_syncProfile));
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
    if (mode == SyncTransportMode.cloud && !canUseCloudSync) {
      return;
    }
    if (mode == SyncTransportMode.lanDebug && !canUseLanDebugSync) {
      return;
    }
    if (mode == _syncTransportMode) {
      return;
    }
    _syncTransportMode = mode;
    _syncProfile = _syncProfile.copyWith(mode: mode);
    unawaited(_vaultStore.saveSyncProfile(_syncProfile));
    notifyListeners();
  }

  void setCloudSyncEnabled(bool value) {
    setSyncTransportMode(
      value ? SyncTransportMode.cloud : SyncTransportMode.localOnly,
    );
  }

  Future<void> continueWithAccountPlaceholder({
    required String accountLabel,
  }) async {
    if (_cloudAccountClient != null) {
      try {
        await continueWithCloudAccount(
          identifier: accountLabel,
          verificationCode: '',
          localCreatorDisplayName: _creatorLabel,
        );
        return;
      } catch (error) {
        _syncProfile = _syncProfile.copyWith(
          status: SyncConnectionStatus.failed,
          lastError: '$error',
          updatedAt: DateTime.now(),
        );
        await _vaultStore.saveSyncProfile(_syncProfile);
        notifyListeners();
        return;
      }
    }

    final label = accountLabel.trim().isEmpty
        ? 'HiddenShield 账户'
        : accountLabel.trim();
    final suffix = _stableIdSuffix(label);
    final now = DateTime.now();
    _syncProfile = _syncProfile.copyWith(
      mode: SyncTransportMode.cloud,
      status: SyncConnectionStatus.connected,
      accountId: 'acct_$suffix',
      accountLabel: label,
      authToken: 'preview-token-$suffix',
      refreshToken: 'preview-refresh-$suffix',
      workspaceId: 'ws_$suffix',
      workspaceName: '个人空间',
      deviceId: _syncProfile.deviceId ?? 'dev_${now.microsecondsSinceEpoch}',
      deviceName: _syncProfile.deviceName ?? '当前移动设备',
      devicePlatform: _syncProfile.devicePlatform ?? _currentDevicePlatform(),
      deviceRegistered: true,
      creatorProfileId: _syncProfile.creatorProfileId ?? 'creator_$suffix',
      creatorDisplayName: _creatorLabel,
      creatorSeedRef: _syncProfile.creatorSeedRef ?? 'local-seed-ref',
      creatorSeedEnvelopeVersion: 1,
      creatorProfileSynced: true,
      entitlementId: 'ent_$suffix',
      entitlementLabel: '免费版',
      entitlementStatus: EntitlementStatus.free,
      entitlementPlanCode: 'free',
      entitlementFeatures: const {
        'batch_processing': false,
        'cloud_video_processing': false,
        'cloud_sync': true,
      },
      entitlementLastCheckedAt: now,
      cloudBaseUrl:
          _cloudAccountClient?.baseUrl ??
          HiddenShieldSystemConfig.fallback.cloudBaseUrl,
      updatedAt: now,
      clearLastError: true,
    );
    _syncTransportMode = SyncTransportMode.cloud;
    await _vaultStore.saveSyncProfile(_syncProfile);
    notifyListeners();
  }

  Future<void> continueWithCloudAccount({
    required String identifier,
    required String verificationCode,
    required String localCreatorDisplayName,
  }) async {
    if (_cloudAccountClient == null) {
      await continueWithAccountPlaceholder(accountLabel: identifier);
      return;
    }

    final session = await _cloudAccountClient.continueWithAccount(
      ContinueAccountRequest(
        identifier: identifier.trim(),
        verificationCode: verificationCode.trim(),
        device: ContinueAccountDevice(
          clientDeviceId:
              _syncProfile.deviceId ??
              'dev_${DateTime.now().microsecondsSinceEpoch}',
          name: _syncProfile.deviceName ?? '当前移动设备',
          platform: _syncProfile.devicePlatform ?? _currentDevicePlatform(),
          appVersion: 'mobile-preview',
        ),
        localCreatorProfile: ContinueAccountCreatorProfile(
          displayName: localCreatorDisplayName.trim().isEmpty
              ? _creatorLabel
              : localCreatorDisplayName.trim(),
          creatorSeedRef: _syncProfile.creatorSeedRef ?? 'local-seed-ref',
          seedEnvelopeVersion: _syncProfile.creatorSeedEnvelopeVersion == 0
              ? 1
              : _syncProfile.creatorSeedEnvelopeVersion,
        ),
      ),
    );

    _syncProfile = session.applyTo(_syncProfile, now: DateTime.now());
    _syncProfile = _syncProfile.copyWith(
      cloudBaseUrl: _cloudAccountClient.baseUrl,
    );
    _syncTransportMode = SyncTransportMode.cloud;
    await _vaultStore.saveSyncProfile(_syncProfile);
    notifyListeners();
  }

  Future<void> signOutCloud() async {
    _syncProfile = _syncProfile.copyWith(
      mode: SyncTransportMode.localOnly,
      status: SyncConnectionStatus.unconfigured,
      clearAccount: true,
      clearAuthToken: true,
      clearWorkspace: true,
      clearCreatorProfile: true,
      clearEntitlement: true,
      updatedAt: DateTime.now(),
    );
    _syncTransportMode = SyncTransportMode.localOnly;
    await _vaultStore.saveSyncProfile(_syncProfile);
    notifyListeners();
  }

  Future<void> saveLanDebugPairing({
    required String lanDebugAddress,
    required String pairingCode,
  }) async {
    final address = lanDebugAddress.trim();
    final code = pairingCode.trim();
    if (address.isEmpty || code.isEmpty) {
      _syncProfile = _syncProfile.copyWith(
        mode: SyncTransportMode.localOnly,
        status: SyncConnectionStatus.unconfigured,
        lanDebugAddress: '',
        lanDebugPairingCode: '',
        updatedAt: DateTime.now(),
      );
      _syncTransportMode = SyncTransportMode.localOnly;
    } else {
      _syncProfile = _syncProfile.copyWith(
        mode: SyncTransportMode.lanDebug,
        lanDebugAddress: address,
        lanDebugPairingCode: code,
        status: SyncConnectionStatus.connected,
        updatedAt: DateTime.now(),
        clearLastError: true,
      );
      _syncTransportMode = SyncTransportMode.lanDebug;
    }
    await _vaultStore.saveSyncProfile(_syncProfile);
    notifyListeners();
  }

  Future<void> testLanDebugConnection() async {
    if (!_syncProfile.canConnectLanDebug) {
      return;
    }
    _syncProfile = _syncProfile.copyWith(
      status: SyncConnectionStatus.connecting,
      updatedAt: DateTime.now(),
      clearLastError: true,
    );
    notifyListeners();
    await _vaultStore.saveSyncProfile(_syncProfile);

    await Future<void>.delayed(const Duration(milliseconds: 250));
    _syncProfile = _syncProfile.copyWith(
      status: SyncConnectionStatus.connected,
      updatedAt: DateTime.now(),
      clearLastError: true,
    );
    await _vaultStore.saveSyncProfile(_syncProfile);
    notifyListeners();
  }

  Future<void> syncPendingQueue() => _syncPendingQueue();

  Future<void> _syncPendingQueue({bool manualRetry = false}) async {
    if (_isSyncing) {
      return;
    }

    final now = DateTime.now();
    final pendingItems = _syncQueue
        .where((item) => _canSyncQueueItem(item, now, manualRetry: manualRetry))
        .toList(growable: false);
    if (pendingItems.isEmpty) {
      return;
    }

    _isSyncing = true;
    notifyListeners();

    try {
      final attemptAt = DateTime.now();
      _syncProfile = _syncProfile.copyWith(
        lastSyncAttemptAt: attemptAt,
        updatedAt: attemptAt,
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
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
      var hasFailure = false;
      String? latestError;
      for (final current in syncingItems) {
        final result = batchResult.resultFor(current.id);
        if (!result.isSuccess) {
          hasFailure = true;
          latestError = result.error;
        }
        final next = _updateQueueItem(
          current.copyWith(
            status: result.isSuccess
                ? SyncQueueItemStatus.synced
                : SyncQueueItemStatus.failed,
            lastError: result.error,
            clearLastError: result.isSuccess,
            nextRetryAt: result.isSuccess
                ? null
                : _nextSyncQueueRetryAt(current.attempts, DateTime.now()),
            clearNextRetryAt: result.isSuccess,
          ),
        );
        await _vaultStore.updateSyncItem(next);
        notifyListeners();
      }
      final completedAt = DateTime.now();
      _syncProfile = _syncProfile.copyWith(
        status: hasFailure
            ? SyncConnectionStatus.failed
            : SyncConnectionStatus.connected,
        lastSyncSuccessAt: hasFailure ? null : completedAt,
        lastSyncFailureAt: hasFailure ? completedAt : null,
        lastError: latestError,
        updatedAt: completedAt,
        clearLastError: !hasFailure,
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
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
          clearNextRetryAt: true,
        ),
      );
      await _vaultStore.updateSyncItem(next);
    }
    notifyListeners();
    await _syncPendingQueue(manualRetry: true);
  }

  Future<void> pullRemoteChanges() async {
    if (_isPullingRemoteChanges ||
        _syncTransportMode == SyncTransportMode.localOnly) {
      return;
    }

    _isPullingRemoteChanges = true;
    notifyListeners();

    try {
      final attemptAt = DateTime.now();
      _syncProfile = _syncProfile.copyWith(
        lastSyncAttemptAt: attemptAt,
        updatedAt: attemptAt,
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
      final result = await _activeSyncTransport().fetchChanges(
        since: _syncProfile.lastRemotePullCursor,
      );
      final completedAt = DateTime.now();
      if (!result.isSuccess) {
        _syncProfile = _syncProfile.copyWith(
          status: SyncConnectionStatus.failed,
          lastError: result.error,
          lastSyncFailureAt: completedAt,
          updatedAt: completedAt,
        );
        await _vaultStore.saveSyncProfile(_syncProfile);
        return;
      }

      for (final change in result.changes) {
        final record = change.toVaultRecord();
        final mergeResult = _mergeRemoteRecord(record);
        if (mergeResult.record != null) {
          await _vaultStore.upsertRecord(mergeResult.record!);
        }
        await _vaultStore.recordSyncResolution(mergeResult.resolution);
        _syncResolutions.insert(0, mergeResult.resolution);
      }
      if (result.nextSince.isNotEmpty) {
        _syncProfile = _syncProfile.copyWith(
          lastRemotePullCursor: result.nextSince,
        );
      }
      _syncProfile = _syncProfile.copyWith(
        status: SyncConnectionStatus.connected,
        lastSyncSuccessAt: completedAt,
        updatedAt: completedAt,
        clearLastError: true,
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
    } finally {
      _isPullingRemoteChanges = false;
      notifyListeners();
    }
  }

  String _newRecordId() => DateTime.now().microsecondsSinceEpoch.toString();

  String _stableIdSuffix(String value) {
    final source = value.trim().toLowerCase();
    final encoded = base64Url.encode(utf8.encode(source)).replaceAll('=', '');
    if (encoded.isEmpty) {
      return 'preview';
    }
    return encoded.length > 18 ? encoded.substring(0, 18) : encoded;
  }

  String _currentDevicePlatform() {
    if (kIsWeb) {
      return 'web';
    }
    return defaultTargetPlatform.name;
  }

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

  bool _canSyncQueueItem(
    SyncQueueItem item,
    DateTime now, {
    required bool manualRetry,
  }) {
    if (item.status == SyncQueueItemStatus.pending) {
      return true;
    }
    if (item.status != SyncQueueItemStatus.failed) {
      return false;
    }
    if (manualRetry) {
      return true;
    }
    if (item.attempts >= syncQueueMaxAttempts) {
      return false;
    }
    final nextRetryAt = item.nextRetryAt;
    return nextRetryAt == null || !nextRetryAt.isAfter(now);
  }

  DateTime? _nextSyncQueueRetryAt(int attempts, DateTime failedAt) {
    if (attempts >= syncQueueMaxAttempts) {
      return null;
    }
    final index = (attempts - 1)
        .clamp(0, _syncQueueRetryBackoff.length - 1)
        .toInt();
    return failedAt.add(_syncQueueRetryBackoff[index]);
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
    return _syncTransportFactory(_syncTransportMode, _syncProfile);
  }

  _RemoteMergeResult _mergeRemoteRecord(VaultRecord incoming) {
    final exactMatchIndex = _records.indexWhere(
      (item) => item.id == incoming.id,
    );
    if (exactMatchIndex != -1) {
      final current = _records[exactMatchIndex];
      _records[exactMatchIndex] = incoming;
      return _RemoteMergeResult(
        record: incoming,
        resolution: MobileSyncResolution(
          id: _newResolutionId(incoming, 'record-replaced'),
          resolvedAt: DateTime.now(),
          resolutionType: MobileSyncResolutionType.recordReplaced,
          reason: 'remote record refreshed by stable id',
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
        return _RemoteMergeResult(
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
        return _RemoteMergeResult(
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
      return _RemoteMergeResult(
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
      return _RemoteMergeResult(
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
    return _RemoteMergeResult(
      record: incoming,
      resolution: MobileSyncResolution(
        id: _newResolutionId(incoming, 'record-inserted'),
        resolvedAt: DateTime.now(),
        resolutionType: MobileSyncResolutionType.recordInserted,
        reason: 'remote record added to local vault',
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

extension on RemoteSyncChange {
  VaultRecord toVaultRecord() {
    final prefix = sourceDevice == 'lanDebug' ? 'lan:' : 'remote:';
    return VaultRecord(
      id: id.contains(':') ? id : '$prefix$id',
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

class _RemoteMergeResult {
  const _RemoteMergeResult({required this.record, required this.resolution});

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
    this.nextRetryAt,
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
  final DateTime? nextRetryAt;

  SyncQueueItem copyWith({
    SyncQueueItemStatus? status,
    int? attempts,
    String? lastError,
    DateTime? nextRetryAt,
    bool clearLastError = false,
    bool clearNextRetryAt = false,
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
      nextRetryAt: clearNextRetryAt ? null : nextRetryAt ?? this.nextRetryAt,
    );
  }
}

enum SyncQueueOperation { upsertVaultRecord, upsertEvidenceRecord }

enum SyncQueueItemStatus { pending, syncing, synced, failed }

class SyncProfile {
  const SyncProfile({
    required this.mode,
    required this.status,
    required this.updatedAt,
    this.accountId,
    this.accountLabel,
    this.authToken,
    this.refreshToken,
    this.workspaceId,
    this.workspaceName,
    this.deviceId,
    this.deviceName,
    this.devicePlatform,
    this.deviceRegistered = false,
    this.creatorProfileId,
    this.creatorDisplayName,
    this.creatorSeedRef,
    this.creatorSeedEnvelopeVersion = 0,
    this.creatorProfileSynced = false,
    this.entitlementId,
    this.entitlementLabel = '免费版',
    this.entitlementStatus = EntitlementStatus.free,
    this.entitlementPlanCode = 'free',
    this.entitlementFeatures = const {},
    this.entitlementLastCheckedAt,
    this.cloudBaseUrl = '',
    this.lanDebugAddress = '',
    this.lanDebugPairingCode = '',
    this.lastError,
    this.lastRemotePullCursor,
    this.lastSyncAttemptAt,
    this.lastSyncSuccessAt,
    this.lastSyncFailureAt,
  });

  factory SyncProfile.localOnly() {
    return SyncProfile(
      mode: SyncTransportMode.localOnly,
      status: SyncConnectionStatus.unconfigured,
      updatedAt: DateTime.fromMillisecondsSinceEpoch(0),
    );
  }

  final SyncTransportMode mode;
  final SyncConnectionStatus status;
  final DateTime updatedAt;
  final String? accountId;
  final String? accountLabel;
  final String? authToken;
  final String? refreshToken;
  final String? workspaceId;
  final String? workspaceName;
  final String? deviceId;
  final String? deviceName;
  final String? devicePlatform;
  final bool deviceRegistered;
  final String? creatorProfileId;
  final String? creatorDisplayName;
  final String? creatorSeedRef;
  final int creatorSeedEnvelopeVersion;
  final bool creatorProfileSynced;
  final String? entitlementId;
  final String entitlementLabel;
  final EntitlementStatus entitlementStatus;
  final String entitlementPlanCode;
  final Map<String, bool> entitlementFeatures;
  final DateTime? entitlementLastCheckedAt;
  final String cloudBaseUrl;
  final String lanDebugAddress;
  final String lanDebugPairingCode;
  final String? lastError;
  final String? lastRemotePullCursor;
  final DateTime? lastSyncAttemptAt;
  final DateTime? lastSyncSuccessAt;
  final DateTime? lastSyncFailureAt;

  @Deprecated('Use lanDebugAddress')
  String get desktopAddress => lanDebugAddress;

  @Deprecated('Use lanDebugPairingCode')
  String get pairingCode => lanDebugPairingCode;

  @Deprecated('Use lastRemotePullCursor')
  String? get lastDesktopPullSince => lastRemotePullCursor;

  bool get canConnectLanDebug =>
      lanDebugAddress.isNotEmpty &&
      lanDebugPairingCode.isNotEmpty &&
      status != SyncConnectionStatus.connecting;

  SyncProfile copyWith({
    SyncTransportMode? mode,
    SyncConnectionStatus? status,
    DateTime? updatedAt,
    String? accountId,
    String? accountLabel,
    String? authToken,
    String? refreshToken,
    String? workspaceId,
    String? workspaceName,
    String? deviceId,
    String? deviceName,
    String? devicePlatform,
    bool? deviceRegistered,
    String? creatorProfileId,
    String? creatorDisplayName,
    String? creatorSeedRef,
    int? creatorSeedEnvelopeVersion,
    bool? creatorProfileSynced,
    String? entitlementId,
    String? entitlementLabel,
    EntitlementStatus? entitlementStatus,
    String? entitlementPlanCode,
    Map<String, bool>? entitlementFeatures,
    DateTime? entitlementLastCheckedAt,
    String? cloudBaseUrl,
    String? lanDebugAddress,
    String? lanDebugPairingCode,
    String? lastError,
    String? lastRemotePullCursor,
    DateTime? lastSyncAttemptAt,
    DateTime? lastSyncSuccessAt,
    DateTime? lastSyncFailureAt,
    bool clearLastError = false,
    bool clearAccount = false,
    bool clearAuthToken = false,
    bool clearWorkspace = false,
    bool clearCreatorProfile = false,
    bool clearEntitlement = false,
  }) {
    return SyncProfile(
      mode: mode ?? this.mode,
      status: status ?? this.status,
      updatedAt: updatedAt ?? this.updatedAt,
      accountId: clearAccount ? null : accountId ?? this.accountId,
      accountLabel: clearAccount ? null : accountLabel ?? this.accountLabel,
      authToken: clearAuthToken ? null : authToken ?? this.authToken,
      refreshToken: clearAuthToken ? null : refreshToken ?? this.refreshToken,
      workspaceId: clearWorkspace ? null : workspaceId ?? this.workspaceId,
      workspaceName: clearWorkspace
          ? null
          : workspaceName ?? this.workspaceName,
      deviceId: deviceId ?? this.deviceId,
      deviceName: deviceName ?? this.deviceName,
      devicePlatform: devicePlatform ?? this.devicePlatform,
      deviceRegistered: deviceRegistered ?? this.deviceRegistered,
      creatorProfileId: clearCreatorProfile
          ? null
          : creatorProfileId ?? this.creatorProfileId,
      creatorDisplayName: clearCreatorProfile
          ? null
          : creatorDisplayName ?? this.creatorDisplayName,
      creatorSeedRef: clearCreatorProfile
          ? null
          : creatorSeedRef ?? this.creatorSeedRef,
      creatorSeedEnvelopeVersion: clearCreatorProfile
          ? 0
          : creatorSeedEnvelopeVersion ?? this.creatorSeedEnvelopeVersion,
      creatorProfileSynced: clearCreatorProfile
          ? false
          : creatorProfileSynced ?? this.creatorProfileSynced,
      entitlementId: clearEntitlement
          ? null
          : entitlementId ?? this.entitlementId,
      entitlementLabel: clearEntitlement
          ? '免费版'
          : entitlementLabel ?? this.entitlementLabel,
      entitlementStatus: clearEntitlement
          ? EntitlementStatus.free
          : entitlementStatus ?? this.entitlementStatus,
      entitlementPlanCode: clearEntitlement
          ? 'free'
          : entitlementPlanCode ?? this.entitlementPlanCode,
      entitlementFeatures: clearEntitlement
          ? const {}
          : entitlementFeatures ?? this.entitlementFeatures,
      entitlementLastCheckedAt: clearEntitlement
          ? null
          : entitlementLastCheckedAt ?? this.entitlementLastCheckedAt,
      cloudBaseUrl: cloudBaseUrl ?? this.cloudBaseUrl,
      lanDebugAddress: lanDebugAddress ?? this.lanDebugAddress,
      lanDebugPairingCode: lanDebugPairingCode ?? this.lanDebugPairingCode,
      lastError: clearLastError ? null : lastError ?? this.lastError,
      lastRemotePullCursor: lastRemotePullCursor ?? this.lastRemotePullCursor,
      lastSyncAttemptAt: lastSyncAttemptAt ?? this.lastSyncAttemptAt,
      lastSyncSuccessAt: lastSyncSuccessAt ?? this.lastSyncSuccessAt,
      lastSyncFailureAt: lastSyncFailureAt ?? this.lastSyncFailureAt,
    );
  }
}

enum SyncConnectionStatus { unconfigured, connected, connecting, failed }

enum SyncTransportMode { localOnly, cloud, lanDebug }

enum EntitlementStatus { free, trial, active, grace, expired }

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

String syncConnectionStatusLabel(SyncConnectionStatus status) {
  return switch (status) {
    SyncConnectionStatus.unconfigured => '未配置',
    SyncConnectionStatus.connected => '已连接',
    SyncConnectionStatus.connecting => '连接中',
    SyncConnectionStatus.failed => '连接失败',
  };
}

String syncTransportModeLabel(SyncTransportMode mode) {
  return switch (mode) {
    SyncTransportMode.localOnly => '仅本机',
    SyncTransportMode.cloud => '云同步',
    SyncTransportMode.lanDebug => '局域网调试',
  };
}

String entitlementStatusLabel(EntitlementStatus status) {
  return switch (status) {
    EntitlementStatus.free => '免费版',
    EntitlementStatus.trial => '试用中',
    EntitlementStatus.active => '订阅有效',
    EntitlementStatus.grace => '宽限期',
    EntitlementStatus.expired => '已过期',
  };
}

typedef SyncTransportFactory =
    SyncTransport Function(SyncTransportMode mode, SyncProfile pairingProfile);

SyncTransport _defaultSyncTransportFactory(
  SyncTransportMode mode,
  SyncProfile pairingProfile,
) {
  return switch (mode) {
    SyncTransportMode.localOnly => const LocalOnlySyncTransport(),
    SyncTransportMode.cloud => CloudSyncTransport(
      baseUrl: pairingProfile.cloudBaseUrl,
      authToken: pairingProfile.authToken,
      deviceId: pairingProfile.deviceId,
      workspaceId: pairingProfile.workspaceId,
    ),
    SyncTransportMode.lanDebug => LanDebugSyncTransport(
      lanDebugAddress: pairingProfile.lanDebugAddress,
      pairingCode: pairingProfile.lanDebugPairingCode,
    ),
  };
}

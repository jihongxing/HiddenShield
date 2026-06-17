import 'dart:convert';

import 'package:path/path.dart' as path;
import 'package:sqflite/sqflite.dart';

import '../app/mobile_app_state.dart';
import '../bridge/watermark_models.dart';

abstract class VaultStore {
  Future<List<VaultRecord>> loadRecords();

  Future<List<SyncQueueItem>> loadSyncQueue();

  Future<List<MobileSyncResolution>> loadSyncResolutions();

  Future<SyncProfile> loadSyncProfile();

  Future<void> upsertRecord(VaultRecord record);

  Future<void> enqueueSyncItem(SyncQueueItem item);

  Future<void> updateSyncItem(SyncQueueItem item);

  Future<void> recordSyncResolution(MobileSyncResolution resolution);

  Future<void> saveSyncProfile(SyncProfile profile);

  Future<void> close();
}

class MemoryVaultStore implements VaultStore {
  final List<VaultRecord> _records = [];
  final List<SyncQueueItem> _syncQueue = [];
  final List<MobileSyncResolution> _syncResolutions = [];
  SyncProfile _syncProfile = SyncProfile.localOnly();

  @override
  Future<List<VaultRecord>> loadRecords() async => List.unmodifiable(_records);

  @override
  Future<List<SyncQueueItem>> loadSyncQueue() async =>
      List.unmodifiable(_syncQueue);

  @override
  Future<List<MobileSyncResolution>> loadSyncResolutions() async =>
      List.unmodifiable(_syncResolutions);

  @override
  Future<SyncProfile> loadSyncProfile() async => _syncProfile;

  @override
  Future<void> upsertRecord(VaultRecord record) async {
    final existingIndex = _records.indexWhere((item) => item.id == record.id);
    if (existingIndex == -1) {
      _records.insert(0, record);
    } else {
      _records[existingIndex] = record;
    }
  }

  @override
  Future<void> enqueueSyncItem(SyncQueueItem item) async {
    final existingIndex = _syncQueue.indexWhere(
      (queued) => queued.id == item.id,
    );
    if (existingIndex == -1) {
      _syncQueue.insert(0, item);
    } else {
      _syncQueue[existingIndex] = item;
    }
  }

  @override
  Future<void> updateSyncItem(SyncQueueItem item) => enqueueSyncItem(item);

  @override
  Future<void> recordSyncResolution(MobileSyncResolution resolution) async {
    final existingIndex = _syncResolutions.indexWhere(
      (item) => item.id == resolution.id,
    );
    if (existingIndex == -1) {
      _syncResolutions.insert(0, resolution);
    } else {
      _syncResolutions[existingIndex] = resolution;
    }
  }

  @override
  Future<void> saveSyncProfile(SyncProfile profile) async {
    _syncProfile = profile;
  }

  @override
  Future<void> close() async {}
}

class SQLiteVaultStore implements VaultStore {
  SQLiteVaultStore._(this._db);

  final Database _db;

  static const _databaseName = 'hidden_shield_mobile.db';
  static const _databaseVersion = 5;
  static const _recordsTable = 'vault_records';
  static const _syncQueueTable = 'sync_queue';
  static const _syncResolutionsTable = 'mobile_sync_resolutions';
  static const _syncProfileTable = 'sync_profile';

  static Future<SQLiteVaultStore> open() async {
    final databasePath = await getDatabasesPath();
    final fullPath = path.join(databasePath, _databaseName);
    final db = await openDatabase(
      fullPath,
      version: _databaseVersion,
      onCreate: (db, version) async {
        await _createVaultRecordsTable(db);
        await _createSyncQueueTable(db);
        await _createSyncResolutionsTable(db);
        await _createSyncProfileTable(db);
      },
      onUpgrade: (db, oldVersion, newVersion) async {
        if (oldVersion < 2) {
          await _createSyncQueueTable(db);
          await _createSyncProfileTable(db);
        }
        if (oldVersion < 3) {
          await _addEvidenceColumns(db);
        }
        if (oldVersion < 4) {
          await _createSyncResolutionsTable(db);
        }
        if (oldVersion < 5) {
          await _addSyncQueueNextRetryAtColumn(db);
        }
      },
    );
    return SQLiteVaultStore._(db);
  }

  static Future<void> _createVaultRecordsTable(Database db) async {
    await db.execute('''
CREATE TABLE $_recordsTable (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  title TEXT NOT NULL,
  watermark_uid TEXT NOT NULL,
  revision INTEGER NOT NULL,
  sha256 TEXT,
  parent_watermark_uid TEXT,
  rewrite_reason TEXT,
  extracted_timestamp INTEGER,
  extracted_device_id_hex TEXT,
  extracted_file_hash_hex TEXT,
  source TEXT NOT NULL,
  sync_status TEXT NOT NULL,
  created_at INTEGER NOT NULL
)
''');
    await db.execute(
      'CREATE INDEX idx_vault_records_created_at '
      'ON $_recordsTable(created_at DESC)',
    );
  }

  static Future<void> _addEvidenceColumns(Database db) async {
    await db.execute(
      'ALTER TABLE $_recordsTable ADD COLUMN extracted_timestamp INTEGER',
    );
    await db.execute(
      'ALTER TABLE $_recordsTable ADD COLUMN extracted_device_id_hex TEXT',
    );
    await db.execute(
      'ALTER TABLE $_recordsTable ADD COLUMN extracted_file_hash_hex TEXT',
    );
  }

  static Future<void> _createSyncQueueTable(Database db) async {
    await db.execute('''
CREATE TABLE $_syncQueueTable (
  id TEXT PRIMARY KEY,
  record_id TEXT NOT NULL,
  operation TEXT NOT NULL,
  payload_type TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  status TEXT NOT NULL,
  attempts INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  last_error TEXT,
  next_retry_at INTEGER
)
''');
    await db.execute(
      'CREATE INDEX idx_sync_queue_status_created_at '
      'ON $_syncQueueTable(status, created_at ASC)',
    );
  }

  static Future<void> _addSyncQueueNextRetryAtColumn(Database db) async {
    final columns = await db.rawQuery('PRAGMA table_info($_syncQueueTable)');
    final hasColumn = columns.any(
      (column) => column['name'] == 'next_retry_at',
    );
    if (!hasColumn) {
      await db.execute(
        'ALTER TABLE $_syncQueueTable ADD COLUMN next_retry_at INTEGER',
      );
    }
  }

  static Future<void> _createSyncProfileTable(Database db) async {
    await db.execute('''
CREATE TABLE $_syncProfileTable (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
)
''');
  }

  static Future<void> _createSyncResolutionsTable(Database db) async {
    await db.execute('''
CREATE TABLE $_syncResolutionsTable (
  id TEXT PRIMARY KEY,
  resolved_at INTEGER NOT NULL,
  resolution_type TEXT NOT NULL,
  reason TEXT NOT NULL,
  incoming_record_id TEXT NOT NULL,
  existing_record_id TEXT,
  watermark_uid TEXT NOT NULL,
  existing_hash TEXT,
  incoming_hash TEXT,
  existing_revision INTEGER,
  incoming_revision INTEGER NOT NULL,
  inserted_record_id TEXT
)
''');
    await db.execute(
      'CREATE INDEX IF NOT EXISTS idx_mobile_sync_resolutions_resolved_at '
      'ON $_syncResolutionsTable(resolved_at DESC)',
    );
    await db.execute(
      'CREATE INDEX IF NOT EXISTS idx_mobile_sync_resolutions_watermark '
      'ON $_syncResolutionsTable(watermark_uid)',
    );
  }

  @override
  Future<List<VaultRecord>> loadRecords() async {
    final rows = await _db.query(_recordsTable, orderBy: 'created_at DESC');
    return rows.map(_recordFromRow).toList(growable: false);
  }

  @override
  Future<List<SyncQueueItem>> loadSyncQueue() async {
    final rows = await _db.query(_syncQueueTable, orderBy: 'created_at DESC');
    return rows.map(_syncQueueItemFromRow).toList(growable: false);
  }

  @override
  Future<List<MobileSyncResolution>> loadSyncResolutions() async {
    final rows = await _db.query(
      _syncResolutionsTable,
      orderBy: 'resolved_at DESC',
    );
    return rows.map(_syncResolutionFromRow).toList(growable: false);
  }

  @override
  Future<SyncProfile> loadSyncProfile() async {
    final rows = await _db.query(_syncProfileTable);
    if (rows.isEmpty) {
      return SyncProfile.localOnly();
    }
    final values = {
      for (final row in rows) row['key']! as String: row['value']! as String,
    };
    final legacyLanAddress = values['desktop_address'];
    final legacyPairingCode = values['pairing_code'];
    final mode = _syncTransportModeFromName(
      values['mode'] ??
          ((legacyLanAddress?.isNotEmpty == true &&
                  legacyPairingCode?.isNotEmpty == true)
              ? 'lanDebug'
              : 'localOnly'),
    );
    return SyncProfile(
      mode: mode,
      status: _syncConnectionStatusFromName(values['status'] ?? 'unconfigured'),
      updatedAt: DateTime.fromMillisecondsSinceEpoch(
        int.tryParse(values['updated_at'] ?? '') ?? 0,
      ),
      accountId: values['account_id'],
      accountLabel: values['account_label'],
      authToken: values['auth_token'],
      refreshToken: values['refresh_token'],
      workspaceId: values['workspace_id'],
      workspaceName: values['workspace_name'],
      deviceId: values['device_id'],
      deviceName: values['device_name'],
      devicePlatform: values['device_platform'],
      deviceRegistered: values['device_registered'] == 'true',
      creatorProfileId: values['creator_profile_id'],
      creatorDisplayName: values['creator_display_name'],
      creatorSeedRef: values['creator_seed_ref'],
      creatorSeedEnvelopeVersion:
          int.tryParse(values['creator_seed_envelope_version'] ?? '') ?? 0,
      creatorProfileSynced: values['creator_profile_synced'] == 'true',
      entitlementId: values['entitlement_id'],
      entitlementLabel: values['entitlement_label'] ?? '免费版',
      entitlementStatus: _entitlementStatusFromName(
        values['entitlement_status'] ?? 'free',
      ),
      entitlementPlanCode: values['entitlement_plan_code'] ?? 'free',
      entitlementFeatures: _decodeBoolMap(values['entitlement_features_json']),
      entitlementLastCheckedAt: _parseDateTime(
        values['entitlement_last_checked_at'],
      ),
      cloudBaseUrl: values['cloud_base_url'] ?? '',
      lanDebugAddress: values['lan_debug_address'] ?? legacyLanAddress ?? '',
      lanDebugPairingCode:
          values['lan_debug_pairing_code'] ?? legacyPairingCode ?? '',
      lastError: values['last_error'],
      lastRemotePullCursor:
          values['last_remote_pull_cursor'] ??
          values['last_desktop_pull_since'],
      lastSyncAttemptAt: _parseDateTime(values['last_sync_attempt_at']),
      lastSyncSuccessAt: _parseDateTime(values['last_sync_success_at']),
      lastSyncFailureAt: _parseDateTime(values['last_sync_failure_at']),
    );
  }

  @override
  Future<void> upsertRecord(VaultRecord record) async {
    await _db.insert(
      _recordsTable,
      _recordToRow(record),
      conflictAlgorithm: ConflictAlgorithm.replace,
    );
  }

  @override
  Future<void> enqueueSyncItem(SyncQueueItem item) async {
    await _db.insert(
      _syncQueueTable,
      _syncQueueItemToRow(item),
      conflictAlgorithm: ConflictAlgorithm.replace,
    );
  }

  @override
  Future<void> updateSyncItem(SyncQueueItem item) => enqueueSyncItem(item);

  @override
  Future<void> recordSyncResolution(MobileSyncResolution resolution) async {
    await _db.insert(
      _syncResolutionsTable,
      _syncResolutionToRow(resolution),
      conflictAlgorithm: ConflictAlgorithm.replace,
    );
  }

  @override
  Future<void> saveSyncProfile(SyncProfile profile) async {
    await _db.transaction((txn) async {
      Future<void> put(String key, String? value) async {
        if (value == null || value.isEmpty) {
          await txn.delete(
            _syncProfileTable,
            where: 'key = ?',
            whereArgs: [key],
          );
          return;
        }
        await txn.insert(_syncProfileTable, {
          'key': key,
          'value': value,
        }, conflictAlgorithm: ConflictAlgorithm.replace);
      }

      await put('mode', profile.mode.name);
      await put('status', profile.status.name);
      await put('updated_at', '${profile.updatedAt.millisecondsSinceEpoch}');
      await put('account_id', profile.accountId);
      await put('account_label', profile.accountLabel);
      await put('auth_token', profile.authToken);
      await put('refresh_token', profile.refreshToken);
      await put('workspace_id', profile.workspaceId);
      await put('workspace_name', profile.workspaceName);
      await put('device_id', profile.deviceId);
      await put('device_name', profile.deviceName);
      await put('device_platform', profile.devicePlatform);
      await put('device_registered', '${profile.deviceRegistered}');
      await put('creator_profile_id', profile.creatorProfileId);
      await put('creator_display_name', profile.creatorDisplayName);
      await put('creator_seed_ref', profile.creatorSeedRef);
      await put(
        'creator_seed_envelope_version',
        '${profile.creatorSeedEnvelopeVersion}',
      );
      await put('creator_profile_synced', '${profile.creatorProfileSynced}');
      await put('entitlement_id', profile.entitlementId);
      await put('entitlement_label', profile.entitlementLabel);
      await put('entitlement_status', profile.entitlementStatus.name);
      await put('entitlement_plan_code', profile.entitlementPlanCode);
      await put(
        'entitlement_features_json',
        jsonEncode(profile.entitlementFeatures),
      );
      await put(
        'entitlement_last_checked_at',
        profile.entitlementLastCheckedAt?.toIso8601String(),
      );
      await put('cloud_base_url', profile.cloudBaseUrl);
      await put('lan_debug_address', profile.lanDebugAddress);
      await put('lan_debug_pairing_code', profile.lanDebugPairingCode);
      await put('last_remote_pull_cursor', profile.lastRemotePullCursor);
      await put('last_error', profile.lastError);
      await put(
        'last_sync_attempt_at',
        profile.lastSyncAttemptAt?.toIso8601String(),
      );
      await put(
        'last_sync_success_at',
        profile.lastSyncSuccessAt?.toIso8601String(),
      );
      await put(
        'last_sync_failure_at',
        profile.lastSyncFailureAt?.toIso8601String(),
      );
    });
  }

  @override
  Future<void> close() => _db.close();
}

Map<String, Object?> _recordToRow(VaultRecord record) {
  return {
    'id': record.id,
    'kind': record.kind.name,
    'title': record.title,
    'watermark_uid': record.watermarkUid,
    'revision': record.revision,
    'sha256': record.sha256,
    'parent_watermark_uid': record.parentWatermarkUid,
    'rewrite_reason': record.rewriteReason,
    'extracted_timestamp': record.extractedTimestamp,
    'extracted_device_id_hex': record.extractedDeviceIdHex,
    'extracted_file_hash_hex': record.extractedFileHashHex,
    'source': record.source.name,
    'sync_status': record.syncStatus.name,
    'created_at': record.createdAt.millisecondsSinceEpoch,
  };
}

Map<String, Object?> _syncQueueItemToRow(SyncQueueItem item) {
  return {
    'id': item.id,
    'record_id': item.recordId,
    'operation': item.operation.name,
    'payload_type': item.payloadType,
    'payload_json': item.payloadJson,
    'status': item.status.name,
    'attempts': item.attempts,
    'created_at': item.createdAt.millisecondsSinceEpoch,
    'last_error': item.lastError,
    'next_retry_at': item.nextRetryAt?.millisecondsSinceEpoch,
  };
}

Map<String, Object?> _syncResolutionToRow(MobileSyncResolution resolution) {
  return {
    'id': resolution.id,
    'resolved_at': resolution.resolvedAt.millisecondsSinceEpoch,
    'resolution_type': resolution.resolutionType.name,
    'reason': resolution.reason,
    'incoming_record_id': resolution.incomingRecordId,
    'existing_record_id': resolution.existingRecordId,
    'watermark_uid': resolution.watermarkUid,
    'existing_hash': resolution.existingHash,
    'incoming_hash': resolution.incomingHash,
    'existing_revision': resolution.existingRevision,
    'incoming_revision': resolution.incomingRevision,
    'inserted_record_id': resolution.insertedRecordId,
  };
}

VaultRecord _recordFromRow(Map<String, Object?> row) {
  return VaultRecord(
    id: row['id']! as String,
    kind: _assetKindFromName(row['kind']! as String),
    title: row['title']! as String,
    watermarkUid: row['watermark_uid']! as String,
    revision: row['revision']! as int,
    sha256: row['sha256'] as String?,
    parentWatermarkUid: row['parent_watermark_uid'] as String?,
    rewriteReason: row['rewrite_reason'] as String?,
    extractedTimestamp: row['extracted_timestamp'] as int?,
    extractedDeviceIdHex: row['extracted_device_id_hex'] as String?,
    extractedFileHashHex: row['extracted_file_hash_hex'] as String?,
    source: _recordSourceFromName(row['source']! as String),
    syncStatus: _syncStatusFromName(row['sync_status']! as String),
    createdAt: DateTime.fromMillisecondsSinceEpoch(row['created_at']! as int),
  );
}

SyncQueueItem _syncQueueItemFromRow(Map<String, Object?> row) {
  return SyncQueueItem(
    id: row['id']! as String,
    recordId: row['record_id']! as String,
    operation: _syncQueueOperationFromName(row['operation']! as String),
    payloadType: row['payload_type']! as String,
    payloadJson: row['payload_json']! as String,
    status: _syncQueueItemStatusFromName(row['status']! as String),
    attempts: row['attempts']! as int,
    createdAt: DateTime.fromMillisecondsSinceEpoch(row['created_at']! as int),
    lastError: row['last_error'] as String?,
    nextRetryAt: _dateTimeFromEpoch(row['next_retry_at']),
  );
}

MobileSyncResolution _syncResolutionFromRow(Map<String, Object?> row) {
  return MobileSyncResolution(
    id: row['id']! as String,
    resolvedAt: DateTime.fromMillisecondsSinceEpoch(row['resolved_at']! as int),
    resolutionType: _mobileSyncResolutionTypeFromName(
      row['resolution_type']! as String,
    ),
    reason: row['reason']! as String,
    incomingRecordId: row['incoming_record_id']! as String,
    existingRecordId: row['existing_record_id'] as String?,
    watermarkUid: row['watermark_uid']! as String,
    existingHash: row['existing_hash'] as String?,
    incomingHash: row['incoming_hash'] as String?,
    existingRevision: row['existing_revision'] as int?,
    incomingRevision: row['incoming_revision']! as int,
    insertedRecordId: row['inserted_record_id'] as String?,
  );
}

DateTime? _dateTimeFromEpoch(Object? value) {
  if (value is int) {
    return DateTime.fromMillisecondsSinceEpoch(value);
  }
  return null;
}

WatermarkAssetKind _assetKindFromName(String name) {
  return WatermarkAssetKind.values.firstWhere(
    (kind) => kind.name == name,
    orElse: () => WatermarkAssetKind.image,
  );
}

VaultRecordSource _recordSourceFromName(String name) {
  return VaultRecordSource.values.firstWhere(
    (source) => source.name == name,
    orElse: () => VaultRecordSource.write,
  );
}

SyncStatus _syncStatusFromName(String name) {
  return SyncStatus.values.firstWhere(
    (status) => status.name == name,
    orElse: () => SyncStatus.localOnly,
  );
}

SyncQueueOperation _syncQueueOperationFromName(String name) {
  return SyncQueueOperation.values.firstWhere(
    (operation) => operation.name == name,
    orElse: () => SyncQueueOperation.upsertVaultRecord,
  );
}

SyncQueueItemStatus _syncQueueItemStatusFromName(String name) {
  return SyncQueueItemStatus.values.firstWhere(
    (status) => status.name == name,
    orElse: () => SyncQueueItemStatus.pending,
  );
}

SyncConnectionStatus _syncConnectionStatusFromName(String name) {
  return SyncConnectionStatus.values.firstWhere(
    (status) => status.name == name,
    orElse: () => SyncConnectionStatus.unconfigured,
  );
}

SyncTransportMode _syncTransportModeFromName(String name) {
  return SyncTransportMode.values.firstWhere(
    (mode) => mode.name == name,
    orElse: () => SyncTransportMode.localOnly,
  );
}

EntitlementStatus _entitlementStatusFromName(String name) {
  return EntitlementStatus.values.firstWhere(
    (status) => status.name == name,
    orElse: () => EntitlementStatus.free,
  );
}

Map<String, bool> _decodeBoolMap(String? raw) {
  if (raw == null || raw.isEmpty) {
    return const {};
  }
  try {
    final decoded = jsonDecode(raw) as Map<String, Object?>;
    return {
      for (final entry in decoded.entries) entry.key: entry.value == true,
    };
  } catch (_) {
    return const {};
  }
}

DateTime? _parseDateTime(String? raw) {
  if (raw == null || raw.isEmpty) {
    return null;
  }
  return DateTime.tryParse(raw);
}

MobileSyncResolutionType _mobileSyncResolutionTypeFromName(String name) {
  return MobileSyncResolutionType.values.firstWhere(
    (type) => type.name == name,
    orElse: () => MobileSyncResolutionType.recordInserted,
  );
}

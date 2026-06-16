import 'package:path/path.dart' as path;
import 'package:sqflite/sqflite.dart';

import '../app/mobile_app_state.dart';
import '../bridge/watermark_models.dart';

abstract class VaultStore {
  Future<List<VaultRecord>> loadRecords();

  Future<List<SyncQueueItem>> loadSyncQueue();

  Future<List<MobileSyncResolution>> loadSyncResolutions();

  Future<DesktopPairingProfile> loadPairingProfile();

  Future<void> upsertRecord(VaultRecord record);

  Future<void> enqueueSyncItem(SyncQueueItem item);

  Future<void> updateSyncItem(SyncQueueItem item);

  Future<void> recordSyncResolution(MobileSyncResolution resolution);

  Future<void> savePairingProfile(DesktopPairingProfile profile);

  Future<void> close();
}

class MemoryVaultStore implements VaultStore {
  final List<VaultRecord> _records = [];
  final List<SyncQueueItem> _syncQueue = [];
  final List<MobileSyncResolution> _syncResolutions = [];
  DesktopPairingProfile _pairingProfile = DesktopPairingProfile.unpaired();

  @override
  Future<List<VaultRecord>> loadRecords() async => List.unmodifiable(_records);

  @override
  Future<List<SyncQueueItem>> loadSyncQueue() async =>
      List.unmodifiable(_syncQueue);

  @override
  Future<List<MobileSyncResolution>> loadSyncResolutions() async =>
      List.unmodifiable(_syncResolutions);

  @override
  Future<DesktopPairingProfile> loadPairingProfile() async => _pairingProfile;

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
  Future<void> savePairingProfile(DesktopPairingProfile profile) async {
    _pairingProfile = profile;
  }

  @override
  Future<void> close() async {}
}

class SQLiteVaultStore implements VaultStore {
  SQLiteVaultStore._(this._db);

  final Database _db;

  static const _databaseName = 'hidden_shield_mobile.db';
  static const _databaseVersion = 4;
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
  last_error TEXT
)
''');
    await db.execute(
      'CREATE INDEX idx_sync_queue_status_created_at '
      'ON $_syncQueueTable(status, created_at ASC)',
    );
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
  Future<DesktopPairingProfile> loadPairingProfile() async {
    final rows = await _db.query(_syncProfileTable);
    if (rows.isEmpty) {
      return DesktopPairingProfile.unpaired();
    }
    final values = {
      for (final row in rows) row['key']! as String: row['value']! as String,
    };
    return DesktopPairingProfile(
      desktopAddress: values['desktop_address'] ?? '',
      pairingCode: values['pairing_code'] ?? '',
      status: _desktopPairingStatusFromName(values['status'] ?? 'unpaired'),
      updatedAt: DateTime.fromMillisecondsSinceEpoch(
        int.tryParse(values['updated_at'] ?? '') ?? 0,
      ),
      lastError: values['last_error'],
      lastDesktopPullSince: values['last_desktop_pull_since'],
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
  Future<void> savePairingProfile(DesktopPairingProfile profile) async {
    await _db.transaction((txn) async {
      await txn.insert(_syncProfileTable, {
        'key': 'desktop_address',
        'value': profile.desktopAddress,
      }, conflictAlgorithm: ConflictAlgorithm.replace);
      await txn.insert(_syncProfileTable, {
        'key': 'pairing_code',
        'value': profile.pairingCode,
      }, conflictAlgorithm: ConflictAlgorithm.replace);
      await txn.insert(_syncProfileTable, {
        'key': 'status',
        'value': profile.status.name,
      }, conflictAlgorithm: ConflictAlgorithm.replace);
      final pullSince = profile.lastDesktopPullSince;
      if (pullSince == null || pullSince.isEmpty) {
        await txn.delete(
          _syncProfileTable,
          where: 'key = ?',
          whereArgs: ['last_desktop_pull_since'],
        );
      } else {
        await txn.insert(_syncProfileTable, {
          'key': 'last_desktop_pull_since',
          'value': pullSince,
        }, conflictAlgorithm: ConflictAlgorithm.replace);
      }
      await txn.insert(_syncProfileTable, {
        'key': 'updated_at',
        'value': '${profile.updatedAt.millisecondsSinceEpoch}',
      }, conflictAlgorithm: ConflictAlgorithm.replace);
      if (profile.lastError == null) {
        await txn.delete(
          _syncProfileTable,
          where: 'key = ?',
          whereArgs: ['last_error'],
        );
      } else {
        await txn.insert(_syncProfileTable, {
          'key': 'last_error',
          'value': profile.lastError!,
        }, conflictAlgorithm: ConflictAlgorithm.replace);
      }
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

DesktopPairingStatus _desktopPairingStatusFromName(String name) {
  return DesktopPairingStatus.values.firstWhere(
    (status) => status.name == name,
    orElse: () => DesktopPairingStatus.unpaired,
  );
}

MobileSyncResolutionType _mobileSyncResolutionTypeFromName(String name) {
  return MobileSyncResolutionType.values.firstWhere(
    (type) => type.name == name,
    orElse: () => MobileSyncResolutionType.recordInserted,
  );
}

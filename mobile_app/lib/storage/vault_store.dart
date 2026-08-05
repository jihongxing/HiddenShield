import 'dart:convert';

import 'package:path/path.dart' as path;
import 'package:sqflite/sqflite.dart';

import '../app/mobile_app_state.dart';
import '../bridge/watermark_models.dart';
import '../licensing/offline_license_state.dart';

abstract class VaultStore {
  Future<List<VaultRecord>> loadRecords();

  Future<List<SyncQueueItem>> loadSyncQueue();

  Future<List<MobileSyncResolution>> loadSyncResolutions();

  Future<List<LocalBatchJob>> loadLocalBatchJobs();

  Future<UsageLedgerSummary> loadUsageLedgerSummary(SyncProfile syncProfile);

  Future<SyncProfile> loadSyncProfile();

  Future<OfflineLicenseMetadata?> loadOfflineLicenseMetadata();

  Future<List<OfflineLicenseAuditEvent>> loadOfflineLicenseAudit();

  Future<void> upsertRecord(VaultRecord record);

  Future<void> enqueueSyncItem(SyncQueueItem item);

  Future<void> updateSyncItem(SyncQueueItem item);

  Future<void> recordSyncResolution(MobileSyncResolution resolution);

  Future<void> saveLocalBatchJob(LocalBatchJob job);

  Future<void> appendUsageLedgerEntry(UsageLedgerEntry entry);

  Future<void> saveSyncProfile(SyncProfile profile);

  Future<void> saveOfflineLicenseMetadata(OfflineLicenseMetadata metadata);

  Future<void> appendOfflineLicenseAudit(OfflineLicenseAuditEvent event);

  Future<void> close();
}

class MemoryVaultStore implements VaultStore {
  final List<VaultRecord> _records = [];
  final List<SyncQueueItem> _syncQueue = [];
  final List<MobileSyncResolution> _syncResolutions = [];
  final List<LocalBatchJob> _localBatchJobs = [];
  final List<UsageLedgerEntry> _usageLedger = [];
  final List<OfflineLicenseAuditEvent> _offlineLicenseAudit = [];
  SyncProfile _syncProfile = SyncProfile.localOnly();
  OfflineLicenseMetadata? _offlineLicenseMetadata;

  @override
  Future<List<VaultRecord>> loadRecords() async => List.unmodifiable(_records);

  @override
  Future<List<SyncQueueItem>> loadSyncQueue() async =>
      List.unmodifiable(_syncQueue);

  @override
  Future<List<MobileSyncResolution>> loadSyncResolutions() async =>
      List.unmodifiable(_syncResolutions);

  @override
  Future<List<LocalBatchJob>> loadLocalBatchJobs() async =>
      List.unmodifiable(_localBatchJobs);

  @override
  Future<UsageLedgerSummary> loadUsageLedgerSummary(
    SyncProfile syncProfile,
  ) async {
    var summary = UsageLedgerSummary.empty(syncProfile);
    final entries = [..._usageLedger]
      ..sort((a, b) => a.occurredAt.compareTo(b.occurredAt));
    for (final entry in entries) {
      summary = summary.withEntry(entry, syncProfile);
    }
    return summary;
  }

  @override
  Future<SyncProfile> loadSyncProfile() async => _syncProfile;

  @override
  Future<OfflineLicenseMetadata?> loadOfflineLicenseMetadata() async =>
      _offlineLicenseMetadata;

  @override
  Future<List<OfflineLicenseAuditEvent>> loadOfflineLicenseAudit() async =>
      List.unmodifiable(_offlineLicenseAudit);

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
  Future<void> saveLocalBatchJob(LocalBatchJob job) async {
    final existingIndex = _localBatchJobs.indexWhere(
      (item) => item.id == job.id,
    );
    if (existingIndex == -1) {
      _localBatchJobs.insert(0, job);
    } else {
      _localBatchJobs[existingIndex] = job;
      _localBatchJobs.sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    }
  }

  @override
  Future<void> appendUsageLedgerEntry(UsageLedgerEntry entry) async {
    final existingIndex = _usageLedger.indexWhere(
      (item) => item.id == entry.id,
    );
    if (existingIndex == -1) {
      _usageLedger.add(entry);
    } else {
      _usageLedger[existingIndex] = entry;
    }
  }

  @override
  Future<void> saveSyncProfile(SyncProfile profile) async {
    _syncProfile = profile;
  }

  @override
  Future<void> saveOfflineLicenseMetadata(
    OfflineLicenseMetadata metadata,
  ) async {
    _offlineLicenseMetadata = metadata;
  }

  @override
  Future<void> appendOfflineLicenseAudit(OfflineLicenseAuditEvent event) async {
    _offlineLicenseAudit.insert(0, event);
  }

  @override
  Future<void> close() async {}
}

class SQLiteVaultStore implements VaultStore {
  SQLiteVaultStore._(this._db);

  final Database _db;

  static const _databaseName = 'hidden_shield_mobile.db';
  static const _databaseVersion = 14;
  static const _recordsTable = 'vault_records';
  static const _syncQueueTable = 'sync_queue';
  static const _syncResolutionsTable = 'mobile_sync_resolutions';
  static const _syncProfileTable = 'sync_profile';
  static const _localBatchJobsTable = 'local_batch_jobs';
  static const _localBatchItemsTable = 'local_batch_items';
  static const _usageLedgerTable = 'usage_ledger';
  static const _offlineLicenseStateTable = 'offline_license_state';
  static const _offlineLicenseAuditTable = 'offline_license_audit';

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
        await _createLocalBatchTables(db);
        await _createUsageLedgerTable(db);
        await _createOfflineLicenseTables(db);
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
        if (oldVersion < 6) {
          await _addWriteVerificationColumns(db);
        }
        if (oldVersion < 7) {
          await _createLocalBatchTables(db);
        }
        if (oldVersion < 8) {
          await _createUsageLedgerTable(db);
        }
        if (oldVersion < 9) {
          await _addVideoNotaryColumns(db);
        }
        if (oldVersion < 10) {
          await _addOnboardingProfileKey(db);
        }
        if (oldVersion < 11) {
          await _addCreatorAndTrustedTimeColumns(db);
        }
        if (oldVersion < 12) {
          await _addProtectedCopyAndDeclarationColumns(db);
        }
        if (oldVersion < 13) {
          await _addPayloadRegistryColumns(db);
        }
        if (oldVersion < 14) {
          await _createOfflineLicenseTables(db);
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
  creator_display_name TEXT,
  trusted_time_status TEXT,
  trusted_time_source TEXT,
  trusted_time_at INTEGER,
  third_party_verification_status TEXT,
  third_party_verification_provider TEXT,
  third_party_verification_path TEXT,
  sha256 TEXT,
  parent_watermark_uid TEXT,
  rewrite_reason TEXT,
  extracted_timestamp INTEGER,
  extracted_device_id_hex TEXT,
  extracted_file_hash_hex TEXT,
  write_verification_status TEXT,
  write_verification_message TEXT,
  write_verification_at INTEGER,
  video_notary_id TEXT,
  video_notary_at INTEGER,
  video_notary_receipt_signature TEXT,
  video_notary_usage_ledger_id TEXT,
  video_fingerprint_root TEXT,
  video_bundle_sha256 TEXT,
  video_bundle_bytes INTEGER,
  video_bundle_scene_count INTEGER,
  video_bundle_elapsed_ms INTEGER,
  video_frame_sample_policy TEXT,
  video_visual_task_id TEXT,
  video_visual_completed_at INTEGER,
  video_visual_strategy_digest TEXT,
  video_visual_self_check_confidence REAL,
  video_visual_self_check_threshold REAL,
  video_visual_checked_frames INTEGER,
  video_visual_media_hash TEXT,
  video_visual_receipt_hash TEXT,
  video_visual_output_bytes INTEGER,
  video_visual_output_content_type TEXT,
  protected_copy_name TEXT,
  protected_copy_hash TEXT,
  payload_protocol_version INTEGER NOT NULL DEFAULT 2,
  payload_bytes_length INTEGER NOT NULL DEFAULT 119,
  watermark_id_issue_mode TEXT NOT NULL DEFAULT 'offline_generated',
  watermark_id_registry_status TEXT NOT NULL DEFAULT 'pending_registration',
  watermark_id_registry_receipt TEXT,
  payload_auth_status TEXT NOT NULL DEFAULT 'verified',
  output_strategy TEXT NOT NULL DEFAULT 'minimal_required_change',
  work_source_declaration TEXT NOT NULL DEFAULT 'unspecified',
  training_permission_declaration TEXT NOT NULL DEFAULT 'prohibited',
  creation_method_declaration TEXT NOT NULL DEFAULT 'unspecified',
  human_edit_level_declaration TEXT NOT NULL DEFAULT 'unspecified',
  authenticity_claim_declaration TEXT NOT NULL DEFAULT 'unspecified',
  custom_rights_statement TEXT,
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

  static Future<void> _addWriteVerificationColumns(Database db) async {
    final columns = await db.rawQuery('PRAGMA table_info($_recordsTable)');
    Future<void> addColumn(String name, String ddl) async {
      final hasColumn = columns.any((column) => column['name'] == name);
      if (!hasColumn) {
        await db.execute('ALTER TABLE $_recordsTable ADD COLUMN $ddl');
      }
    }

    await addColumn(
      'write_verification_status',
      'write_verification_status TEXT',
    );
    await addColumn(
      'write_verification_message',
      'write_verification_message TEXT',
    );
    await addColumn('write_verification_at', 'write_verification_at INTEGER');
  }

  static Future<void> _addVideoNotaryColumns(Database db) async {
    final columns = await db.rawQuery('PRAGMA table_info($_recordsTable)');
    Future<void> addColumn(String name, String ddl) async {
      final hasColumn = columns.any((column) => column['name'] == name);
      if (!hasColumn) {
        await db.execute('ALTER TABLE $_recordsTable ADD COLUMN $ddl');
      }
    }

    await addColumn('video_notary_id', 'video_notary_id TEXT');
    await addColumn('video_notary_at', 'video_notary_at INTEGER');
    await addColumn(
      'video_notary_receipt_signature',
      'video_notary_receipt_signature TEXT',
    );
    await addColumn(
      'video_notary_usage_ledger_id',
      'video_notary_usage_ledger_id TEXT',
    );
    await addColumn('video_fingerprint_root', 'video_fingerprint_root TEXT');
    await addColumn('video_bundle_sha256', 'video_bundle_sha256 TEXT');
    await addColumn('video_bundle_bytes', 'video_bundle_bytes INTEGER');
    await addColumn(
      'video_bundle_scene_count',
      'video_bundle_scene_count INTEGER',
    );
    await addColumn(
      'video_bundle_elapsed_ms',
      'video_bundle_elapsed_ms INTEGER',
    );
    await addColumn(
      'video_frame_sample_policy',
      'video_frame_sample_policy TEXT',
    );
    await addColumn('video_visual_task_id', 'video_visual_task_id TEXT');
    await addColumn(
      'video_visual_completed_at',
      'video_visual_completed_at INTEGER',
    );
    await addColumn(
      'video_visual_strategy_digest',
      'video_visual_strategy_digest TEXT',
    );
    await addColumn(
      'video_visual_self_check_confidence',
      'video_visual_self_check_confidence REAL',
    );
    await addColumn(
      'video_visual_self_check_threshold',
      'video_visual_self_check_threshold REAL',
    );
    await addColumn(
      'video_visual_checked_frames',
      'video_visual_checked_frames INTEGER',
    );
    await addColumn('video_visual_media_hash', 'video_visual_media_hash TEXT');
    await addColumn(
      'video_visual_receipt_hash',
      'video_visual_receipt_hash TEXT',
    );
    await addColumn(
      'video_visual_output_bytes',
      'video_visual_output_bytes INTEGER',
    );
    await addColumn(
      'video_visual_output_content_type',
      'video_visual_output_content_type TEXT',
    );
  }

  static Future<void> _addCreatorAndTrustedTimeColumns(Database db) async {
    final columns = await db.rawQuery('PRAGMA table_info($_recordsTable)');
    Future<void> addColumn(String name, String ddl) async {
      final hasColumn = columns.any((column) => column['name'] == name);
      if (!hasColumn) {
        await db.execute('ALTER TABLE $_recordsTable ADD COLUMN $ddl');
      }
    }

    await addColumn('creator_display_name', 'creator_display_name TEXT');
    await addColumn('trusted_time_status', 'trusted_time_status TEXT');
    await addColumn('trusted_time_source', 'trusted_time_source TEXT');
    await addColumn('trusted_time_at', 'trusted_time_at INTEGER');
    await addColumn(
      'third_party_verification_status',
      'third_party_verification_status TEXT',
    );
    await addColumn(
      'third_party_verification_provider',
      'third_party_verification_provider TEXT',
    );
    await addColumn(
      'third_party_verification_path',
      'third_party_verification_path TEXT',
    );
  }

  static Future<void> _addProtectedCopyAndDeclarationColumns(
    Database db,
  ) async {
    final columns = await db.rawQuery('PRAGMA table_info($_recordsTable)');
    Future<void> addColumn(String name, String ddl) async {
      final hasColumn = columns.any((column) => column['name'] == name);
      if (!hasColumn) {
        await db.execute('ALTER TABLE $_recordsTable ADD COLUMN $ddl');
      }
    }

    await addColumn('protected_copy_name', 'protected_copy_name TEXT');
    await addColumn('protected_copy_hash', 'protected_copy_hash TEXT');
    await addColumn(
      'output_strategy',
      "output_strategy TEXT NOT NULL DEFAULT 'minimal_required_change'",
    );
    await addColumn(
      'work_source_declaration',
      "work_source_declaration TEXT NOT NULL DEFAULT 'unspecified'",
    );
    await addColumn(
      'training_permission_declaration',
      "training_permission_declaration TEXT NOT NULL DEFAULT 'prohibited'",
    );
    await addColumn(
      'creation_method_declaration',
      "creation_method_declaration TEXT NOT NULL DEFAULT 'unspecified'",
    );
    await addColumn(
      'human_edit_level_declaration',
      "human_edit_level_declaration TEXT NOT NULL DEFAULT 'unspecified'",
    );
    await addColumn(
      'authenticity_claim_declaration',
      "authenticity_claim_declaration TEXT NOT NULL DEFAULT 'unspecified'",
    );
    await addColumn('custom_rights_statement', 'custom_rights_statement TEXT');
  }

  static Future<void> _addPayloadRegistryColumns(Database db) async {
    final columns = await db.rawQuery('PRAGMA table_info($_recordsTable)');
    Future<void> addColumn(String name, String ddl) async {
      final hasColumn = columns.any((column) => column['name'] == name);
      if (!hasColumn) {
        await db.execute('ALTER TABLE $_recordsTable ADD COLUMN $ddl');
      }
    }

    await addColumn(
      'payload_protocol_version',
      'payload_protocol_version INTEGER NOT NULL DEFAULT 2',
    );
    await addColumn(
      'payload_bytes_length',
      'payload_bytes_length INTEGER NOT NULL DEFAULT 119',
    );
    await addColumn(
      'watermark_id_issue_mode',
      "watermark_id_issue_mode TEXT NOT NULL DEFAULT 'offline_generated'",
    );
    await addColumn(
      'watermark_id_registry_status',
      "watermark_id_registry_status TEXT NOT NULL DEFAULT 'pending_registration'",
    );
    await addColumn(
      'watermark_id_registry_receipt',
      'watermark_id_registry_receipt TEXT',
    );
    await addColumn(
      'payload_auth_status',
      "payload_auth_status TEXT NOT NULL DEFAULT 'verified'",
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

  static Future<void> _addOnboardingProfileKey(Database db) async {
    await db.insert(_syncProfileTable, {
      'key': 'onboarding_completed',
      'value': 'false',
    }, conflictAlgorithm: ConflictAlgorithm.ignore);
  }

  static Future<void> _createLocalBatchTables(Database db) async {
    await db.execute('''
CREATE TABLE IF NOT EXISTS $_localBatchJobsTable (
  id TEXT PRIMARY KEY,
  status TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  entitlement_plan_code TEXT NOT NULL,
  entitlement_status TEXT NOT NULL
)
''');
    await db.execute('''
CREATE TABLE IF NOT EXISTS $_localBatchItemsTable (
  id TEXT PRIMARY KEY,
  job_id TEXT NOT NULL,
  input_ref TEXT NOT NULL,
  file_name TEXT NOT NULL,
  media_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  attempts INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  last_error TEXT,
  output_ref TEXT,
  vault_record_id TEXT,
  write_verification_status TEXT,
  write_verification_message TEXT
)
''');
    await db.execute(
      'CREATE INDEX IF NOT EXISTS idx_local_batch_jobs_updated_at '
      'ON $_localBatchJobsTable(updated_at DESC)',
    );
    await db.execute(
      'CREATE INDEX IF NOT EXISTS idx_local_batch_items_job '
      'ON $_localBatchItemsTable(job_id, updated_at ASC)',
    );
  }

  static Future<void> _createUsageLedgerTable(Database db) async {
    await db.execute('''
CREATE TABLE IF NOT EXISTS $_usageLedgerTable (
  id TEXT PRIMARY KEY,
  occurred_at INTEGER NOT NULL,
  feature_name TEXT NOT NULL,
  media_type TEXT NOT NULL,
  file_size_bucket TEXT NOT NULL,
  quantity INTEGER NOT NULL,
  event_type TEXT NOT NULL,
  entitlement_status TEXT NOT NULL,
  entitlement_plan_code TEXT NOT NULL,
  entitlement_plan_name TEXT,
  pipeline_id TEXT,
  vault_record_id TEXT
)
''');
    await db.execute(
      'CREATE INDEX IF NOT EXISTS idx_usage_ledger_occurred_at '
      'ON $_usageLedgerTable(occurred_at DESC)',
    );
    await db.execute(
      'CREATE INDEX IF NOT EXISTS idx_usage_ledger_media_type '
      'ON $_usageLedgerTable(media_type, occurred_at DESC)',
    );
  }

  static Future<void> _createOfflineLicenseTables(Database db) async {
    await db.execute('''
CREATE TABLE IF NOT EXISTS $_offlineLicenseStateTable (
  singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
  installation_id TEXT NOT NULL,
  status TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  license_id TEXT,
  product_code TEXT,
  key_id TEXT,
  not_before INTEGER,
  expires_at INTEGER,
  revocation_list_id TEXT,
  revocation_sequence INTEGER,
  last_error TEXT
)
''');
    await db.execute('''
CREATE TABLE IF NOT EXISTS $_offlineLicenseAuditTable (
  id TEXT PRIMARY KEY,
  occurred_at INTEGER NOT NULL,
  action TEXT NOT NULL,
  result TEXT NOT NULL,
  license_id TEXT,
  key_id TEXT,
  detail_code TEXT
)
''');
    await db.execute(
      'CREATE INDEX IF NOT EXISTS idx_offline_license_audit_occurred_at '
      'ON $_offlineLicenseAuditTable(occurred_at DESC)',
    );
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
  Future<List<LocalBatchJob>> loadLocalBatchJobs() async {
    final jobRows = await _db.query(
      _localBatchJobsTable,
      orderBy: 'updated_at DESC',
    );
    final itemRows = await _db.query(
      _localBatchItemsTable,
      orderBy: 'updated_at ASC',
    );
    final itemsByJob = <String, List<LocalBatchItem>>{};
    for (final row in itemRows) {
      final item = _localBatchItemFromRow(row);
      itemsByJob.putIfAbsent(item.jobId, () => []).add(item);
    }
    return [
      for (final row in jobRows)
        _localBatchJobFromRow(row, itemsByJob[row['id'] as String] ?? const []),
    ];
  }

  @override
  Future<UsageLedgerSummary> loadUsageLedgerSummary(
    SyncProfile syncProfile,
  ) async {
    final counts = await _db.rawQuery('''
SELECT
  COALESCE(SUM(quantity), 0) AS total_units,
  COUNT(*) AS total_events,
  COALESCE(SUM(CASE WHEN media_type = 'image' THEN quantity ELSE 0 END), 0) AS image_units,
  COALESCE(SUM(CASE WHEN media_type = 'video' THEN quantity ELSE 0 END), 0) AS video_units,
  COALESCE(SUM(CASE WHEN media_type = 'audio' THEN quantity ELSE 0 END), 0) AS audio_units
FROM $_usageLedgerTable
''');
    final latestRows = await _db.query(
      _usageLedgerTable,
      columns: ['occurred_at', 'feature_name'],
      orderBy: 'occurred_at DESC',
      limit: 1,
    );
    final countRow = counts.single;
    final latest = latestRows.isEmpty ? null : latestRows.single;
    return UsageLedgerSummary(
      totalUnits: _asInt(countRow['total_units']),
      totalEvents: _asInt(countRow['total_events']),
      imageUnits: _asInt(countRow['image_units']),
      videoUnits: _asInt(countRow['video_units']),
      audioUnits: _asInt(countRow['audio_units']),
      lastUsedAt: latest == null
          ? null
          : DateTime.fromMillisecondsSinceEpoch(_asInt(latest['occurred_at'])),
      lastFeatureName: latest?['feature_name'] as String?,
      entitlementStatus: syncProfile.entitlementStatus,
      entitlementPlanCode: syncProfile.entitlementPlanCode,
      entitlementPlanName: syncProfile.entitlementLabel,
    );
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
    final entitlementFeatures = _decodeBoolMap(
      values['entitlement_features_json'],
    );
    final entitlementPlanCode = values['entitlement_plan_code'] ?? 'free';
    final entitlementPlanKey = normalizeEntitlementPlanKey(
      planKey: values['entitlement_plan_key'],
      planCode: entitlementPlanCode,
      features: entitlementFeatures,
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
      onboardingCompleted: values['onboarding_completed'] == 'true',
      entitlementId: values['entitlement_id'],
      entitlementLabel: entitlementPlanLabel(entitlementPlanKey),
      entitlementStatus: _entitlementStatusFromName(
        values['entitlement_status'] ?? 'free',
      ),
      entitlementPlanCode: entitlementPlanCode,
      entitlementPlanKey: entitlementPlanKey,
      entitlementFeatures: entitlementFeatures,
      entitlementLastCheckedAt: _parseDateTime(
        values['entitlement_last_checked_at'],
      ),
      syncPolicy:
          values['sync_policy'] ??
          (values['entitlement_features_json']?.contains('"cloud_sync":true') ==
                  true
              ? 'auto_cloud_vault'
              : 'blocked_by_entitlement'),
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
      anonymousFeedbackEnabled: values['anonymous_feedback_enabled'] == 'true',
      experienceImprovementEnabled:
          values['experience_improvement_enabled'] != 'false',
      anonymousInstallId: values['anonymous_install_id'],
      anonymousFeedbackLastEventAt: _parseDateTime(
        values['anonymous_feedback_last_event_at'],
      ),
      anonymousFeedbackLastAttemptAt: _parseDateTime(
        values['anonymous_feedback_last_attempt_at'],
      ),
      anonymousFeedbackLastSuccessAt: _parseDateTime(
        values['anonymous_feedback_last_success_at'],
      ),
      anonymousFeedbackNextRetryAt: _parseDateTime(
        values['anonymous_feedback_next_retry_at'],
      ),
      anonymousFeedbackLastFlushError:
          values['anonymous_feedback_last_flush_error'],
      anonymousFeedbackConsecutiveFailures:
          int.tryParse(
            values['anonymous_feedback_consecutive_failures'] ?? '',
          ) ??
          0,
      anonymousFeedbackQueueJson: values['anonymous_feedback_queue_json'],
      reportPurchaseGrantsJson: values['report_purchase_grants_json'],
    );
  }

  @override
  Future<OfflineLicenseMetadata?> loadOfflineLicenseMetadata() async {
    final rows = await _db.query(
      _offlineLicenseStateTable,
      where: 'singleton_id = 1',
      limit: 1,
    );
    if (rows.isEmpty) return null;
    final row = rows.single;
    return OfflineLicenseMetadata(
      installationId: row['installation_id'] as String,
      status: OfflineLicenseStatus.values.byName(row['status'] as String),
      updatedAt: DateTime.fromMillisecondsSinceEpoch(row['updated_at'] as int),
      licenseId: row['license_id'] as String?,
      productCode: row['product_code'] as String?,
      keyId: row['key_id'] as String?,
      notBefore: _dateTimeFromEpoch(row['not_before']),
      expiresAt: _dateTimeFromEpoch(row['expires_at']),
      revocationListId: row['revocation_list_id'] as String?,
      revocationSequence: row['revocation_sequence'] as int?,
      lastError: row['last_error'] as String?,
    );
  }

  @override
  Future<List<OfflineLicenseAuditEvent>> loadOfflineLicenseAudit() async {
    final rows = await _db.query(
      _offlineLicenseAuditTable,
      orderBy: 'occurred_at DESC',
    );
    return rows
        .map(
          (row) => OfflineLicenseAuditEvent(
            id: row['id'] as String,
            occurredAt: DateTime.fromMillisecondsSinceEpoch(
              row['occurred_at'] as int,
            ),
            action: row['action'] as String,
            result: row['result'] as String,
            licenseId: row['license_id'] as String?,
            keyId: row['key_id'] as String?,
            detailCode: row['detail_code'] as String?,
          ),
        )
        .toList(growable: false);
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
  Future<void> saveLocalBatchJob(LocalBatchJob job) async {
    await _db.transaction((txn) async {
      await txn.insert(
        _localBatchJobsTable,
        _localBatchJobToRow(job),
        conflictAlgorithm: ConflictAlgorithm.replace,
      );
      await txn.delete(
        _localBatchItemsTable,
        where: 'job_id = ?',
        whereArgs: [job.id],
      );
      for (final item in job.items) {
        await txn.insert(
          _localBatchItemsTable,
          _localBatchItemToRow(item),
          conflictAlgorithm: ConflictAlgorithm.replace,
        );
      }
    });
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
      await put('onboarding_completed', '${profile.onboardingCompleted}');
      await put('entitlement_id', profile.entitlementId);
      await put('entitlement_label', profile.entitlementLabel);
      await put('entitlement_status', profile.entitlementStatus.name);
      await put('entitlement_plan_code', profile.entitlementPlanCode);
      await put('entitlement_plan_key', profile.entitlementPlanKey);
      await put(
        'entitlement_features_json',
        jsonEncode(profile.entitlementFeatures),
      );
      await put(
        'entitlement_last_checked_at',
        profile.entitlementLastCheckedAt?.toIso8601String(),
      );
      await put('sync_policy', profile.syncPolicy);
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
      await put(
        'anonymous_feedback_enabled',
        '${profile.anonymousFeedbackEnabled}',
      );
      await put(
        'experience_improvement_enabled',
        '${profile.experienceImprovementEnabled}',
      );
      await put('anonymous_install_id', profile.anonymousInstallId);
      await put(
        'anonymous_feedback_last_event_at',
        profile.anonymousFeedbackLastEventAt?.toIso8601String(),
      );
      await put(
        'anonymous_feedback_last_attempt_at',
        profile.anonymousFeedbackLastAttemptAt?.toIso8601String(),
      );
      await put(
        'anonymous_feedback_last_success_at',
        profile.anonymousFeedbackLastSuccessAt?.toIso8601String(),
      );
      await put(
        'anonymous_feedback_next_retry_at',
        profile.anonymousFeedbackNextRetryAt?.toIso8601String(),
      );
      await put(
        'anonymous_feedback_last_flush_error',
        profile.anonymousFeedbackLastFlushError,
      );
      await put(
        'anonymous_feedback_consecutive_failures',
        '${profile.anonymousFeedbackConsecutiveFailures}',
      );
      await put(
        'anonymous_feedback_queue_json',
        profile.anonymousFeedbackQueueJson,
      );
      await put(
        'report_purchase_grants_json',
        profile.reportPurchaseGrantsJson,
      );
    });
  }

  @override
  Future<void> saveOfflineLicenseMetadata(
    OfflineLicenseMetadata metadata,
  ) async {
    await _db.insert(_offlineLicenseStateTable, {
      'singleton_id': 1,
      'installation_id': metadata.installationId,
      'status': metadata.status.name,
      'updated_at': metadata.updatedAt.millisecondsSinceEpoch,
      'license_id': metadata.licenseId,
      'product_code': metadata.productCode,
      'key_id': metadata.keyId,
      'not_before': metadata.notBefore?.millisecondsSinceEpoch,
      'expires_at': metadata.expiresAt?.millisecondsSinceEpoch,
      'revocation_list_id': metadata.revocationListId,
      'revocation_sequence': metadata.revocationSequence,
      'last_error': metadata.lastError,
    }, conflictAlgorithm: ConflictAlgorithm.replace);
  }

  @override
  Future<void> appendOfflineLicenseAudit(OfflineLicenseAuditEvent event) async {
    await _db.insert(_offlineLicenseAuditTable, {
      'id': event.id,
      'occurred_at': event.occurredAt.millisecondsSinceEpoch,
      'action': event.action,
      'result': event.result,
      'license_id': event.licenseId,
      'key_id': event.keyId,
      'detail_code': event.detailCode,
    }, conflictAlgorithm: ConflictAlgorithm.replace);
  }

  @override
  Future<void> appendUsageLedgerEntry(UsageLedgerEntry entry) async {
    await _db.insert(
      _usageLedgerTable,
      _usageLedgerEntryToRow(entry),
      conflictAlgorithm: ConflictAlgorithm.replace,
    );
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
    'creator_display_name': record.creatorDisplayName,
    'trusted_time_status': record.trustedTimeStatus,
    'trusted_time_source': record.trustedTimeSource,
    'trusted_time_at': record.trustedTimeAt?.millisecondsSinceEpoch,
    'third_party_verification_status': record.thirdPartyVerificationStatus,
    'third_party_verification_provider': record.thirdPartyVerificationProvider,
    'third_party_verification_path': record.thirdPartyVerificationPath,
    'sha256': record.sha256,
    'parent_watermark_uid': record.parentWatermarkUid,
    'rewrite_reason': record.rewriteReason,
    'extracted_timestamp': record.extractedTimestamp,
    'extracted_device_id_hex': record.extractedDeviceIdHex,
    'extracted_file_hash_hex': record.extractedFileHashHex,
    'write_verification_status': record.writeVerificationStatus?.name,
    'write_verification_message': record.writeVerificationMessage,
    'write_verification_at': record.writeVerificationAt?.millisecondsSinceEpoch,
    'protected_copy_name': record.protectedCopyName,
    'protected_copy_hash': record.protectedCopyHash,
    'payload_protocol_version': record.payloadProtocolVersion,
    'payload_bytes_length': record.payloadBytesLength,
    'watermark_id_issue_mode': record.watermarkIdIssueMode,
    'watermark_id_registry_status': record.watermarkIdRegistryStatus,
    'watermark_id_registry_receipt': record.watermarkIdRegistryReceipt,
    'payload_auth_status': record.payloadAuthStatus,
    'output_strategy': record.outputStrategy,
    'work_source_declaration': record.workSourceDeclaration,
    'training_permission_declaration': record.trainingPermissionDeclaration,
    'creation_method_declaration': record.creationMethodDeclaration,
    'human_edit_level_declaration': record.humanEditLevelDeclaration,
    'authenticity_claim_declaration': record.authenticityClaimDeclaration,
    'custom_rights_statement': record.customRightsStatement,
    'video_notary_id': record.videoNotaryId,
    'video_notary_at': record.videoNotaryAt?.millisecondsSinceEpoch,
    'video_notary_receipt_signature': record.videoNotaryReceiptSignature,
    'video_notary_usage_ledger_id': record.videoNotaryUsageLedgerId,
    'video_fingerprint_root': record.videoFingerprintRoot,
    'video_bundle_sha256': record.videoBundleSha256,
    'video_bundle_bytes': record.videoBundleBytes,
    'video_bundle_scene_count': record.videoBundleSceneCount,
    'video_bundle_elapsed_ms': record.videoBundleElapsedMs,
    'video_frame_sample_policy': record.videoFrameSamplePolicy,
    'video_visual_task_id': record.videoVisualTaskId,
    'video_visual_completed_at':
        record.videoVisualCompletedAt?.millisecondsSinceEpoch,
    'video_visual_strategy_digest': record.videoVisualStrategyDigest,
    'video_visual_self_check_confidence': record.videoVisualSelfCheckConfidence,
    'video_visual_self_check_threshold': record.videoVisualSelfCheckThreshold,
    'video_visual_checked_frames': record.videoVisualCheckedFrames,
    'video_visual_media_hash': record.videoVisualMediaHash,
    'video_visual_receipt_hash': record.videoVisualReceiptHash,
    'video_visual_output_bytes': record.videoVisualOutputBytes,
    'video_visual_output_content_type': record.videoVisualOutputContentType,
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

Map<String, Object?> _localBatchJobToRow(LocalBatchJob job) {
  return {
    'id': job.id,
    'status': job.status.name,
    'created_at': job.createdAt.millisecondsSinceEpoch,
    'updated_at': job.updatedAt.millisecondsSinceEpoch,
    'entitlement_plan_code': job.entitlementPlanCode,
    'entitlement_status': job.entitlementStatus.name,
  };
}

Map<String, Object?> _localBatchItemToRow(LocalBatchItem item) {
  return {
    'id': item.id,
    'job_id': item.jobId,
    'input_ref': item.inputRef,
    'file_name': item.fileName,
    'media_kind': item.mediaKind.name,
    'status': item.status.name,
    'attempts': item.attempts,
    'created_at': item.createdAt.millisecondsSinceEpoch,
    'updated_at': item.updatedAt.millisecondsSinceEpoch,
    'last_error': item.lastError,
    'output_ref': item.outputRef,
    'vault_record_id': item.vaultRecordId,
    'write_verification_status': item.writeVerificationStatus?.name,
    'write_verification_message': item.writeVerificationMessage,
  };
}

Map<String, Object?> _usageLedgerEntryToRow(UsageLedgerEntry entry) {
  return {
    'id': entry.id,
    'occurred_at': entry.occurredAt.millisecondsSinceEpoch,
    'feature_name': entry.featureName,
    'media_type': entry.mediaType.name,
    'file_size_bucket': entry.fileSizeBucket,
    'quantity': entry.quantity,
    'event_type': entry.eventType,
    'entitlement_status': entry.entitlementStatus.name,
    'entitlement_plan_code': entry.entitlementPlanCode,
    'entitlement_plan_name': entry.entitlementPlanName,
    'pipeline_id': entry.pipelineId,
    'vault_record_id': entry.vaultRecordId,
  };
}

VaultRecord _recordFromRow(Map<String, Object?> row) {
  return VaultRecord(
    id: row['id']! as String,
    kind: _assetKindFromName(row['kind']! as String),
    title: row['title']! as String,
    watermarkUid: row['watermark_uid']! as String,
    revision: row['revision']! as int,
    creatorDisplayName: row['creator_display_name'] as String?,
    trustedTimeStatus: row['trusted_time_status'] as String?,
    trustedTimeSource: row['trusted_time_source'] as String?,
    trustedTimeAt: _dateTimeFromEpoch(row['trusted_time_at']),
    thirdPartyVerificationStatus:
        row['third_party_verification_status'] as String?,
    thirdPartyVerificationProvider:
        row['third_party_verification_provider'] as String?,
    thirdPartyVerificationPath: row['third_party_verification_path'] as String?,
    sha256: row['sha256'] as String?,
    parentWatermarkUid: row['parent_watermark_uid'] as String?,
    rewriteReason: row['rewrite_reason'] as String?,
    extractedTimestamp: row['extracted_timestamp'] as int?,
    extractedDeviceIdHex: row['extracted_device_id_hex'] as String?,
    extractedFileHashHex: row['extracted_file_hash_hex'] as String?,
    writeVerificationStatus: _writeVerificationStatusFromName(
      row['write_verification_status'] as String?,
    ),
    writeVerificationMessage: row['write_verification_message'] as String?,
    writeVerificationAt: _dateTimeFromEpoch(row['write_verification_at']),
    protectedCopyName: row['protected_copy_name'] as String?,
    protectedCopyHash: row['protected_copy_hash'] as String?,
    payloadProtocolVersion: _asInt(
      row['payload_protocol_version'],
      fallback: 2,
    ),
    payloadBytesLength: _asInt(row['payload_bytes_length'], fallback: 119),
    watermarkIdIssueMode:
        row['watermark_id_issue_mode'] as String? ?? 'offline_generated',
    watermarkIdRegistryStatus:
        row['watermark_id_registry_status'] as String? ??
        'pending_registration',
    watermarkIdRegistryReceipt: row['watermark_id_registry_receipt'] as String?,
    payloadAuthStatus: row['payload_auth_status'] as String? ?? 'verified',
    outputStrategy:
        row['output_strategy'] as String? ?? 'minimal_required_change',
    workSourceDeclaration:
        row['work_source_declaration'] as String? ?? 'unspecified',
    trainingPermissionDeclaration:
        row['training_permission_declaration'] as String? ?? 'prohibited',
    creationMethodDeclaration:
        row['creation_method_declaration'] as String? ?? 'unspecified',
    humanEditLevelDeclaration:
        row['human_edit_level_declaration'] as String? ?? 'unspecified',
    authenticityClaimDeclaration:
        row['authenticity_claim_declaration'] as String? ?? 'unspecified',
    customRightsStatement: row['custom_rights_statement'] as String?,
    videoNotaryId: row['video_notary_id'] as String?,
    videoNotaryAt: _dateTimeFromEpoch(row['video_notary_at']),
    videoNotaryReceiptSignature:
        row['video_notary_receipt_signature'] as String?,
    videoNotaryUsageLedgerId: row['video_notary_usage_ledger_id'] as String?,
    videoFingerprintRoot: row['video_fingerprint_root'] as String?,
    videoBundleSha256: row['video_bundle_sha256'] as String?,
    videoBundleBytes: row['video_bundle_bytes'] as int?,
    videoBundleSceneCount: row['video_bundle_scene_count'] as int?,
    videoBundleElapsedMs: row['video_bundle_elapsed_ms'] as int?,
    videoFrameSamplePolicy: row['video_frame_sample_policy'] as String?,
    videoVisualTaskId: row['video_visual_task_id'] as String?,
    videoVisualCompletedAt: _dateTimeFromEpoch(
      row['video_visual_completed_at'],
    ),
    videoVisualStrategyDigest: row['video_visual_strategy_digest'] as String?,
    videoVisualSelfCheckConfidence:
        (row['video_visual_self_check_confidence'] as num?)?.toDouble(),
    videoVisualSelfCheckThreshold:
        (row['video_visual_self_check_threshold'] as num?)?.toDouble(),
    videoVisualCheckedFrames: row['video_visual_checked_frames'] as int?,
    videoVisualMediaHash: row['video_visual_media_hash'] as String?,
    videoVisualReceiptHash: row['video_visual_receipt_hash'] as String?,
    videoVisualOutputBytes: row['video_visual_output_bytes'] as int?,
    videoVisualOutputContentType:
        row['video_visual_output_content_type'] as String?,
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

LocalBatchJob _localBatchJobFromRow(
  Map<String, Object?> row,
  List<LocalBatchItem> items,
) {
  return LocalBatchJob(
    id: row['id']! as String,
    status: _batchJobStatusFromName(row['status']! as String),
    createdAt: DateTime.fromMillisecondsSinceEpoch(row['created_at']! as int),
    updatedAt: DateTime.fromMillisecondsSinceEpoch(row['updated_at']! as int),
    entitlementPlanCode: row['entitlement_plan_code']! as String,
    entitlementStatus: _entitlementStatusFromName(
      row['entitlement_status']! as String,
    ),
    items: items,
  );
}

LocalBatchItem _localBatchItemFromRow(Map<String, Object?> row) {
  return LocalBatchItem(
    id: row['id']! as String,
    jobId: row['job_id']! as String,
    inputRef: row['input_ref']! as String,
    fileName: row['file_name']! as String,
    mediaKind: _batchMediaKindFromName(row['media_kind']! as String),
    status: _batchItemStatusFromName(row['status']! as String),
    attempts: row['attempts']! as int,
    createdAt: DateTime.fromMillisecondsSinceEpoch(row['created_at']! as int),
    updatedAt: DateTime.fromMillisecondsSinceEpoch(row['updated_at']! as int),
    lastError: row['last_error'] as String?,
    outputRef: row['output_ref'] as String?,
    vaultRecordId: row['vault_record_id'] as String?,
    writeVerificationStatus: _writeVerificationStatusFromName(
      row['write_verification_status'] as String?,
    ),
    writeVerificationMessage: row['write_verification_message'] as String?,
  );
}

WriteVerificationStatus? _writeVerificationStatusFromName(String? name) {
  return switch (name) {
    'verified' => WriteVerificationStatus.verified,
    'failed' => WriteVerificationStatus.failed,
    _ => null,
  };
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

BatchMediaKind _batchMediaKindFromName(String name) {
  return BatchMediaKind.values.firstWhere(
    (kind) => kind.name == name,
    orElse: () => BatchMediaKind.unsupported,
  );
}

BatchJobStatus _batchJobStatusFromName(String name) {
  return BatchJobStatus.values.firstWhere(
    (status) => status.name == name,
    orElse: () => BatchJobStatus.draft,
  );
}

BatchItemStatus _batchItemStatusFromName(String name) {
  return BatchItemStatus.values.firstWhere(
    (status) => status.name == name,
    orElse: () => BatchItemStatus.queued,
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

int _asInt(Object? value, {int fallback = 0}) {
  if (value is int) {
    return value;
  }
  if (value is num) {
    return value.toInt();
  }
  if (value is String) {
    return int.tryParse(value) ?? fallback;
  }
  return fallback;
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

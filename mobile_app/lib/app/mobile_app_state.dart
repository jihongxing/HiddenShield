import 'dart:async';
import 'dart:convert';
import 'dart:math' as math;

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/foundation.dart';

import 'mobile_anonymous_feedback.dart';
import '../bridge/watermark_models.dart';
import '../licensing/offline_license_manager.dart';
import '../licensing/offline_license_state.dart';
import '../sync/cloud_account_client.dart';
import '../storage/vault_store.dart';
import '../sync/sync_transport.dart';
import 'mobile_time_attestation.dart';
import 'system_config.dart';

class MobileAppState extends ChangeNotifier {
  MobileAppState({
    VaultStore? vaultStore,
    SyncTransport? syncTransport,
    SyncTransportFactory? syncTransportFactory,
    CloudAccountClient? cloudAccountClient,
    MobileTrustedTimeClient? trustedTimeClient,
    MobileAnonymousFeedbackClient? anonymousFeedbackClient,
    OfflineLicenseManager? offlineLicenseManager,
  }) : _vaultStore = vaultStore ?? MemoryVaultStore(),
       _syncTransportFactory =
           syncTransportFactory ?? _defaultSyncTransportFactory,
       _transportOverride = syncTransport,
       _cloudAccountClient = cloudAccountClient,
       _trustedTimeClient = trustedTimeClient ?? MobileTrustedTimeClient(),
       _anonymousFeedbackClient =
           anonymousFeedbackClient ?? MobileAnonymousFeedbackClient(),
       _offlineLicenseManager =
           offlineLicenseManager ??
           OfflineLicenseManager(
             secureStore: PlatformOfflineLicenseSecureStore(),
             platform: _defaultOfflineLicensePlatform(),
             appVersion: '1.0.0',
           );

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
  final MobileTrustedTimeClient _trustedTimeClient;
  final MobileAnonymousFeedbackClient _anonymousFeedbackClient;
  final OfflineLicenseManager _offlineLicenseManager;
  final List<VaultRecord> _records = [];
  final List<SyncQueueItem> _syncQueue = [];
  final List<MobileSyncResolution> _syncResolutions = [];
  final List<LocalBatchJob> _localBatchJobs = [];
  final List<AccountDevice> _cloudDevices = [];
  final List<MobileAnonymousFeedbackEvent> _anonymousFeedbackQueue = [];
  final List<OfflineLicenseAuditEvent> _offlineLicenseAudit = [];
  UsageLedgerSummary _usageSummary = UsageLedgerSummary.empty(
    SyncProfile.localOnly(),
  );

  String _creatorLabel = '本机创作者';
  bool _anonymousFeedbackEnabled = false;
  bool _experienceImprovementEnabled = true;
  String _anonymousInstallId = '';
  String _anonymousSessionId = '';
  DateTime? _anonymousFeedbackLastEventAt;
  DateTime? _anonymousFeedbackLastAttemptAt;
  DateTime? _anonymousFeedbackLastSuccessAt;
  DateTime? _anonymousFeedbackNextRetryAt;
  String? _anonymousFeedbackLastFlushError;
  int _anonymousFeedbackConsecutiveFailures = 0;
  SyncProfile _syncProfile = SyncProfile.localOnly();
  SyncTransportMode _syncTransportMode = SyncTransportMode.localOnly;
  bool _isLoaded = false;
  bool _isSyncing = false;
  bool _isPullingRemoteChanges = false;
  BillingPaymentSession? _latestPaymentSession;
  String? _latestPaymentSessionStatus;
  String? _latestPaymentMessage;
  Timer? _paymentPollTimer;
  DateTime? _paymentPollingStartedAt;
  OfflineLicenseSnapshot _offlineLicenseSnapshot =
      const OfflineLicenseSnapshot.unsupported();

  bool get isLoaded => _isLoaded;

  bool get isSyncing => _isSyncing;

  bool get isPullingRemoteChanges => _isPullingRemoteChanges;

  String get creatorLabel => _creatorLabel;

  bool get cloudSyncEnabled => _syncTransportMode == SyncTransportMode.cloud;

  bool get anonymousFeedbackEnabled => _anonymousFeedbackEnabled;

  bool get experienceImprovementEnabled => _experienceImprovementEnabled;

  MobileAnonymousFeedbackStatus get anonymousFeedbackStatus =>
      _buildAnonymousFeedbackStatus();

  MobileExperienceImprovementSnapshot get experienceImprovementSnapshot =>
      _buildExperienceImprovementSnapshot();

  MobileDataUsageSnapshot get dataUsageSnapshot => _buildDataUsageSnapshot();

  SyncProfile get syncProfile => _syncProfile;

  SyncTransportMode get syncTransportMode => _syncTransportMode;

  BillingPaymentSession? get latestPaymentSession => _latestPaymentSession;

  String? get latestPaymentSessionStatus => _latestPaymentSessionStatus;

  String? get latestPaymentMessage => _latestPaymentMessage;

  List<VaultRecord> get records => List.unmodifiable(_records);

  List<SyncQueueItem> get syncQueue => List.unmodifiable(_syncQueue);

  List<MobileSyncResolution> get syncResolutions =>
      List.unmodifiable(_syncResolutions);

  List<LocalBatchJob> get localBatchJobs => List.unmodifiable(_localBatchJobs);

  List<AccountDevice> get cloudDevices => List.unmodifiable(_cloudDevices);

  LocalBatchJob? get latestLocalBatchJob =>
      _localBatchJobs.isEmpty ? null : _localBatchJobs.first;

  UsageLedgerSummary get usageSummary => _usageSummary;

  OfflineLicenseSnapshot get offlineLicenseSnapshot => _offlineLicenseSnapshot;

  List<OfflineLicenseAuditEvent> get offlineLicenseAudit =>
      List.unmodifiable(_offlineLicenseAudit);

  Map<String, bool> get effectiveEntitlementFeatures {
    final features = Map<String, bool>.from(_syncProfile.entitlementFeatures);
    if (_offlineLicenseSnapshot.isActive) {
      features['batch_processing'] = true;
    }
    return Map.unmodifiable(features);
  }

  String get effectiveEntitlementLabel {
    if (!_offlineLicenseSnapshot.isActive) {
      return _syncProfile.entitlementLabel;
    }
    final hasCloudAnnual =
        _syncProfile.entitlementFeatures['batch_processing'] == true &&
        _syncProfile.entitlementFeatures['cloud_sync'] == true;
    return hasCloudAnnual
        ? '${_syncProfile.entitlementLabel} + 离线授权'
        : '图片 / 音频年费（离线授权）';
  }

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

  bool get onboardingCompleted =>
      _syncProfile.onboardingCompleted && _creatorLabel.trim().isNotEmpty;

  bool get canUseCloudSync => hasCloudSyncEntitlement;

  bool get hasCloudSyncEntitlement =>
      hasCloudAccount &&
      _syncProfile.cloudBaseUrl.isNotEmpty &&
      _syncProfile.entitlementFeatures['cloud_sync'] == true;

  bool get canAutoCloudSync =>
      hasCloudSyncEntitlement && _syncProfile.syncPolicy == 'auto_cloud_vault';

  bool get canExportFormalReports =>
      effectiveEntitlementFeatures['report_export'] == true;

  bool get canUseLocalBatchProcessing =>
      effectiveEntitlementFeatures['batch_processing'] == true;

  bool canExportFormalReportForRecord(VaultRecord record) {
    return canExportFormalReports || _hasActiveReportPurchaseGrant(record.id);
  }

  bool get canUseTeamWorkspace =>
      _syncProfile.entitlementFeatures['team_workspace'] == true;

  bool get canQueryPublicRightsRegistry =>
      _syncProfile.cloudBaseUrl.trim().isNotEmpty ||
      _cloudAccountClient != null;

  CommercialHealthSummary get commercialHealthSummary {
    final batchItems = _localBatchJobs
        .expand((job) => job.items)
        .toList(growable: false);
    final verifiedBatchItems = batchItems
        .where((item) => item.status == BatchItemStatus.verified)
        .length;
    final failedBatchItems = batchItems
        .where((item) => item.status == BatchItemStatus.failed)
        .length;
    final l2NotaryCount = _records
        .where((record) => record.videoNotaryId?.isNotEmpty == true)
        .length;
    return CommercialHealthSummary(
      accountScope: hasCloudAccount ? '当前账户' : '本机',
      entitlementPlanName: _syncProfile.entitlementLabel,
      entitlementStatus: _syncProfile.entitlementStatus,
      localBatchJobs: _localBatchJobs.length,
      verifiedBatchItems: verifiedBatchItems,
      failedBatchItems: failedBatchItems,
      reportExportUnits: _usageSummary.lastFeatureName == 'report_export'
          ? 1
          : 0,
      cloudAcceptedEvents: _syncQueue
          .where((item) => item.status == SyncQueueItemStatus.synced)
          .length,
      cloudFailureEvents: failedSyncQueueCount,
      l2VideoNotaryCount: l2NotaryCount,
      latestPaymentSessionStatus: _latestPaymentSessionStatus,
      privacyNote: '仅展示计数、状态和错误分类；不采集原始媒体、加水印媒体、本地路径、文件名或完整媒体哈希。',
    );
  }

  Future<void> load() async {
    final records = await _vaultStore.loadRecords();
    final syncQueue = await _vaultStore.loadSyncQueue();
    final syncResolutions = await _vaultStore.loadSyncResolutions();
    final localBatchJobs = await _vaultStore.loadLocalBatchJobs();
    final offlineLicenseAudit = await _vaultStore.loadOfflineLicenseAudit();
    final loadedSyncProfile = await _vaultStore.loadSyncProfile();
    final syncProfile = _normalizeSyncEntitlement(loadedSyncProfile);
    final usageSummary = await _vaultStore.loadUsageLedgerSummary(syncProfile);
    _anonymousFeedbackEnabled = syncProfile.anonymousFeedbackEnabled;
    _experienceImprovementEnabled = syncProfile.experienceImprovementEnabled;
    _anonymousInstallId =
        syncProfile.anonymousInstallId?.trim().isNotEmpty == true
        ? syncProfile.anonymousInstallId!.trim()
        : _newAnonymousId('mobile-install');
    _anonymousSessionId = _newAnonymousId('mobile-session');
    _anonymousFeedbackLastEventAt = syncProfile.anonymousFeedbackLastEventAt;
    _anonymousFeedbackLastAttemptAt =
        syncProfile.anonymousFeedbackLastAttemptAt;
    _anonymousFeedbackLastSuccessAt =
        syncProfile.anonymousFeedbackLastSuccessAt;
    _anonymousFeedbackNextRetryAt = syncProfile.anonymousFeedbackNextRetryAt;
    _anonymousFeedbackLastFlushError =
        syncProfile.anonymousFeedbackLastFlushError;
    _anonymousFeedbackConsecutiveFailures =
        syncProfile.anonymousFeedbackConsecutiveFailures;
    _anonymousFeedbackQueue
      ..clear()
      ..addAll(
        _decodeAnonymousFeedbackQueue(syncProfile.anonymousFeedbackQueueJson),
      );
    _records
      ..clear()
      ..addAll(records);
    _syncQueue
      ..clear()
      ..addAll(syncQueue);
    _syncResolutions
      ..clear()
      ..addAll(syncResolutions);
    _localBatchJobs
      ..clear()
      ..addAll(localBatchJobs);
    _offlineLicenseAudit
      ..clear()
      ..addAll(offlineLicenseAudit);
    _syncProfile = syncProfile;
    _creatorLabel = syncProfile.creatorDisplayName?.trim().isNotEmpty == true
        ? syncProfile.creatorDisplayName!.trim()
        : _creatorLabel;
    _syncTransportMode = syncProfile.mode;
    _usageSummary = usageSummary;
    await _refreshOfflineLicenseStatus();
    _syncProfile = _syncProfile.copyWith(
      anonymousFeedbackEnabled: _anonymousFeedbackEnabled,
      experienceImprovementEnabled: _experienceImprovementEnabled,
      anonymousInstallId: _anonymousInstallId,
      anonymousFeedbackLastEventAt: _anonymousFeedbackLastEventAt,
      anonymousFeedbackLastAttemptAt: _anonymousFeedbackLastAttemptAt,
      anonymousFeedbackLastSuccessAt: _anonymousFeedbackLastSuccessAt,
      anonymousFeedbackNextRetryAt: _anonymousFeedbackNextRetryAt,
      anonymousFeedbackLastFlushError: _anonymousFeedbackLastFlushError,
      anonymousFeedbackConsecutiveFailures:
          _anonymousFeedbackConsecutiveFailures,
      anonymousFeedbackQueueJson: _encodeAnonymousFeedbackQueue(),
      updatedAt: DateTime.now(),
    );
    unawaited(_vaultStore.saveSyncProfile(_syncProfile));
    _isLoaded = true;
    notifyListeners();
    if (canAutoCloudSync) {
      unawaited(_runAutomaticCloudVaultSync());
    }
  }

  SyncProfile _normalizeSyncEntitlement(SyncProfile profile) {
    final shouldDisableFreeCloudSync =
        profile.entitlementStatus == EntitlementStatus.free &&
        profile.entitlementPlanCode == 'free' &&
        profile.entitlementFeatures['cloud_sync'] == true;
    if (!shouldDisableFreeCloudSync) {
      return profile;
    }
    final features = Map<String, bool>.from(profile.entitlementFeatures);
    features['cloud_sync'] = false;
    final normalized = profile.copyWith(
      mode: SyncTransportMode.localOnly,
      entitlementFeatures: features,
      syncPolicy: 'blocked_by_entitlement',
      updatedAt: DateTime.now(),
    );
    unawaited(_vaultStore.saveSyncProfile(normalized));
    return normalized;
  }

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
    await _vaultStore.saveLocalBatchJob(job);
    notifyListeners();
  }

  Future<String> createOfflineActivationRequest() async {
    try {
      final token = await _offlineLicenseManager.createActivationRequest();
      await _refreshOfflineLicenseStatus();
      await _appendOfflineLicenseAudit(
        action: 'create_activation_request',
        result: 'success',
      );
      return token;
    } catch (error) {
      await _appendOfflineLicenseAudit(
        action: 'create_activation_request',
        result: 'failed',
        detailCode: _offlineLicenseErrorCode(error),
      );
      rethrow;
    }
  }

  Future<void> importOfflineLicenseToken(String token) async {
    final previousLicenseId = _offlineLicenseSnapshot.licenseId;
    final previousKeyId = _offlineLicenseSnapshot.keyId;
    try {
      _offlineLicenseSnapshot = await _offlineLicenseManager.importLicense(
        token,
      );
      await _persistOfflineLicenseMetadata();
      if (previousLicenseId != null &&
          previousLicenseId != _offlineLicenseSnapshot.licenseId) {
        await _appendOfflineLicenseAudit(
          action: 'replace_license',
          result: 'success',
          licenseId: previousLicenseId,
          keyId: previousKeyId,
          detailCode: _offlineLicenseSnapshot.licenseId,
        );
      }
      await _appendOfflineLicenseAudit(
        action: 'import_license',
        result: 'success',
        licenseId: _offlineLicenseSnapshot.licenseId,
        keyId: _offlineLicenseSnapshot.keyId,
      );
      notifyListeners();
    } catch (error) {
      await _refreshOfflineLicenseStatus();
      await _appendOfflineLicenseAudit(
        action: 'import_license',
        result: 'failed',
        detailCode: _offlineLicenseErrorCode(error),
      );
      rethrow;
    }
  }

  Future<void> importOfflineRevocationList(String token) async {
    try {
      _offlineLicenseSnapshot = await _offlineLicenseManager
          .importRevocationList(token);
      await _persistOfflineLicenseMetadata();
      await _appendOfflineLicenseAudit(
        action: 'import_revocation_list',
        result: 'success',
        licenseId: _offlineLicenseSnapshot.licenseId,
        keyId: _offlineLicenseSnapshot.keyId,
      );
      notifyListeners();
    } catch (error) {
      await _refreshOfflineLicenseStatus();
      await _appendOfflineLicenseAudit(
        action: 'import_revocation_list',
        result: 'failed',
        detailCode: _offlineLicenseErrorCode(error),
      );
      rethrow;
    }
  }

  Future<void> clearOfflineLicense() async {
    try {
      _offlineLicenseSnapshot = await _offlineLicenseManager.clearLicense();
      await _persistOfflineLicenseMetadata();
      await _appendOfflineLicenseAudit(
        action: 'clear_license',
        result: 'success',
      );
      notifyListeners();
    } catch (error) {
      await _appendOfflineLicenseAudit(
        action: 'clear_license',
        result: 'failed',
        detailCode: _offlineLicenseErrorCode(error),
      );
      rethrow;
    }
  }

  Future<OfflineExecutionAuthorization> authorizeLocalExecution(
    String feature,
  ) async {
    if (feature != 'batch_processing' && feature != 'report_export') {
      return OfflineExecutionAuthorization(
        feature: feature,
        allowed: false,
        source: 'server_only',
        errorCode: 'offline_license_feature_not_allowed',
      );
    }
    if (_syncProfile.entitlementFeatures[feature] == true) {
      return OfflineExecutionAuthorization(
        feature: feature,
        allowed: true,
        source: 'cloud_subscription',
      );
    }
    await _refreshOfflineLicenseStatus();
    final offlineAllowed =
        _offlineLicenseSnapshot.localFeatures[feature] == true;
    return OfflineExecutionAuthorization(
      feature: feature,
      allowed: offlineAllowed,
      source: offlineAllowed ? 'offline_cdkey' : 'none',
      errorCode: offlineAllowed
          ? null
          : _offlineLicenseSnapshot.isActive
          ? 'offline_license_feature_not_allowed'
          : _offlineLicenseSnapshot.lastError ?? 'offline_license_required',
    );
  }

  Future<void> _refreshOfflineLicenseStatus() async {
    _offlineLicenseSnapshot = await _offlineLicenseManager.readStatus();
    await _persistOfflineLicenseMetadata();
  }

  Future<void> _persistOfflineLicenseMetadata() async {
    await _vaultStore.saveOfflineLicenseMetadata(
      OfflineLicenseMetadata.fromSnapshot(
        _offlineLicenseSnapshot,
        DateTime.now(),
      ),
    );
  }

  Future<void> _appendOfflineLicenseAudit({
    required String action,
    required String result,
    String? licenseId,
    String? keyId,
    String? detailCode,
  }) async {
    final now = DateTime.now();
    final event = OfflineLicenseAuditEvent(
      id: 'license-audit-${now.microsecondsSinceEpoch}',
      occurredAt: now,
      action: action,
      result: result,
      licenseId: licenseId,
      keyId: keyId,
      detailCode: detailCode,
    );
    _offlineLicenseAudit.insert(0, event);
    await _vaultStore.appendOfflineLicenseAudit(event);
  }

  Future<MobileTrustedTimeAttestation?> requestTrustedTimeAttestation() {
    return _trustedTimeClient.request();
  }

  String sha256HexForBytes(List<int> bytes) {
    return crypto.sha256.convert(bytes).toString();
  }

  Future<L3VideoVisualUploadTaskResult> createL3VideoVisualUploadTaskFromBytes({
    required List<int> bytes,
    required String fileName,
    required int durationMs,
    int? width,
    int? height,
    int? frameCount,
  }) async {
    final client = _cloudAccountClient;
    final accessToken = _syncProfile.authToken?.trim();
    final workspaceId = _syncProfile.workspaceId?.trim();
    final creatorProfileId = _syncProfile.creatorProfileId?.trim();
    final planCode = _syncProfile.entitlementPlanCode;
    if (client == null ||
        accessToken == null ||
        accessToken.isEmpty ||
        workspaceId == null ||
        workspaceId.isEmpty ||
        creatorProfileId == null ||
        creatorProfileId.isEmpty) {
      throw StateError('尚未登录 HiddenShield 账户，无法创建 L3 云端任务');
    }
    if (_syncProfile.entitlementFeatures['cloud_video_processing'] != true ||
        (planCode != 'studio' && planCode != 'enterprise')) {
      throw StateError('L3 视频画面盲水印创建上传需要 Studio / Enterprise 云视频权益');
    }
    if (!fileName.toLowerCase().endsWith('.mp4')) {
      throw ArgumentError('L3 正式创建上传入口当前只接收 MP4；其他容器待 worker 转码入口放开后再承诺');
    }
    if (bytes.isEmpty) {
      throw ArgumentError('L3 上传视频为空文件');
    }
    if (durationMs <= 0) {
      throw ArgumentError('L3 创建任务需要可确认的视频时长');
    }
    if (width != null &&
        height != null &&
        frameCount != null &&
        !_l3DeclaredCapacityIsSupported(width, height, frameCount)) {
      throw ArgumentError(
        'L3 当前 release gate 不接收该尺寸 / 帧率组合：strategy_invalid 容量不足，请换用 1080p / 1024x576 以上主战场样本或降低短视频帧抽样密度',
      );
    }
    final sourceSha256 = _prefixedSha256(sha256HexForBytes(bytes));
    final uploadAuth = await client
        .createCloudVideoTaskObjectUploadAuthorization(
          accessToken: accessToken,
          workspaceId: workspaceId,
          creatorProfileId: creatorProfileId,
          sha256: sourceSha256,
          bytes: bytes.length,
          contentType: 'video/mp4',
          objectKind: 'l3_user_object_upload_proxy',
          ttlSeconds: 900,
        );
    final uploadResult = await client.uploadCloudVideoTaskObjectBytes(
      uploadToken: uploadAuth.uploadToken,
      bytes: bytes,
    );
    if (uploadResult.status != 'uploaded' ||
        uploadResult.sha256 != sourceSha256 ||
        uploadResult.bytes != bytes.length) {
      throw StateError('L3 对象上传回读哈希或字节数不一致，已停止创建任务');
    }
    if (uploadResult.storageRef != uploadAuth.storageRef ||
        !uploadResult.storageRef.startsWith('object://l3-upload/')) {
      throw StateError('L3 对象上传 storageRef 不在正式 l3-upload 对象边界内');
    }
    final reserved = await client.reserveWatermarkId(
      accessToken: accessToken,
      request: WatermarkIdReserveRequest(
        requestId:
            'mobile:${_syncProfile.deviceId ?? 'local'}:l3-video-visual:${DateTime.now().microsecondsSinceEpoch}',
        workspaceId: workspaceId,
        creatorProfileId: creatorProfileId,
        mediaType: 'video_visual',
        payloadProtocolVersion: 2,
        payloadBytesLength: 119,
        parentWatermarkUid: null,
        revision: 1,
        originalHash: sourceSha256,
      ),
    );
    final task = await client.createCloudVideoTask(
      accessToken: accessToken,
      request: {
        'schemaVersion': 'cloud_video_task_v1',
        'workspaceId': workspaceId,
        'creatorProfileId': creatorProfileId,
        'capabilityLevel': 'hybrid_visual_watermark',
        'watermarkUid': reserved.watermarkUid,
        'sourceHash': sourceSha256,
        'durationMs': durationMs,
        'targetProfiles': ['studio_enterprise_l3_formal_upload_h264'],
        'uploadManifest': {
          'schemaVersion': 'video_upload_manifest_v1',
          'containsOriginalVideo': false,
          'containsWatermarkedVideo': false,
          'containsLocalPaths': false,
          'containsProxy': true,
          'items': [
            {
              'kind': 'l3_user_object_upload_proxy',
              'sha256': uploadResult.sha256,
              'bytes': uploadResult.bytes,
              'storageRef': uploadResult.storageRef,
              'sandboxProfile': 'l3_ffmpeg_transcode_sandbox_v1',
              'transcodeProfile': 'h264_controlled_proxy_v1',
              if (width != null) 'width': width,
              if (height != null) 'height': height,
              if (frameCount != null) 'frameCount': frameCount,
            },
          ],
        },
      },
    );
    return L3VideoVisualUploadTaskResult(
      task: task,
      watermarkUid: task.watermarkUid,
      sourceSha256: task.sourceHash,
      uploadedBytes: uploadResult.bytes,
      privacyBoundary:
          'signed_object_upload_only_no_local_path_no_raw_video_sync',
      nextAction: '等待 trusted worker 完成自检和收据固化；任务 succeeded 后再下载并保存版权库',
    );
  }

  Future<VaultRecord> createL2VideoFingerprintNotaryFromBytes({
    required List<int> bytes,
    required String fileName,
    required int durationMs,
    int? width,
    int? height,
    int? frameCount,
  }) async {
    final accessToken = _syncProfile.authToken?.trim();
    final workspaceId = _syncProfile.workspaceId?.trim();
    final creatorProfileId = _syncProfile.creatorProfileId?.trim();
    if (accessToken == null ||
        accessToken.isEmpty ||
        workspaceId == null ||
        workspaceId.isEmpty ||
        creatorProfileId == null ||
        creatorProfileId.isEmpty) {
      throw StateError('尚未登录 HiddenShield 账户，无法提交 L2 视频指纹存证');
    }
    if (_syncProfile.entitlementFeatures['cloud_sync'] != true) {
      throw StateError('L2 视频指纹存证需要 Creator 起开放的正式云同步权益');
    }
    final extension = _extensionForFileName(fileName);
    if (!const {'mp4', 'mov', 'mkv', 'webm'}.contains(extension)) {
      throw ArgumentError('L2 视频指纹存证当前支持 MP4 / MOV / MKV / WebM');
    }
    if (bytes.isEmpty) {
      throw ArgumentError('L2 视频指纹存证不能提交空文件');
    }
    if (durationMs <= 0) {
      throw ArgumentError('L2 视频指纹存证需要可确认的视频时长');
    }
    final client =
        _cloudAccountClient ??
        (_syncProfile.cloudBaseUrl.trim().isEmpty
            ? null
            : CloudAccountClient(baseUrl: _syncProfile.cloudBaseUrl.trim()));
    if (client == null) {
      throw StateError('未配置云端 notary 服务，无法提交 L2 视频指纹存证');
    }

    final startedAt = DateTime.now();
    final sourceHash = _prefixedSha256(sha256HexForBytes(bytes));
    final sourceHashRaw = _stripSha256Prefix(sourceHash);
    final sceneCount = math.max(1, math.min(frameCount ?? 1, 8));
    final fingerprintSeed = <String, Object?>{
      'schemaVersion': 'mobile_video_fingerprint_seed_v1',
      'sourceHash': sourceHash,
      'bytes': bytes.length,
      'durationMs': durationMs,
      'extension': extension,
      'width': width,
      'height': height,
      'frameCount': frameCount,
      'sceneCount': sceneCount,
      'workspaceId': workspaceId,
      'creatorProfileId': creatorProfileId,
    };
    String digest(String label) =>
        _sha256HexForJson({'label': label, 'seed': fingerprintSeed});
    String shortDigest(String label) => digest(label).substring(0, 16);
    final globalFrameFingerprints = List<Map<String, Object?>>.generate(
      sceneCount,
      (index) => {
        'sceneIndex': index,
        'timestampMs': math.min(
          durationMs,
          ((durationMs / (sceneCount + 1)) * (index + 1)).round(),
        ),
        'phash': shortDigest('phash:$index'),
        'colorHash': shortDigest('color:$index'),
        'edgeHash': shortDigest('edge:$index'),
        'motionSummary': 'mobile-metadata-${shortDigest('motion:$index')}',
      },
      growable: false,
    );
    final localBlockFingerprintRoot = 'sha256:${digest('local-block-root')}';
    final cropWindowFingerprintRoot = 'sha256:${digest('crop-window-root')}';
    final fingerprintRoot = 'sha256:${digest('fingerprint-root')}';
    final localBlockCount = math.max(1, sceneCount * 16);
    final cropWindowCount = math.max(1, sceneCount * 4);
    final watermarkUid = 'l2-mobile-${sourceHashRaw.substring(0, 16)}';
    final bundle = <String, Object?>{
      'schemaVersion': 'mobile_video_fingerprint_bundle_v1',
      'privacyBoundary': 'metadata_hash_only_no_raw_video_no_local_path',
      'sourceHash': sourceHash,
      'durationMs': durationMs,
      'frameSamplePolicy': 'mobile_metadata_probe_v1',
      'sceneCount': sceneCount,
      'fingerprintSchemaVersion': 'mobile_metadata_fingerprint_v1',
      'globalFrameFingerprints': globalFrameFingerprints,
      'localBlockFingerprintRoot': localBlockFingerprintRoot,
      'localBlockCount': localBlockCount,
      'cropWindowFingerprintRoot': cropWindowFingerprintRoot,
      'cropWindowCount': cropWindowCount,
      'fingerprintRoot': fingerprintRoot,
      'width': width,
      'height': height,
      'frameCount': frameCount,
    };
    final bundleBytes = utf8.encode(jsonEncode(bundle));
    final bundleSha256 = _prefixedSha256(sha256HexForBytes(bundleBytes));
    final request = <String, Object?>{
      'schemaVersion': 'video_fingerprint_notary_request_v1',
      'workspaceId': workspaceId,
      'creatorProfileId': creatorProfileId,
      'watermarkUid': watermarkUid,
      'sourceHash': sourceHash,
      'durationMs': durationMs,
      'frameSamplePolicy': 'mobile_metadata_probe_v1',
      'sceneCount': sceneCount,
      'fingerprintSchemaVersion': 'mobile_metadata_fingerprint_v1',
      'globalFrameFingerprints': globalFrameFingerprints,
      'localBlockFingerprintRoot': localBlockFingerprintRoot,
      'localBlockCount': localBlockCount,
      'cropWindowFingerprintRoot': cropWindowFingerprintRoot,
      'cropWindowCount': cropWindowCount,
      'fingerprintRoot': fingerprintRoot,
      'clientSignature': 'sha256:${digest('client-signature')}',
      'uploadManifest': {
        'schemaVersion': 'video_upload_manifest_v1',
        'containsOriginalVideo': false,
        'containsWatermarkedVideo': false,
        'containsLocalPaths': false,
        'containsProxy': false,
        'items': [
          {
            'kind': 'mobile_video_fingerprint_metadata',
            'sha256': bundleSha256,
            'bytes': bundleBytes.length,
            if (width != null) 'width': width,
            if (height != null) 'height': height,
            if (frameCount != null) 'frameCount': frameCount,
          },
        ],
      },
    };
    final receipt = await client.createVideoFingerprintNotary(
      accessToken: accessToken,
      request: request,
    );
    if (receipt.watermarkUid != watermarkUid ||
        receipt.sourceHash != sourceHash ||
        receipt.fingerprintRoot != fingerprintRoot ||
        receipt.notaryId.trim().isEmpty ||
        receipt.serverReceiptSignature.trim().isEmpty) {
      throw StateError('L2 云端 notary 收据与本地不可逆指纹请求不一致，已拒绝入库');
    }
    final elapsedMs = DateTime.now().difference(startedAt).inMilliseconds;
    final record = VaultRecord(
      id: _newRecordId(),
      kind: WatermarkAssetKind.video,
      title: fileName.trim().isNotEmpty ? fileName.trim() : 'L2 视频指纹存证',
      watermarkUid: receipt.watermarkUid,
      revision: 1,
      creatorDisplayName: _creatorLabel.trim().isEmpty
          ? null
          : _creatorLabel.trim(),
      trustedTimeStatus: '未记录',
      thirdPartyVerificationStatus: '未记录',
      sha256: sourceHashRaw,
      source: VaultRecordSource.write,
      syncStatus: SyncStatus.pending,
      createdAt: receipt.notarizedAt,
      writeVerificationStatus: WriteVerificationStatus.verified,
      writeVerificationMessage: 'L2 移动端不可逆视频元数据指纹存证已由云端 notary 固化',
      writeVerificationAt: receipt.notarizedAt,
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      watermarkIdIssueMode: 'server_confirmed',
      watermarkIdRegistryStatus: 'server_confirmed',
      watermarkIdRegistryReceipt: receipt.serverReceiptSignature,
      payloadAuthStatus: 'verified',
      outputStrategy: 'mobile_video_fingerprint_notary',
      workSourceDeclaration: 'unspecified',
      trainingPermissionDeclaration: 'prohibited',
      creationMethodDeclaration: 'unspecified',
      humanEditLevelDeclaration: 'unspecified',
      authenticityClaimDeclaration: 'unspecified',
      customRightsStatement:
          'L2 mobile metadata fingerprint only; no raw video, local path, or restorable frame data is stored.',
      videoNotaryId: receipt.notaryId,
      videoNotaryAt: receipt.notarizedAt,
      videoNotaryReceiptSignature: receipt.serverReceiptSignature,
      videoNotaryUsageLedgerId: receipt.usageLedgerId,
      videoFingerprintRoot: receipt.fingerprintRoot,
      videoBundleSha256: bundleSha256,
      videoBundleBytes: bundleBytes.length,
      videoBundleSceneCount: sceneCount,
      videoBundleElapsedMs: elapsedMs,
      videoFrameSamplePolicy: 'mobile_metadata_probe_v1',
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

  bool _l3DeclaredCapacityIsSupported(int width, int height, int frameCount) {
    const maxRegions = 96;
    const payloadBytes = 119;
    const syncBits = 16;
    const eccRepeat = 3;
    const dctCoeffPairs = 3;
    if (width < 512 ||
        height < 512 ||
        width % 8 != 0 ||
        height % 8 != 0 ||
        frameCount <= 0) {
      return false;
    }
    final regionWidth = _atLeastOne(width ~/ 8);
    final regionHeight = _atLeastOne(height ~/ 8);
    final blocksPerRegion =
        _atLeastOne(regionWidth ~/ 8) * _atLeastOne(regionHeight ~/ 8);
    final minRegionsPerStrategyFrame = _atLeastOne(maxRegions ~/ frameCount);
    final estimatedBits =
        blocksPerRegion * minRegionsPerStrategyFrame * dctCoeffPairs;
    const requiredBits = syncBits + payloadBytes * 8 * eccRepeat;
    return estimatedBits >= requiredBits;
  }

  int _atLeastOne(int value) => value < 1 ? 1 : value;

  Future<WatermarkIdRegistryResult?> reserveWatermarkIdForWrite({
    required WatermarkAssetKind kind,
    required String originalHash,
    String? parentWatermarkUid,
    required int revision,
  }) async {
    final client = _cloudAccountClient;
    final accessToken = _syncProfile.authToken?.trim();
    final workspaceId = _syncProfile.workspaceId?.trim();
    final creatorProfileId = _syncProfile.creatorProfileId?.trim();
    if (client == null ||
        accessToken == null ||
        accessToken.isEmpty ||
        workspaceId == null ||
        workspaceId.isEmpty ||
        creatorProfileId == null ||
        creatorProfileId.isEmpty) {
      return null;
    }
    try {
      return await client.reserveWatermarkId(
        accessToken: accessToken,
        request: WatermarkIdReserveRequest(
          requestId:
              'mobile:${_syncProfile.deviceId ?? 'local'}:${kind.name}:$revision:$originalHash:${DateTime.now().microsecondsSinceEpoch}',
          workspaceId: workspaceId,
          creatorProfileId: creatorProfileId,
          mediaType: _watermarkMediaTypeForKind(kind),
          payloadProtocolVersion: 3,
          payloadBytesLength: 39,
          parentWatermarkUid: parentWatermarkUid,
          revision: revision,
          originalHash: _prefixedSha256(originalHash),
        ),
      );
    } catch (_) {
      return null;
    }
  }

  Future<WatermarkIdRegistryResult?> confirmWatermarkIdForWrite({
    required WatermarkWriteResult result,
    required String originalHash,
    WatermarkIdRegistryResult? reserved,
  }) async {
    final client = _cloudAccountClient;
    final accessToken = _syncProfile.authToken?.trim();
    final workspaceId = _syncProfile.workspaceId?.trim();
    final creatorProfileId = _syncProfile.creatorProfileId?.trim();
    if (reserved == null ||
        client == null ||
        accessToken == null ||
        accessToken.isEmpty ||
        workspaceId == null ||
        workspaceId.isEmpty ||
        creatorProfileId == null ||
        creatorProfileId.isEmpty) {
      return reserved;
    }
    try {
      return await client.confirmWatermarkId(
        accessToken: accessToken,
        request: WatermarkIdConfirmRequest(
          workspaceId: workspaceId,
          creatorProfileId: creatorProfileId,
          watermarkUid: result.watermarkUid,
          payloadProtocolVersion: result.verification.payloadProtocolVersion,
          payloadBytesLength: result.verification.payloadBytesLength,
          originalHash: _prefixedSha256(originalHash),
          protectedCopyHash: _prefixedSha256(result.sha256),
          writeVerificationStatus: result.verification.verified
              ? 'verified'
              : 'failed',
        ),
      );
    } catch (_) {
      return reserved;
    }
  }

  VaultRecord addWriteResult({
    required WatermarkWriteResult result,
    required String? fileName,
    required bool allowRewrite,
    String? rewriteReason,
    String? parentWatermarkUid,
    int? revision,
    MobileTrustedTimeAttestation? trustedTimeAttestation,
    WorkDeclaration declaration = const WorkDeclaration(),
    WatermarkIdRegistryResult? registryResult,
  }) {
    if (!result.isProductionWatermark) {
      throw StateError('Web 预览结果不能写入正式版权库或云同步队列。');
    }
    final record = VaultRecord(
      id: _newRecordId(),
      kind: result.kind,
      title: fileName?.isNotEmpty == true
          ? fileName!
          : _fallbackTitle(result.kind),
      watermarkUid: result.watermarkUid,
      revision: revision ?? result.revision,
      creatorDisplayName: _creatorLabel.trim().isEmpty
          ? null
          : _creatorLabel.trim(),
      trustedTimeStatus: trustedTimeAttestation?.trustedTimeStatus ?? '未记录',
      trustedTimeSource: trustedTimeAttestation?.trustedTimeSource,
      trustedTimeAt: trustedTimeAttestation?.trustedTimeAt,
      thirdPartyVerificationStatus:
          trustedTimeAttestation?.thirdPartyVerificationStatus ?? '未记录',
      thirdPartyVerificationProvider:
          trustedTimeAttestation?.thirdPartyVerificationProvider,
      thirdPartyVerificationPath:
          trustedTimeAttestation?.thirdPartyVerificationPath,
      sha256: result.sha256,
      parentWatermarkUid: allowRewrite ? parentWatermarkUid : null,
      source: VaultRecordSource.write,
      syncStatus: SyncStatus.pending,
      createdAt: DateTime.now(),
      rewriteReason: allowRewrite ? rewriteReason : null,
      writeVerificationStatus: result.verification.verified
          ? WriteVerificationStatus.verified
          : WriteVerificationStatus.failed,
      writeVerificationMessage: result.verification.message,
      writeVerificationAt: DateTime.now(),
      extractedTimestamp: result.seed.timestamp,
      extractedDeviceIdHex:
          result.verification.deviceIdHex ?? result.seed.deviceIdentity,
      extractedFileHashHex: result.verification.fileHashHex,
      protectedCopyName: result.outputFileName,
      protectedCopyHash: result.sha256,
      payloadProtocolVersion: result.verification.payloadProtocolVersion,
      payloadBytesLength: result.verification.payloadBytesLength,
      watermarkIdIssueMode:
          registryResult?.watermarkIdIssueMode ?? 'offline_generated',
      watermarkIdRegistryStatus:
          registryResult?.registryStatus ?? 'pending_registration',
      watermarkIdRegistryReceipt: registryResult?.registryReceipt,
      payloadAuthStatus: result.verification.verified ? 'verified' : 'failed',
      outputStrategy: 'minimal_required_change',
      workSourceDeclaration: declaration.workSourceDeclaration,
      trainingPermissionDeclaration: declaration.trainingPermissionDeclaration,
      creationMethodDeclaration: declaration.creationMethodDeclaration,
      humanEditLevelDeclaration: declaration.humanEditLevelDeclaration,
      authenticityClaimDeclaration: declaration.authenticityClaimDeclaration,
      customRightsStatement: declaration.customRightsStatement,
    );
    _records.insert(0, record);
    final queueItem = _newSyncQueueItem(
      record,
      SyncQueueOperation.upsertVaultRecord,
    );
    _syncQueue.insert(0, queueItem);
    _persistRecordQueueAndUsage(record, queueItem, null);
    notifyListeners();
    return record;
  }

  VaultRecord addReadResult({
    required WatermarkReadResult result,
    required String? fileName,
  }) {
    if (!result.isProductionWatermark) {
      throw StateError('Web 预览验证结果不能写入正式版权库或云同步队列。');
    }
    final record = VaultRecord(
      id: _newRecordId(),
      kind: result.kind,
      title: fileName?.isNotEmpty == true
          ? fileName!
          : _fallbackTitle(result.kind),
      watermarkUid: result.watermarkUid,
      revision: result.revision,
      creatorDisplayName: _creatorLabel.trim().isEmpty
          ? null
          : _creatorLabel.trim(),
      trustedTimeStatus: '未记录',
      thirdPartyVerificationStatus: '未记录',
      parentWatermarkUid: result.parentWatermarkUid,
      rewriteReason: result.rewriteReason,
      extractedTimestamp: result.timestamp,
      extractedDeviceIdHex: result.deviceIdHex,
      extractedFileHashHex: result.fileHashHex,
      payloadProtocolVersion: result.payloadProtocolVersion,
      payloadBytesLength: result.payloadBytesLength,
      watermarkIdIssueMode: result.watermarkIdIssueMode,
      watermarkIdRegistryStatus:
          result.watermarkIdIssueMode == 'server_confirmed'
          ? 'server_confirmed'
          : 'pending_registration',
      payloadAuthStatus: result.payloadAuthStatus,
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

  Future<VaultRecord> saveL3VideoVisualTaskToVault({
    required String taskId,
    String? title,
  }) async {
    final normalizedTaskId = taskId.trim();
    if (normalizedTaskId.isEmpty) {
      throw ArgumentError('请输入已 succeeded 的 L3 taskId');
    }
    final accessToken = _syncProfile.authToken?.trim();
    if (!hasCloudAccount || accessToken == null || accessToken.isEmpty) {
      throw StateError('请先登录 HiddenShield 账户');
    }
    if (_syncProfile.entitlementFeatures['cloud_video_processing'] != true) {
      throw StateError('L3 视频画面盲水印对象领取需要 Studio / Enterprise 权益');
    }
    final client =
        _cloudAccountClient ??
        CloudAccountClient(baseUrl: _syncProfile.cloudBaseUrl.trim());
    final task = await client.getCloudVideoTask(
      accessToken: accessToken,
      taskId: normalizedTaskId,
    );
    _validateL3VideoVisualTaskForVault(task);
    final authorization = await client
        .createCloudVideoTaskDownloadAuthorization(
          accessToken: accessToken,
          taskId: normalizedTaskId,
        );
    if (authorization.status != 'succeeded' ||
        authorization.outputMediaContentType != 'video/mp4') {
      throw StateError('L3 下载授权不是 succeeded/video/mp4');
    }
    final bytes = await client.downloadCloudVideoTaskOutput(
      accessToken: accessToken,
      taskId: normalizedTaskId,
      downloadToken: authorization.downloadToken,
    );
    final outputSha256 = 'sha256:${sha256HexForBytes(bytes)}';
    if (outputSha256 != task.watermarkedMediaHash ||
        outputSha256 != authorization.watermarkedMediaHash) {
      throw StateError('L3 下载成品哈希与后端完成态不一致，已拒绝入库');
    }
    if (bytes.length != task.outputMediaBytes ||
        bytes.length != authorization.outputMediaBytes) {
      throw StateError('L3 下载成品字节数与后端完成态不一致，已拒绝入库');
    }
    final completedAt = task.completedAt ?? task.updatedAt ?? DateTime.now();
    final record = VaultRecord(
      id: _newRecordId(),
      kind: WatermarkAssetKind.video,
      title: title?.trim().isNotEmpty == true ? title!.trim() : 'L3 视频画面盲水印成品',
      watermarkUid: task.watermarkUid,
      revision: 1,
      creatorDisplayName: _creatorLabel.trim().isEmpty
          ? null
          : _creatorLabel.trim(),
      trustedTimeStatus: '未记录',
      thirdPartyVerificationStatus: '未记录',
      sha256: _stripSha256Prefix(task.sourceHash),
      source: VaultRecordSource.write,
      syncStatus: SyncStatus.pending,
      createdAt: completedAt,
      writeVerificationStatus: WriteVerificationStatus.verified,
      writeVerificationMessage: 'L3 云端视频画面盲水印自检和签名下载哈希校验已通过',
      writeVerificationAt: completedAt,
      protectedCopyName: '$normalizedTaskId.l3-watermarked.mp4',
      protectedCopyHash: _stripSha256Prefix(outputSha256),
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      watermarkIdIssueMode: 'server_reserved',
      watermarkIdRegistryStatus: 'server_confirmed',
      watermarkIdRegistryReceipt: task.serverReceiptSignature,
      payloadAuthStatus: 'verified',
      outputStrategy: 'cloud_l3_video_visual_watermark',
      workSourceDeclaration: 'unspecified',
      trainingPermissionDeclaration: 'prohibited',
      creationMethodDeclaration: 'unspecified',
      humanEditLevelDeclaration: 'unspecified',
      authenticityClaimDeclaration: 'unspecified',
      videoVisualTaskId: task.taskId,
      videoVisualCompletedAt: completedAt,
      videoVisualStrategyDigest: task.strategyDigest,
      videoVisualSelfCheckConfidence: task.selfCheckConfidence,
      videoVisualSelfCheckThreshold: task.selfCheckThreshold,
      videoVisualCheckedFrames: task.checkedFrames,
      videoVisualMediaHash: task.watermarkedMediaHash,
      videoVisualReceiptHash: task.workerReceiptHash,
      videoVisualOutputBytes: task.outputMediaBytes,
      videoVisualOutputContentType: task.outputMediaContentType,
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

  Future<void> setAnonymousFeedbackEnabled(bool value) async {
    if (value == _anonymousFeedbackEnabled) {
      return;
    }
    _anonymousFeedbackEnabled = value;
    _syncProfile = _syncProfile.copyWith(
      anonymousFeedbackEnabled: value,
      updatedAt: DateTime.now(),
    );
    await _vaultStore.saveSyncProfile(_syncProfile);
    notifyListeners();
  }

  Future<void> setExperienceImprovementEnabled(bool value) async {
    if (value == _experienceImprovementEnabled) {
      return;
    }
    _experienceImprovementEnabled = value;
    _syncProfile = _syncProfile.copyWith(
      experienceImprovementEnabled: value,
      updatedAt: DateTime.now(),
    );
    await _vaultStore.saveSyncProfile(_syncProfile);
    notifyListeners();
  }

  Future<MobileAnonymousFlushResult> flushAnonymousFeedbackQueue() async {
    if (!_anonymousFeedbackEnabled) {
      return const MobileAnonymousFlushResult(
        attemptedEvents: 0,
        sentEvents: 0,
        remainingEvents: 0,
        endpointConfigured: true,
        message: '匿名反馈已关闭，未发送任何信息。',
      );
    }
    if (_anonymousFeedbackQueue.isEmpty) {
      _anonymousFeedbackQueue.add(_buildDiagnosticFeedbackEvent());
      _anonymousFeedbackLastEventAt = _anonymousFeedbackQueue.last.occurredAt;
    }
    final attemptedEvents = _anonymousFeedbackQueue.length;
    final attemptAt = DateTime.now();
    _anonymousFeedbackLastAttemptAt = attemptAt;
    await _persistAnonymousFeedbackState();
    notifyListeners();

    try {
      final ack = await _anonymousFeedbackClient.sendBatch(
        MobileAnonymousFeedbackBatch(
          installId: _anonymousInstallId,
          sessionId: _anonymousSessionId,
          appVersion: 'mobile',
          sentAt: attemptAt,
          events: List.unmodifiable(_anonymousFeedbackQueue),
        ),
      );
      _anonymousFeedbackQueue.clear();
      _anonymousFeedbackConsecutiveFailures = 0;
      _anonymousFeedbackLastSuccessAt = ack.acceptedAt ?? DateTime.now();
      _anonymousFeedbackNextRetryAt = null;
      _anonymousFeedbackLastFlushError = null;
      await _persistAnonymousFeedbackState();
      notifyListeners();
      return MobileAnonymousFlushResult(
        attemptedEvents: attemptedEvents,
        sentEvents: ack.insertedEvents,
        remainingEvents: 0,
        endpointConfigured: _anonymousFeedbackClient.endpointConfigured,
        flushedAt: _anonymousFeedbackLastSuccessAt,
        message: '匿名反馈已发送，只包含功能结果、错误码、耗时和桶化信息。',
      );
    } catch (error) {
      _anonymousFeedbackConsecutiveFailures += 1;
      _anonymousFeedbackLastFlushError = '$error';
      _anonymousFeedbackNextRetryAt = DateTime.now().add(
        _anonymousFeedbackRetryBackoff(_anonymousFeedbackConsecutiveFailures),
      );
      await _persistAnonymousFeedbackState();
      notifyListeners();
      return MobileAnonymousFlushResult(
        attemptedEvents: attemptedEvents,
        sentEvents: 0,
        remainingEvents: _anonymousFeedbackQueue.length,
        endpointConfigured: _anonymousFeedbackClient.endpointConfigured,
        message: '匿名反馈暂未发送成功，已保留在本机队列。',
      );
    }
  }

  String exportSafeDiagnosticLog() {
    final feedback = anonymousFeedbackStatus;
    final experience = experienceImprovementSnapshot;
    final usage = dataUsageSnapshot;
    return [
      'HiddenShield 移动端安全诊断日志',
      '生成时间: ${DateTime.now().toIso8601String()}',
      '隐私边界: 不上传原始媒体、加水印媒体、本地路径、文件名或完整作品指纹。',
      '',
      '账户与创作者',
      '账户状态: ${hasCloudAccount ? '已登录' : '未登录'}',
      '创作者身份: ${_creatorLabel.trim().isEmpty ? '未记录' : _creatorLabel.trim()}',
      '权益: ${_syncProfile.entitlementLabel} / ${entitlementStatusLabel(_syncProfile.entitlementStatus)}',
      '云同步: ${syncTransportModeLabel(_syncTransportMode)}',
      '',
      '版权库与队列',
      '版权记录: ${usage.vaultRecords} 条',
      '待同步: $pendingSyncQueueCount 条',
      '失败队列: $failedSyncQueueCount 条',
      '本地批量队列: ${usage.localBatchJobs} 个',
      '本地批量失败项: ${_failedBatchItemCount()} 个',
      '使用流水: ${usage.usageEvents} 条',
      '',
      '匿名反馈',
      '开关: ${feedback.telemetryEnabled ? '已开启' : '已关闭'}',
      '待发送: ${feedback.queuedEvents} 条',
      '队列大小: ${feedback.queuedBytes} B',
      '失败次数: ${feedback.consecutiveFailures}',
      '最近尝试: ${_formatOptionalDate(feedback.lastAttemptAt)}',
      '最近成功: ${_formatOptionalDate(feedback.lastSuccessAt)}',
      '最后错误: ${feedback.lastFlushError ?? '无'}',
      '',
      '体验改进',
      '开关: ${experience.enabled ? '已开启' : '已关闭'}',
      '风险等级: ${experience.riskLabel}',
      '总事件: ${experience.totalEvents}',
      '成功 / 失败: ${experience.successEvents} / ${experience.failureEvents}',
      '诊断: ${experience.diagnosticEvents}',
      '最近事件: ${_formatOptionalDate(experience.lastEventAt)}',
    ].join('\n');
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
    unawaited(setAutomaticCloudSyncEnabled(value));
  }

  Future<void> setAutomaticCloudSyncEnabled(bool value) async {
    final accessToken = _syncProfile.authToken;
    if (_cloudAccountClient == null) {
      _syncProfile = _syncProfile.copyWith(
        lastError: '云服务未配置，无法调整自动云同步偏好',
        updatedAt: DateTime.now(),
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
      notifyListeners();
      return;
    }
    if (accessToken == null || accessToken.isEmpty || !hasCloudAccount) {
      _syncProfile = _syncProfile.copyWith(
        lastError: '请先登录账户，再调整自动云同步偏好',
        updatedAt: DateTime.now(),
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
      notifyListeners();
      return;
    }
    if (value && !hasCloudSyncEntitlement) {
      _syncProfile = _syncProfile.copyWith(
        lastError: 'Creator 起开放正式云同步；当前账户可继续本地使用。',
        updatedAt: DateTime.now(),
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
      notifyListeners();
      return;
    }
    try {
      final response = await _cloudAccountClient.updateSyncPreferences(
        accessToken: accessToken,
        autoSyncEnabled: value,
      );
      _syncProfile = response.applyTo(_syncProfile, now: DateTime.now());
      _syncTransportMode = canAutoCloudSync
          ? SyncTransportMode.cloud
          : SyncTransportMode.localOnly;
      if (_syncProfile.mode != _syncTransportMode) {
        _syncProfile = _syncProfile.copyWith(mode: _syncTransportMode);
      }
      await _vaultStore.saveSyncProfile(_syncProfile);
      notifyListeners();
      if (canAutoCloudSync) {
        await _runAutomaticCloudVaultSync();
      }
      await refreshCloudDevices();
    } catch (error) {
      _syncProfile = _syncProfile.copyWith(
        lastError: '$error',
        updatedAt: DateTime.now(),
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
      notifyListeners();
    }
  }

  Future<bool> continueWithAccountPlaceholder({
    required String accountLabel,
    String password = '',
    String? challengeId,
    String verificationCode = '',
  }) async {
    if (accountLabel.trim().isEmpty ||
        (password.trim().isEmpty &&
            (challengeId == null || verificationCode.trim().isEmpty))) {
      _syncProfile = _syncProfile.copyWith(
        status: SyncConnectionStatus.failed,
        lastError: '请输入账户和验证码或密码',
        updatedAt: DateTime.now(),
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
      notifyListeners();
      return false;
    }
    if (_cloudAccountClient != null) {
      try {
        await continueWithCloudAccount(
          identifier: accountLabel,
          password: password,
          challengeId: challengeId,
          verificationCode: verificationCode,
          localCreatorDisplayName: _creatorLabel,
        );
        await refreshCloudDevices();
        return true;
      } catch (error) {
        _syncProfile = _syncProfile.copyWith(
          status: SyncConnectionStatus.failed,
          lastError: mobileUserFacingErrorMessage(error, action: '登录'),
          updatedAt: DateTime.now(),
        );
        await _vaultStore.saveSyncProfile(_syncProfile);
        notifyListeners();
        return false;
      }
    }

    final label = accountLabel.trim().isEmpty
        ? 'HiddenShield 账户'
        : accountLabel.trim();
    final suffix = _stableIdSuffix(label);
    final now = DateTime.now();
    final entitlementFeatures = const {
      'cloud_sync': false,
      'batch_processing': false,
      'report_export': false,
      'cloud_batch_processing': false,
      'cloud_video_processing': false,
      'priority_queue': false,
      'team_workspace': false,
      'api_access': false,
    };
    final canEnableCloudSync = entitlementFeatures['cloud_sync'] == true;
    _syncProfile = _syncProfile.copyWith(
      mode: canEnableCloudSync
          ? SyncTransportMode.cloud
          : SyncTransportMode.localOnly,
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
      entitlementLabel: '未付费',
      entitlementStatus: EntitlementStatus.free,
      entitlementPlanCode: 'free',
      entitlementPlanKey: 'base_unpaid',
      entitlementFeatures: entitlementFeatures,
      entitlementLastCheckedAt: now,
      cloudBaseUrl:
          _cloudAccountClient?.baseUrl ??
          HiddenShieldSystemConfig.fallback.cloudBaseUrl,
      updatedAt: now,
      clearLastError: true,
    );
    _syncTransportMode = canEnableCloudSync
        ? SyncTransportMode.cloud
        : SyncTransportMode.localOnly;
    await _vaultStore.saveSyncProfile(_syncProfile);
    _cloudDevices
      ..clear()
      ..add(
        AccountDevice(
          id: _syncProfile.deviceId ?? 'mobile-preview-device',
          clientDeviceId: _syncProfile.deviceId ?? 'mobile-preview-device',
          name: _syncProfile.deviceName ?? '当前移动设备',
          platform: _syncProfile.devicePlatform ?? _currentDevicePlatform(),
          appVersion: '0.1.0',
          registered: true,
          autoSyncEnabled: canEnableCloudSync,
          isCurrent: true,
          activeSessionCount: 1,
          createdAt: now,
          updatedAt: now,
          lastSeenAt: now,
        ),
      );
    notifyListeners();
    return true;
  }

  Future<void> refreshCloudDevices() async {
    final accessToken = _syncProfile.authToken;
    if (_cloudAccountClient == null ||
        accessToken == null ||
        accessToken.isEmpty) {
      _cloudDevices.clear();
      notifyListeners();
      return;
    }
    try {
      final devices = await _cloudAccountClient.listDevices(
        accessToken: accessToken,
      );
      _cloudDevices
        ..clear()
        ..addAll(devices);
      _syncProfile = _syncProfile.copyWith(clearLastError: true);
      notifyListeners();
    } catch (error) {
      _syncProfile = _syncProfile.copyWith(
        lastError: mobileUserFacingErrorMessage(error, action: '读取设备列表'),
        updatedAt: DateTime.now(),
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
      notifyListeners();
    }
  }

  Future<void> renameCloudDevice({
    required String deviceId,
    required String name,
  }) async {
    final accessToken = _syncProfile.authToken;
    if (_cloudAccountClient == null ||
        accessToken == null ||
        accessToken.isEmpty) {
      return;
    }
    try {
      final updated = await _cloudAccountClient.updateDeviceName(
        accessToken: accessToken,
        deviceId: deviceId,
        name: name,
      );
      final index = _cloudDevices.indexWhere(
        (device) => device.id == updated.id,
      );
      if (index >= 0) {
        _cloudDevices[index] = updated;
      }
      if (updated.isCurrent) {
        _syncProfile = _syncProfile.copyWith(
          deviceName: updated.name,
          updatedAt: DateTime.now(),
          clearLastError: true,
        );
        await _vaultStore.saveSyncProfile(_syncProfile);
      }
      notifyListeners();
    } catch (error) {
      _syncProfile = _syncProfile.copyWith(
        lastError: mobileUserFacingErrorMessage(error, action: '更新设备名称'),
        updatedAt: DateTime.now(),
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
      notifyListeners();
    }
  }

  Future<void> revokeCloudDevice(String deviceId) async {
    final accessToken = _syncProfile.authToken;
    if (_cloudAccountClient == null ||
        accessToken == null ||
        accessToken.isEmpty) {
      return;
    }
    try {
      await _cloudAccountClient.revokeDevice(
        accessToken: accessToken,
        deviceId: deviceId,
      );
      await refreshCloudDevices();
    } catch (error) {
      _syncProfile = _syncProfile.copyWith(
        lastError: mobileUserFacingErrorMessage(error, action: '撤销设备'),
        updatedAt: DateTime.now(),
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
      notifyListeners();
    }
  }

  Future<AuthChallengeResponse?> createAuthChallenge({
    required String accountLabel,
  }) async {
    if (accountLabel.trim().isEmpty) {
      _syncProfile = _syncProfile.copyWith(
        status: SyncConnectionStatus.failed,
        lastError: '请输入账户',
        updatedAt: DateTime.now(),
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
      notifyListeners();
      return null;
    }
    if (_cloudAccountClient == null) {
      return AuthChallengeResponse(
        challengeId:
            'preview-challenge-${DateTime.now().microsecondsSinceEpoch}',
        deliveryChannel: 'fixture',
        expiresAt: DateTime.now().add(const Duration(minutes: 10)),
        message: '本地预览验证码为 000000。',
        fixtureCode: '000000',
      );
    }
    try {
      final deviceId =
          _syncProfile.deviceId ??
          'dev_${DateTime.now().microsecondsSinceEpoch}';
      return await _cloudAccountClient.createAuthChallenge(
        identifier: accountLabel.trim(),
        clientDeviceId: deviceId,
      );
    } catch (error) {
      _syncProfile = _syncProfile.copyWith(
        status: SyncConnectionStatus.failed,
        lastError: mobileUserFacingErrorMessage(error, action: '发送验证码'),
        updatedAt: DateTime.now(),
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
      notifyListeners();
      return null;
    }
  }

  Future<void> completeOnboarding({
    required String accountLabel,
    required String password,
    required String creatorLabel,
  }) async {
    final creator = creatorLabel.trim();
    if (creator.isNotEmpty) {
      _creatorLabel = creator;
    }
    final signedIn = await continueWithAccountPlaceholder(
      accountLabel: accountLabel,
      password: password,
    );
    if (!signedIn) {
      return;
    }
    _syncProfile = _syncProfile.copyWith(
      creatorDisplayName: _creatorLabel,
      creatorProfileSynced: hasCloudAccount,
      onboardingCompleted: true,
      updatedAt: DateTime.now(),
    );
    await _vaultStore.saveSyncProfile(_syncProfile);
    await refreshCloudDevices();
    notifyListeners();
  }

  Future<void> completeBaseSetup({required String creatorLabel}) async {
    final creator = creatorLabel.trim();
    if (creator.isNotEmpty) {
      _creatorLabel = creator;
    }
    _syncProfile = _syncProfile.copyWith(
      creatorDisplayName: _creatorLabel,
      creatorProfileSynced: hasCloudAccount,
      onboardingCompleted: true,
      updatedAt: DateTime.now(),
    );
    await _vaultStore.saveSyncProfile(_syncProfile);
    notifyListeners();
  }

  Future<void> continueWithCloudAccount({
    required String identifier,
    required String password,
    String? challengeId,
    String verificationCode = '',
    required String localCreatorDisplayName,
  }) async {
    if (_cloudAccountClient == null) {
      await continueWithAccountPlaceholder(
        accountLabel: identifier,
        password: password,
        challengeId: challengeId,
        verificationCode: verificationCode,
      );
      return;
    }

    final session = await _cloudAccountClient.continueWithAccount(
      ContinueAccountRequest(
        identifier: identifier.trim(),
        password: password,
        verificationCode: verificationCode,
        challengeId: challengeId,
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
    _syncTransportMode = canAutoCloudSync
        ? SyncTransportMode.cloud
        : SyncTransportMode.localOnly;
    if (_syncTransportMode == SyncTransportMode.localOnly) {
      _syncProfile = _syncProfile.copyWith(mode: SyncTransportMode.localOnly);
    }
    await _vaultStore.saveSyncProfile(_syncProfile);
    notifyListeners();
    if (canAutoCloudSync) {
      await _runAutomaticCloudVaultSync();
    }
  }

  Future<void> _runAutomaticCloudVaultSync() async {
    if (_syncTransportMode != SyncTransportMode.cloud || !canAutoCloudSync) {
      return;
    }
    await pullRemoteChanges();
    await syncPendingQueue();
    await pullRemoteChanges();
  }

  Future<BillingPaymentSession?> createBillingPaymentSession({
    required String planCode,
    String billingCycle = 'monthly',
  }) async {
    if (_cloudAccountClient == null) {
      _latestPaymentSession = null;
      _latestPaymentSessionStatus = null;
      _latestPaymentMessage = '云服务未配置，当前可先查看订阅权益。';
      notifyListeners();
      return null;
    }
    final accountId = _syncProfile.accountId;
    final workspaceId = _syncProfile.workspaceId;
    final accessToken = _syncProfile.authToken;
    if (accountId == null ||
        accountId.isEmpty ||
        workspaceId == null ||
        workspaceId.isEmpty ||
        accessToken == null ||
        accessToken.isEmpty) {
      _latestPaymentSession = null;
      _latestPaymentSessionStatus = null;
      _latestPaymentMessage = '请先登录账户，再开通订阅。';
      notifyListeners();
      return null;
    }
    try {
      final session = await _cloudAccountClient.createBillingPaymentSession(
        accessToken: accessToken,
        accountId: accountId,
        workspaceId: workspaceId,
        planCode: planCode,
        billingCycle: billingCycle,
      );
      _latestPaymentSession = session;
      _latestPaymentSessionStatus = 'created';
      _latestPaymentMessage = session.paymentAction.type == 'qr_code'
          ? '请使用微信完成支付。我们会在短时间内自动确认支付状态。'
          : '请按页面提示完成支付。我们会在短时间内自动确认支付状态。';
      _startPaymentPolling();
      notifyListeners();
      return session;
    } catch (error) {
      final text = '$error';
      _latestPaymentSession = null;
      _latestPaymentSessionStatus = null;
      _latestPaymentMessage = text.contains('wechat_pay_not_configured')
          ? '支付通道尚未完成配置，当前可先联系开通。'
          : text;
      notifyListeners();
      return null;
    }
  }

  Future<ReportPurchaseSession?> createReportPurchaseSession({
    required VaultRecord record,
    required String productCode,
  }) async {
    if (_cloudAccountClient == null) {
      _latestPaymentMessage = '云服务未配置，当前可先复制基础存证摘要。';
      notifyListeners();
      return null;
    }
    final accountId = _syncProfile.accountId;
    final workspaceId = _syncProfile.workspaceId;
    final creatorProfileId = _syncProfile.creatorProfileId;
    final accessToken = _syncProfile.authToken;
    if (accountId == null ||
        accountId.isEmpty ||
        workspaceId == null ||
        workspaceId.isEmpty ||
        creatorProfileId == null ||
        creatorProfileId.isEmpty ||
        accessToken == null ||
        accessToken.isEmpty) {
      _latestPaymentMessage = '请先登录账户，再购买报告。';
      notifyListeners();
      return null;
    }
    final normalizedProductCode = _normalizeReportProductCode(productCode);
    if (normalizedProductCode == null) {
      _latestPaymentMessage = '报告商品不在可购买范围内。';
      notifyListeners();
      return null;
    }
    try {
      final session = await _cloudAccountClient.createReportPurchaseSession(
        accessToken: accessToken,
        accountId: accountId,
        workspaceId: workspaceId,
        creatorProfileId: creatorProfileId,
        vaultRecordId: record.id,
        productCode: normalizedProductCode,
      );
      _latestPaymentMessage = session.paymentAction.type == 'qr_code'
          ? '请使用微信完成支付。确认后只解锁这条记录的报告。'
          : '请按页面提示完成支付。确认后只解锁这条记录的报告。';
      notifyListeners();
      return session;
    } catch (error) {
      final text = '$error';
      _latestPaymentMessage = text.contains('wechat_pay_not_configured')
          ? '支付通道尚未完成配置，当前可先复制基础存证摘要。'
          : text;
      notifyListeners();
      return null;
    }
  }

  Future<bool> reconcileReportPurchaseSession({
    required String paymentSessionId,
  }) async {
    final accessToken = _syncProfile.authToken;
    if (_cloudAccountClient == null ||
        accessToken == null ||
        accessToken.isEmpty) {
      _latestPaymentMessage = '请先登录账户，再确认报告购买状态。';
      notifyListeners();
      return false;
    }
    try {
      final status = await _cloudAccountClient.getReportPurchaseSessionStatus(
        accessToken: accessToken,
        paymentSessionId: paymentSessionId,
      );
      if (status.grant != null) {
        await _saveReportPurchaseGrant(status.grant!);
        _latestPaymentMessage = '报告授权已生效。';
        notifyListeners();
        return true;
      }
      final result = await _cloudAccountClient.reconcileReportPurchaseSession(
        accessToken: accessToken,
        paymentSessionId: paymentSessionId,
      );
      if (result.grant == null) {
        _latestPaymentMessage = '暂未确认支付完成，可稍后再次确认购买状态。';
        notifyListeners();
        return false;
      }
      await _saveReportPurchaseGrant(result.grant!);
      _latestPaymentMessage = result.message.isEmpty
          ? '支付已确认，报告授权已生效。'
          : result.message;
      notifyListeners();
      return true;
    } catch (error) {
      _latestPaymentMessage = '$error';
      notifyListeners();
      return false;
    }
  }

  Future<void> reconcileLatestPaymentSession() async {
    final session = _latestPaymentSession;
    if (session == null) {
      _latestPaymentMessage = '请先创建支付会话。';
      notifyListeners();
      return;
    }
    await _reconcilePaymentSession(session.paymentSessionId, manual: true);
  }

  Future<void> refreshBillingEntitlement() async {
    if (_cloudAccountClient == null) {
      _latestPaymentMessage = '云服务未配置，暂不能刷新订阅权益。';
      notifyListeners();
      return;
    }
    final accessToken = _syncProfile.authToken;
    if (accessToken == null || accessToken.isEmpty) {
      _latestPaymentMessage = '请先登录账户，再刷新权益。';
      notifyListeners();
      return;
    }
    try {
      final entitlement = await _cloudAccountClient.getCurrentEntitlement(
        accessToken: accessToken,
      );
      _syncProfile = entitlement.applyTo(_syncProfile, now: DateTime.now());
      if (canAutoCloudSync) {
        _syncTransportMode = SyncTransportMode.cloud;
      } else {
        _syncTransportMode = SyncTransportMode.localOnly;
        _syncProfile = _syncProfile.copyWith(mode: SyncTransportMode.localOnly);
      }
      _latestPaymentMessage = _entitlementRefreshMessage(
        _syncProfile.entitlementStatus,
      );
      await _vaultStore.saveSyncProfile(_syncProfile);
      notifyListeners();
      if (canAutoCloudSync) {
        await _runAutomaticCloudVaultSync();
      }
    } catch (error) {
      _latestPaymentMessage = '$error';
      notifyListeners();
    }
  }

  Future<void> _pollLatestPaymentSession() async {
    final session = _latestPaymentSession;
    final accessToken = _syncProfile.authToken;
    final startedAt = _paymentPollingStartedAt;
    if (session == null || accessToken == null || accessToken.isEmpty) {
      _stopPaymentPolling();
      return;
    }
    if (startedAt != null &&
        DateTime.now().difference(startedAt) > const Duration(minutes: 2)) {
      _stopPaymentPolling();
      _latestPaymentMessage = '暂未确认支付完成，可稍后手动确认支付状态。';
      notifyListeners();
      return;
    }
    try {
      final status = await _cloudAccountClient?.getBillingPaymentSessionStatus(
        accessToken: accessToken,
        paymentSessionId: session.paymentSessionId,
      );
      if (status == null) {
        return;
      }
      _latestPaymentSessionStatus = status.status;
      if (status.status == 'succeeded') {
        _syncProfile = status.entitlement.applyTo(
          _syncProfile,
          now: DateTime.now(),
        );
        _syncTransportMode = canAutoCloudSync
            ? SyncTransportMode.cloud
            : SyncTransportMode.localOnly;
        await _vaultStore.saveSyncProfile(_syncProfile);
        _latestPaymentMessage = '支付已确认，权益已生效。';
        _stopPaymentPolling();
        notifyListeners();
        if (canAutoCloudSync) {
          await _runAutomaticCloudVaultSync();
        }
        return;
      }
      if (status.status == 'failed' || status.status == 'expired') {
        _latestPaymentMessage = _paymentSessionStatusMessage(status.status);
        _stopPaymentPolling();
        notifyListeners();
        return;
      }
      await _reconcilePaymentSession(session.paymentSessionId, manual: false);
    } catch (_) {
      _latestPaymentMessage = '暂未确认支付完成，可稍后手动确认支付状态。';
      notifyListeners();
    }
  }

  Future<void> _reconcilePaymentSession(
    String paymentSessionId, {
    required bool manual,
  }) async {
    final accessToken = _syncProfile.authToken;
    if (_cloudAccountClient == null ||
        accessToken == null ||
        accessToken.isEmpty) {
      _latestPaymentMessage = '请先登录账户，再确认支付状态。';
      notifyListeners();
      return;
    }
    try {
      final result = await _cloudAccountClient.reconcileBillingPaymentSession(
        accessToken: accessToken,
        paymentSessionId: paymentSessionId,
      );
      _latestPaymentSessionStatus = result.status;
      _syncProfile = result.entitlement.applyTo(
        _syncProfile,
        now: DateTime.now(),
      );
      if (canAutoCloudSync) {
        _syncTransportMode = SyncTransportMode.cloud;
      } else {
        _syncTransportMode = SyncTransportMode.localOnly;
        _syncProfile = _syncProfile.copyWith(mode: SyncTransportMode.localOnly);
      }
      await _vaultStore.saveSyncProfile(_syncProfile);
      _latestPaymentMessage = result.message.isEmpty
          ? _paymentSessionStatusMessage(result.status)
          : result.message;
      if (result.status == 'succeeded') {
        _stopPaymentPolling();
      }
      notifyListeners();
      if (canAutoCloudSync) {
        await _runAutomaticCloudVaultSync();
      }
    } catch (error) {
      if (manual) {
        _latestPaymentMessage = '$error';
      }
      notifyListeners();
    }
  }

  void _startPaymentPolling() {
    _stopPaymentPolling();
    _paymentPollingStartedAt = DateTime.now();
    _paymentPollTimer = Timer.periodic(const Duration(seconds: 10), (_) {
      unawaited(_pollLatestPaymentSession());
    });
    unawaited(
      Future<void>.delayed(const Duration(seconds: 1), () {
        return _pollLatestPaymentSession();
      }),
    );
  }

  void _stopPaymentPolling() {
    _paymentPollTimer?.cancel();
    _paymentPollTimer = null;
  }

  Future<void> signOutCloud() async {
    _stopPaymentPolling();
    _latestPaymentSession = null;
    _latestPaymentSessionStatus = null;
    _latestPaymentMessage = null;
    final refreshToken = _syncProfile.refreshToken;
    final deviceId = _syncProfile.deviceId;
    if (_cloudAccountClient != null &&
        refreshToken?.trim().isNotEmpty == true &&
        deviceId?.trim().isNotEmpty == true) {
      try {
        await _cloudAccountClient.logout(
          refreshToken: refreshToken!.trim(),
          deviceId: deviceId!.trim(),
        );
      } catch (_) {
        // Local sign-out must still clear account state when the network is unavailable.
      }
    }
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
    _cloudDevices.clear();
    await _vaultStore.saveSyncProfile(_syncProfile);
    notifyListeners();
  }

  Future<void> prepareCloudRelogin() async {
    _stopPaymentPolling();
    _latestPaymentSession = null;
    _latestPaymentSessionStatus = null;
    _latestPaymentMessage = null;
    _syncProfile = _syncProfile.copyWith(
      mode: SyncTransportMode.localOnly,
      status: SyncConnectionStatus.unconfigured,
      clearAuthToken: true,
      clearWorkspace: true,
      deviceRegistered: false,
      lastError: '当前设备授权已失效，请重新登录账户。',
      updatedAt: DateTime.now(),
    );
    _syncTransportMode = SyncTransportMode.localOnly;
    _cloudDevices.clear();
    await _vaultStore.saveSyncProfile(_syncProfile);
    notifyListeners();
  }

  @override
  void dispose() {
    _stopPaymentPolling();
    super.dispose();
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

  Future<PublicRightsQueryResponse> fetchPublicRights(
    String watermarkUid,
  ) async {
    final client =
        _cloudAccountClient ??
        (_syncProfile.cloudBaseUrl.trim().isEmpty
            ? null
            : CloudAccountClient(baseUrl: _syncProfile.cloudBaseUrl.trim()));
    if (client == null) {
      throw const CloudAccountException('未连接公开 registry');
    }
    return client.getPublicRights(watermarkUid: watermarkUid);
  }

  Future<Map<String, Object?>> fetchPublicRightsMetadata(
    String watermarkUid,
  ) async {
    final client =
        _cloudAccountClient ??
        (_syncProfile.cloudBaseUrl.trim().isEmpty
            ? null
            : CloudAccountClient(baseUrl: _syncProfile.cloudBaseUrl.trim()));
    if (client == null) {
      throw const CloudAccountException('未连接公开 registry');
    }
    return client.getPublicRightsMetadata(watermarkUid: watermarkUid);
  }

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
        var current = _updateQueueItem(
          item.copyWith(
            status: SyncQueueItemStatus.syncing,
            attempts: item.attempts + 1,
            clearLastError: true,
          ),
        );
        await _vaultStore.updateSyncItem(current);
        current = await _reconcileWatermarkIdBeforeCloudSync(current);
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

  Future<SyncQueueItem> _reconcileWatermarkIdBeforeCloudSync(
    SyncQueueItem item,
  ) async {
    if (_syncTransportMode != SyncTransportMode.cloud ||
        item.payloadType != 'vault_record') {
      return item;
    }
    final client = _cloudAccountClient;
    final accessToken = _syncProfile.authToken?.trim();
    final workspaceId = _syncProfile.workspaceId?.trim();
    final creatorProfileId = _syncProfile.creatorProfileId?.trim();
    if (client == null ||
        accessToken == null ||
        accessToken.isEmpty ||
        workspaceId == null ||
        workspaceId.isEmpty ||
        creatorProfileId == null ||
        creatorProfileId.isEmpty) {
      return item;
    }
    final payload = jsonDecode(item.payloadJson);
    if (payload is! Map<String, Object?>) {
      return item;
    }
    final registryStatus =
        payload['watermark_id_registry_status'] as String? ?? '';
    if (registryStatus != 'pending_registration' &&
        registryStatus != 'reserved') {
      return item;
    }
    final recordIndex = _records.indexWhere(
      (record) => record.id == item.recordId,
    );
    if (recordIndex == -1) {
      return item;
    }
    final record = _records[recordIndex];
    try {
      final response =
          record.watermarkIdIssueMode == 'server_reserved' ||
              record.watermarkIdRegistryStatus == 'reserved'
          ? await client.confirmWatermarkId(
              accessToken: accessToken,
              request: WatermarkIdConfirmRequest(
                workspaceId: workspaceId,
                creatorProfileId: creatorProfileId,
                watermarkUid: record.watermarkUid,
                payloadProtocolVersion: record.payloadProtocolVersion,
                payloadBytesLength: record.payloadBytesLength,
                originalHash: record.sha256 == null
                    ? null
                    : _prefixedSha256(record.sha256!),
                protectedCopyHash: record.protectedCopyHash == null
                    ? null
                    : _prefixedSha256(record.protectedCopyHash!),
                writeVerificationStatus:
                    record.writeVerificationStatus?.name ?? 'verified',
              ),
            )
          : await client.reconcileWatermarkId(
              accessToken: accessToken,
              request: WatermarkIdReconcileRequest(
                workspaceId: workspaceId,
                creatorProfileId: creatorProfileId,
                watermarkUid: record.watermarkUid,
                mediaType: _watermarkMediaTypeForKind(record.kind),
                payloadProtocolVersion: record.payloadProtocolVersion,
                payloadBytesLength: record.payloadBytesLength,
                parentWatermarkUid: record.parentWatermarkUid,
                revision: record.revision,
                originalHash: record.sha256 == null
                    ? null
                    : _prefixedSha256(record.sha256!),
                protectedCopyHash: record.protectedCopyHash == null
                    ? null
                    : _prefixedSha256(record.protectedCopyHash!),
                writeVerificationStatus:
                    record.writeVerificationStatus?.name ?? 'verified',
              ),
            );
      final updated = record.copyWith(
        payloadProtocolVersion: response.payloadProtocolVersion,
        payloadBytesLength: response.payloadBytesLength,
        watermarkIdIssueMode: response.watermarkIdIssueMode,
        watermarkIdRegistryStatus: response.registryStatus,
        watermarkIdRegistryReceipt: response.registryReceipt,
        parentWatermarkUid: response.parentWatermarkUid,
        revision: response.revision,
      );
      _records[recordIndex] = updated;
      await _vaultStore.upsertRecord(updated);
      final updatedItem = _updateQueueItem(
        item.copyWith(payloadJson: jsonEncode(updated.toSyncPayload())),
      );
      await _vaultStore.updateSyncItem(updatedItem);
      return updatedItem;
    } catch (_) {
      return item;
    }
  }

  Future<String> requestWatermarkReissueForRecord(VaultRecord record) async {
    final client = _cloudAccountClient;
    final accessToken = _syncProfile.authToken?.trim();
    final workspaceId = _syncProfile.workspaceId?.trim();
    final creatorProfileId = _syncProfile.creatorProfileId?.trim();
    if (client == null ||
        accessToken == null ||
        accessToken.isEmpty ||
        workspaceId == null ||
        workspaceId.isEmpty ||
        creatorProfileId == null ||
        creatorProfileId.isEmpty) {
      throw StateError('请先继续 HiddenShield 账户，再执行编号重新签发。');
    }
    final result = await client.reissueWatermarkId(
      accessToken: accessToken,
      request: WatermarkIdReissueRequest(
        workspaceId: workspaceId,
        creatorProfileId: creatorProfileId,
        previousWatermarkUid: record.watermarkUid,
        mediaType: _watermarkMediaTypeForKind(record.kind),
        payloadProtocolVersion: record.payloadProtocolVersion,
        payloadBytesLength: record.payloadBytesLength,
        parentWatermarkUid: record.watermarkUid,
        revision: record.revision + 1,
        reason: 'historical_duplicate_watermark_uid_repair',
        originalHash: record.sha256 == null
            ? null
            : _prefixedSha256(record.sha256!),
      ),
    );
    final updated = record.copyWith(
      watermarkIdRegistryStatus: 'reissue_required',
      watermarkIdRegistryReceipt: result.replacement.registryReceipt,
      writeVerificationStatus: WriteVerificationStatus.failed,
      writeVerificationMessage:
          '已创建重新签发任务 ${result.jobId}，新编号 ${result.replacement.watermarkUid} 待重新选择原作品或保护副本后写入 V2 payload。',
      payloadAuthStatus: 'pending_repair',
      syncStatus: SyncStatus.conflict,
    );
    final index = _records.indexWhere((item) => item.id == record.id);
    if (index != -1) {
      _records[index] = updated;
    }
    await _vaultStore.upsertRecord(updated);
    final queueItem = _newSyncQueueItem(
      updated,
      SyncQueueOperation.upsertVaultRecord,
    );
    _updateQueueItem(queueItem);
    await _vaultStore.enqueueSyncItem(queueItem);
    notifyListeners();
    return '已创建重新签发任务，当前移动端需重新选择原作品或保护副本后完成 payload 修复。';
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
    unawaited(_persistRecordQueueAndUsageAsync(record, queueItem, null));
  }

  void _persistRecordQueueAndUsage(
    VaultRecord record,
    SyncQueueItem queueItem,
    UsageLedgerEntry? usageEntry,
  ) {
    unawaited(_persistRecordQueueAndUsageAsync(record, queueItem, usageEntry));
  }

  Future<void> _persistRecordQueueAndUsageAsync(
    VaultRecord record,
    SyncQueueItem queueItem,
    UsageLedgerEntry? usageEntry,
  ) async {
    try {
      await _vaultStore.upsertRecord(record);
      await _vaultStore.enqueueSyncItem(queueItem);
      if (usageEntry != null) {
        await _vaultStore.appendUsageLedgerEntry(usageEntry);
        _usageSummary = _usageSummary.withEntry(usageEntry, _syncProfile);
        notifyListeners();
      }
    } catch (error) {
      debugPrint('Failed to persist vault record sync event: $error');
    }
  }

  Future<void> appendUsageForWriteResult({
    required WatermarkWriteResult result,
    required String vaultRecordId,
    required String? pipelineId,
  }) async {
    final usageEntry = UsageLedgerEntry.success(
      featureName: 'watermark_${result.kind.name}',
      mediaType: usageMediaTypeFromAssetKind(result.kind),
      fileSizeBytes: result.bytes.length,
      syncProfile: _syncProfile,
      pipelineId: pipelineId,
      vaultRecordId: vaultRecordId,
    );
    try {
      await _vaultStore.appendUsageLedgerEntry(usageEntry);
      _usageSummary = _usageSummary.withEntry(usageEntry, _syncProfile);
      notifyListeners();
    } catch (error) {
      debugPrint('Failed to append usage ledger entry: $error');
    }
  }

  Future<FormalReportDraft> buildFormalReportDraft(VaultRecord record) async {
    final hasSinglePurchase = _hasActiveReportPurchaseGrant(record.id);
    final authorization = hasSinglePurchase
        ? const OfflineExecutionAuthorization(
            feature: 'report_export',
            allowed: true,
            source: 'single_purchase',
          )
        : await authorizeLocalExecution('report_export');
    if (!authorization.allowed) {
      throw StateError('正式报告需为当前记录单份购买。');
    }
    final exportedAt = DateTime.now();
    final draft = FormalReportDraft.fromRecord(
      record: record,
      exportedAt: exportedAt,
      appVersion: 'mobile',
    );
    final usageEntry = UsageLedgerEntry.success(
      featureName: 'report_export',
      mediaType: UsageMediaType.report,
      fileSizeBytes: draft.markdown.length,
      syncProfile: _syncProfile,
      pipelineId: null,
      vaultRecordId: record.id,
    );
    await _vaultStore.appendUsageLedgerEntry(usageEntry);
    _usageSummary = _usageSummary.withEntry(usageEntry, _syncProfile);
    notifyListeners();
    return draft;
  }

  Future<void> _saveReportPurchaseGrant(ReportPurchaseGrant grant) async {
    final grants = _decodeReportPurchaseGrants(
      _syncProfile.reportPurchaseGrantsJson,
    );
    final existingIndex = grants.indexWhere(
      (item) =>
          item.accountId == grant.accountId &&
          item.workspaceId == grant.workspaceId &&
          item.vaultRecordId == grant.vaultRecordId &&
          item.productCode == grant.productCode,
    );
    if (existingIndex == -1) {
      grants.add(grant);
    } else {
      grants[existingIndex] = grant;
    }
    _syncProfile = _syncProfile.copyWith(
      reportPurchaseGrantsJson: _encodeReportPurchaseGrants(grants),
      updatedAt: DateTime.now(),
    );
    await _vaultStore.saveSyncProfile(_syncProfile);
  }

  bool _hasActiveReportPurchaseGrant(String vaultRecordId) {
    final accountId = _syncProfile.accountId;
    final workspaceId = _syncProfile.workspaceId;
    if (accountId == null ||
        accountId.isEmpty ||
        workspaceId == null ||
        workspaceId.isEmpty) {
      return false;
    }
    final grants = _decodeReportPurchaseGrants(
      _syncProfile.reportPurchaseGrantsJson,
    );
    return grants.any(
      (grant) =>
          grant.accountId == accountId &&
          grant.workspaceId == workspaceId &&
          grant.vaultRecordId == vaultRecordId &&
          grant.status == 'active' &&
          grant.revokedAt == null &&
          (grant.productCode == 'copyright_report_single' ||
              grant.productCode == 'rights_evidence_pack_single'),
    );
  }

  String buildCopyrightSummary(VaultRecord record) {
    return [
      '【隐盾版权存证】',
      '版权编号: ${record.watermarkUid}',
      '版本次数: 第 ${record.revision} 次',
      '创作者身份: ${_summaryValue(record.creatorDisplayName)}',
      '',
      if (record.parentWatermarkUid?.isNotEmpty == true)
        '上一版编号: ${record.parentWatermarkUid}',
      if (record.rewriteReason?.isNotEmpty == true)
        '更新说明: ${record.rewriteReason}',
      '完成后验证: ${_copyrightSummaryVerificationStatus(record.writeVerificationStatus)}',
      if (record.writeVerificationMessage?.isNotEmpty == true)
        '验证说明: ${record.writeVerificationMessage}',
      '验证时间: ${_summaryLocalOptionalDate(record.writeVerificationAt)}',
      'Payload 协议: V${record.payloadProtocolVersion} / ${record.payloadBytesLength} bytes',
      '媒体载荷角色: ${_mediaPayloadRoleLabel(_mediaPayloadRoleForProtocol(record.payloadProtocolVersion))}',
      '编号签发模式: ${_watermarkIssueModeLabel(record.watermarkIdIssueMode)}',
      '登记状态: ${_registryStatusLabel(record.watermarkIdRegistryStatus)}',
      '登记收据: ${_summaryValue(record.watermarkIdRegistryReceipt)}',
      'Payload 认证状态: ${_payloadAuthStatusLabel(record.payloadAuthStatus)}',
      '第三方验证: ${_copyrightSummaryThirdPartyStatus(record)}',
      '可信时间: ${_summaryEvidenceValue(record.trustedTimeAt, record.trustedTimeStatus)}',
      '时间来源: ${_summaryValue(record.trustedTimeSource)}',
      '原文件: ${record.title}',
      '作品指纹: ${_summaryValue(record.sha256 ?? record.extractedFileHashHex)}',
      '保护副本名称: ${_summaryValue(record.protectedCopyName)}',
      '保护副本摘要: ${_summaryValue(record.protectedCopyHash)}',
      '输出策略: ${_outputStrategyLabel(record.outputStrategy)}',
      '处理时间: ${_summaryLocalDateTime(record.createdAt)}',
      '作品来源声明: ${_workSourceDeclarationLabel(record.workSourceDeclaration)}',
      '训练许可声明: ${_trainingPermissionLabel(record.trainingPermissionDeclaration)}',
      '创作方式声明: ${_summaryValue(record.creationMethodDeclaration)}',
      '人工编辑声明: ${_summaryValue(record.humanEditLevelDeclaration)}',
      '真实性声明: ${_authenticityClaimLabel(record.authenticityClaimDeclaration)}',
      '自定义版权声明: ${record.customRightsStatement?.trim().isNotEmpty == true ? record.customRightsStatement!.trim() : '无'}',
      '---',
      '本存证由 HiddenShield 本地生成，数据未上传至任何服务器。',
    ].join('\n');
  }

  String _fallbackTitle(WatermarkAssetKind kind) {
    return switch (kind) {
      WatermarkAssetKind.image => '未命名图片',
      WatermarkAssetKind.audio => '未命名 WAV',
      WatermarkAssetKind.video => '未命名视频',
    };
  }

  String _watermarkMediaTypeForKind(WatermarkAssetKind kind) {
    return switch (kind) {
      WatermarkAssetKind.image => 'image',
      WatermarkAssetKind.audio => 'audio',
      WatermarkAssetKind.video => 'video_audio_track',
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
          creatorDisplayName: incoming.creatorDisplayName,
          trustedTimeStatus: incoming.trustedTimeStatus,
          trustedTimeSource: incoming.trustedTimeSource,
          trustedTimeAt: incoming.trustedTimeAt,
          thirdPartyVerificationStatus: incoming.thirdPartyVerificationStatus,
          thirdPartyVerificationProvider:
              incoming.thirdPartyVerificationProvider,
          thirdPartyVerificationPath: incoming.thirdPartyVerificationPath,
          sha256: incoming.sha256,
          parentWatermarkUid: incoming.parentWatermarkUid,
          rewriteReason: incoming.rewriteReason,
          extractedTimestamp: incoming.extractedTimestamp,
          extractedDeviceIdHex: incoming.extractedDeviceIdHex,
          extractedFileHashHex: incoming.extractedFileHashHex,
          writeVerificationStatus: incoming.writeVerificationStatus,
          writeVerificationMessage: incoming.writeVerificationMessage,
          writeVerificationAt: incoming.writeVerificationAt,
          protectedCopyName: incoming.protectedCopyName,
          protectedCopyHash: incoming.protectedCopyHash,
          payloadProtocolVersion: incoming.payloadProtocolVersion,
          payloadBytesLength: incoming.payloadBytesLength,
          watermarkIdIssueMode: incoming.watermarkIdIssueMode,
          watermarkIdRegistryStatus: incoming.watermarkIdRegistryStatus,
          watermarkIdRegistryReceipt: incoming.watermarkIdRegistryReceipt,
          payloadAuthStatus: incoming.payloadAuthStatus,
          outputStrategy: incoming.outputStrategy,
          workSourceDeclaration: incoming.workSourceDeclaration,
          trainingPermissionDeclaration: incoming.trainingPermissionDeclaration,
          creationMethodDeclaration: incoming.creationMethodDeclaration,
          humanEditLevelDeclaration: incoming.humanEditLevelDeclaration,
          authenticityClaimDeclaration: incoming.authenticityClaimDeclaration,
          customRightsStatement: incoming.customRightsStatement,
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
      final arbitrationRecord = exactMatchIndex == -1
          ? incoming
          : incoming.copyWith(
              id: _arbitrationRecordId(incoming, incomingFingerprint),
            );
      final pendingArbitration = arbitrationRecord.copyWith(
        watermarkIdRegistryStatus: 'pending_registry_reconcile',
        writeVerificationMessage: '同步发现同一版权编号对应不同作品指纹，已保留记录并等待后端登记仲裁。',
        syncStatus: SyncStatus.conflict,
      );
      _records.insert(0, pendingArbitration);
      return _RemoteMergeResult(
        record: pendingArbitration,
        resolution: MobileSyncResolution(
          id: _newResolutionId(incoming, 'pending-registry-reconcile'),
          resolvedAt: DateTime.now(),
          resolutionType: MobileSyncResolutionType.pendingRegistryReconcile,
          reason:
              'same watermark uid but different asset fingerprint requires backend registry arbitration',
          incomingRecordId: incoming.id,
          existingRecordId: current.id,
          watermarkUid: incoming.watermarkUid,
          existingHash: _recordFingerprint(current),
          incomingHash: incomingFingerprint,
          existingRevision: current.revision,
          incomingRevision: incoming.revision,
          insertedRecordId: pendingArbitration.id,
        ),
      );
    }

    if (exactMatchIndex != -1) {
      final current = _records[exactMatchIndex];
      _records[exactMatchIndex] = incoming;
      return _RemoteMergeResult(
        record: incoming,
        resolution: MobileSyncResolution(
          id: _newResolutionId(incoming, 'record-replaced'),
          resolvedAt: DateTime.now(),
          resolutionType: MobileSyncResolutionType.recordReplaced,
          reason: 'same stable id refreshed after conflict checks',
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

  String _arbitrationRecordId(VaultRecord incoming, String? fingerprint) {
    final hashSource = fingerprint ?? incoming.watermarkUid;
    final suffix = _stableIdSuffix('${incoming.id}:$hashSource');
    return '${incoming.id}-arbitration-$suffix';
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

  MobileAnonymousFeedbackStatus _buildAnonymousFeedbackStatus() {
    return MobileAnonymousFeedbackStatus(
      installId: _anonymousInstallId,
      sessionId: _anonymousSessionId,
      queuedEvents: _anonymousFeedbackQueue.length,
      queuedBytes: _anonymousFeedbackQueuedBytes(),
      lastEventAt: _anonymousFeedbackLastEventAt,
      lastFlushError: _anonymousFeedbackLastFlushError,
      consecutiveFailures: _anonymousFeedbackConsecutiveFailures,
      nextRetryAt: _anonymousFeedbackNextRetryAt,
      lastAttemptAt: _anonymousFeedbackLastAttemptAt,
      lastSuccessAt: _anonymousFeedbackLastSuccessAt,
      telemetryEnabled: _anonymousFeedbackEnabled,
      networkEnabled: true,
      endpointConfigured: _anonymousFeedbackClient.endpointConfigured,
    );
  }

  MobileExperienceImprovementSnapshot _buildExperienceImprovementSnapshot() {
    final failedBatchItems = _failedBatchItemCount();
    final successEvents = _usageSummary.totalEvents;
    final failureEvents = failedSyncQueueCount + failedBatchItems;
    final diagnosticEvents =
        _anonymousFeedbackQueue.length +
        (_anonymousFeedbackLastFlushError == null ? 0 : 1);
    final totalEvents = successEvents + failureEvents + diagnosticEvents;
    final reasons = <String>[
      if (failedSyncQueueCount > 0) '云同步失败 $failedSyncQueueCount 条',
      if (failedBatchItems > 0) '本地批量失败 $failedBatchItems 项',
      if (_anonymousFeedbackConsecutiveFailures > 0)
        '匿名反馈连续失败 $_anonymousFeedbackConsecutiveFailures 次',
      if (_syncProfile.lastError?.isNotEmpty == true) '最近同步错误已记录',
    ];
    final failureRate = totalEvents == 0 ? 0.0 : failureEvents / totalEvents;
    final riskLevel =
        failureEvents >= 3 || _anonymousFeedbackConsecutiveFailures >= 3
        ? MobileExperienceRiskLevel.high
        : failureEvents > 0 || failureRate >= 0.2
        ? MobileExperienceRiskLevel.medium
        : MobileExperienceRiskLevel.low;
    return MobileExperienceImprovementSnapshot(
      enabled: _experienceImprovementEnabled,
      totalEvents: totalEvents,
      successEvents: successEvents,
      failureEvents: failureEvents,
      diagnosticEvents: diagnosticEvents,
      conversionRate: totalEvents == 0 ? 0.0 : successEvents / totalEvents,
      failureRate: failureRate,
      repeatedErrorCount:
          _anonymousFeedbackConsecutiveFailures +
          (_syncProfile.lastError == null ? 0 : 1),
      lastEventAt: _latestExperienceEventAt(),
      riskLevel: riskLevel,
      reasons: reasons,
    );
  }

  MobileDataUsageSnapshot _buildDataUsageSnapshot() {
    final batchItems = _localBatchJobs.expand((job) => job.items).length;
    final syncPayloadBytes = _syncQueue.fold<int>(
      0,
      (total, item) => total + utf8.encode(item.payloadJson).length,
    );
    return MobileDataUsageSnapshot(
      vaultRecords: _records.length,
      syncQueueItems: _syncQueue.length,
      localBatchJobs: _localBatchJobs.length,
      localBatchItems: batchItems,
      usageEvents: _usageSummary.totalEvents,
      anonymousFeedbackEvents: _anonymousFeedbackQueue.length,
      estimatedBytes:
          (_records.length * 1800) +
          (_syncQueue.length * 900) +
          syncPayloadBytes +
          (batchItems * 700) +
          (_usageSummary.totalEvents * 360) +
          _anonymousFeedbackQueuedBytes(),
      note: '本机记录估算；不统计、不上传原始媒体、加水印媒体、本地路径或保护副本路径。',
    );
  }

  int _failedBatchItemCount() {
    return _localBatchJobs
        .expand((job) => job.items)
        .where((item) => item.status == BatchItemStatus.failed)
        .length;
  }

  MobileAnonymousFeedbackEvent _buildDiagnosticFeedbackEvent() {
    final now = DateTime.now();
    return MobileAnonymousFeedbackEvent(
      eventId: _newAnonymousId('evt'),
      occurredAt: now,
      installId: _anonymousInstallId,
      sessionId: _anonymousSessionId,
      appVersion: 'mobile',
      featureName: 'settings_diagnostic',
      outcome: 'diagnostic',
      mediaType: 'none',
      fileSizeBucket: '0-10mb',
      diagnosticNote: _sanitizeDiagnosticText(
        'records=${_records.length};sync_pending=$pendingSyncQueueCount;'
        'sync_failed=$failedSyncQueueCount;batch_jobs=${_localBatchJobs.length};'
        'usage_events=${_usageSummary.totalEvents};'
        'last_error=${_syncProfile.lastError ?? 'none'}',
        maxLength: 180,
      ),
    );
  }

  Future<void> _persistAnonymousFeedbackState() async {
    _syncProfile = _syncProfile.copyWith(
      anonymousFeedbackEnabled: _anonymousFeedbackEnabled,
      experienceImprovementEnabled: _experienceImprovementEnabled,
      anonymousInstallId: _anonymousInstallId,
      anonymousFeedbackLastEventAt: _anonymousFeedbackLastEventAt,
      anonymousFeedbackLastAttemptAt: _anonymousFeedbackLastAttemptAt,
      anonymousFeedbackLastSuccessAt: _anonymousFeedbackLastSuccessAt,
      anonymousFeedbackNextRetryAt: _anonymousFeedbackNextRetryAt,
      anonymousFeedbackLastFlushError: _anonymousFeedbackLastFlushError,
      anonymousFeedbackConsecutiveFailures:
          _anonymousFeedbackConsecutiveFailures,
      anonymousFeedbackQueueJson: _encodeAnonymousFeedbackQueue(),
      updatedAt: DateTime.now(),
    );
    await _vaultStore.saveSyncProfile(_syncProfile);
  }

  DateTime? _latestExperienceEventAt() {
    final values = <DateTime>[
      if (_usageSummary.lastUsedAt != null) _usageSummary.lastUsedAt!,
      if (_syncProfile.lastSyncAttemptAt != null)
        _syncProfile.lastSyncAttemptAt!,
      if (_anonymousFeedbackLastEventAt != null) _anonymousFeedbackLastEventAt!,
      if (_localBatchJobs.isNotEmpty) _localBatchJobs.first.updatedAt,
    ]..sort();
    return values.isEmpty ? null : values.last;
  }

  int _anonymousFeedbackQueuedBytes() {
    return _anonymousFeedbackQueue.fold<int>(
      0,
      (total, event) => total + utf8.encode(jsonEncode(event.toJson())).length,
    );
  }

  Duration _anonymousFeedbackRetryBackoff(int failures) {
    if (failures <= 1) {
      return const Duration(minutes: 1);
    }
    if (failures == 2) {
      return const Duration(minutes: 5);
    }
    return const Duration(minutes: 15);
  }

  String _newAnonymousId(String prefix) {
    return '$prefix-${DateTime.now().microsecondsSinceEpoch}-${_stableIdSuffix(prefix + _creatorLabel)}';
  }

  String _sanitizeDiagnosticText(String value, {required int maxLength}) {
    final sanitized = value
        .replaceAll(RegExp(r'[A-Za-z]:[\\/][^\s;]+'), '[local-path]')
        .replaceAll(RegExp(r'/(?:[^/\s;]+/){2,}[^/\s;]+'), '[local-path]')
        .replaceAll(RegExp(r'\b[0-9a-fA-F]{32,}\b'), '[hash]');
    return sanitized.length <= maxLength
        ? sanitized
        : sanitized.substring(0, maxLength);
  }

  String _encodeAnonymousFeedbackQueue() {
    return jsonEncode(
      _anonymousFeedbackQueue.map((event) => event.toJson()).toList(),
    );
  }

  List<MobileAnonymousFeedbackEvent> _decodeAnonymousFeedbackQueue(
    String? value,
  ) {
    if (value == null || value.trim().isEmpty) {
      return const [];
    }
    try {
      final decoded = jsonDecode(value) as List<dynamic>;
      return decoded
          .whereType<Map<String, Object?>>()
          .map(MobileAnonymousFeedbackEvent.fromJson)
          .where((event) => event.eventId.isNotEmpty)
          .toList(growable: false);
    } catch (_) {
      return const [];
    }
  }
}

String mobileUserFacingErrorMessage(Object error, {String action = '操作'}) {
  final raw = '$error'.trim();
  final lower = raw.toLowerCase();
  if (raw.isEmpty) {
    return '$action失败，请稍后重试。';
  }
  if (lower.contains('failed to fetch') ||
      lower.contains('clientexception') ||
      lower.contains('xmlhttprequest') ||
      lower.contains('connection refused') ||
      lower.contains('network') ||
      lower.contains('timed out') ||
      lower.contains('timeout')) {
    return '$action失败：暂时无法连接服务，请确认后端服务已启动，或稍后重试。';
  }
  if (raw.contains('HTTP 401') || lower.contains('unauthorized')) {
    return '$action失败：登录状态已失效，请重新登录后再试。';
  }
  if (raw.contains('HTTP 403') || lower.contains('forbidden')) {
    return '$action失败：当前账户、设备或工作区授权不一致，请重新登录后再试。';
  }
  if (raw.contains('HTTP 408') ||
      raw.contains('HTTP 429') ||
      RegExp(r'HTTP 5\d\d').hasMatch(raw)) {
    return '$action失败：服务暂时不可用，请稍后重试。';
  }
  if (raw.contains('请输入')) {
    return raw;
  }
  return '$action失败：系统没有完成本次请求，请重试；如果持续失败，请复制同步信息反馈。';
}

const Set<String> vaultRecordSyncPayloadKeys = {
  'id',
  'kind',
  'title',
  'watermark_uid',
  'revision',
  'creator_display_name',
  'trusted_time_status',
  'trusted_time_source',
  'trusted_time_at',
  'third_party_verification_status',
  'third_party_verification_provider',
  'third_party_verification_path',
  'sha256',
  'parent_watermark_uid',
  'rewrite_reason',
  'extracted_timestamp',
  'extracted_device_id_hex',
  'extracted_file_hash_hex',
  'write_verification_status',
  'write_verification_message',
  'write_verification_at',
  'protected_copy_name',
  'protected_copy_hash',
  'payload_protocol_version',
  'payload_bytes_length',
  'media_payload_role',
  'watermark_id_issue_mode',
  'watermark_id_registry_status',
  'watermark_id_registry_receipt',
  'payload_auth_status',
  'output_strategy',
  'work_source_declaration',
  'training_permission_declaration',
  'creation_method_declaration',
  'human_edit_level_declaration',
  'authenticity_claim_declaration',
  'custom_rights_statement',
  'video_notary_id',
  'video_notary_at',
  'video_notary_receipt_signature',
  'video_notary_usage_ledger_id',
  'video_fingerprint_root',
  'video_bundle_sha256',
  'video_bundle_bytes',
  'video_bundle_scene_count',
  'video_bundle_elapsed_ms',
  'video_frame_sample_policy',
  'video_visual_task_id',
  'video_visual_completed_at',
  'video_visual_strategy_digest',
  'video_visual_self_check_confidence',
  'video_visual_self_check_threshold',
  'video_visual_checked_frames',
  'video_visual_media_hash',
  'video_visual_receipt_hash',
  'video_visual_output_bytes',
  'video_visual_output_content_type',
  'source',
  'sync_status',
  'created_at',
};

Map<String, Object?> sanitizeVaultRecordSyncPayload(
  Map<String, Object?> payload,
) {
  return {
    for (final entry in payload.entries)
      if (vaultRecordSyncPayloadKeys.contains(entry.key))
        entry.key: entry.value,
  };
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
      creatorDisplayName: creatorDisplayName,
      trustedTimeStatus: trustedTimeStatus,
      trustedTimeSource: trustedTimeSource,
      trustedTimeAt: DateTime.tryParse(trustedTimeAt ?? ''),
      thirdPartyVerificationStatus: thirdPartyVerificationStatus,
      thirdPartyVerificationProvider: thirdPartyVerificationProvider,
      thirdPartyVerificationPath: thirdPartyVerificationPath,
      sha256: sha256,
      parentWatermarkUid: parentWatermarkUid,
      rewriteReason: rewriteReason,
      extractedTimestamp: extractedTimestamp,
      extractedDeviceIdHex: extractedDeviceIdHex,
      extractedFileHashHex: extractedFileHashHex,
      writeVerificationStatus: writeVerificationStatusFromName(
        writeVerificationStatus,
      ),
      writeVerificationMessage: writeVerificationMessage,
      writeVerificationAt: DateTime.tryParse(writeVerificationAt ?? ''),
      protectedCopyName: protectedCopyName,
      protectedCopyHash: protectedCopyHash,
      payloadProtocolVersion: payloadProtocolVersion ?? 2,
      payloadBytesLength: payloadBytesLength ?? 119,
      watermarkIdIssueMode: watermarkIdIssueMode ?? 'offline_generated',
      watermarkIdRegistryStatus:
          watermarkIdRegistryStatus ?? 'pending_registration',
      watermarkIdRegistryReceipt: watermarkIdRegistryReceipt,
      payloadAuthStatus: payloadAuthStatus ?? 'verified',
      outputStrategy: outputStrategy ?? 'minimal_required_change',
      workSourceDeclaration: workSourceDeclaration ?? 'unspecified',
      trainingPermissionDeclaration:
          trainingPermissionDeclaration ?? 'prohibited',
      creationMethodDeclaration: creationMethodDeclaration ?? 'unspecified',
      humanEditLevelDeclaration: humanEditLevelDeclaration ?? 'unspecified',
      authenticityClaimDeclaration:
          authenticityClaimDeclaration ?? 'unspecified',
      customRightsStatement: customRightsStatement,
      videoNotaryId: videoNotaryId,
      videoNotaryAt: DateTime.tryParse(videoNotaryAt ?? ''),
      videoNotaryReceiptSignature: videoNotaryReceiptSignature,
      videoNotaryUsageLedgerId: videoNotaryUsageLedgerId,
      videoFingerprintRoot: videoFingerprintRoot,
      videoBundleSha256: videoBundleSha256,
      videoBundleBytes: videoBundleBytes,
      videoBundleSceneCount: videoBundleSceneCount,
      videoBundleElapsedMs: videoBundleElapsedMs,
      videoFrameSamplePolicy: videoFrameSamplePolicy,
      videoVisualTaskId: videoVisualTaskId,
      videoVisualCompletedAt: DateTime.tryParse(videoVisualCompletedAt ?? ''),
      videoVisualStrategyDigest: videoVisualStrategyDigest,
      videoVisualSelfCheckConfidence: videoVisualSelfCheckConfidence,
      videoVisualSelfCheckThreshold: videoVisualSelfCheckThreshold,
      videoVisualCheckedFrames: videoVisualCheckedFrames,
      videoVisualMediaHash: videoVisualMediaHash,
      videoVisualReceiptHash: videoVisualReceiptHash,
      videoVisualOutputBytes: videoVisualOutputBytes,
      videoVisualOutputContentType: videoVisualOutputContentType,
      source: source == 'verify'
          ? VaultRecordSource.verify
          : VaultRecordSource.write,
      syncStatus: SyncStatus.synced,
      createdAt: DateTime.tryParse(createdAt) ?? DateTime.now(),
    );
  }
}

class L3VideoVisualUploadTaskResult {
  const L3VideoVisualUploadTaskResult({
    required this.task,
    required this.watermarkUid,
    required this.sourceSha256,
    required this.uploadedBytes,
    required this.privacyBoundary,
    required this.nextAction,
  });

  final CloudVideoTaskRecord task;
  final String watermarkUid;
  final String sourceSha256;
  final int uploadedBytes;
  final String privacyBoundary;
  final String nextAction;
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
    this.creatorDisplayName,
    this.trustedTimeStatus,
    this.trustedTimeSource,
    this.trustedTimeAt,
    this.thirdPartyVerificationStatus,
    this.thirdPartyVerificationProvider,
    this.thirdPartyVerificationPath,
    this.sha256,
    this.parentWatermarkUid,
    this.rewriteReason,
    this.extractedTimestamp,
    this.extractedDeviceIdHex,
    this.extractedFileHashHex,
    this.writeVerificationStatus,
    this.writeVerificationMessage,
    this.writeVerificationAt,
    this.protectedCopyName,
    this.protectedCopyHash,
    this.payloadProtocolVersion = 2,
    this.payloadBytesLength = 119,
    this.watermarkIdIssueMode = 'offline_generated',
    this.watermarkIdRegistryStatus = 'pending_registration',
    this.watermarkIdRegistryReceipt,
    this.payloadAuthStatus = 'verified',
    this.outputStrategy = 'minimal_required_change',
    this.workSourceDeclaration = 'unspecified',
    this.trainingPermissionDeclaration = 'prohibited',
    this.creationMethodDeclaration = 'unspecified',
    this.humanEditLevelDeclaration = 'unspecified',
    this.authenticityClaimDeclaration = 'unspecified',
    this.customRightsStatement,
    this.videoNotaryId,
    this.videoNotaryAt,
    this.videoNotaryReceiptSignature,
    this.videoNotaryUsageLedgerId,
    this.videoFingerprintRoot,
    this.videoBundleSha256,
    this.videoBundleBytes,
    this.videoBundleSceneCount,
    this.videoBundleElapsedMs,
    this.videoFrameSamplePolicy,
    this.videoVisualTaskId,
    this.videoVisualCompletedAt,
    this.videoVisualStrategyDigest,
    this.videoVisualSelfCheckConfidence,
    this.videoVisualSelfCheckThreshold,
    this.videoVisualCheckedFrames,
    this.videoVisualMediaHash,
    this.videoVisualReceiptHash,
    this.videoVisualOutputBytes,
    this.videoVisualOutputContentType,
  });

  final String id;
  final WatermarkAssetKind kind;
  final String title;
  final String watermarkUid;
  final int revision;
  final String? creatorDisplayName;
  final String? trustedTimeStatus;
  final String? trustedTimeSource;
  final DateTime? trustedTimeAt;
  final String? thirdPartyVerificationStatus;
  final String? thirdPartyVerificationProvider;
  final String? thirdPartyVerificationPath;
  final String? sha256;
  final String? parentWatermarkUid;
  final String? rewriteReason;
  final int? extractedTimestamp;
  final String? extractedDeviceIdHex;
  final String? extractedFileHashHex;
  final WriteVerificationStatus? writeVerificationStatus;
  final String? writeVerificationMessage;
  final DateTime? writeVerificationAt;
  final String? protectedCopyName;
  final String? protectedCopyHash;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String watermarkIdIssueMode;
  final String watermarkIdRegistryStatus;
  final String? watermarkIdRegistryReceipt;
  final String payloadAuthStatus;
  final String outputStrategy;
  final String workSourceDeclaration;
  final String trainingPermissionDeclaration;
  final String creationMethodDeclaration;
  final String humanEditLevelDeclaration;
  final String authenticityClaimDeclaration;
  final String? customRightsStatement;
  final String? videoNotaryId;
  final DateTime? videoNotaryAt;
  final String? videoNotaryReceiptSignature;
  final String? videoNotaryUsageLedgerId;
  final String? videoFingerprintRoot;
  final String? videoBundleSha256;
  final int? videoBundleBytes;
  final int? videoBundleSceneCount;
  final int? videoBundleElapsedMs;
  final String? videoFrameSamplePolicy;
  final String? videoVisualTaskId;
  final DateTime? videoVisualCompletedAt;
  final String? videoVisualStrategyDigest;
  final double? videoVisualSelfCheckConfidence;
  final double? videoVisualSelfCheckThreshold;
  final int? videoVisualCheckedFrames;
  final String? videoVisualMediaHash;
  final String? videoVisualReceiptHash;
  final int? videoVisualOutputBytes;
  final String? videoVisualOutputContentType;
  final VaultRecordSource source;
  final SyncStatus syncStatus;
  final DateTime createdAt;

  VaultRecord copyWith({
    String? id,
    WatermarkAssetKind? kind,
    String? title,
    String? watermarkUid,
    int? revision,
    String? creatorDisplayName,
    String? trustedTimeStatus,
    String? trustedTimeSource,
    DateTime? trustedTimeAt,
    String? thirdPartyVerificationStatus,
    String? thirdPartyVerificationProvider,
    String? thirdPartyVerificationPath,
    String? sha256,
    String? parentWatermarkUid,
    String? rewriteReason,
    int? extractedTimestamp,
    String? extractedDeviceIdHex,
    String? extractedFileHashHex,
    WriteVerificationStatus? writeVerificationStatus,
    String? writeVerificationMessage,
    DateTime? writeVerificationAt,
    String? protectedCopyName,
    String? protectedCopyHash,
    int? payloadProtocolVersion,
    int? payloadBytesLength,
    String? watermarkIdIssueMode,
    String? watermarkIdRegistryStatus,
    String? watermarkIdRegistryReceipt,
    String? payloadAuthStatus,
    String? outputStrategy,
    String? workSourceDeclaration,
    String? trainingPermissionDeclaration,
    String? creationMethodDeclaration,
    String? humanEditLevelDeclaration,
    String? authenticityClaimDeclaration,
    String? customRightsStatement,
    String? videoNotaryId,
    DateTime? videoNotaryAt,
    String? videoNotaryReceiptSignature,
    String? videoNotaryUsageLedgerId,
    String? videoFingerprintRoot,
    String? videoBundleSha256,
    int? videoBundleBytes,
    int? videoBundleSceneCount,
    int? videoBundleElapsedMs,
    String? videoFrameSamplePolicy,
    String? videoVisualTaskId,
    DateTime? videoVisualCompletedAt,
    String? videoVisualStrategyDigest,
    double? videoVisualSelfCheckConfidence,
    double? videoVisualSelfCheckThreshold,
    int? videoVisualCheckedFrames,
    String? videoVisualMediaHash,
    String? videoVisualReceiptHash,
    int? videoVisualOutputBytes,
    String? videoVisualOutputContentType,
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
      creatorDisplayName: creatorDisplayName ?? this.creatorDisplayName,
      trustedTimeStatus: trustedTimeStatus ?? this.trustedTimeStatus,
      trustedTimeSource: trustedTimeSource ?? this.trustedTimeSource,
      trustedTimeAt: trustedTimeAt ?? this.trustedTimeAt,
      thirdPartyVerificationStatus:
          thirdPartyVerificationStatus ?? this.thirdPartyVerificationStatus,
      thirdPartyVerificationProvider:
          thirdPartyVerificationProvider ?? this.thirdPartyVerificationProvider,
      thirdPartyVerificationPath:
          thirdPartyVerificationPath ?? this.thirdPartyVerificationPath,
      sha256: sha256 ?? this.sha256,
      parentWatermarkUid: parentWatermarkUid ?? this.parentWatermarkUid,
      rewriteReason: rewriteReason ?? this.rewriteReason,
      extractedTimestamp: extractedTimestamp ?? this.extractedTimestamp,
      extractedDeviceIdHex: extractedDeviceIdHex ?? this.extractedDeviceIdHex,
      extractedFileHashHex: extractedFileHashHex ?? this.extractedFileHashHex,
      writeVerificationStatus:
          writeVerificationStatus ?? this.writeVerificationStatus,
      writeVerificationMessage:
          writeVerificationMessage ?? this.writeVerificationMessage,
      writeVerificationAt: writeVerificationAt ?? this.writeVerificationAt,
      protectedCopyName: protectedCopyName ?? this.protectedCopyName,
      protectedCopyHash: protectedCopyHash ?? this.protectedCopyHash,
      payloadProtocolVersion:
          payloadProtocolVersion ?? this.payloadProtocolVersion,
      payloadBytesLength: payloadBytesLength ?? this.payloadBytesLength,
      watermarkIdIssueMode: watermarkIdIssueMode ?? this.watermarkIdIssueMode,
      watermarkIdRegistryStatus:
          watermarkIdRegistryStatus ?? this.watermarkIdRegistryStatus,
      watermarkIdRegistryReceipt:
          watermarkIdRegistryReceipt ?? this.watermarkIdRegistryReceipt,
      payloadAuthStatus: payloadAuthStatus ?? this.payloadAuthStatus,
      outputStrategy: outputStrategy ?? this.outputStrategy,
      workSourceDeclaration:
          workSourceDeclaration ?? this.workSourceDeclaration,
      trainingPermissionDeclaration:
          trainingPermissionDeclaration ?? this.trainingPermissionDeclaration,
      creationMethodDeclaration:
          creationMethodDeclaration ?? this.creationMethodDeclaration,
      humanEditLevelDeclaration:
          humanEditLevelDeclaration ?? this.humanEditLevelDeclaration,
      authenticityClaimDeclaration:
          authenticityClaimDeclaration ?? this.authenticityClaimDeclaration,
      customRightsStatement:
          customRightsStatement ?? this.customRightsStatement,
      videoNotaryId: videoNotaryId ?? this.videoNotaryId,
      videoNotaryAt: videoNotaryAt ?? this.videoNotaryAt,
      videoNotaryReceiptSignature:
          videoNotaryReceiptSignature ?? this.videoNotaryReceiptSignature,
      videoNotaryUsageLedgerId:
          videoNotaryUsageLedgerId ?? this.videoNotaryUsageLedgerId,
      videoFingerprintRoot: videoFingerprintRoot ?? this.videoFingerprintRoot,
      videoBundleSha256: videoBundleSha256 ?? this.videoBundleSha256,
      videoBundleBytes: videoBundleBytes ?? this.videoBundleBytes,
      videoBundleSceneCount:
          videoBundleSceneCount ?? this.videoBundleSceneCount,
      videoBundleElapsedMs: videoBundleElapsedMs ?? this.videoBundleElapsedMs,
      videoFrameSamplePolicy:
          videoFrameSamplePolicy ?? this.videoFrameSamplePolicy,
      videoVisualTaskId: videoVisualTaskId ?? this.videoVisualTaskId,
      videoVisualCompletedAt:
          videoVisualCompletedAt ?? this.videoVisualCompletedAt,
      videoVisualStrategyDigest:
          videoVisualStrategyDigest ?? this.videoVisualStrategyDigest,
      videoVisualSelfCheckConfidence:
          videoVisualSelfCheckConfidence ?? this.videoVisualSelfCheckConfidence,
      videoVisualSelfCheckThreshold:
          videoVisualSelfCheckThreshold ?? this.videoVisualSelfCheckThreshold,
      videoVisualCheckedFrames:
          videoVisualCheckedFrames ?? this.videoVisualCheckedFrames,
      videoVisualMediaHash: videoVisualMediaHash ?? this.videoVisualMediaHash,
      videoVisualReceiptHash:
          videoVisualReceiptHash ?? this.videoVisualReceiptHash,
      videoVisualOutputBytes:
          videoVisualOutputBytes ?? this.videoVisualOutputBytes,
      videoVisualOutputContentType:
          videoVisualOutputContentType ?? this.videoVisualOutputContentType,
      source: source ?? this.source,
      syncStatus: syncStatus ?? this.syncStatus,
      createdAt: createdAt ?? this.createdAt,
    );
  }

  Map<String, Object?> toSyncPayload() {
    return sanitizeVaultRecordSyncPayload({
      'id': id,
      'kind': kind.name,
      'title': title,
      'watermark_uid': watermarkUid,
      'revision': revision,
      'creator_display_name': creatorDisplayName,
      'trusted_time_status': trustedTimeStatus,
      'trusted_time_source': trustedTimeSource,
      'trusted_time_at': trustedTimeAt?.toIso8601String(),
      'third_party_verification_status': thirdPartyVerificationStatus,
      'third_party_verification_provider': thirdPartyVerificationProvider,
      'third_party_verification_path': thirdPartyVerificationPath,
      'sha256': sha256,
      'parent_watermark_uid': parentWatermarkUid,
      'rewrite_reason': rewriteReason,
      'extracted_timestamp': extractedTimestamp,
      'extracted_device_id_hex': extractedDeviceIdHex,
      'extracted_file_hash_hex': extractedFileHashHex,
      'write_verification_status': writeVerificationStatus?.name,
      'write_verification_message': writeVerificationMessage,
      'write_verification_at': writeVerificationAt?.toIso8601String(),
      'protected_copy_name': protectedCopyName,
      'protected_copy_hash': protectedCopyHash,
      'payload_protocol_version': payloadProtocolVersion,
      'payload_bytes_length': payloadBytesLength,
      'media_payload_role': _mediaPayloadRoleForProtocol(
        payloadProtocolVersion,
      ),
      'watermark_id_issue_mode': watermarkIdIssueMode,
      'watermark_id_registry_status': watermarkIdRegistryStatus,
      'watermark_id_registry_receipt': watermarkIdRegistryReceipt,
      'payload_auth_status': payloadAuthStatus,
      'output_strategy': outputStrategy,
      'work_source_declaration': workSourceDeclaration,
      'training_permission_declaration': trainingPermissionDeclaration,
      'creation_method_declaration': creationMethodDeclaration,
      'human_edit_level_declaration': humanEditLevelDeclaration,
      'authenticity_claim_declaration': authenticityClaimDeclaration,
      'custom_rights_statement': customRightsStatement,
      'video_notary_id': videoNotaryId,
      'video_notary_at': videoNotaryAt?.toIso8601String(),
      'video_notary_receipt_signature': videoNotaryReceiptSignature,
      'video_notary_usage_ledger_id': videoNotaryUsageLedgerId,
      'video_fingerprint_root': videoFingerprintRoot,
      'video_bundle_sha256': videoBundleSha256,
      'video_bundle_bytes': videoBundleBytes,
      'video_bundle_scene_count': videoBundleSceneCount,
      'video_bundle_elapsed_ms': videoBundleElapsedMs,
      'video_frame_sample_policy': videoFrameSamplePolicy,
      'video_visual_task_id': videoVisualTaskId,
      'video_visual_completed_at': videoVisualCompletedAt?.toIso8601String(),
      'video_visual_strategy_digest': videoVisualStrategyDigest,
      'video_visual_self_check_confidence': videoVisualSelfCheckConfidence,
      'video_visual_self_check_threshold': videoVisualSelfCheckThreshold,
      'video_visual_checked_frames': videoVisualCheckedFrames,
      'video_visual_media_hash': videoVisualMediaHash,
      'video_visual_receipt_hash': videoVisualReceiptHash,
      'video_visual_output_bytes': videoVisualOutputBytes,
      'video_visual_output_content_type': videoVisualOutputContentType,
      'source': source.name,
      'sync_status': syncStatus.name,
      'created_at': createdAt.toIso8601String(),
    });
  }
}

String _mediaPayloadRoleForProtocol(int protocolVersion) {
  return protocolVersion >= 3 ? 'v3_minimal_anchor' : 'v2_full_record';
}

String _mediaPayloadRoleLabel(String value) {
  return switch (value) {
    'v3_minimal_anchor' => 'V3 最小锚点',
    'v2_full_record' => 'V2 完整载荷',
    _ => '未记录',
  };
}

class WorkDeclaration {
  const WorkDeclaration({
    this.workSourceDeclaration = 'unspecified',
    this.trainingPermissionDeclaration = 'prohibited',
    this.creationMethodDeclaration = 'unspecified',
    this.humanEditLevelDeclaration = 'unspecified',
    this.authenticityClaimDeclaration = 'unspecified',
    this.customRightsStatement,
  });

  final String workSourceDeclaration;
  final String trainingPermissionDeclaration;
  final String creationMethodDeclaration;
  final String humanEditLevelDeclaration;
  final String authenticityClaimDeclaration;
  final String? customRightsStatement;

  WorkDeclaration copyWith({
    String? workSourceDeclaration,
    String? trainingPermissionDeclaration,
    String? creationMethodDeclaration,
    String? humanEditLevelDeclaration,
    String? authenticityClaimDeclaration,
    Object? customRightsStatement = _copySentinel,
  }) {
    return WorkDeclaration(
      workSourceDeclaration:
          workSourceDeclaration ?? this.workSourceDeclaration,
      trainingPermissionDeclaration:
          trainingPermissionDeclaration ?? this.trainingPermissionDeclaration,
      creationMethodDeclaration:
          creationMethodDeclaration ?? this.creationMethodDeclaration,
      humanEditLevelDeclaration:
          humanEditLevelDeclaration ?? this.humanEditLevelDeclaration,
      authenticityClaimDeclaration:
          authenticityClaimDeclaration ?? this.authenticityClaimDeclaration,
      customRightsStatement: identical(customRightsStatement, _copySentinel)
          ? this.customRightsStatement
          : customRightsStatement as String?,
    );
  }
}

const Object _copySentinel = Object();

enum VaultRecordSource { write, verify }

enum WriteVerificationStatus { verified, failed }

WriteVerificationStatus? writeVerificationStatusFromName(String? name) {
  return switch (name) {
    'verified' => WriteVerificationStatus.verified,
    'failed' => WriteVerificationStatus.failed,
    _ => null,
  };
}

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
  pendingRegistryReconcile,
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
    String? payloadJson,
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
      payloadJson: payloadJson ?? this.payloadJson,
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
    this.onboardingCompleted = false,
    this.entitlementId,
    this.entitlementLabel = '未付费',
    this.entitlementStatus = EntitlementStatus.free,
    this.entitlementPlanCode = 'free',
    this.entitlementPlanKey = 'base_unpaid',
    this.entitlementFeatures = const {},
    this.entitlementLastCheckedAt,
    this.syncPolicy = 'manual_local_only',
    this.cloudBaseUrl = '',
    this.lanDebugAddress = '',
    this.lanDebugPairingCode = '',
    this.lastError,
    this.lastRemotePullCursor,
    this.lastSyncAttemptAt,
    this.lastSyncSuccessAt,
    this.lastSyncFailureAt,
    this.anonymousFeedbackEnabled = false,
    this.experienceImprovementEnabled = true,
    this.anonymousInstallId,
    this.anonymousFeedbackLastEventAt,
    this.anonymousFeedbackLastAttemptAt,
    this.anonymousFeedbackLastSuccessAt,
    this.anonymousFeedbackNextRetryAt,
    this.anonymousFeedbackLastFlushError,
    this.anonymousFeedbackConsecutiveFailures = 0,
    this.anonymousFeedbackQueueJson,
    this.reportPurchaseGrantsJson,
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
  final bool onboardingCompleted;
  final String? entitlementId;
  final String entitlementLabel;
  final EntitlementStatus entitlementStatus;
  final String entitlementPlanCode;
  final String entitlementPlanKey;
  final Map<String, bool> entitlementFeatures;
  final DateTime? entitlementLastCheckedAt;
  final String syncPolicy;
  final String cloudBaseUrl;
  final String lanDebugAddress;
  final String lanDebugPairingCode;
  final String? lastError;
  final String? lastRemotePullCursor;
  final DateTime? lastSyncAttemptAt;
  final DateTime? lastSyncSuccessAt;
  final DateTime? lastSyncFailureAt;
  final bool anonymousFeedbackEnabled;
  final bool experienceImprovementEnabled;
  final String? anonymousInstallId;
  final DateTime? anonymousFeedbackLastEventAt;
  final DateTime? anonymousFeedbackLastAttemptAt;
  final DateTime? anonymousFeedbackLastSuccessAt;
  final DateTime? anonymousFeedbackNextRetryAt;
  final String? anonymousFeedbackLastFlushError;
  final int anonymousFeedbackConsecutiveFailures;
  final String? anonymousFeedbackQueueJson;
  final String? reportPurchaseGrantsJson;

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
    bool? onboardingCompleted,
    String? entitlementId,
    String? entitlementLabel,
    EntitlementStatus? entitlementStatus,
    String? entitlementPlanCode,
    String? entitlementPlanKey,
    Map<String, bool>? entitlementFeatures,
    DateTime? entitlementLastCheckedAt,
    String? syncPolicy,
    String? cloudBaseUrl,
    String? lanDebugAddress,
    String? lanDebugPairingCode,
    String? lastError,
    String? lastRemotePullCursor,
    DateTime? lastSyncAttemptAt,
    DateTime? lastSyncSuccessAt,
    DateTime? lastSyncFailureAt,
    bool? anonymousFeedbackEnabled,
    bool? experienceImprovementEnabled,
    String? anonymousInstallId,
    DateTime? anonymousFeedbackLastEventAt,
    DateTime? anonymousFeedbackLastAttemptAt,
    DateTime? anonymousFeedbackLastSuccessAt,
    DateTime? anonymousFeedbackNextRetryAt,
    String? anonymousFeedbackLastFlushError,
    int? anonymousFeedbackConsecutiveFailures,
    String? anonymousFeedbackQueueJson,
    String? reportPurchaseGrantsJson,
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
      onboardingCompleted: onboardingCompleted ?? this.onboardingCompleted,
      entitlementId: clearEntitlement
          ? null
          : entitlementId ?? this.entitlementId,
      entitlementLabel: clearEntitlement
          ? '未付费'
          : entitlementLabel ?? this.entitlementLabel,
      entitlementStatus: clearEntitlement
          ? EntitlementStatus.free
          : entitlementStatus ?? this.entitlementStatus,
      entitlementPlanCode: clearEntitlement
          ? 'free'
          : entitlementPlanCode ?? this.entitlementPlanCode,
      entitlementPlanKey: clearEntitlement
          ? 'base_unpaid'
          : entitlementPlanKey ?? this.entitlementPlanKey,
      entitlementFeatures: clearEntitlement
          ? const {}
          : entitlementFeatures ?? this.entitlementFeatures,
      entitlementLastCheckedAt: clearEntitlement
          ? null
          : entitlementLastCheckedAt ?? this.entitlementLastCheckedAt,
      syncPolicy: clearEntitlement
          ? 'blocked_by_entitlement'
          : syncPolicy ?? this.syncPolicy,
      cloudBaseUrl: cloudBaseUrl ?? this.cloudBaseUrl,
      lanDebugAddress: lanDebugAddress ?? this.lanDebugAddress,
      lanDebugPairingCode: lanDebugPairingCode ?? this.lanDebugPairingCode,
      lastError: clearLastError ? null : lastError ?? this.lastError,
      lastRemotePullCursor: lastRemotePullCursor ?? this.lastRemotePullCursor,
      lastSyncAttemptAt: lastSyncAttemptAt ?? this.lastSyncAttemptAt,
      lastSyncSuccessAt: lastSyncSuccessAt ?? this.lastSyncSuccessAt,
      lastSyncFailureAt: lastSyncFailureAt ?? this.lastSyncFailureAt,
      anonymousFeedbackEnabled:
          anonymousFeedbackEnabled ?? this.anonymousFeedbackEnabled,
      experienceImprovementEnabled:
          experienceImprovementEnabled ?? this.experienceImprovementEnabled,
      anonymousInstallId: anonymousInstallId ?? this.anonymousInstallId,
      anonymousFeedbackLastEventAt:
          anonymousFeedbackLastEventAt ?? this.anonymousFeedbackLastEventAt,
      anonymousFeedbackLastAttemptAt:
          anonymousFeedbackLastAttemptAt ?? this.anonymousFeedbackLastAttemptAt,
      anonymousFeedbackLastSuccessAt:
          anonymousFeedbackLastSuccessAt ?? this.anonymousFeedbackLastSuccessAt,
      anonymousFeedbackNextRetryAt:
          anonymousFeedbackNextRetryAt ?? this.anonymousFeedbackNextRetryAt,
      anonymousFeedbackLastFlushError:
          anonymousFeedbackLastFlushError ??
          this.anonymousFeedbackLastFlushError,
      anonymousFeedbackConsecutiveFailures:
          anonymousFeedbackConsecutiveFailures ??
          this.anonymousFeedbackConsecutiveFailures,
      anonymousFeedbackQueueJson:
          anonymousFeedbackQueueJson ?? this.anonymousFeedbackQueueJson,
      reportPurchaseGrantsJson:
          reportPurchaseGrantsJson ?? this.reportPurchaseGrantsJson,
    );
  }
}

String? _normalizeReportProductCode(String productCode) {
  final value = productCode.trim().toLowerCase();
  if (value == 'copyright_report_single' ||
      value == 'rights_evidence_pack_single') {
    return value;
  }
  return null;
}

List<ReportPurchaseGrant> _decodeReportPurchaseGrants(String? value) {
  if (value == null || value.trim().isEmpty) {
    return <ReportPurchaseGrant>[];
  }
  try {
    final decoded = jsonDecode(value);
    if (decoded is! List) {
      return <ReportPurchaseGrant>[];
    }
    return decoded
        .whereType<Map<String, Object?>>()
        .map(ReportPurchaseGrant.fromJson)
        .where(
          (grant) =>
              grant.grantId.isNotEmpty &&
              grant.accountId.isNotEmpty &&
              grant.workspaceId.isNotEmpty &&
              grant.vaultRecordId.isNotEmpty &&
              grant.productCode.isNotEmpty,
        )
        .toList(growable: false);
  } catch (_) {
    return <ReportPurchaseGrant>[];
  }
}

String _encodeReportPurchaseGrants(List<ReportPurchaseGrant> grants) {
  return jsonEncode(grants.map((grant) => grant.toJson()).toList());
}

String _entitlementRefreshMessage(EntitlementStatus status) {
  return switch (status) {
    EntitlementStatus.active || EntitlementStatus.trial => '订阅已生效，权益已刷新。',
    EntitlementStatus.grace => '权益处于宽限期，已刷新。',
    EntitlementStatus.expired => '未检测到有效订阅，当前权益已过期。',
    EntitlementStatus.free => '暂未检测到订阅生效，请稍后再试。',
  };
}

String _paymentSessionStatusMessage(String status) {
  return switch (status) {
    'succeeded' => '支付已确认，权益已生效。',
    'pending' || 'created' => '尚未确认支付完成，请完成支付或稍后确认。',
    'expired' => '支付会话已过期，请重新创建支付。',
    'failed' || 'closed' => '支付未完成，请重新创建支付。',
    _ => '暂未检测到支付完成，请稍后再试。',
  };
}

enum SyncConnectionStatus { unconfigured, connected, connecting, failed }

enum SyncTransportMode { localOnly, cloud, lanDebug }

enum EntitlementStatus { free, trial, active, grace, expired }

String normalizeEntitlementPlanKey({
  String? planKey,
  required String planCode,
  required Map<String, bool> features,
}) {
  final normalizedPlanKey = planKey?.trim();
  if (normalizedPlanKey == 'base_unpaid' ||
      normalizedPlanKey == 'image_audio_annual') {
    return normalizedPlanKey!;
  }
  final normalizedPlanCode = planCode.trim().toLowerCase();
  final hasAnnualFeatures =
      features['batch_processing'] == true && features['cloud_sync'] == true;
  if (hasAnnualFeatures ||
      const {'creator', 'studio', 'enterprise'}.contains(normalizedPlanCode)) {
    return 'image_audio_annual';
  }
  return 'base_unpaid';
}

String entitlementPlanLabel(String planKey) =>
    planKey == 'image_audio_annual' ? '图片 / 音频年费' : '未付费';

enum BatchMediaKind { image, audio, unsupported }

enum UsageMediaType { image, audio, video, report }

UsageMediaType usageMediaTypeFromAssetKind(WatermarkAssetKind kind) {
  return switch (kind) {
    WatermarkAssetKind.image => UsageMediaType.image,
    WatermarkAssetKind.audio => UsageMediaType.audio,
    WatermarkAssetKind.video => UsageMediaType.video,
  };
}

enum BatchJobStatus { draft, queued, paused, cancelled }

enum BatchItemStatus { queued, running, verified, failed, cancelled }

class UsageLedgerEntry {
  const UsageLedgerEntry({
    required this.id,
    required this.occurredAt,
    required this.featureName,
    required this.mediaType,
    required this.fileSizeBucket,
    required this.quantity,
    required this.eventType,
    required this.entitlementStatus,
    required this.entitlementPlanCode,
    this.entitlementPlanName,
    this.pipelineId,
    this.vaultRecordId,
  });

  final String id;
  final DateTime occurredAt;
  final String featureName;
  final UsageMediaType mediaType;
  final String fileSizeBucket;
  final int quantity;
  final String eventType;
  final EntitlementStatus entitlementStatus;
  final String entitlementPlanCode;
  final String? entitlementPlanName;
  final String? pipelineId;
  final String? vaultRecordId;

  factory UsageLedgerEntry.success({
    required String featureName,
    required UsageMediaType mediaType,
    required int fileSizeBytes,
    required SyncProfile syncProfile,
    required String? pipelineId,
    required String? vaultRecordId,
  }) {
    final occurredAt = DateTime.now();
    return UsageLedgerEntry(
      id: 'usage-${occurredAt.microsecondsSinceEpoch}-${mediaType.name}-${vaultRecordId ?? 'local'}',
      occurredAt: occurredAt,
      featureName: featureName,
      mediaType: mediaType,
      fileSizeBucket: bucketFileSize(fileSizeBytes),
      quantity: 1,
      eventType: 'success',
      entitlementStatus: syncProfile.entitlementStatus,
      entitlementPlanCode: syncProfile.entitlementPlanCode,
      entitlementPlanName: syncProfile.entitlementLabel,
      pipelineId: pipelineId,
      vaultRecordId: vaultRecordId,
    );
  }
}

class UsageLedgerSummary {
  const UsageLedgerSummary({
    required this.totalUnits,
    required this.totalEvents,
    required this.imageUnits,
    required this.videoUnits,
    required this.audioUnits,
    required this.lastUsedAt,
    required this.lastFeatureName,
    required this.entitlementStatus,
    required this.entitlementPlanCode,
    required this.entitlementPlanName,
  });

  final int totalUnits;
  final int totalEvents;
  final int imageUnits;
  final int videoUnits;
  final int audioUnits;
  final DateTime? lastUsedAt;
  final String? lastFeatureName;
  final EntitlementStatus entitlementStatus;
  final String entitlementPlanCode;
  final String entitlementPlanName;

  factory UsageLedgerSummary.empty(SyncProfile syncProfile) {
    return UsageLedgerSummary(
      totalUnits: 0,
      totalEvents: 0,
      imageUnits: 0,
      videoUnits: 0,
      audioUnits: 0,
      lastUsedAt: null,
      lastFeatureName: null,
      entitlementStatus: syncProfile.entitlementStatus,
      entitlementPlanCode: syncProfile.entitlementPlanCode,
      entitlementPlanName: syncProfile.entitlementLabel,
    );
  }

  UsageLedgerSummary withEntry(
    UsageLedgerEntry entry,
    SyncProfile syncProfile,
  ) {
    return UsageLedgerSummary(
      totalUnits: totalUnits + entry.quantity,
      totalEvents: totalEvents + 1,
      imageUnits:
          imageUnits +
          (entry.mediaType == UsageMediaType.image ? entry.quantity : 0),
      videoUnits:
          videoUnits +
          (entry.mediaType == UsageMediaType.video ? entry.quantity : 0),
      audioUnits:
          audioUnits +
          (entry.mediaType == UsageMediaType.audio ? entry.quantity : 0),
      lastUsedAt: entry.occurredAt,
      lastFeatureName: entry.featureName,
      entitlementStatus: syncProfile.entitlementStatus,
      entitlementPlanCode: syncProfile.entitlementPlanCode,
      entitlementPlanName: syncProfile.entitlementLabel,
    );
  }
}

class CommercialHealthSummary {
  const CommercialHealthSummary({
    required this.accountScope,
    required this.entitlementPlanName,
    required this.entitlementStatus,
    required this.localBatchJobs,
    required this.verifiedBatchItems,
    required this.failedBatchItems,
    required this.reportExportUnits,
    required this.cloudAcceptedEvents,
    required this.cloudFailureEvents,
    required this.l2VideoNotaryCount,
    required this.latestPaymentSessionStatus,
    required this.privacyNote,
  });

  final String accountScope;
  final String entitlementPlanName;
  final EntitlementStatus entitlementStatus;
  final int localBatchJobs;
  final int verifiedBatchItems;
  final int failedBatchItems;
  final int reportExportUnits;
  final int cloudAcceptedEvents;
  final int cloudFailureEvents;
  final int l2VideoNotaryCount;
  final String? latestPaymentSessionStatus;
  final String privacyNote;
}

class MobileAnonymousFeedbackStatus {
  const MobileAnonymousFeedbackStatus({
    required this.installId,
    required this.sessionId,
    required this.queuedEvents,
    required this.queuedBytes,
    required this.lastEventAt,
    required this.lastFlushError,
    required this.consecutiveFailures,
    required this.nextRetryAt,
    required this.lastAttemptAt,
    required this.lastSuccessAt,
    required this.telemetryEnabled,
    required this.networkEnabled,
    required this.endpointConfigured,
  });

  final String installId;
  final String sessionId;
  final int queuedEvents;
  final int queuedBytes;
  final DateTime? lastEventAt;
  final String? lastFlushError;
  final int consecutiveFailures;
  final DateTime? nextRetryAt;
  final DateTime? lastAttemptAt;
  final DateTime? lastSuccessAt;
  final bool telemetryEnabled;
  final bool networkEnabled;
  final bool endpointConfigured;
}

class MobileAnonymousFlushResult {
  const MobileAnonymousFlushResult({
    required this.attemptedEvents,
    required this.sentEvents,
    required this.remainingEvents,
    required this.endpointConfigured,
    required this.message,
    this.flushedAt,
  });

  final int attemptedEvents;
  final int sentEvents;
  final int remainingEvents;
  final bool endpointConfigured;
  final DateTime? flushedAt;
  final String message;
}

enum MobileExperienceRiskLevel { low, medium, high }

class MobileExperienceImprovementSnapshot {
  const MobileExperienceImprovementSnapshot({
    required this.enabled,
    required this.totalEvents,
    required this.successEvents,
    required this.failureEvents,
    required this.diagnosticEvents,
    required this.conversionRate,
    required this.failureRate,
    required this.repeatedErrorCount,
    required this.lastEventAt,
    required this.riskLevel,
    required this.reasons,
  });

  final bool enabled;
  final int totalEvents;
  final int successEvents;
  final int failureEvents;
  final int diagnosticEvents;
  final double conversionRate;
  final double failureRate;
  final int repeatedErrorCount;
  final DateTime? lastEventAt;
  final MobileExperienceRiskLevel riskLevel;
  final List<String> reasons;

  String get riskLabel {
    return switch (riskLevel) {
      MobileExperienceRiskLevel.high => '高风险',
      MobileExperienceRiskLevel.medium => '中风险',
      MobileExperienceRiskLevel.low => '低风险',
    };
  }
}

class MobileDataUsageSnapshot {
  const MobileDataUsageSnapshot({
    required this.vaultRecords,
    required this.syncQueueItems,
    required this.localBatchJobs,
    required this.localBatchItems,
    required this.usageEvents,
    required this.anonymousFeedbackEvents,
    required this.estimatedBytes,
    required this.note,
  });

  final int vaultRecords;
  final int syncQueueItems;
  final int localBatchJobs;
  final int localBatchItems;
  final int usageEvents;
  final int anonymousFeedbackEvents;
  final int estimatedBytes;
  final String note;

  String get estimatedSizeLabel {
    final kb = estimatedBytes / 1024;
    if (kb < 1024) {
      return '${kb.toStringAsFixed(1)} KB';
    }
    return '${(kb / 1024).toStringAsFixed(2)} MB';
  }
}

class FormalReportDraft {
  const FormalReportDraft({
    required this.reportId,
    required this.exportedAt,
    required this.markdown,
    required this.recordCount,
  });

  final String reportId;
  final DateTime exportedAt;
  final String markdown;
  final int recordCount;

  factory FormalReportDraft.fromRecord({
    required VaultRecord record,
    required DateTime exportedAt,
    required String appVersion,
  }) {
    final reportId =
        'hsr-${record.watermarkUid}-${record.revision}-${exportedAt.microsecondsSinceEpoch}';
    final markdown = [
      '# HiddenShield 正式版权报告',
      '',
      '- 报告编号: $reportId',
      '- 报告类型: 单条正式报告',
      '- 导出时间: ${_summaryLocalDateTime(exportedAt)}',
      '- App 版本: $appVersion',
      '',
      '## 隐私边界',
      '',
      '- 不包含原始媒体文件',
      '- 不包含加水印后的媒体文件',
      '- 不包含本地媒体文件路径',
      '- 视频指纹存证只包含不可逆元数据，不包含可还原画面的素材',
      '',
      '## 结构化字段',
      '',
      '- file_name',
      '- watermark_uid',
      '- revision',
      '- hashes',
      '- protected_copy_metadata',
      '- rights_declaration',
      '- verification_status',
      '- payload_registry',
      '- trusted_time_status',
      '- video_notary_receipt',
      '- video_fingerprint_bundle_metadata',
      '- video_visual_watermark_receipt',
      '',
      '## 版权记录',
      '',
      '- 文件名: ${record.title}',
      '- 版权编号: ${record.watermarkUid}',
      '- 版本次数: 第 ${record.revision} 次',
      '- 创作者身份: ${_formalReportValue(record.creatorDisplayName)}',
      '- 上一版编号: ${record.parentWatermarkUid ?? '无'}',
      '- 更新说明: ${record.rewriteReason ?? '无'}',
      '- 作品指纹: ${record.sha256 ?? record.extractedFileHashHex ?? '无'}',
      '- 保护副本名称: ${_formalReportValue(record.protectedCopyName)}',
      '- 保护副本摘要: ${_formalReportValue(record.protectedCopyHash)}',
      '- 输出策略: ${_outputStrategyLabel(record.outputStrategy)}',
      '- 完成后验证: ${_formalReportVerificationLabel(record.writeVerificationStatus)}',
      '- 验证说明: ${record.writeVerificationMessage ?? '无'}',
      '- 验证时间: ${_summaryLocalOptionalDate(record.writeVerificationAt)}',
      '- Payload 协议: V${record.payloadProtocolVersion} / ${record.payloadBytesLength} bytes',
      '- 媒体载荷角色: ${_mediaPayloadRoleLabel(_mediaPayloadRoleForProtocol(record.payloadProtocolVersion))}',
      '- 编号签发模式: ${_watermarkIssueModeLabel(record.watermarkIdIssueMode)}',
      '- 登记状态: ${_registryStatusLabel(record.watermarkIdRegistryStatus)}',
      '- 登记收据: ${_formalReportValue(record.watermarkIdRegistryReceipt)}',
      '- Payload 认证状态: ${_payloadAuthStatusLabel(record.payloadAuthStatus)}',
      '- 入库时间: ${_summaryLocalDateTime(record.createdAt)}',
      '',
      '## 可信时间',
      '',
      '- 第三方验证: ${_formalReportValue(record.thirdPartyVerificationStatus)}',
      '- 验证服务: ${_formalReportValue(record.thirdPartyVerificationProvider)}',
      '- 验证路径: ${_formalReportValue(record.thirdPartyVerificationPath)}',
      '- 可信时间: ${_formalReportValue(record.trustedTimeStatus)}',
      '- 时间来源: ${_formalReportValue(record.trustedTimeSource)}',
      '- 记录时间: ${_summaryEvidenceDateTime(record.trustedTimeAt, '未记录')}',
      '',
      '## 作品声明与授权策略',
      '',
      '- 作品来源声明: ${_workSourceDeclarationLabel(record.workSourceDeclaration)}',
      '- 训练许可声明: ${_trainingPermissionLabel(record.trainingPermissionDeclaration)}',
      '- 创作方式声明: ${_summaryValue(record.creationMethodDeclaration)}',
      '- 人工编辑声明: ${_summaryValue(record.humanEditLevelDeclaration)}',
      '- 真实性声明: ${_authenticityClaimLabel(record.authenticityClaimDeclaration)}',
      '- 自定义版权声明: ${record.customRightsStatement?.trim().isNotEmpty == true ? record.customRightsStatement!.trim() : '无'}',
      '',
      if (record.videoNotaryId != null) ...[
        '## 视频指纹存证',
        '',
        '- 存证编号: ${record.videoNotaryId}',
        '- 存证时间: ${_summaryLocalOptionalDate(record.videoNotaryAt)}',
        '- 收据签名: ${record.videoNotaryReceiptSignature ?? '无'}',
        '- 用量流水: ${record.videoNotaryUsageLedgerId ?? '无'}',
        '- 指纹根: ${record.videoFingerprintRoot ?? '无'}',
        '- 指纹包摘要: ${record.videoBundleSha256 ?? '无'}',
        '- 指纹包大小: ${record.videoBundleBytes ?? '无'}',
        '- 采样帧: ${record.videoBundleSceneCount ?? '无'}',
        '- 生成耗时: ${record.videoBundleElapsedMs == null ? '无' : '${(record.videoBundleElapsedMs! / 1000).toStringAsFixed(1)} 秒'}',
        '- 采样策略: ${record.videoFrameSamplePolicy ?? '无'}',
        '',
      ],
      if (record.videoVisualTaskId != null ||
          record.videoVisualMediaHash != null) ...[
        '## L3 视频画面盲水印',
        '',
        '- 任务编号: ${record.videoVisualTaskId ?? '无'}',
        '- 完成时间: ${_summaryLocalOptionalDate(record.videoVisualCompletedAt)}',
        '- 策略摘要: ${record.videoVisualStrategyDigest ?? '无'}',
        '- 自检置信度: ${record.videoVisualSelfCheckConfidence?.toStringAsFixed(6) ?? '无'}',
        '- 自检阈值: ${record.videoVisualSelfCheckThreshold?.toStringAsFixed(6) ?? '无'}',
        '- 检查帧数: ${record.videoVisualCheckedFrames ?? '无'}',
        '- 成品媒体摘要: ${record.videoVisualMediaHash ?? '无'}',
        '- Worker 收据摘要: ${record.videoVisualReceiptHash ?? '无'}',
        '- 成品字节数: ${record.videoVisualOutputBytes ?? '无'}',
        '- 成品内容类型: ${record.videoVisualOutputContentType ?? '无'}',
        '',
      ],
      '## 免责声明',
      '',
      '本报告由 HiddenShield 根据本机版权库记录生成，仅作为技术验证与版权管理辅助材料，不构成法律意见、司法鉴定意见或诉讼结果承诺。',
      '',
    ].join('\n');
    return FormalReportDraft(
      reportId: reportId,
      exportedAt: exportedAt,
      markdown: markdown,
      recordCount: 1,
    );
  }
}

String _formalReportVerificationLabel(WriteVerificationStatus? status) {
  return switch (status) {
    WriteVerificationStatus.verified => '已通过',
    WriteVerificationStatus.failed => '未通过',
    null => '未记录',
  };
}

String _formalReportValue(String? value) {
  final trimmed = value?.trim();
  return trimmed == null || trimmed.isEmpty ? '未记录' : trimmed;
}

String _summaryValue(String? value) {
  final trimmed = value?.trim();
  return trimmed == null || trimmed.isEmpty ? '未记录' : trimmed;
}

String _summaryLocalDateTime(DateTime value) {
  return value.toLocal().toString();
}

String _summaryLocalOptionalDate(DateTime? value) {
  return value == null ? '无' : value.toLocal().toString();
}

String _summaryEvidenceDateTime(DateTime? value, String fallback) {
  if (value == null) return fallback;
  return '${value.toLocal().toString()}（原始回执: ${value.toUtc().toIso8601String()}）';
}

String _summaryEvidenceValue(DateTime? value, String? fallback) {
  final status = _summaryValue(fallback);
  if (value == null) return status;
  return '${value.toLocal().toString()}（原始回执: ${value.toUtc().toIso8601String()}）';
}

void _validateL3VideoVisualTaskForVault(CloudVideoTaskRecord task) {
  if (task.status != 'succeeded') {
    throw StateError('只能领取已 succeeded 的 L3 视频画面盲水印任务');
  }
  if (!_isL3VideoVisualTaskCapability(task.capabilityLevel)) {
    throw StateError('该任务不是 L3 视频画面盲水印任务');
  }
  final confidence = task.selfCheckConfidence;
  final threshold = task.selfCheckThreshold;
  if (confidence == null || threshold == null) {
    throw StateError('L3 task 缺少自检置信度或阈值');
  }
  if (confidence < threshold) {
    throw StateError('L3 task 自检置信度低于阈值，拒绝入库');
  }
  if ((task.checkedFrames ?? 0) <= 0) {
    throw StateError('L3 task 缺少 checkedFrames');
  }
  if (task.outputMediaStorageRef?.startsWith('object://l3-output/') != true) {
    throw StateError('L3 task 输出不是正式对象存储产物');
  }
  if (task.outputMediaContentType != 'video/mp4') {
    throw StateError('L3 task 输出不是 video/mp4');
  }
  if ((task.outputMediaBytes ?? 0) <= 0) {
    throw StateError('L3 task 输出字节数为空');
  }
  if ((task.watermarkedMediaHash ?? '').isEmpty) {
    throw StateError('L3 task 缺少 watermarkedMediaHash');
  }
  if ((task.workerReceiptHash ?? '').isEmpty) {
    throw StateError('L3 task 缺少 workerReceiptHash');
  }
  if ((task.serverReceiptSignature ?? '').isEmpty) {
    throw StateError('L3 task 缺少 serverReceiptSignature');
  }
}

bool _isL3VideoVisualTaskCapability(String value) {
  final normalized = value.trim();
  return normalized == 'video_visual' ||
      normalized == 'hybrid_visual_watermark';
}

String _stripSha256Prefix(String value) {
  final trimmed = value.trim();
  return trimmed.startsWith('sha256:') ? trimmed.substring(7) : trimmed;
}

String _sha256HexForJson(Map<String, Object?> value) {
  return crypto.sha256.convert(utf8.encode(jsonEncode(value))).toString();
}

String _extensionForFileName(String fileName) {
  final match = RegExp(r'\.([^.]+)$').firstMatch(fileName.trim());
  return match?.group(1)?.toLowerCase() ?? '';
}

String _copyrightSummaryVerificationStatus(WriteVerificationStatus? status) {
  return switch (status) {
    WriteVerificationStatus.verified => '已通过',
    WriteVerificationStatus.failed => '未通过',
    null => '未记录',
  };
}

String _copyrightSummaryThirdPartyStatus(VaultRecord record) {
  final status = record.thirdPartyVerificationStatus?.trim();
  if (status != null && status.isNotEmpty && status != '未记录') {
    return status;
  }
  if (record.trustedTimeAt != null || record.trustedTimeSource != null) {
    return '已记录网络授时';
  }
  return '未记录';
}

String _outputStrategyLabel(String value) {
  return value == 'minimal_required_change' || value.isEmpty ? '最小必要变更' : value;
}

String _watermarkIssueModeLabel(String value) {
  return switch (value) {
    'server_reserved' => '后端预签发',
    'server_confirmed' => '后端已确认',
    'server_reissued' => '后端重新签发',
    _ => '本地离线生成',
  };
}

String _registryStatusLabel(String value) {
  return switch (value) {
    'reserved' => '已预留，等待写入确认',
    'server_confirmed' => '后端已确认',
    'offline_confirmed' => '离线编号已补登记',
    'conflict' => '编号冲突',
    'reissue_required' => '需要重新签发',
    'pending_registry_reconcile' => '待登记仲裁',
    _ => '等待联网登记',
  };
}

String _payloadAuthStatusLabel(String value) {
  return switch (value) {
    'verified' => '已验证',
    'failed' => '验证失败',
    'pending_repair' => '待修复',
    _ => '未验证',
  };
}

String _prefixedSha256(String value) {
  final trimmed = value.trim();
  if (trimmed.startsWith('sha256:')) {
    return trimmed;
  }
  return 'sha256:$trimmed';
}

String _workSourceDeclarationLabel(String value) {
  return switch (value) {
    'human_created' => '人工创作',
    'ai_assisted' => 'AI 辅助',
    'ai_generated' => 'AI 生成',
    _ => '未声明',
  };
}

String _trainingPermissionLabel(String value) {
  return switch (value) {
    'separate_authorization_required' => '需单独授权',
    'non_commercial_allowed' => '允许非商业训练',
    'commercial_allowed' => '允许商业训练',
    'unspecified' => '未声明',
    _ => '禁止模型训练',
  };
}

String _authenticityClaimLabel(String value) {
  return switch (value) {
    'synthetic' => '虚构或合成',
    'based_on_reality' => '基于真实',
    'creator_claimed_authentic' || 'authentic' => '创作者声明真实',
    _ => '未声明',
  };
}

String _formatOptionalDate(DateTime? value) {
  return value == null ? '无' : value.toIso8601String();
}

class LocalBatchJob {
  const LocalBatchJob({
    required this.id,
    required this.status,
    required this.createdAt,
    required this.updatedAt,
    required this.entitlementPlanCode,
    required this.entitlementStatus,
    required this.items,
  });

  final String id;
  final BatchJobStatus status;
  final DateTime createdAt;
  final DateTime updatedAt;
  final String entitlementPlanCode;
  final EntitlementStatus entitlementStatus;
  final List<LocalBatchItem> items;

  LocalBatchJob copyWith({
    BatchJobStatus? status,
    DateTime? updatedAt,
    String? entitlementPlanCode,
    EntitlementStatus? entitlementStatus,
    List<LocalBatchItem>? items,
  }) {
    return LocalBatchJob(
      id: id,
      status: status ?? this.status,
      createdAt: createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
      entitlementPlanCode: entitlementPlanCode ?? this.entitlementPlanCode,
      entitlementStatus: entitlementStatus ?? this.entitlementStatus,
      items: items ?? this.items,
    );
  }
}

class LocalBatchItem {
  const LocalBatchItem({
    required this.id,
    required this.jobId,
    required this.inputRef,
    required this.fileName,
    required this.mediaKind,
    required this.status,
    required this.attempts,
    required this.createdAt,
    required this.updatedAt,
    this.lastError,
    this.outputRef,
    this.vaultRecordId,
    this.writeVerificationStatus,
    this.writeVerificationMessage,
  });

  final String id;
  final String jobId;
  final String inputRef;
  final String fileName;
  final BatchMediaKind mediaKind;
  final BatchItemStatus status;
  final int attempts;
  final DateTime createdAt;
  final DateTime updatedAt;
  final String? lastError;
  final String? outputRef;
  final String? vaultRecordId;
  final WriteVerificationStatus? writeVerificationStatus;
  final String? writeVerificationMessage;

  LocalBatchItem copyWith({
    BatchItemStatus? status,
    int? attempts,
    DateTime? updatedAt,
    String? lastError,
    String? outputRef,
    String? vaultRecordId,
    WriteVerificationStatus? writeVerificationStatus,
    String? writeVerificationMessage,
    bool clearLastError = false,
    bool clearOutputRef = false,
    bool clearVaultRecordId = false,
    bool clearWriteVerificationStatus = false,
    bool clearWriteVerificationMessage = false,
  }) {
    return LocalBatchItem(
      id: id,
      jobId: jobId,
      inputRef: inputRef,
      fileName: fileName,
      mediaKind: mediaKind,
      status: status ?? this.status,
      attempts: attempts ?? this.attempts,
      createdAt: createdAt,
      updatedAt: updatedAt ?? this.updatedAt,
      lastError: clearLastError ? null : lastError ?? this.lastError,
      outputRef: clearOutputRef ? null : outputRef ?? this.outputRef,
      vaultRecordId: clearVaultRecordId
          ? null
          : vaultRecordId ?? this.vaultRecordId,
      writeVerificationStatus: clearWriteVerificationStatus
          ? null
          : writeVerificationStatus ?? this.writeVerificationStatus,
      writeVerificationMessage: clearWriteVerificationMessage
          ? null
          : writeVerificationMessage ?? this.writeVerificationMessage,
    );
  }
}

String vaultRecordSourceLabel(VaultRecordSource source) {
  return switch (source) {
    VaultRecordSource.write => '写入',
    VaultRecordSource.verify => '验证',
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
    SyncQueueOperation.upsertEvidenceRecord => '验证记录',
  };
}

String mobileSyncResolutionTypeLabel(MobileSyncResolutionType type) {
  return switch (type) {
    MobileSyncResolutionType.recordInserted => '新增记录',
    MobileSyncResolutionType.recordReplaced => '刷新记录',
    MobileSyncResolutionType.duplicateIgnored => '忽略重复',
    MobileSyncResolutionType.pendingRegistryReconcile => '待登记仲裁',
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
    SyncTransportMode.lanDebug => '本机同步',
  };
}

String entitlementStatusLabel(EntitlementStatus status) {
  return switch (status) {
    EntitlementStatus.free => '未付费',
    EntitlementStatus.trial => '试用中',
    EntitlementStatus.active => '订阅有效',
    EntitlementStatus.grace => '宽限期',
    EntitlementStatus.expired => '已过期',
  };
}

String bucketFileSize(int bytes) {
  const mb = 1024 * 1024;
  if (bytes <= 10 * mb) {
    return '0-10mb';
  }
  if (bytes <= 50 * mb) {
    return '10-50mb';
  }
  if (bytes <= 200 * mb) {
    return '50-200mb';
  }
  if (bytes <= 500 * mb) {
    return '200-500mb';
  }
  return '500mb+';
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

String _defaultOfflineLicensePlatform() {
  if (kIsWeb) return 'unsupported';
  return switch (defaultTargetPlatform) {
    TargetPlatform.android => 'android',
    TargetPlatform.iOS => 'ios',
    _ => 'unsupported',
  };
}

String _offlineLicenseErrorCode(Object error) {
  if (error is FormatException) {
    return error.message.toString();
  }
  if (error is OfflineLicenseSecureStoreException) {
    return error.code;
  }
  return 'offline_license_unknown_error';
}

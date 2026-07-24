import 'dart:convert';

import 'package:http/http.dart' as http;

import '../app/mobile_app_state.dart';

abstract class SyncTransport {
  Future<SyncSendResult> send(SyncQueueItem item);

  Future<SyncChangesResult> fetchChanges({String? since}) async {
    return const SyncChangesResult.success(changes: [], nextSince: '');
  }

  Future<SyncBatchSendResult> sendBatch(List<SyncQueueItem> items) async {
    final results = <String, SyncSendResult>{};
    for (final item in items) {
      results[item.id] = await send(item);
    }
    return SyncBatchSendResult(results);
  }
}

class LocalOnlySyncTransport implements SyncTransport {
  const LocalOnlySyncTransport();

  @override
  Future<SyncSendResult> send(SyncQueueItem item) async {
    return const SyncSendResult.failure('local-only sync is disabled');
  }

  @override
  Future<SyncChangesResult> fetchChanges({String? since}) async {
    return const SyncChangesResult.success(changes: [], nextSince: '');
  }

  @override
  Future<SyncBatchSendResult> sendBatch(List<SyncQueueItem> items) async {
    return SyncBatchSendResult({
      for (final item in items)
        item.id: const SyncSendResult.failure('local-only sync is disabled'),
    });
  }
}

class LocalMockSyncTransport implements SyncTransport {
  const LocalMockSyncTransport({this.shouldFail = false});

  final bool shouldFail;

  @override
  Future<SyncSendResult> send(SyncQueueItem item) async {
    await Future<void>.delayed(const Duration(milliseconds: 80));
    if (shouldFail) {
      return const SyncSendResult.failure('local mock sync failed');
    }
    return const SyncSendResult.success();
  }

  @override
  Future<SyncChangesResult> fetchChanges({String? since}) async {
    return const SyncChangesResult.success(changes: [], nextSince: '');
  }

  @override
  Future<SyncBatchSendResult> sendBatch(List<SyncQueueItem> items) async {
    await Future<void>.delayed(const Duration(milliseconds: 80));
    return SyncBatchSendResult({
      for (final item in items)
        item.id: shouldFail
            ? const SyncSendResult.failure('local mock sync failed')
            : const SyncSendResult.success(),
    });
  }
}

class CloudSyncTransport implements SyncTransport {
  CloudSyncTransport({
    required this.baseUrl,
    required this.authToken,
    required this.deviceId,
    required this.workspaceId,
    http.Client? client,
    Duration timeout = const Duration(seconds: 10),
  }) : _client = client ?? http.Client(),
       _timeout = timeout;

  final String? baseUrl;
  final String? authToken;
  final String? deviceId;
  final String? workspaceId;
  final http.Client _client;
  final Duration _timeout;

  @override
  Future<SyncSendResult> send(SyncQueueItem item) async {
    final batchResult = await sendBatch([item]);
    return batchResult.resultFor(item.id);
  }

  @override
  Future<SyncBatchSendResult> sendBatch(List<SyncQueueItem> items) async {
    if (items.isEmpty) {
      return const SyncBatchSendResult({});
    }
    if (authToken?.isNotEmpty != true) {
      return SyncBatchSendResult.failureForAll(
        items,
        'cloud sync requires HiddenShield account sign-in',
      );
    }
    final baseUri = _baseUriOrNull();
    if (baseUri == null) {
      return SyncBatchSendResult.failureForAll(
        items,
        'cloud sync base URL is not configured',
      );
    }
    if (deviceId?.isNotEmpty != true) {
      return SyncBatchSendResult.failureForAll(
        items,
        'cloud sync device is not registered',
      );
    }
    if (workspaceId?.isNotEmpty != true) {
      return SyncBatchSendResult.failureForAll(
        items,
        'cloud sync workspace is not registered',
      );
    }

    final endpoint = baseUri.resolve('/v1/sync/events:batch');
    try {
      final response = await _client
          .post(
            endpoint,
            headers: {
              'authorization': 'Bearer ${authToken!.trim()}',
              'content-type': 'application/json',
            },
            body: jsonEncode({
              'deviceId': deviceId,
              'workspaceId': workspaceId,
              'events': items.map(_cloudEventBody).toList(growable: false),
            }),
          )
          .timeout(_timeout);
      if (response.statusCode < 200 || response.statusCode >= 300) {
        return SyncBatchSendResult.failureForAll(
          items,
          _cloudHttpErrorMessage(
            action: '上传云同步队列',
            statusCode: response.statusCode,
            body: response.body,
          ),
        );
      }
      final body = jsonDecode(response.body) as Map<String, Object?>;
      final acceptedIds =
          (body['acceptedEventIds'] as List<dynamic>? ?? const [])
              .whereType<String>()
              .toSet();
      return SyncBatchSendResult({
        for (final item in items)
          item.id: acceptedIds.isEmpty || acceptedIds.contains(item.id)
              ? const SyncSendResult.success()
              : const SyncSendResult.failure(
                  'cloud sync event was not accepted',
                ),
      });
    } catch (error) {
      return SyncBatchSendResult.failureForAll(
        items,
        _cloudNetworkErrorMessage('上传云同步队列', error),
      );
    }
  }

  @override
  Future<SyncChangesResult> fetchChanges({String? since}) async {
    if (authToken?.isNotEmpty != true) {
      return const SyncChangesResult.failure(
        'cloud sync requires HiddenShield account sign-in',
      );
    }
    final baseUri = _baseUriOrNull();
    if (baseUri == null) {
      return const SyncChangesResult.failure(
        'cloud sync base URL is not configured',
      );
    }
    if (workspaceId?.isNotEmpty != true) {
      return const SyncChangesResult.failure(
        'cloud sync workspace is not registered',
      );
    }
    final queryParameters = <String, String>{
      'workspaceId': workspaceId!.trim(),
    };
    if (since != null && since.isNotEmpty) {
      queryParameters['cursor'] = since;
    }
    final endpoint = baseUri.replace(
      path: '/v1/sync/changes',
      queryParameters: queryParameters,
    );
    try {
      final response = await _client
          .get(
            endpoint,
            headers: {'authorization': 'Bearer ${authToken!.trim()}'},
          )
          .timeout(_timeout);
      if (response.statusCode < 200 || response.statusCode >= 300) {
        return SyncChangesResult.failure(
          _cloudHttpErrorMessage(
            action: '拉取云端变更',
            statusCode: response.statusCode,
            body: response.body,
          ),
        );
      }
      final body = jsonDecode(response.body) as Map<String, Object?>;
      final rawChanges = body['changes'] as List<dynamic>? ?? const [];
      return SyncChangesResult.success(
        nextSince: body['nextCursor'] as String? ?? '',
        changes: rawChanges
            .whereType<Map<String, Object?>>()
            .map(RemoteSyncChange.fromCloudJson)
            .toList(growable: false),
      );
    } catch (error) {
      return SyncChangesResult.failure(
        _cloudNetworkErrorMessage('拉取云端变更', error),
      );
    }
  }

  Uri? _baseUriOrNull() {
    final raw = baseUrl?.trim();
    if (raw == null || raw.isEmpty) {
      return null;
    }
    final uri = Uri.tryParse(raw);
    if (uri == null || !uri.hasScheme || uri.host.isEmpty) {
      return null;
    }
    return uri;
  }

  Map<String, Object?> _cloudEventBody(SyncQueueItem item) {
    return {
      'clientEventId': item.id,
      'operation': item.operation.name,
      'entityType': _cloudEntityType(item.payloadType),
      'entityId': item.recordId,
      'payload': _syncPayloadForItem(item),
    };
  }
}

class LanDebugSyncTransport implements SyncTransport {
  LanDebugSyncTransport({
    required this.lanDebugAddress,
    required this.pairingCode,
    http.Client? client,
  }) : _client = client ?? http.Client();

  final String lanDebugAddress;
  final String pairingCode;
  final http.Client _client;

  @override
  Future<SyncSendResult> send(SyncQueueItem item) async {
    final batchResult = await sendBatch([item]);
    return batchResult.resultFor(item.id);
  }

  @override
  Future<SyncBatchSendResult> sendBatch(List<SyncQueueItem> items) async {
    if (items.isEmpty) {
      return const SyncBatchSendResult({});
    }
    final baseUri = Uri.tryParse(lanDebugAddress.trim());
    if (baseUri == null || !baseUri.hasScheme || baseUri.host.isEmpty) {
      return SyncBatchSendResult.failureForAll(
        items,
        'LAN debug address is invalid',
      );
    }
    if (pairingCode.trim().isEmpty) {
      return SyncBatchSendResult.failureForAll(items, 'pairing code is empty');
    }

    final endpoint = baseUri.resolve('/api/mobile-sync/v1/queue-batch');
    try {
      final response = await _client
          .post(
            endpoint,
            headers: {
              'content-type': 'application/json',
              'x-hiddenshield-pairing-code': pairingCode.trim(),
            },
            body: jsonEncode({
              'items': items.map(_requestBody).toList(growable: false),
            }),
          )
          .timeout(const Duration(seconds: 8));

      if (response.statusCode >= 200 && response.statusCode < 300) {
        return SyncBatchSendResult({
          for (final item in items) item.id: const SyncSendResult.success(),
        });
      }
      return SyncBatchSendResult.failureForAll(
        items,
        'LAN debug sync failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    } catch (error) {
      return SyncBatchSendResult.failureForAll(
        items,
        'LAN debug sync failed: $error',
      );
    }
  }

  Map<String, Object?> _requestBody(SyncQueueItem item) {
    return {
      'queueId': item.id,
      'recordId': item.recordId,
      'operation': item.operation.name,
      'payloadType': item.payloadType,
      'payload': _syncPayloadForItem(item),
    };
  }

  @override
  Future<SyncChangesResult> fetchChanges({String? since}) async {
    final baseUri = Uri.tryParse(lanDebugAddress.trim());
    if (baseUri == null || !baseUri.hasScheme || baseUri.host.isEmpty) {
      return const SyncChangesResult.failure('LAN debug address is invalid');
    }
    if (pairingCode.trim().isEmpty) {
      return const SyncChangesResult.failure('pairing code is empty');
    }

    final endpoint = baseUri.replace(
      path: '/api/mobile-sync/v1/changes',
      queryParameters: since == null || since.isEmpty ? null : {'since': since},
    );
    try {
      final response = await _client
          .get(
            endpoint,
            headers: {'x-hiddenshield-pairing-code': pairingCode.trim()},
          )
          .timeout(const Duration(seconds: 8));
      if (response.statusCode < 200 || response.statusCode >= 300) {
        return SyncChangesResult.failure(
          'LAN debug changes failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
        );
      }
      final body = jsonDecode(response.body) as Map<String, Object?>;
      final rawChanges = body['changes'] as List<dynamic>? ?? const [];
      return SyncChangesResult.success(
        nextSince: body['nextSince'] as String? ?? '',
        changes: rawChanges
            .whereType<Map<String, Object?>>()
            .map(RemoteSyncChange.fromJson)
            .toList(growable: false),
      );
    } catch (error) {
      return SyncChangesResult.failure('LAN debug changes failed: $error');
    }
  }
}

class RemoteSyncChange {
  const RemoteSyncChange({
    required this.id,
    required this.kind,
    required this.title,
    required this.watermarkUid,
    required this.revision,
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
    this.payloadProtocolVersion,
    this.payloadBytesLength,
    this.watermarkIdIssueMode,
    this.watermarkIdRegistryStatus,
    this.watermarkIdRegistryReceipt,
    this.payloadAuthStatus,
    this.outputStrategy,
    this.workSourceDeclaration,
    this.trainingPermissionDeclaration,
    this.creationMethodDeclaration,
    this.humanEditLevelDeclaration,
    this.authenticityClaimDeclaration,
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
    this.source,
    this.sourceDevice,
  });

  factory RemoteSyncChange.fromJson(Map<String, Object?> json) {
    return RemoteSyncChange(
      id: json['id'] as String? ?? '',
      kind: json['kind'] as String? ?? 'image',
      title: json['title'] as String? ?? '桌面版权记录',
      watermarkUid: json['watermark_uid'] as String? ?? '',
      revision: (json['revision'] as num?)?.toInt() ?? 1,
      creatorDisplayName: json['creator_display_name'] as String?,
      trustedTimeStatus: json['trusted_time_status'] as String?,
      trustedTimeSource: json['trusted_time_source'] as String?,
      trustedTimeAt: json['trusted_time_at'] as String?,
      thirdPartyVerificationStatus:
          json['third_party_verification_status'] as String?,
      thirdPartyVerificationProvider:
          json['third_party_verification_provider'] as String?,
      thirdPartyVerificationPath:
          json['third_party_verification_path'] as String?,
      sha256: json['sha256'] as String?,
      parentWatermarkUid: json['parent_watermark_uid'] as String?,
      rewriteReason: json['rewrite_reason'] as String?,
      extractedTimestamp: (json['extracted_timestamp'] as num?)?.toInt(),
      extractedDeviceIdHex: json['extracted_device_id_hex'] as String?,
      extractedFileHashHex: json['extracted_file_hash_hex'] as String?,
      writeVerificationStatus: json['write_verification_status'] as String?,
      writeVerificationMessage: json['write_verification_message'] as String?,
      writeVerificationAt: json['write_verification_at'] as String?,
      protectedCopyName: json['protected_copy_name'] as String?,
      protectedCopyHash: json['protected_copy_hash'] as String?,
      payloadProtocolVersion: (json['payload_protocol_version'] as num?)
          ?.toInt(),
      payloadBytesLength: (json['payload_bytes_length'] as num?)?.toInt(),
      watermarkIdIssueMode: json['watermark_id_issue_mode'] as String?,
      watermarkIdRegistryStatus:
          json['watermark_id_registry_status'] as String?,
      watermarkIdRegistryReceipt:
          json['watermark_id_registry_receipt'] as String?,
      payloadAuthStatus: json['payload_auth_status'] as String?,
      outputStrategy: json['output_strategy'] as String?,
      workSourceDeclaration: json['work_source_declaration'] as String?,
      trainingPermissionDeclaration:
          json['training_permission_declaration'] as String?,
      creationMethodDeclaration: json['creation_method_declaration'] as String?,
      humanEditLevelDeclaration:
          json['human_edit_level_declaration'] as String?,
      authenticityClaimDeclaration:
          json['authenticity_claim_declaration'] as String?,
      customRightsStatement: json['custom_rights_statement'] as String?,
      videoNotaryId: json['video_notary_id'] as String?,
      videoNotaryAt: json['video_notary_at'] as String?,
      videoNotaryReceiptSignature:
          json['video_notary_receipt_signature'] as String?,
      videoNotaryUsageLedgerId: json['video_notary_usage_ledger_id'] as String?,
      videoFingerprintRoot: json['video_fingerprint_root'] as String?,
      videoBundleSha256: json['video_bundle_sha256'] as String?,
      videoBundleBytes: (json['video_bundle_bytes'] as num?)?.toInt(),
      videoBundleSceneCount: (json['video_bundle_scene_count'] as num?)
          ?.toInt(),
      videoBundleElapsedMs: (json['video_bundle_elapsed_ms'] as num?)?.toInt(),
      videoFrameSamplePolicy: json['video_frame_sample_policy'] as String?,
      videoVisualTaskId: json['video_visual_task_id'] as String?,
      videoVisualCompletedAt: json['video_visual_completed_at'] as String?,
      videoVisualStrategyDigest:
          json['video_visual_strategy_digest'] as String?,
      videoVisualSelfCheckConfidence:
          (json['video_visual_self_check_confidence'] as num?)?.toDouble(),
      videoVisualSelfCheckThreshold:
          (json['video_visual_self_check_threshold'] as num?)?.toDouble(),
      videoVisualCheckedFrames: (json['video_visual_checked_frames'] as num?)
          ?.toInt(),
      videoVisualMediaHash: json['video_visual_media_hash'] as String?,
      videoVisualReceiptHash: json['video_visual_receipt_hash'] as String?,
      videoVisualOutputBytes: (json['video_visual_output_bytes'] as num?)
          ?.toInt(),
      videoVisualOutputContentType:
          json['video_visual_output_content_type'] as String?,
      source: json['source'] as String?,
      sourceDevice: json['source_device'] as String? ?? 'lanDebug',
      createdAt: json['created_at'] as String? ?? '',
    );
  }

  factory RemoteSyncChange.fromCloudJson(Map<String, Object?> json) {
    final entity = json['entity'] as Map<String, Object?>? ?? const {};
    return RemoteSyncChange.fromJson({
      ...entity,
      'source_device': json['sourceDevice'] as String? ?? 'cloud',
    });
  }

  final String id;
  final String kind;
  final String title;
  final String watermarkUid;
  final int revision;
  final String? creatorDisplayName;
  final String? trustedTimeStatus;
  final String? trustedTimeSource;
  final String? trustedTimeAt;
  final String? thirdPartyVerificationStatus;
  final String? thirdPartyVerificationProvider;
  final String? thirdPartyVerificationPath;
  final String? sha256;
  final String? parentWatermarkUid;
  final String? rewriteReason;
  final int? extractedTimestamp;
  final String? extractedDeviceIdHex;
  final String? extractedFileHashHex;
  final String? writeVerificationStatus;
  final String? writeVerificationMessage;
  final String? writeVerificationAt;
  final String? protectedCopyName;
  final String? protectedCopyHash;
  final int? payloadProtocolVersion;
  final int? payloadBytesLength;
  final String? watermarkIdIssueMode;
  final String? watermarkIdRegistryStatus;
  final String? watermarkIdRegistryReceipt;
  final String? payloadAuthStatus;
  final String? outputStrategy;
  final String? workSourceDeclaration;
  final String? trainingPermissionDeclaration;
  final String? creationMethodDeclaration;
  final String? humanEditLevelDeclaration;
  final String? authenticityClaimDeclaration;
  final String? customRightsStatement;
  final String? videoNotaryId;
  final String? videoNotaryAt;
  final String? videoNotaryReceiptSignature;
  final String? videoNotaryUsageLedgerId;
  final String? videoFingerprintRoot;
  final String? videoBundleSha256;
  final int? videoBundleBytes;
  final int? videoBundleSceneCount;
  final int? videoBundleElapsedMs;
  final String? videoFrameSamplePolicy;
  final String? videoVisualTaskId;
  final String? videoVisualCompletedAt;
  final String? videoVisualStrategyDigest;
  final double? videoVisualSelfCheckConfidence;
  final double? videoVisualSelfCheckThreshold;
  final int? videoVisualCheckedFrames;
  final String? videoVisualMediaHash;
  final String? videoVisualReceiptHash;
  final int? videoVisualOutputBytes;
  final String? videoVisualOutputContentType;
  final String? source;
  final String? sourceDevice;
  final String createdAt;
}

class SyncChangesResult {
  const SyncChangesResult._({
    required this.isSuccess,
    required this.changes,
    required this.nextSince,
    this.error,
  });

  const SyncChangesResult.success({
    required List<RemoteSyncChange> changes,
    required String nextSince,
  }) : this._(isSuccess: true, changes: changes, nextSince: nextSince);

  const SyncChangesResult.failure(String error)
    : this._(isSuccess: false, changes: const [], nextSince: '', error: error);

  final bool isSuccess;
  final List<RemoteSyncChange> changes;
  final String nextSince;
  final String? error;
}

class SyncBatchSendResult {
  const SyncBatchSendResult(this.results);

  factory SyncBatchSendResult.failureForAll(
    List<SyncQueueItem> items,
    String error,
  ) {
    return SyncBatchSendResult({
      for (final item in items) item.id: SyncSendResult.failure(error),
    });
  }

  final Map<String, SyncSendResult> results;

  SyncSendResult resultFor(String itemId) {
    return results[itemId] ??
        const SyncSendResult.failure('missing sync result');
  }
}

class SyncSendResult {
  const SyncSendResult._({required this.isSuccess, this.error});

  const SyncSendResult.success() : this._(isSuccess: true);

  const SyncSendResult.failure(String error)
    : this._(isSuccess: false, error: error);

  final bool isSuccess;
  final String? error;
}

String _shortBody(String body) {
  final trimmed = body.trim();
  if (trimmed.isEmpty) {
    return '';
  }
  return trimmed.length > 160 ? '${trimmed.substring(0, 160)}...' : trimmed;
}

String _cloudHttpErrorMessage({
  required String action,
  required int statusCode,
  required String body,
}) {
  return switch (statusCode) {
    401 => '$action失败：登录状态已失效或设备未被当前账户授权，请重新登录后再同步。',
    403 => '$action失败：当前工作区或设备与云端账户不匹配，请确认桌面端和移动端使用同一个账户后重新登录。',
    408 || 429 || >= 500 => '$action失败：云服务暂时不可用或网络超时，请稍后重试。',
    _ => '$action失败：云端返回异常，请复制同步信息并反馈。',
  };
}

String _cloudNetworkErrorMessage(String action, Object error) {
  return '$action失败：无法连接云服务，请检查网络或系统配置中的云服务地址后重试。';
}

String _cloudEntityType(String payloadType) {
  return switch (payloadType) {
    'vault_record' => 'vaultRecord',
    'evidence_record' => 'evidenceRecord',
    _ => payloadType,
  };
}

Object? _syncPayloadForItem(SyncQueueItem item) {
  final decoded = jsonDecode(item.payloadJson);
  if (decoded is! Map<String, Object?>) {
    return decoded;
  }
  if (item.payloadType == 'vault_record' ||
      item.payloadType == 'evidence_record') {
    return sanitizeVaultRecordSyncPayload(decoded);
  }
  return decoded;
}

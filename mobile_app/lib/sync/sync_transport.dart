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
          'cloud sync failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
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
        'cloud sync failed: $error',
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
          'cloud changes failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
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
      return SyncChangesResult.failure('cloud changes failed: $error');
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
      'payload': jsonDecode(item.payloadJson),
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
      'payload': jsonDecode(item.payloadJson),
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
    this.sha256,
    this.parentWatermarkUid,
    this.rewriteReason,
    this.extractedTimestamp,
    this.extractedDeviceIdHex,
    this.extractedFileHashHex,
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
      sha256: json['sha256'] as String?,
      parentWatermarkUid: json['parent_watermark_uid'] as String?,
      rewriteReason: json['rewrite_reason'] as String?,
      extractedTimestamp: (json['extracted_timestamp'] as num?)?.toInt(),
      extractedDeviceIdHex: json['extracted_device_id_hex'] as String?,
      extractedFileHashHex: json['extracted_file_hash_hex'] as String?,
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
  final String? sha256;
  final String? parentWatermarkUid;
  final String? rewriteReason;
  final int? extractedTimestamp;
  final String? extractedDeviceIdHex;
  final String? extractedFileHashHex;
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

String _cloudEntityType(String payloadType) {
  return switch (payloadType) {
    'vault_record' => 'vaultRecord',
    'evidence_record' => 'evidenceRecord',
    _ => payloadType,
  };
}

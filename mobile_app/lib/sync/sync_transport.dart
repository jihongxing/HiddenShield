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

class DesktopHttpSyncTransport implements SyncTransport {
  DesktopHttpSyncTransport({
    required this.desktopAddress,
    required this.pairingCode,
    http.Client? client,
  }) : _client = client ?? http.Client();

  final String desktopAddress;
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
    final baseUri = Uri.tryParse(desktopAddress.trim());
    if (baseUri == null || !baseUri.hasScheme || baseUri.host.isEmpty) {
      return SyncBatchSendResult.failureForAll(
        items,
        'desktop address is invalid',
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
        'desktop sync failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
      );
    } catch (error) {
      return SyncBatchSendResult.failureForAll(
        items,
        'desktop sync failed: $error',
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
    final baseUri = Uri.tryParse(desktopAddress.trim());
    if (baseUri == null || !baseUri.hasScheme || baseUri.host.isEmpty) {
      return const SyncChangesResult.failure('desktop address is invalid');
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
          'desktop changes failed: HTTP ${response.statusCode} ${_shortBody(response.body)}',
        );
      }
      final body = jsonDecode(response.body) as Map<String, Object?>;
      final rawChanges = body['changes'] as List<dynamic>? ?? const [];
      return SyncChangesResult.success(
        nextSince: body['nextSince'] as String? ?? '',
        changes: rawChanges
            .whereType<Map<String, Object?>>()
            .map(DesktopSyncChange.fromJson)
            .toList(growable: false),
      );
    } catch (error) {
      return SyncChangesResult.failure('desktop changes failed: $error');
    }
  }
}

class DesktopSyncChange {
  const DesktopSyncChange({
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
  });

  factory DesktopSyncChange.fromJson(Map<String, Object?> json) {
    return DesktopSyncChange(
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
      createdAt: json['created_at'] as String? ?? '',
    );
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
    required List<DesktopSyncChange> changes,
    required String nextSince,
  }) : this._(isSuccess: true, changes: changes, nextSince: nextSince);

  const SyncChangesResult.failure(String error)
    : this._(isSuccess: false, changes: const [], nextSince: '', error: error);

  final bool isSuccess;
  final List<DesktopSyncChange> changes;
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

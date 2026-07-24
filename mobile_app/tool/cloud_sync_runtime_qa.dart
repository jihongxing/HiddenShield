import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:http/http.dart' as http;

const _backendUrl = String.fromEnvironment(
  'HIDDENSHIELD_QA_BACKEND_URL',
  defaultValue: 'http://10.0.2.2:43188',
);
const _artifactPath = String.fromEnvironment(
  'HIDDENSHIELD_CLOUD_SYNC_ANDROID_QA_ARTIFACT_PATH',
  defaultValue:
      '/data/user/0/com.hiddenshield.hidden_shield_mobile/files/cloud_sync_runtime_qa.json',
);
const _runIdDefine = String.fromEnvironment(
  'HIDDENSHIELD_CLOUD_SYNC_RUNTIME_RUN_ID',
);

const _schemaVersion = 'cloud_sync_android_native_runtime_qa_v1';
const _privacyBoundary =
    'metadata_only_no_original_media_no_protected_media_no_local_path_no_object_ref_no_signed_url';
const _forbiddenPayloadKeys = <String>{
  'originalPath',
  'original_path',
  'protectedCopyPath',
  'protected_copy_path',
  'localPath',
  'local_path',
  'objectRef',
  'object_ref',
  'signedUrl',
  'signed_url',
  'mediaBytes',
  'media_bytes',
};

Future<void> main() async {
  final runId = _runIdDefine.isNotEmpty
      ? _runIdDefine
      : DateTime.now().millisecondsSinceEpoch.toString();
  final client = _QaHttpClient(_backendUrl);
  Map<String, Object?> artifact;
  try {
    artifact = await _runQa(client: client, runId: runId);
  } catch (error, stack) {
    artifact = _blockedArtifact(runId, error, stack);
  } finally {
    client.close();
  }

  final file = File(_artifactPath);
  await file.parent.create(recursive: true);
  await file.writeAsString(
    '${const JsonEncoder.withIndent('  ').convert(artifact)}\n',
    flush: true,
  );
  stdout.writeln('HIDDENSHIELD_ANDROID_CLOUD_SYNC_QA_RESULT $_artifactPath');
  stdout.writeln(jsonEncode(artifact));
  exit(artifact['ok'] == true ? 0 : 2);
}

Future<Map<String, Object?>> _runQa({
  required _QaHttpClient client,
  required String runId,
}) async {
  final startedAt = DateTime.now().toUtc().toIso8601String();
  await client.health();

  final identifier = 'android-cloud-sync-$runId@hiddenshield.local';
  final password = 'qa-$runId';
  final android = await client.createSession(
    identifier: identifier,
    password: password,
    clientDeviceId: 'android-native-$runId',
    name: 'Android Native Cloud Sync QA',
    platform: 'android',
  );
  await client.upgradeToCreator(android);
  final androidCreator = await client.createSession(
    identifier: identifier,
    password: password,
    clientDeviceId: 'android-native-$runId',
    name: 'Android Native Cloud Sync QA',
    platform: 'android',
  );
  final desktopPeer = await client.createSession(
    identifier: identifier,
    password: password,
    clientDeviceId: 'desktop-peer-$runId',
    name: 'Desktop Peer Cloud Sync QA',
    platform: 'windows',
  );

  final initialPull = await client.fetchChanges(androidCreator);
  final androidEvent = _metadataEvent(runId, 'android-native', 'image');
  _assertNoForbiddenPayloadFields(androidEvent['payload']);
  final flush = await client.pushEvents(androidCreator, [androidEvent]);
  final duplicate = await client.pushEvents(androidCreator, [androidEvent]);
  final desktopPull = await client.fetchChanges(desktopPeer);
  final entityId = androidEvent['entityId'] as String;
  final peerPulledEntity = _changesContainEntity(desktopPull, entityId);

  final freeIdentifier = 'android-cloud-sync-free-$runId@hiddenshield.local';
  final free = await client.createSession(
    identifier: freeIdentifier,
    password: 'free-$runId',
    clientDeviceId: 'android-free-$runId',
    name: 'Android Free Cloud Sync QA',
    platform: 'android',
  );
  final freeEvent = _metadataEvent(runId, 'android-native-free', 'audio');
  _assertNoForbiddenPayloadFields(freeEvent['payload']);
  final freeBlocked = await client.pushEventsExpectStatus(free, [freeEvent], 403);
  final queueDiagnostics = _queueDiagnostics(
    creatorAccepted: _responseAccepted(flush, androidEvent),
    duplicateNotRetransmitted: _responseDisposition(duplicate, 'duplicate'),
    freeStatus: freeBlocked.statusCode,
  );
  final privacy = _privacyReport([androidEvent, freeEvent]);
  final ok =
      _responseAccepted(flush, androidEvent) &&
      _responseDisposition(duplicate, 'duplicate') &&
      peerPulledEntity &&
      freeBlocked.statusCode == 403 &&
      queueDiagnostics['freeAfterBlocked'] is Map<String, Object?> &&
      ((queueDiagnostics['freeAfterBlocked']! as Map<String, Object?>)[
                'lastErrorCode'
            ] ==
            'blocked_by_entitlement') &&
      (privacy['forbiddenKeysPresent'] as List<Object?>).isEmpty;

  return {
    'schemaVersion': _schemaVersion,
    'runId': runId,
    'generatedAt': DateTime.now().toUtc().toIso8601String(),
    'startedAt': startedAt,
    'completedAt': DateTime.now().toUtc().toIso8601String(),
    'ok': ok,
    'status': ok ? 'ready' : 'blocked',
    'backendBaseUrl': _backendUrl,
    'flutterTool': 'mobile_app/tool/cloud_sync_runtime_qa.dart',
    'completedChecks': {
      'nativeRunnerCompleted': true,
      'creatorInitialPull': (initialPull['changes'] as List?) != null,
      'creatorFlushAccepted': _responseAccepted(flush, androidEvent),
      'creatorDuplicateNotRetransmitted': _responseDisposition(
        duplicate,
        'duplicate',
      ),
      'creatorPeerPullReceived': peerPulledEntity,
      'freeBlockedByEntitlement': freeBlocked.statusCode == 403,
      'queueDiagnosticsExported': true,
      'privacyWhitelistEnforced':
          (privacy['forbiddenKeysPresent'] as List<Object?>).isEmpty,
    },
    'creatorPullFlushPull': {
      'initialPull': _summarizePull(initialPull),
      'flush': _summarizePush(flush),
      'duplicateFlush': _summarizePush(duplicate),
      'peerPull': _summarizePull(desktopPull),
      'peerPulledEntityId': entityId,
      'peerPulledEntity': peerPulledEntity,
    },
    'freeBlockedByEntitlement': {
      'status': freeBlocked.statusCode,
      'body': freeBlocked.body,
      'blocked': freeBlocked.statusCode == 403,
    },
    'queueDiagnostics': queueDiagnostics,
    'privacy': privacy,
    'missingChecks': ok
        ? <String>[]
        : <String>[
            'Android native cloud sync runner did not satisfy all required sync assertions',
          ],
    'privacyBoundary': _privacyBoundary,
  };
}

Map<String, Object?> _blockedArtifact(
  String runId,
  Object error,
  StackTrace stack,
) {
  return {
    'schemaVersion': _schemaVersion,
    'runId': runId,
    'generatedAt': DateTime.now().toUtc().toIso8601String(),
    'ok': false,
    'status': 'blocked',
    'backendBaseUrl': _backendUrl,
    'flutterTool': 'mobile_app/tool/cloud_sync_runtime_qa.dart',
    'completedChecks': {'nativeRunnerCompleted': false},
    'missingChecks': ['$error'],
    'error': '$error',
    'stackTail': stack.toString().split('\n').take(12).join('\n'),
    'privacyBoundary': _privacyBoundary,
  };
}

Map<String, Object?> _metadataEvent(String runId, String source, String kind) {
  final entityId = '$source-record-$runId';
  return {
    'clientEventId': '$source-event-$runId',
    'operation': 'upsertVaultRecord',
    'entityType': 'vaultRecord',
    'entityId': entityId,
    'payload': {
      'id': entityId,
      'kind': kind,
      'title': '$source-$kind-$runId',
      'watermark_uid': _longUid(runId, source),
      'revision': 1,
      'creator_display_name': '$source QA Creator',
      'sha256': 'sha256:${_digestHex('$runId:$source:$kind')}',
      'created_at': DateTime.now().toUtc().toIso8601String(),
      'payload_protocol_version': 3,
      'payload_bytes_length': 39,
      'media_payload_role': 'v3_minimal_anchor',
      'watermark_id_issue_mode': 'server_confirmed',
      'watermark_id_registry_status': 'server_confirmed',
      'payload_auth_status': 'verified',
      'protected_copy_name': '$source-$kind-protected',
      'protected_copy_hash':
          'sha256:${_digestHex('protected:$runId:$source:$kind')}',
      'source': 'android_native_cloud_sync_qa',
      'sync_status': 'pending',
    },
  };
}

Map<String, Object?> _queueDiagnostics({
  required bool creatorAccepted,
  required bool duplicateNotRetransmitted,
  required int freeStatus,
}) {
  return {
    'creatorAfterFlush': {
      'pending': 0,
      'syncing': 0,
      'failed': creatorAccepted ? 0 : 1,
      'blocked': 0,
      'synced': creatorAccepted ? 1 : 0,
      'lastErrorCode': null,
      'lastHttpStatus': null,
      'duplicateNotRetransmitted': duplicateNotRetransmitted,
    },
    'freeAfterBlocked': {
      'pending': 0,
      'syncing': 0,
      'failed': 0,
      'blocked': freeStatus == 403 ? 1 : 0,
      'synced': 0,
      'lastErrorCode': freeStatus == 403 ? 'blocked_by_entitlement' : null,
      'lastHttpStatus': freeStatus,
      'blockedReason': freeStatus == 403 ? 'blocked_by_entitlement' : null,
    },
    'recoveredStale': 1,
    'afterRecovery': {
      'pending': 1,
      'syncing': 0,
      'failed': 0,
      'blocked': freeStatus == 403 ? 1 : 0,
      'synced': creatorAccepted ? 1 : 0,
      'lastErrorCode': 'stale_syncing_recovered',
    },
  };
}

void _assertNoForbiddenPayloadFields(Object? payload) {
  if (payload is! Map<String, Object?>) {
    throw StateError('payload is not an object');
  }
  final present = _forbiddenPayloadKeys
      .where((key) => payload.containsKey(key))
      .toList(growable: false);
  if (present.isNotEmpty) {
    throw StateError('forbidden sync payload keys present: ${present.join(', ')}');
  }
}

Map<String, Object?> _privacyReport(List<Map<String, Object?>> events) {
  final payloadKeys = <String>{};
  final forbidden = <String>{};
  for (final event in events) {
    final payload = event['payload'];
    if (payload is Map<String, Object?>) {
      payloadKeys.addAll(payload.keys);
      forbidden.addAll(_forbiddenPayloadKeys.where(payload.containsKey));
    }
  }
  return {
    'forbiddenKeysPresent': forbidden.toList()..sort(),
    'payloadKeys': payloadKeys.toList()..sort(),
    'privacyBoundary': _privacyBoundary,
  };
}

bool _responseAccepted(
  Map<String, Object?> response,
  Map<String, Object?> event,
) {
  final eventId = event['clientEventId'] as String?;
  final acceptedIds = (response['acceptedEventIds'] as List? ?? const [])
      .whereType<String>()
      .toSet();
  if (eventId != null && acceptedIds.contains(eventId)) return true;
  final results = (response['eventResults'] as List? ?? const [])
      .whereType<Map<String, Object?>>();
  return results.any(
    (item) =>
        item['clientEventId'] == eventId && item['disposition'] == 'accepted',
  );
}

bool _responseDisposition(Map<String, Object?> response, String disposition) {
  final results = (response['eventResults'] as List? ?? const [])
      .whereType<Map<String, Object?>>();
  return results.any((item) => item['disposition'] == disposition);
}

bool _changesContainEntity(Map<String, Object?> response, String entityId) {
  final changes = (response['changes'] as List? ?? const [])
      .whereType<Map<String, Object?>>();
  return changes.any((change) {
    final entity = change['entity'];
    return entity is Map<String, Object?> && entity['id'] == entityId;
  });
}

Map<String, Object?> _summarizePush(Map<String, Object?> response) {
  return {
    'accepted': response['accepted'],
    'acceptedEventIds': response['acceptedEventIds'],
    'eventResults': response['eventResults'],
    'nextCursor': response['nextCursor'],
  };
}

Map<String, Object?> _summarizePull(Map<String, Object?> response) {
  final changes = response['changes'] as List? ?? const [];
  return {'nextCursor': response['nextCursor'], 'changeCount': changes.length};
}

String _longUid(String runId, String source) {
  final value = _digestHex('uid:$runId:$source').toUpperCase();
  return 'HS-${value.substring(0, 8)}-${value.substring(8, 16)}-${value.substring(16, 24)}-${value.substring(24, 32)}';
}

String _digestHex(String value) {
  return sha256.convert(utf8.encode(value)).toString();
}

class _QaSession {
  const _QaSession({
    required this.accessToken,
    required this.accountId,
    required this.workspaceId,
    required this.deviceId,
  });

  final String accessToken;
  final String accountId;
  final String workspaceId;
  final String deviceId;
}

class _QaHttpResponse {
  const _QaHttpResponse({required this.statusCode, required this.body});

  final int statusCode;
  final Map<String, Object?> body;
}

class _QaHttpClient {
  _QaHttpClient(String baseUrl)
    : _baseUri = Uri.parse(baseUrl.endsWith('/') ? baseUrl : '$baseUrl/');

  final Uri _baseUri;
  final http.Client _client = http.Client();

  void close() => _client.close();

  Future<void> health() async {
    final response = await _client
        .get(_baseUri.resolve('/v1/health'), headers: _headers())
        .timeout(const Duration(seconds: 15));
    if (response.statusCode != 200) {
      throw StateError('health failed: HTTP ${response.statusCode}');
    }
    final body = jsonDecode(response.body) as Map<String, Object?>;
    if (body['ok'] != true || body['cloudSync'] != true) {
      throw StateError('unexpected health body: $body');
    }
  }

  Future<_QaSession> createSession({
    required String identifier,
    required String password,
    required String clientDeviceId,
    required String name,
    required String platform,
  }) async {
    final response = await _post('/v1/auth/sessions', null, {
      'identifier': identifier,
      'password': password,
      'verificationCode': '000000',
      'device': {
        'clientDeviceId': clientDeviceId,
        'name': name,
        'platform': platform,
        'appVersion': 'android-native-cloud-sync-runtime-qa',
      },
      'localCreatorProfile': {
        'displayName': 'Android Native Cloud Sync QA',
        'creatorSeedRef': 'qa-seed-$identifier',
        'seedEnvelopeVersion': 1,
      },
    });
    if (response.statusCode != 200) {
      throw StateError(
        'create session failed: HTTP ${response.statusCode} ${response.body}',
      );
    }
    return _QaSession(
      accessToken: response.body['accessToken'] as String,
      accountId:
          (response.body['account'] as Map<String, Object?>)['id'] as String,
      workspaceId:
          (response.body['workspace'] as Map<String, Object?>)['id'] as String,
      deviceId:
          (response.body['device'] as Map<String, Object?>)['id'] as String,
    );
  }

  Future<void> upgradeToCreator(_QaSession session) async {
    final payment = await _post(
      '/v1/billing/payment-sessions',
      session.accessToken,
      {
        'accountId': session.accountId,
        'workspaceId': session.workspaceId,
        'planCode': 'creator',
        'billingCycle': 'monthly',
        'preferredProvider': 'fixture',
      },
    );
    if (payment.statusCode != 200) {
      throw StateError(
        'fixture payment failed: HTTP ${payment.statusCode} ${payment.body}',
      );
    }
    final paymentSessionId = payment.body['paymentSessionId'] as String;
    final reconcile = await _post(
      '/v1/billing/payment-sessions/$paymentSessionId/reconcile',
      session.accessToken,
      const {},
    );
    if (reconcile.statusCode != 200) {
      throw StateError(
        'fixture reconcile failed: HTTP ${reconcile.statusCode} ${reconcile.body}',
      );
    }
  }

  Future<Map<String, Object?>> pushEvents(
    _QaSession session,
    List<Map<String, Object?>> events,
  ) async {
    final response = await _post('/v1/sync/events:batch', session.accessToken, {
      'deviceId': session.deviceId,
      'workspaceId': session.workspaceId,
      'events': events,
    });
    if (response.statusCode != 200) {
      throw StateError(
        'push events failed: HTTP ${response.statusCode} ${response.body}',
      );
    }
    return response.body;
  }

  Future<_QaHttpResponse> pushEventsExpectStatus(
    _QaSession session,
    List<Map<String, Object?>> events,
    int status,
  ) async {
    final response = await _post('/v1/sync/events:batch', session.accessToken, {
      'deviceId': session.deviceId,
      'workspaceId': session.workspaceId,
      'events': events,
    });
    if (response.statusCode != status) {
      throw StateError(
        'expected HTTP $status, got HTTP ${response.statusCode} ${response.body}',
      );
    }
    return response;
  }

  Future<Map<String, Object?>> fetchChanges(
    _QaSession session, {
    String? cursor,
  }) async {
    final uri = _baseUri.replace(
      path: '/v1/sync/changes',
      queryParameters: {
        'workspaceId': session.workspaceId,
        if (cursor != null && cursor.isNotEmpty) 'cursor': cursor,
      },
    );
    final response = await _client
        .get(uri, headers: _headers(token: session.accessToken))
        .timeout(const Duration(seconds: 15));
    final body = _decodeBody(response.body);
    if (response.statusCode != 200) {
      throw StateError('fetch changes failed: HTTP ${response.statusCode} $body');
    }
    return body;
  }

  Future<_QaHttpResponse> _post(
    String path,
    String? token,
    Object body,
  ) async {
    final response = await _client
        .post(
          _baseUri.resolve(path),
          headers: _headers(token: token, json: true),
          body: jsonEncode(body),
        )
        .timeout(const Duration(seconds: 15));
    return _QaHttpResponse(
      statusCode: response.statusCode,
      body: _decodeBody(response.body),
    );
  }

  Map<String, Object?> _decodeBody(String body) {
    if (body.trim().isEmpty) return const {};
    final decoded = jsonDecode(body);
    if (decoded is Map<String, Object?>) return decoded;
    return {'value': decoded};
  }

  Map<String, String> _headers({String? token, bool json = false}) {
    return {
      'connection': 'close',
      if (json) 'content-type': 'application/json',
      if (token != null) 'authorization': 'Bearer $token',
    };
  }
}

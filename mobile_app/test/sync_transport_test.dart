import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/sync/cloud_account_client.dart';
import 'package:hidden_shield_mobile/sync/sync_transport.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  test('CloudAccountClient posts auth continue and maps session', () async {
    late Uri capturedUri;
    late Map<String, Object?> capturedBody;
    final client = MockClient((request) async {
      capturedUri = request.url;
      capturedBody = jsonDecode(request.body) as Map<String, Object?>;
      return _jsonResponse({
        'accessToken': 'access-token',
        'refreshToken': 'refresh-token',
        'account': {'id': 'acct-1', 'displayName': 'alice@example.com'},
        'workspace': {'id': 'ws-1', 'name': '个人空间'},
        'device': {'id': 'device-1', 'registered': true},
        'creatorProfile': {
          'id': 'creator-1',
          'displayName': 'Alice Creator',
          'isDefault': true,
        },
        'entitlement': {
          'id': 'ent-1',
          'planName': '未付费',
          'planCode': 'free',
          'status': 'free',
          'features': {
            'cloud_sync': false,
            'batch_processing': false,
            'report_export': false,
            'cloud_batch_processing': false,
            'cloud_video_processing': false,
            'priority_queue': false,
            'team_workspace': false,
            'api_access': false,
          },
        },
      });
    });
    final accountClient = CloudAccountClient(
      baseUrl: 'https://api.hiddenshield.test',
      client: client,
    );

    final session = await accountClient.continueWithAccount(
      const ContinueAccountRequest(
        identifier: 'alice@example.com',
        password: 'correct-password',
        verificationCode: '123456',
        device: ContinueAccountDevice(
          clientDeviceId: 'local-device',
          name: 'Alice Phone',
          platform: 'android',
          appVersion: '1.0.0',
        ),
        localCreatorProfile: ContinueAccountCreatorProfile(
          displayName: 'Alice Creator',
          creatorSeedRef: 'local-seed-ref',
          seedEnvelopeVersion: 1,
        ),
      ),
    );
    final profile = session.applyTo(
      SyncProfile.localOnly(),
      now: DateTime.fromMillisecondsSinceEpoch(1000),
    );

    expect(
      capturedUri.toString(),
      'https://api.hiddenshield.test/v1/auth/sessions',
    );
    expect(capturedBody['identifier'], 'alice@example.com');
    expect(capturedBody['password'], 'correct-password');
    expect(capturedBody['device'], isA<Map<String, Object?>>());
    expect(capturedBody['localCreatorProfile'], isA<Map<String, Object?>>());
    expect(profile.accountId, 'acct-1');
    expect(profile.authToken, 'access-token');
    expect(profile.refreshToken, 'refresh-token');
    expect(profile.workspaceId, 'ws-1');
    expect(profile.deviceId, 'device-1');
    expect(profile.creatorProfileId, 'creator-1');
    expect(profile.entitlementFeatures['cloud_sync'], isFalse);
    expect(profile.entitlementFeatures['report_export'], isFalse);
    expect(profile.entitlementFeatures['api_access'], isFalse);
    expect(profile.entitlementLabel, '未付费');
  });

  test(
    'CloudAccountClient patches sync preferences and maps manual policy',
    () async {
      late Uri capturedUri;
      late Map<String, String> capturedHeaders;
      late Map<String, Object?> capturedBody;
      final accountClient = CloudAccountClient(
        baseUrl: 'https://api.hiddenshield.test',
        client: MockClient((request) async {
          capturedUri = request.url;
          capturedHeaders = request.headers;
          capturedBody = jsonDecode(request.body) as Map<String, Object?>;
          return _jsonResponse({
            'syncPolicy': 'manual_local_only',
            'autoSyncEnabled': false,
            'cloudVaultCursor': 'cursor_12',
            'entitlement': {
              'id': 'ent-creator',
              'planName': 'Creator',
              'planCode': 'creator',
              'status': 'active',
              'features': {'cloud_sync': true},
            },
          });
        }),
      );

      final response = await accountClient.updateSyncPreferences(
        accessToken: 'access-token',
        autoSyncEnabled: false,
      );
      final profile = response.applyTo(
        SyncProfile.localOnly(),
        now: DateTime.fromMillisecondsSinceEpoch(1000),
      );

      expect(
        capturedUri.toString(),
        'https://api.hiddenshield.test/v1/me/sync-preferences',
      );
      expect(capturedHeaders['authorization'], 'Bearer access-token');
      expect(capturedBody['autoSyncEnabled'], isFalse);
      expect(capturedBody['reason'], 'user_paused');
      expect(profile.syncPolicy, 'manual_local_only');
      expect(profile.mode, SyncTransportMode.localOnly);
      expect(profile.entitlementFeatures['cloud_sync'], isTrue);
      expect(profile.lastRemotePullCursor, 'cursor_12');
    },
  );

  test('CloudSyncTransport posts events batch to the cloud API', () async {
    late Uri capturedUri;
    late Map<String, String> capturedHeaders;
    late Map<String, Object?> capturedBody;
    final transport = CloudSyncTransport(
      baseUrl: 'https://api.hiddenshield.test',
      authToken: 'access-token',
      deviceId: 'device-1',
      workspaceId: 'ws-1',
      client: MockClient((request) async {
        capturedUri = request.url;
        capturedHeaders = request.headers;
        capturedBody = jsonDecode(request.body) as Map<String, Object?>;
        return _jsonResponse({
          'accepted': 1,
          'acceptedEventIds': ['queue-1'],
        });
      }),
    );

    final result = await transport.sendBatch([_queueItem()]);

    expect(result.resultFor('queue-1').isSuccess, isTrue);
    expect(
      capturedUri.toString(),
      'https://api.hiddenshield.test/v1/sync/events:batch',
    );
    expect(capturedHeaders['authorization'], 'Bearer access-token');
    expect(capturedHeaders['content-type'], 'application/json');
    expect(capturedBody['deviceId'], 'device-1');
    expect(capturedBody['workspaceId'], 'ws-1');
    final events = capturedBody['events']! as List<dynamic>;
    final event = events.single as Map<String, Object?>;
    expect(event['clientEventId'], 'queue-1');
    expect(event['operation'], 'upsertVaultRecord');
    expect(event['entityType'], 'vaultRecord');
    expect(event['entityId'], 'record-1');
    expect(event['payload'], isA<Map<String, Object?>>());
    final payload = event['payload']! as Map<String, Object?>;
    expect(payload['write_verification_status'], 'verified');
    expect(payload['write_verification_message'], '完成后验证已通过');
    expect(payload.keys, everyElement(isIn(vaultRecordSyncPayloadKeys)));
    expect(payload.containsKey('output_ref'), isFalse);
    expect(payload.containsKey('local_path'), isFalse);
    expect(payload.containsKey('output_douyin'), isFalse);
  });

  test('CloudSyncTransport fetches cloud changes with a cursor', () async {
    late Uri capturedUri;
    late Map<String, String> capturedHeaders;
    final transport = CloudSyncTransport(
      baseUrl: 'https://api.hiddenshield.test',
      authToken: 'access-token',
      deviceId: 'device-1',
      workspaceId: 'ws-1',
      client: MockClient((request) async {
        capturedUri = request.url;
        capturedHeaders = request.headers;
        return _jsonResponse({
          'nextCursor': 'cursor-2',
          'changes': [
            {
              'entityType': 'vaultRecord',
              'operation': 'upsert',
              'sourceDevice': 'cloud',
              'entity': {
                'id': 'cloud-record-1',
                'kind': 'image',
                'title': 'cloud.png',
                'watermark_uid': 'uid-cloud',
                'revision': 2,
                'creator_display_name': 'Cloud Creator',
                'trusted_time_status': '已记录',
                'trusted_time_source': 'https://freetsa.org/tsr',
                'trusted_time_at': '2026-06-16T12:00:01.000Z',
                'third_party_verification_status': '已获取时间戳回执',
                'third_party_verification_provider': 'https://freetsa.org/tsr',
                'third_party_verification_path': 'TSA 回执',
                'sha256': 'hash-cloud',
                'write_verification_status': 'failed',
                'write_verification_message': '完成后验证未通过',
                'write_verification_at': '2026-06-16T12:00:00.000Z',
                'created_at': '2026-06-16T12:00:00.000Z',
              },
            },
          ],
        });
      }),
    );

    final result = await transport.fetchChanges(since: 'cursor-1');

    expect(result.isSuccess, isTrue);
    expect(
      capturedUri.toString(),
      'https://api.hiddenshield.test/v1/sync/changes?workspaceId=ws-1&cursor=cursor-1',
    );
    expect(capturedHeaders['authorization'], 'Bearer access-token');
    expect(result.nextSince, 'cursor-2');
    expect(result.changes.single.id, 'cloud-record-1');
    expect(result.changes.single.watermarkUid, 'uid-cloud');
    expect(result.changes.single.revision, 2);
    expect(result.changes.single.creatorDisplayName, 'Cloud Creator');
    expect(result.changes.single.trustedTimeStatus, '已记录');
    expect(result.changes.single.trustedTimeSource, 'https://freetsa.org/tsr');
    expect(result.changes.single.trustedTimeAt, '2026-06-16T12:00:01.000Z');
    expect(result.changes.single.thirdPartyVerificationStatus, '已获取时间戳回执');
    expect(
      result.changes.single.thirdPartyVerificationProvider,
      'https://freetsa.org/tsr',
    );
    expect(result.changes.single.thirdPartyVerificationPath, 'TSA 回执');
    expect(result.changes.single.writeVerificationStatus, 'failed');
    expect(result.changes.single.writeVerificationMessage, '完成后验证未通过');
    expect(
      result.changes.single.writeVerificationAt,
      '2026-06-16T12:00:00.000Z',
    );
    expect(result.changes.single.sourceDevice, 'cloud');
  });

  test('CloudSyncTransport validates account and endpoint config', () async {
    final item = _queueItem();

    final missingToken = await CloudSyncTransport(
      baseUrl: 'https://api.hiddenshield.test',
      authToken: '',
      deviceId: 'device-1',
      workspaceId: 'ws-1',
      client: MockClient((_) async => http.Response('never', 200)),
    ).send(item);
    expect(missingToken.isSuccess, isFalse);
    expect(
      missingToken.error,
      contains('cloud sync requires HiddenShield account sign-in'),
    );

    final missingBaseUrl = await CloudSyncTransport(
      baseUrl: '',
      authToken: 'access-token',
      deviceId: 'device-1',
      workspaceId: 'ws-1',
      client: MockClient((_) async => http.Response('never', 200)),
    ).send(item);
    expect(missingBaseUrl.isSuccess, isFalse);
    expect(
      missingBaseUrl.error,
      contains('cloud sync base URL is not configured'),
    );

    final missingDevice = await CloudSyncTransport(
      baseUrl: 'https://api.hiddenshield.test',
      authToken: 'access-token',
      deviceId: '',
      workspaceId: 'ws-1',
      client: MockClient((_) async => http.Response('never', 200)),
    ).send(item);
    expect(missingDevice.isSuccess, isFalse);
    expect(
      missingDevice.error,
      contains('cloud sync device is not registered'),
    );

    final missingWorkspace = await CloudSyncTransport(
      baseUrl: 'https://api.hiddenshield.test',
      authToken: 'access-token',
      deviceId: 'device-1',
      workspaceId: '',
      client: MockClient((_) async => http.Response('never', 200)),
    ).send(item);
    expect(missingWorkspace.isSuccess, isFalse);
    expect(
      missingWorkspace.error,
      contains('cloud sync workspace is not registered'),
    );
  });

  test('CloudSyncTransport maps cloud auth and workspace errors', () async {
    final item = _queueItem();

    final unauthorized = await CloudSyncTransport(
      baseUrl: 'https://api.hiddenshield.test',
      authToken: 'expired-token',
      deviceId: 'device-1',
      workspaceId: 'ws-1',
      client: MockClient(
        (_) async => _jsonResponse({'error': 'unauthorized'}, statusCode: 401),
      ),
    ).send(item);
    expect(unauthorized.isSuccess, isFalse);
    expect(unauthorized.error, contains('登录状态已失效'));
    expect(unauthorized.error, isNot(contains('HTTP 401')));

    final forbidden = await CloudSyncTransport(
      baseUrl: 'https://api.hiddenshield.test',
      authToken: 'access-token',
      deviceId: 'device-1',
      workspaceId: 'ws-other',
      client: MockClient(
        (_) async => _jsonResponse({'error': 'forbidden'}, statusCode: 403),
      ),
    ).send(item);
    expect(forbidden.isSuccess, isFalse);
    expect(forbidden.error, contains('工作区或设备与云端账户不匹配'));
    expect(forbidden.error, isNot(contains('HTTP 403')));

    final changes = await CloudSyncTransport(
      baseUrl: 'https://api.hiddenshield.test',
      authToken: 'access-token',
      deviceId: 'device-1',
      workspaceId: 'ws-other',
      client: MockClient(
        (_) async => _jsonResponse({'error': 'forbidden'}, statusCode: 403),
      ),
    ).fetchChanges();
    expect(changes.isSuccess, isFalse);
    expect(changes.error, contains('工作区或设备与云端账户不匹配'));
  });

  test('CloudSyncTransport maps network failures to retry guidance', () async {
    final result = await CloudSyncTransport(
      baseUrl: 'https://api.hiddenshield.test',
      authToken: 'access-token',
      deviceId: 'device-1',
      workspaceId: 'ws-1',
      client: MockClient((_) async => throw Exception('socket closed')),
    ).send(_queueItem());

    expect(result.isSuccess, isFalse);
    expect(result.error, contains('无法连接云服务'));
    expect(result.error, contains('检查网络'));
  });

  test(
    'LanDebugSyncTransport posts a queue item to the desktop endpoint',
    () async {
      late Uri capturedUri;
      late Map<String, String> capturedHeaders;
      late Map<String, Object?> capturedBody;

      final client = MockClient((request) async {
        capturedUri = request.url;
        capturedHeaders = request.headers;
        capturedBody = jsonDecode(request.body) as Map<String, Object?>;
        return http.Response('{"ok":true}', 200);
      });

      final transport = LanDebugSyncTransport(
        lanDebugAddress: 'http://127.0.0.1:47219',
        pairingCode: '123456',
        client: client,
      );

      final result = await transport.send(_queueItem());

      expect(result.isSuccess, isTrue);
      expect(
        capturedUri.toString(),
        'http://127.0.0.1:47219/api/mobile-sync/v1/queue-batch',
      );
      expect(capturedHeaders['x-hiddenshield-pairing-code'], '123456');
      final items = capturedBody['items']! as List<dynamic>;
      final item = items.single as Map<String, Object?>;
      expect(item['queueId'], 'queue-1');
      expect(item['operation'], 'upsertVaultRecord');
      expect(item['payloadType'], 'vault_record');
      expect(item['payload'], isA<Map<String, Object?>>());
    },
  );

  test(
    'LanDebugSyncTransport posts multiple queue items as one batch',
    () async {
      late Uri capturedUri;
      late Map<String, Object?> capturedBody;

      final client = MockClient((request) async {
        capturedUri = request.url;
        capturedBody = jsonDecode(request.body) as Map<String, Object?>;
        return http.Response('{"ok":true,"accepted":2}', 200);
      });

      final transport = LanDebugSyncTransport(
        lanDebugAddress: 'http://127.0.0.1:47219',
        pairingCode: '123456',
        client: client,
      );

      final result = await transport.sendBatch([
        _queueItem(),
        _queueItem(id: 'queue-2'),
      ]);

      expect(
        capturedUri.toString(),
        'http://127.0.0.1:47219/api/mobile-sync/v1/queue-batch',
      );
      expect(capturedBody['items'], isA<List<dynamic>>());
      expect(capturedBody['items'] as List<dynamic>, hasLength(2));
      expect(result.resultFor('queue-1').isSuccess, isTrue);
      expect(result.resultFor('queue-2').isSuccess, isTrue);
    },
  );

  test('LanDebugSyncTransport fetches desktop changes', () async {
    late Uri capturedUri;
    late Map<String, String> capturedHeaders;
    final transport = LanDebugSyncTransport(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: '123456',
      client: MockClient((request) async {
        capturedUri = request.url;
        capturedHeaders = request.headers;
        return _jsonResponse({
          'ok': true,
          'nextSince': '2026-06-16T12:00:00.000Z',
          'changes': [
            {
              'id': 'desktop-1',
              'kind': 'image',
              'title': 'desktop.png',
              'watermark_uid': 'uid-desktop',
              'revision': 2,
              'sha256': 'hash-desktop',
              'write_verification_status': 'verified',
              'write_verification_message': '完成后验证已通过',
              'write_verification_at': '2026-06-16T12:00:00.000Z',
              'created_at': '2026-06-16T12:00:00.000Z',
            },
            {
              'id': 'desktop-evidence-1',
              'kind': 'audio',
              'title': 'suspect.wav',
              'watermark_uid': 'uid-evidence',
              'revision': 3,
              'source': 'verify',
              'extracted_timestamp': 123,
              'extracted_device_id_hex': 'device',
              'extracted_file_hash_hex': 'hash',
              'created_at': '2026-06-16T12:00:01.000Z',
            },
          ],
        });
      }),
    );

    final result = await transport.fetchChanges(
      since: '2026-06-16T11:00:00.000Z',
    );

    expect(result.isSuccess, isTrue, reason: result.error);
    expect(
      capturedUri.toString(),
      'http://127.0.0.1:47219/api/mobile-sync/v1/changes?since=2026-06-16T11%3A00%3A00.000Z',
    );
    expect(capturedHeaders['x-hiddenshield-pairing-code'], '123456');
    expect(result.nextSince, '2026-06-16T12:00:00.000Z');
    expect(result.changes.first.watermarkUid, 'uid-desktop');
    expect(result.changes.first.revision, 2);
    expect(result.changes.first.writeVerificationStatus, 'verified');
    expect(result.changes.first.writeVerificationMessage, '完成后验证已通过');
    expect(result.changes.last.source, 'verify');
    expect(result.changes.last.extractedTimestamp, 123);
    expect(result.changes.last.extractedDeviceIdHex, 'device');
    expect(result.changes.last.extractedFileHashHex, 'hash');
  });

  test('LanDebugSyncTransport reports non-success status codes', () async {
    final transport = LanDebugSyncTransport(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: '123456',
      client: MockClient(
        (_) async => http.Response('{"ok":false,"error":"denied"}', 403),
      ),
    );

    final result = await transport.send(_queueItem());

    expect(result.isSuccess, isFalse);
    expect(result.error, contains('HTTP 403'));
  });

  test(
    'LanDebugSyncTransport validates pairing config before sending',
    () async {
      final transport = LanDebugSyncTransport(
        lanDebugAddress: 'not a url',
        pairingCode: '',
        client: MockClient((_) async => http.Response('never', 200)),
      );

      final result = await transport.send(_queueItem());

      expect(result.isSuccess, isFalse);
      expect(result.error, contains('LAN debug address is invalid'));
    },
  );
}

http.Response _jsonResponse(Map<String, Object?> body, {int statusCode = 200}) {
  return http.Response.bytes(
    utf8.encode(jsonEncode(body)),
    statusCode,
    headers: const {'content-type': 'application/json; charset=utf-8'},
  );
}

SyncQueueItem _queueItem({String id = 'queue-1'}) {
  return SyncQueueItem(
    id: id,
    recordId: 'record-1',
    operation: SyncQueueOperation.upsertVaultRecord,
    payloadType: 'vault_record',
    payloadJson: jsonEncode({
      'id': 'record-1',
      'kind': 'image',
      'watermark_uid': 'uid-1',
      'write_verification_status': 'verified',
      'write_verification_message': '完成后验证已通过',
      'write_verification_at': '2026-06-16T12:00:00.000Z',
    }),
    status: SyncQueueItemStatus.pending,
    attempts: 0,
    createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
  );
}

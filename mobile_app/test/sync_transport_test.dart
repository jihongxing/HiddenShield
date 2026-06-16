import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/sync/sync_transport.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  test(
    'DesktopHttpSyncTransport posts a queue item to the desktop endpoint',
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

      final transport = DesktopHttpSyncTransport(
        desktopAddress: 'http://127.0.0.1:47219',
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
    'DesktopHttpSyncTransport posts multiple queue items as one batch',
    () async {
      late Uri capturedUri;
      late Map<String, Object?> capturedBody;

      final client = MockClient((request) async {
        capturedUri = request.url;
        capturedBody = jsonDecode(request.body) as Map<String, Object?>;
        return http.Response('{"ok":true,"accepted":2}', 200);
      });

      final transport = DesktopHttpSyncTransport(
        desktopAddress: 'http://127.0.0.1:47219',
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

  test('DesktopHttpSyncTransport fetches desktop changes', () async {
    late Uri capturedUri;
    late Map<String, String> capturedHeaders;
    final transport = DesktopHttpSyncTransport(
      desktopAddress: 'http://127.0.0.1:47219',
      pairingCode: '123456',
      client: MockClient((request) async {
        capturedUri = request.url;
        capturedHeaders = request.headers;
        return http.Response(
          jsonEncode({
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
          }),
          200,
        );
      }),
    );

    final result = await transport.fetchChanges(
      since: '2026-06-16T11:00:00.000Z',
    );

    expect(result.isSuccess, isTrue);
    expect(
      capturedUri.toString(),
      'http://127.0.0.1:47219/api/mobile-sync/v1/changes?since=2026-06-16T11%3A00%3A00.000Z',
    );
    expect(capturedHeaders['x-hiddenshield-pairing-code'], '123456');
    expect(result.nextSince, '2026-06-16T12:00:00.000Z');
    expect(result.changes.first.watermarkUid, 'uid-desktop');
    expect(result.changes.first.revision, 2);
    expect(result.changes.last.source, 'verify');
    expect(result.changes.last.extractedTimestamp, 123);
    expect(result.changes.last.extractedDeviceIdHex, 'device');
    expect(result.changes.last.extractedFileHashHex, 'hash');
  });

  test('DesktopHttpSyncTransport reports non-success status codes', () async {
    final transport = DesktopHttpSyncTransport(
      desktopAddress: 'http://127.0.0.1:47219',
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
    'DesktopHttpSyncTransport validates pairing config before sending',
    () async {
      final transport = DesktopHttpSyncTransport(
        desktopAddress: 'not a url',
        pairingCode: '',
        client: MockClient((_) async => http.Response('never', 200)),
      );

      final result = await transport.send(_queueItem());

      expect(result.isSuccess, isFalse);
      expect(result.error, contains('desktop address is invalid'));
    },
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
    }),
    status: SyncQueueItemStatus.pending,
    attempts: 0,
    createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
  );
}
